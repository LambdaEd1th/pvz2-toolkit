use anyhow::{Context, Result, anyhow};
use image::RgbaImage;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::types::{PamInfo, SpriteInfo};

#[derive(Clone, Debug, PartialEq)]
pub enum RenderFormat {
    PngSequence,
    Gif,
}

pub struct AnimationHelperSetting {
    pub frame_name: String,
    pub image_by_path: bool,
    pub append_width: i32,
    pub append_height: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub disable_sprite: Vec<i32>,
    pub format: RenderFormat,
}

impl Default for AnimationHelperSetting {
    fn default() -> Self {
        Self {
            frame_name: "frame".to_string(),
            image_by_path: false,
            append_width: 0,
            append_height: 0,
            pos_x: 0,
            pos_y: 0,
            disable_sprite: Vec::new(),
            format: RenderFormat::PngSequence,
        }
    }
}

#[derive(Clone, Debug)]
struct ImageSequenceList {
    pub image_width: u32,
    pub image_height: u32,
    pub matrix: [f64; 6],

    pub image_index: usize,
    pub disable_sprite: bool,
    pub transform: Vec<[f64; 6]>,
    pub color: Vec<[f64; 4]>,
}

pub fn render_animation(
    pam: &PamInfo,
    out_folder: &Path,
    media_path: &Path,
    setting: &AnimationHelperSetting,
) -> Result<BTreeMap<String, [u32; 2]>> {
    if !out_folder.exists() {
        std::fs::create_dir_all(out_folder)?;
    }

    let mut image_sequence_list: HashMap<usize, Vec<ImageSequenceList>> = HashMap::new();
    let mut image_list: Vec<RgbaImage> = Vec::with_capacity(pam.image.len());

    for (i, image_info) in pam.image.iter().enumerate() {
        let name_parts: Vec<&str> = image_info.name.split('|').collect();
        let image_name = if setting.image_by_path {
            name_parts[0].to_string()
        } else {
            name_parts.get(1).unwrap_or(&name_parts[0]).to_string()
        };

        let width = image_info.size.first().copied().unwrap_or(-1);
        let height = image_info.size.get(1).copied().unwrap_or(-1);

        let t = &image_info.transform;
        let transform = if t.len() >= 6 {
            [t[0], t[1], t[2], t[3], t[4], t[5]]
        } else if t.len() >= 2 {
            [1.0, 0.0, 0.0, 1.0, t[0], t[1]]
        } else {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        };

        let seq = ImageSequenceList {
            image_width: width.max(0) as u32,
            image_height: height.max(0) as u32,
            matrix: transform,
            image_index: i,
            disable_sprite: false,
            transform: Vec::new(),
            color: Vec::new(),
        };

        let img_path = media_path.join(format!("{}.png", image_name));
        let mut img = image::open(&img_path)
            .with_context(|| format!("Failed to open image: {}", img_path.display()))?
            .into_rgba8();

        if seq.image_width > 0
            && seq.image_height > 0
            && (img.width() != seq.image_width || img.height() != seq.image_height)
        {
            img = image::imageops::resize(
                &img,
                seq.image_width,
                seq.image_height,
                image::imageops::FilterType::Triangle,
            );
        } else {
            // Update sequence list width/height to actual image size if it was not provided correctly in PAM
            // Though PAM says what it should be.
        }

        image_sequence_list.insert(i, vec![seq]);
        image_list.push(img);
    }

    let mut sprite_list: HashMap<usize, Vec<ImageSequenceList>> = HashMap::new();
    let mut main_sprite_frame: HashMap<i32, Vec<ImageSequenceList>> = HashMap::new();

    for (i, sprite_info) in pam.sprite.iter().enumerate() {
        let sprite_image_list = read_sprite(
            i as i32,
            sprite_info,
            &pam.sprite,
            &image_sequence_list,
            &sprite_list,
            &setting.disable_sprite,
            &mut main_sprite_frame,
        )?;
        sprite_list.insert(i, sprite_image_list);
    }

    read_sprite(
        -1,
        &pam.main_sprite,
        &pam.sprite,
        &image_sequence_list,
        &sprite_list,
        &setting.disable_sprite,
        &mut main_sprite_frame,
    )?;

    let mut max_pos = [0.0, 0.0];
    let (width, height) = find_image_square(&mut main_sprite_frame, &mut max_pos);

    let width = width as u32 + setting.append_width as u32;
    let height = height as u32 + setting.append_height as u32;

    let ctx = WriteImageContext {
        main_sprite_frame: &main_sprite_frame,
        setting,
        image_list: &image_list,
        max_pos: &max_pos,
        out_folder,
        frame_rate: pam.frame_rate,
    };
    write_image(&ctx, width, height)?;

    Ok(write_label_info(pam))
}

