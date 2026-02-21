use crate::error::{AtlasError, Result};
use crate::types::OfficialAtlas;
use image::ImageReader;
use std::fs;
use std::path::{Path, PathBuf};

pub fn split_atlas(
    json_path: &Path,
    image_path: Option<&Path>,
    output_dir: Option<&Path>,
) -> Result<()> {
    // 1. Read JSON
    let json_content = fs::read_to_string(json_path)?;
    let atlas: OfficialAtlas = serde_json::from_str(&json_content)?;

    println!(
        "Loaded Atlas: {} ({} resources)",
        atlas.id,
        atlas.resources.len()
    );

    // 2. Determine Image Path
    let img_path = if let Some(p) = image_path {
        p.to_path_buf()
    } else {
        // Try to replace extension with png
        json_path.with_extension("png")
    };

    if !img_path.exists() {
        return Err(AtlasError::Generic(format!(
            "Image file not found: {:?}. Please provide it explicitly if it has a different name.",
            img_path
        )));
    }

    println!("Using Image: {:?}", img_path);

    // 3. Open Image
    let img = ImageReader::open(&img_path)?.decode()?;

    // 4. Determine Output Directory
    let out_dir = if let Some(p) = output_dir {
        p.to_path_buf()
    } else {
        let file_stem = json_path.file_stem().unwrap_or_default();
        let mut dir_name = file_stem.to_os_string();
        dir_name.push(".sprite");
        // Maintain relative path structure if possible, or just use current dir logic
        // But here we construct a path relative to the json file's directory if implied
        if let Some(parent) = json_path.parent() {
            parent.join(dir_name).join("media")
        } else {
            PathBuf::from(dir_name).join("media")
        }
    };

    fs::create_dir_all(&out_dir)?;
    println!("Output Directory: {:?}", out_dir);

    // 5. Iterate and Split
    let mut count = 0;
    for res in atlas.resources {
        if let (Some(ax), Some(ay), Some(aw), Some(ah)) = (res.ax, res.ay, res.aw, res.ah) {
            // Check bounds to avoid panics or errors
            if ax + aw > img.width() || ay + ah > img.height() {
                eprintln!("Warning: Resource '{}' is out of bounds (x: {}, y: {}, w: {}, h: {}) for image size {}x{}. Skipping.", 
                    res.id, ax, ay, aw, ah, img.width(), img.height());
                continue;
            }

            let sub_img = img.crop_imm(ax, ay, aw, ah);
            // Default to ID for filename.
            // Sanitize ID?
            let filename = format!("{}.png", res.id);
            let save_path = out_dir.join(&filename);

            sub_img.save(&save_path)?;
            count += 1;
        }
    }

    println!("Successfully extracted {} sprites.", count);

    Ok(())
}

pub fn merge_atlas(
    json_path: &Path,
    input_dir: Option<&Path>,
    output_image: Option<&Path>,
    output_json: Option<&Path>,
) -> Result<()> {
    // 1. Read JSON
    let json_content = fs::read_to_string(json_path)?;
    let mut atlas: OfficialAtlas = serde_json::from_str(&json_content)?;

    println!(
        "Loaded Atlas: {} ({} resources)",
        atlas.id,
        atlas.resources.len()
    );

    // 2. Determine Input Directory
    let in_dir = if let Some(p) = input_dir {
        p.to_path_buf()
    } else {
        let file_stem = json_path.file_stem().unwrap_or_default();
        let mut dir_name = file_stem.to_os_string();
        dir_name.push(".sprite");
        if let Some(parent) = json_path.parent() {
            parent.join(dir_name).join("media")
        } else {
            PathBuf::from(dir_name).join("media")
        }
    };

    if !in_dir.exists() {
        return Err(AtlasError::Generic(format!(
            "Input media directory not found: {:?}",
            in_dir
        )));
    }

    println!("Using Input Directory: {:?}", in_dir);

    // 3. Configure Texture Packer
    use texture_packer::exporter::ImageExporter;
    use texture_packer::importer::ImageImporter;
    use texture_packer::TexturePackerConfig;

    // Use a large maximum size, e.g., 4096x4096 or 8192x8192
    let config = TexturePackerConfig {
        max_width: 8192,
        max_height: 8192,
        allow_rotation: false,
        texture_outlines: false,
        border_padding: 0,
        texture_padding: 0,
        trim: false, // We rely on original JSON for coordinates
        texture_extrusion: 0,
        force_max_dimensions: false,
    };

    let mut packer = texture_packer::TexturePacker::new_skyline(config);

    // 4. Load Images and Pack
    let mut packed_count = 0;
    for res in &atlas.resources {
        let filename = format!("{}.png", res.id);
        let sprite_path = in_dir.join(&filename);

        if sprite_path.exists() {
            if let Ok(texture) = ImageImporter::import_from_file(&sprite_path) {
                if let Err(e) = packer.pack_own(res.id.clone(), texture) {
                    eprintln!("Warning: Failed to pack '{}': {:?}", res.id, e);
                } else {
                    packed_count += 1;
                }
            } else {
                eprintln!("Warning: Failed to load image '{}'", filename);
            }
        } else {
            eprintln!("Warning: Sprite not found: {:?}", sprite_path);
        }
    }

    println!("Successfully packed {} sprites.", packed_count);

    // 5. Update Coordinates in JSON
    for res in &mut atlas.resources {
        if let Some(frame) = packer.get_frame(&res.id) {
            res.ax = Some(frame.frame.x);
            res.ay = Some(frame.frame.y);
            res.aw = Some(frame.frame.w);
            res.ah = Some(frame.frame.h);
        }
    }

    // 6. Calculate Final Dimensions (Nearest Power of 2)
    let frames = packer.get_frames();
    let mut max_x = 0;
    let mut max_y = 0;
    for frame in frames.values() {
        if frame.frame.x + frame.frame.w > max_x {
            max_x = frame.frame.x + frame.frame.w;
        }
        if frame.frame.y + frame.frame.h > max_y {
            max_y = frame.frame.y + frame.frame.h;
        }
    }

    let final_width = max_x.next_power_of_two();
    let final_height = max_y.next_power_of_two();

    println!("Final Atlas Size: {}x{}", final_width, final_height);

    // We might need to resize the final exported image to power of 2,
    // texture_packer usually outputs the minimum bounding box.
    // ImageExporter exports what is provided by texture_packer.
    let exported_image = ImageExporter::export(&packer, None)
        .map_err(|e| AtlasError::Generic(format!("Failed to export packed image: {}", e)))?;

    // Create a new image with power-of-two dimensions and copy the packed image into it
    let mut final_image = image::DynamicImage::new_rgba8(final_width, final_height);
    if let image::DynamicImage::ImageRgba8(ref mut buf) = final_image {
        if let image::DynamicImage::ImageRgba8(ref packed_buf) = exported_image {
            image::imageops::overlay(buf, packed_buf, 0, 0);
        }
    }

    // 7. Save Image
    let out_img_path = if let Some(p) = output_image {
        p.to_path_buf()
    } else {
        json_path.with_extension("png")
    };

    final_image.save(&out_img_path)?;
    println!("Saved merged atlas to {:?}", out_img_path);

    // 8. Save JSON
    let out_json_path = if let Some(p) = output_json {
        p.to_path_buf()
    } else {
        json_path.to_path_buf()
    };

    let updated_json = serde_json::to_string_pretty(&atlas)?;
    fs::write(&out_json_path, updated_json)?;
    println!("Saved updated atlas JSON to {:?}", out_json_path);

    Ok(())
}