fn write_label_info(pam: &PamInfo) -> BTreeMap<String, [u32; 2]> {
    let mut label_info = BTreeMap::new();
    let frames = &pam.main_sprite.frame;
    let mut end_frame_index = frames.len() as u32 - 1;
    for (i, frame) in frames.iter().enumerate().rev() {
        if let Some(label) = &frame.label {
            label_info.insert(label.clone(), [i as u32, end_frame_index]);
            end_frame_index = i.saturating_sub(1) as u32;
        }
    }
    label_info
}

fn mix_transform(source: &[f64; 6], change: &[f64; 6]) -> [f64; 6] {
    [
        change[0] * source[0] + change[2] * source[1],
        change[1] * source[0] + change[3] * source[1],
        change[0] * source[2] + change[2] * source[3],
        change[1] * source[2] + change[3] * source[3],
        change[0] * source[4] + change[2] * source[5] + change[4],
        change[1] * source[4] + change[3] * source[5] + change[5],
    ]
}

fn find_image_square(
    main_sprite_frame: &mut HashMap<i32, Vec<ImageSequenceList>>,
    max_pos: &mut [f64; 2],
) -> (i32, i32) {
    let mut image_global_width: Vec<f64> = Vec::new();
    let mut image_global_height: Vec<f64> = Vec::new();
    let mut image_global_pos_x: Vec<f64> = Vec::new();
    let mut image_global_pos_y: Vec<f64> = Vec::new();

    for layer_sprites in main_sprite_frame.values_mut() {
        for layer_sprite in layer_sprites.iter_mut() {
            for t in &layer_sprite.transform {
                layer_sprite.matrix = mix_transform(&layer_sprite.matrix, t);
            }
            // we clear transform so we don't apply it again later
            layer_sprite.transform.clear();

            image_global_width.push(layer_sprite.matrix[4] + layer_sprite.image_width as f64);
            image_global_height.push(layer_sprite.matrix[5] + layer_sprite.image_height as f64);
            image_global_pos_x.push(layer_sprite.matrix[4]);
            image_global_pos_y.push(layer_sprite.matrix[5]);
        }
    }

    let min_x = image_global_pos_x
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let min_y = image_global_pos_y
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);

    let pos_x = if min_x >= 0.0 { 0.0 } else { -min_x };
    let pos_y = if min_y >= 0.0 { 0.0 } else { -min_y };
    max_pos[0] = pos_x;
    max_pos[1] = pos_y;

    let max_w = image_global_width
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let max_h = image_global_height
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let w = max_w + pos_x;
    let h = max_h + pos_y;

    ((w.ceil() as i32).max(1), (h.ceil() as i32).max(1))
}

struct LayerState {
    resource: usize,
    sprite: bool,

    frame_duration: i32,
    color: [f64; 4],
    transform: [f64; 6],
    active: bool,
}

fn variant_to_standard(transform: &[f64]) -> Result<[f64; 6]> {
    if transform.len() == 2 {
        Ok([1.0, 0.0, 0.0, 1.0, transform[0], transform[1]])
    } else if transform.len() >= 6 {
        Ok([
            transform[0],
            transform[1],
            transform[2],
            transform[3],
            transform[4],
            transform[5],
        ])
    } else if transform.len() == 3 {
        let cos_val = transform[0].cos();
        let sin_val = transform[0].sin();
        Ok([
            cos_val,
            sin_val,
            -sin_val,
            cos_val,
            transform[1],
            transform[2],
        ])
    } else {
        Err(anyhow!("Invalid transform size"))
    }
}

fn read_sprite(
    index: i32,
    sprite: &SpriteInfo,
    _sub_sprites: &[SpriteInfo],
    image_sequence_list: &HashMap<usize, Vec<ImageSequenceList>>,
    sprite_list: &HashMap<usize, Vec<ImageSequenceList>>,
    disable_sprite: &[i32],
    main_sprite_frame: &mut HashMap<i32, Vec<ImageSequenceList>>,
) -> Result<Vec<ImageSequenceList>> {
    let mut layers: HashMap<i32, LayerState> = HashMap::new();
    let mut sprite_image_list = Vec::new();

    let frames = &sprite.frame;

    // Instead of doing DOM frames to XML, we process timeline directly
    // state mapping layer_id -> list of rendering layers per frame.
    // Actually, Sen goes Frame -> Layer -> Changes -> then builds timeline.
    // Let's emulate Sen's frame_node_list but straight into ImageSequenceList.
    for (i, frame) in frames.iter().enumerate() {
        let removes = &frame.remove;
        for remove in removes {
            if let Some(layer) = layers.get_mut(&remove.index) {
                layer.active = false;
            }
        }

        let appends = &frame.append;
        for append in appends {
            layers.insert(
                append.index,
                LayerState {
                    resource: append.resource as usize,
                    sprite: append.sprite,
                    frame_duration: 1,
                    color: [1.0, 1.0, 1.0, 1.0],
                    transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    active: true,
                },
            );
        }

        let changes = &frame.change;
        for change in changes {
            if let Some(layer) = layers.get_mut(&change.index) {
                layer.active = true;
                let t = &change.transform;
                if !t.is_empty() {
                    layer.transform = variant_to_standard(t)?;
                }
                if let Some(c) = &change.color
                    && c.len() >= 4
                    && (c[0] != 0.0 || c[1] != 0.0)
                {
                    layer.color = [c[0], c[1], c[2], c[3]];
                }
            }
        }

        // For each active layer, emit the frame into the timeline
        let mut keys: Vec<i32> = layers.keys().cloned().collect();
        keys.sort_unstable(); // draw lowest index first

        for k in keys {
            let layer = layers.get_mut(&k).unwrap();

            if !layer.active {
                // Sen explicitly removed it when active is false
                layers.remove(&k);
                continue;
            }

            let resource_index = layer.resource;
            let is_sprite = layer.sprite;

            let frame_sprite_list_opt = if is_sprite {
                sprite_list.get(&resource_index)
            } else {
                image_sequence_list.get(&resource_index)
            };

            if let Some(frame_sprite_list) = frame_sprite_list_opt {
                let frame_index_duration = i as i32;

                // Note: The frame offset inside a sub-sprite is:
                // `(i - layer.frame_start) % sub_sprite.frame.len()` in Sen, but the sub-sprite might be flattened into `sprite_list` list already?
                // Wait, if it's flattened, does `sprite_list` hold just the frames?
                // In Sen: frameSpriteList is copied for each layer. The recursion handles nesting.
                // Let's just clone and append transform.

                for frame_sprite in frame_sprite_list {
                    let mut frame_sprite = frame_sprite.clone();
                    if is_sprite && disable_sprite.contains(&(resource_index as i32 + 1)) {
                        frame_sprite.disable_sprite = true;
                    }
                    frame_sprite.transform.push(layer.transform);
                    frame_sprite.color.push(layer.color);

                    if index != -1 {
                        sprite_image_list.push(frame_sprite);
                    } else {
                        main_sprite_frame
                            .entry(frame_index_duration)
                            .or_default()
                            .push(frame_sprite);
                    }
                }
            }

            layer.frame_duration += 1;
        }
    }

    Ok(sprite_image_list)
}

fn composite_images(
    source_image: &mut RgbaImage,
    layer_sprite: &ImageSequenceList,
    sprite_image: &RgbaImage,
    max_pos: &[f64; 2],
    setting: &AnimationHelperSetting,
) {
    let mut image_matrix = layer_sprite.matrix;
    image_matrix[4] += max_pos[0] + setting.pos_x as f64;
    image_matrix[5] += max_pos[1] + setting.pos_y as f64;

    let mut image_color = [0.0; 4];
    for color in &layer_sprite.color {
        image_color[0] += color[0];
        image_color[1] += color[1];
        image_color[2] += color[2];
        image_color[3] += color[3];
    }

    let color_count = layer_sprite.color.len() as f64;
    if color_count > 0.0 {
        image_color[0] /= color_count;
        image_color[1] /= color_count;
        image_color[2] /= color_count;
        image_color[3] /= color_count;
    } else {
        image_color = [1.0, 1.0, 1.0, 1.0];
    }

    // Affine composite
    // image_matrix is a, b, c, d, tx, ty.
    // X' = a*X + c*Y + tx
    // Y' = b*X + d*Y + ty

    // To iterate over pixels in destination and map back, we need the inverse matrix.
    // Denom = a*d - b*c
    let det = image_matrix[0] * image_matrix[3] - image_matrix[1] * image_matrix[2];
    if det.abs() < 1e-6 {
        return; // Degenerate matrix, ignore drawing
    }

    let inv_a = image_matrix[3] / det;
    let inv_b = -image_matrix[1] / det;
    let inv_c = -image_matrix[2] / det;
    let inv_d = image_matrix[0] / det;
    let inv_tx = (image_matrix[2] * image_matrix[5] - image_matrix[3] * image_matrix[4]) / det;
    let inv_ty = (image_matrix[1] * image_matrix[4] - image_matrix[0] * image_matrix[5]) / det;

    // Find bounding box in destination to limit pixel iteration
    let (sw, sh) = (sprite_image.width() as f64, sprite_image.height() as f64);
    let corners = [(0.0, 0.0), (sw, 0.0), (0.0, sh), (sw, sh)];
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &(cx, cy) in &corners {
        let nx = image_matrix[0] * cx + image_matrix[2] * cy + image_matrix[4];
        let ny = image_matrix[1] * cx + image_matrix[3] * cy + image_matrix[5];
        if nx < min_x {
            min_x = nx;
        }
        if nx > max_x {
            max_x = nx;
        }
        if ny < min_y {
            min_y = ny;
        }
        if ny > max_y {
            max_y = ny;
        }
    }

    let min_x = (min_x.floor() as i32).max(0) as u32;
    let max_x = (max_x.ceil() as i32).min((source_image.width() - 1) as i32) as u32;
    let min_y = (min_y.floor() as i32).max(0) as u32;
    let max_y = (max_y.ceil() as i32).min((source_image.height() - 1) as i32) as u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            // Source coordinates
            let px = x as f64;
            let py = y as f64;
            let sx = inv_a * px + inv_c * py + inv_tx;
            let sy = inv_b * px + inv_d * py + inv_ty;

            // Bilinear interpolation
            let sxf = sx.floor();
            let syf = sy.floor();
            let sx_idx = sxf as i32;
            let sy_idx = syf as i32;

            if sx_idx >= 0 && sx_idx < sw as i32 - 1 && sy_idx >= 0 && sy_idx < sh as i32 - 1 {
                let dx = sx - sxf;
                let dy = sy - syf;
                // Get pixels
                let p00 = sprite_image.get_pixel(sx_idx as u32, sy_idx as u32);
                let p10 = sprite_image.get_pixel((sx_idx + 1) as u32, sy_idx as u32);
                let p01 = sprite_image.get_pixel(sx_idx as u32, (sy_idx + 1) as u32);
                let p11 = sprite_image.get_pixel((sx_idx + 1) as u32, (sy_idx + 1) as u32);

                let interp = |c: usize| -> f64 {
                    let top = p00[c] as f64 * (1.0 - dx) + p10[c] as f64 * dx;
                    let bottom = p01[c] as f64 * (1.0 - dx) + p11[c] as f64 * dx;
                    top * (1.0 - dy) + bottom * dy
                };

                let a = interp(3);
                if a > 0.0 {
                    let r = interp(0) * image_color[0];
                    let g = interp(1) * image_color[1];
                    let b = interp(2) * image_color[2];
                    let a = a * image_color[3];

                    let src_px = [
                        r.clamp(0.0, 255.0) as u8,
                        g.clamp(0.0, 255.0) as u8,
                        b.clamp(0.0, 255.0) as u8,
                        a.clamp(0.0, 255.0) as u8,
                    ];

                    // Alpha blending
                    let dst_px = source_image.get_pixel_mut(x, y);
                    let inv_a = 255.0 - src_px[3] as f64;
                    dst_px[0] = ((src_px[0] as f64 * src_px[3] as f64 / 255.0)
                        + (dst_px[0] as f64 * inv_a / 255.0))
                        .clamp(0.0, 255.0) as u8;
                    dst_px[1] = ((src_px[1] as f64 * src_px[3] as f64 / 255.0)
                        + (dst_px[1] as f64 * inv_a / 255.0))
                        .clamp(0.0, 255.0) as u8;
                    dst_px[2] = ((src_px[2] as f64 * src_px[3] as f64 / 255.0)
                        + (dst_px[2] as f64 * inv_a / 255.0))
                        .clamp(0.0, 255.0) as u8;
                    dst_px[3] = (src_px[3] as f64 + dst_px[3] as f64 * inv_a / 255.0)
                        .clamp(0.0, 255.0) as u8;
                }
            } else if sx_idx == sw as i32 - 1 || sy_idx == sh as i32 - 1 {
                // edge case fallback to nearest
                if sx_idx >= 0 && sx_idx < sw as i32 && sy_idx >= 0 && sy_idx < sh as i32 {
                    let p = sprite_image.get_pixel(sx_idx as u32, sy_idx as u32);
                    if p[3] > 0 {
                        let r = p[0] as f64 * image_color[0];
                        let g = p[1] as f64 * image_color[1];
                        let b = p[2] as f64 * image_color[2];
                        let a = p[3] as f64 * image_color[3];

                        let src_px = [
                            r.clamp(0.0, 255.0) as u8,
                            g.clamp(0.0, 255.0) as u8,
                            b.clamp(0.0, 255.0) as u8,
                            a.clamp(0.0, 255.0) as u8,
                        ];

                        let dst_px = source_image.get_pixel_mut(x, y);
                        let inv_a = 255.0 - src_px[3] as f64;
                        dst_px[0] = ((src_px[0] as f64 * src_px[3] as f64 / 255.0)
                            + (dst_px[0] as f64 * inv_a / 255.0))
                            .clamp(0.0, 255.0) as u8;
                        dst_px[1] = ((src_px[1] as f64 * src_px[3] as f64 / 255.0)
                            + (dst_px[1] as f64 * inv_a / 255.0))
                            .clamp(0.0, 255.0) as u8;
                        dst_px[2] = ((src_px[2] as f64 * src_px[3] as f64 / 255.0)
                            + (dst_px[2] as f64 * inv_a / 255.0))
                            .clamp(0.0, 255.0) as u8;
                        dst_px[3] = (src_px[3] as f64 + dst_px[3] as f64 * inv_a / 255.0)
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

use rayon::prelude::*;

struct WriteImageContext<'a> {
    main_sprite_frame: &'a HashMap<i32, Vec<ImageSequenceList>>,
    setting: &'a AnimationHelperSetting,
    image_list: &'a [RgbaImage],
    max_pos: &'a [f64; 2],
    out_folder: &'a Path,
    frame_rate: i32,
}

fn write_image(ctx: &WriteImageContext, width: u32, height: u32) -> Result<()> {
    let mut frame_keys: Vec<i32> = ctx.main_sprite_frame.keys().cloned().collect();
    frame_keys.sort_unstable();

    let delay_ms = if ctx.frame_rate > 0 {
        1000 / ctx.frame_rate as u32
    } else {
        100
    };
    let delay = image::Delay::from_numer_denom_ms(delay_ms, 1);
    let write_sequence = ctx.setting.format == RenderFormat::PngSequence;

    if write_sequence {
        // Render and write to disk in parallel
        frame_keys
            .par_iter()
            .try_for_each(|frame_index| -> Result<()> {
                let mut image = RgbaImage::new(width, height);
                if let Some(layers) = ctx.main_sprite_frame.get(frame_index) {
                    for layer_sprite in layers {
                        if !layer_sprite.disable_sprite
                            && layer_sprite.image_index < ctx.image_list.len()
                        {
                            let sprite_image = &ctx.image_list[layer_sprite.image_index];
                            composite_images(
                                &mut image,
                                layer_sprite,
                                sprite_image,
                                ctx.max_pos,
                                ctx.setting,
                            );
                        }
                    }
                }
                let out_file = ctx
                    .out_folder
                    .join(format!("{}_{}.png", ctx.setting.frame_name, frame_index));
                image.save(&out_file)?;
                Ok(())
            })?;
    } else {
        // For GIF, render in parallel but collect sequentially
        let rendered_images: Vec<RgbaImage> = frame_keys
            .par_iter()
            .map(|frame_index| {
                let mut image = RgbaImage::new(width, height);
                if let Some(layers) = ctx.main_sprite_frame.get(frame_index) {
                    for layer_sprite in layers {
                        if !layer_sprite.disable_sprite
                            && layer_sprite.image_index < ctx.image_list.len()
                        {
                            let sprite_image = &ctx.image_list[layer_sprite.image_index];
                            composite_images(
                                &mut image,
                                layer_sprite,
                                sprite_image,
                                ctx.max_pos,
                                ctx.setting,
                            );
                        }
                    }
                }
                image
            })
            .collect();

        let frames_for_anim: Vec<image::Frame> = rendered_images
            .into_iter()
            .map(|image| image::Frame::from_parts(image, 0, 0, delay))
            .collect();

        if ctx.setting.format == RenderFormat::Gif {
            let out_file = ctx
                .out_folder
                .join(format!("{}.gif", ctx.setting.frame_name));
            let mut file = std::fs::File::create(&out_file)?;
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut file);
            encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;
            encoder.encode_frames(frames_for_anim.into_iter())?;
        }
    }

    Ok(())
}
