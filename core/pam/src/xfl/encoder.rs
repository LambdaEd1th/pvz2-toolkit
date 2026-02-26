use super::xml_writer::XmlWriter;
use crate::PamInfo;
use crate::types::{ImageInfo, SpriteInfo};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const XFL_NS: &str = "http://ns.adobe.com/xfl/2008/";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

pub fn convert_to_xfl(pam: &PamInfo, output_dir: &Path, resolution: i32) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    // Create folders
    let library_dir = output_dir.join("LIBRARY");
    fs::create_dir_all(&library_dir)?;
    fs::create_dir_all(library_dir.join("media"))?;
    fs::create_dir_all(library_dir.join("source"))?;
    fs::create_dir_all(library_dir.join("image"))?;
    fs::create_dir_all(library_dir.join("sprite"))?;

    // Write media placeholders
    for (i, image) in pam.image.iter().enumerate() {
        write_source_document(
            i,
            image,
            &library_dir
                .join("source")
                .join(format!("source_{}.xml", i + 1)),
            resolution,
        )?;
        write_image_document(
            i,
            image,
            &library_dir
                .join("image")
                .join(format!("image_{}.xml", i + 1)),
        )?;
    }

    for (i, sprite) in pam.sprite.iter().enumerate() {
        write_sprite_document(
            i as i32,
            sprite,
            &library_dir
                .join("sprite")
                .join(format!("sprite_{}.xml", i + 1)),
            &pam.sprite,
        )?;
    }

    // Main sprite
    write_sprite_document(
        -1,
        &pam.main_sprite,
        &library_dir.join("main.xml"),
        &pam.sprite,
    )?;

    // DOMDocument.xml
    write_dom_document(pam, &output_dir.join("DOMDocument.xml"))
        .context("DOMDocument generation failed")?;

    // main.xfl
    fs::write(output_dir.join("main.xfl"), "PROXY-CS5")
        .context("main.xfl proxy generation failed")?;

    Ok(())
}

fn write_source_document(
    index: usize,
    image: &ImageInfo,
    path: &Path,
    resolution: i32,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(path)?;
    let mut w = XmlWriter::new(file);

    let name = format!("source/source_{}", index + 1);
    let media_name = format!(
        "media/{}",
        image.name.split('|').next().unwrap_or(&image.name)
    );

    w.start_element(
        "DOMSymbolItem",
        &[
            ("xmlns:xsi", XSI_NS),
            ("xmlns", XFL_NS),
            ("name", &name),
            ("symbolType", "graphic"),
        ],
    )?;

    w.start_element("timeline", &[])?;
    w.start_element("DOMTimeline", &[("name", &format!("source_{}", index + 1))])?;
    w.start_element("layers", &[])?;
    w.start_element("DOMLayer", &[("name", "Layer 1")])?;
    w.start_element("frames", &[])?;
    w.start_element("DOMFrame", &[("index", "0"), ("keyMode", "9728")])?;
    w.start_element("elements", &[])?;

    w.start_element("DOMBitmapInstance", &[("libraryItemName", &media_name)])?;
    w.start_element("matrix", &[])?;

    let scale = 1200.0 / (resolution as f64);
    let scale_str = format_float(scale);
    w.write_element("Matrix", &[("a", &scale_str), ("d", &scale_str)], None)?;
    w.end_element("matrix")?;
    w.end_element("DOMBitmapInstance")?;

    w.end_element("elements")?;
    w.end_element("DOMFrame")?;
    w.end_element("frames")?;
    w.end_element("DOMLayer")?;
    w.end_element("layers")?;
    w.end_element("DOMTimeline")?;
    w.end_element("timeline")?;
    w.end_element("DOMSymbolItem")?;

    Ok(())
}

fn write_image_document(index: usize, image: &ImageInfo, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(path)?;
    let mut w = XmlWriter::new(file);

    let name = format!("image/image_{}", index + 1);
    let source_name = format!("source/source_{}", index + 1);

    w.start_element(
        "DOMSymbolItem",
        &[
            ("xmlns:xsi", XSI_NS),
            ("xmlns", XFL_NS),
            ("name", &name),
            ("symbolType", "graphic"),
        ],
    )?;

    w.start_element("timeline", &[])?;
    w.start_element("DOMTimeline", &[("name", &format!("image_{}", index + 1))])?;
    w.start_element("layers", &[])?;
    w.start_element("DOMLayer", &[("name", "Layer 1")])?;
    w.start_element("frames", &[])?;
    w.start_element("DOMFrame", &[("index", "0"), ("keyMode", "9728")])?;
    w.start_element("elements", &[])?;

    w.start_element("DOMSymbolInstance", &[("libraryItemName", &source_name)])?;
    w.start_element("matrix", &[])?;

    let m = format_matrix(&image.transform);
    w.write_element(
        "Matrix",
        &[
            ("a", &format_float(m[0])),
            ("b", &format_float(m[1])),
            ("c", &format_float(m[2])),
            ("d", &format_float(m[3])),
            ("tx", &format_float(m[4])),
            ("ty", &format_float(m[5])),
        ],
        None,
    )?;

    w.end_element("matrix")?;
    w.end_element("DOMSymbolInstance")?;

    w.end_element("elements")?;
    w.end_element("DOMFrame")?;
    w.end_element("frames")?;
    w.end_element("DOMLayer")?;
    w.end_element("layers")?;
    w.end_element("DOMTimeline")?;
    w.end_element("timeline")?;
    w.end_element("DOMSymbolItem")?;

    Ok(())
}

fn write_sprite_document(
    index: i32,
    sprite: &SpriteInfo,
    path: &Path,
    sub_sprites: &[SpriteInfo],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(path)?;
    let mut w = XmlWriter::new(file);

    let name = if index == -1 {
        "main".to_string()
    } else {
        format!("sprite/sprite_{}", index + 1)
    };
    let timeline_name = if index == -1 {
        "main".to_string()
    } else {
        format!("sprite_{}", index + 1)
    };

    w.start_element(
        "DOMSymbolItem",
        &[
            ("xmlns:xsi", XSI_NS),
            ("xmlns", XFL_NS),
            ("name", &name),
            ("symbolType", "graphic"),
        ],
    )?;

    w.start_element("timeline", &[])?;
    w.start_element("DOMTimeline", &[("name", &timeline_name)])?;
    w.start_element("layers", &[])?;

    let layers = decode_frame_node_list(sprite, sub_sprites);

    let mut sorted_indices: Vec<i32> = layers.keys().cloned().collect();
    sorted_indices.sort_by(|a, b| b.cmp(a));

    for layer_idx in sorted_indices {
        w.start_element("DOMLayer", &[("name", &layer_idx.to_string())])?;
        w.start_element("frames", &[])?;

        if let Some(frames) = layers.get(&layer_idx) {
            for frame in frames {
                w.start_element(
                    "DOMFrame",
                    &[
                        ("index", &frame.start_frame.to_string()),
                        ("duration", &frame.duration.to_string()),
                        ("keyMode", "9728"),
                    ],
                )?;
                w.start_element("elements", &[])?;

                if let Some(elem) = &frame.element {
                    let lib_name = if elem.is_sprite {
                        format!("sprite/sprite_{}", elem.resource + 1)
                    } else {
                        format!("image/image_{}", elem.resource + 1)
                    };

                    let mut attrs = vec![
                        ("libraryItemName", lib_name.as_str()),
                        ("symbolType", "graphic"),
                        ("loop", "loop"),
                    ];

                    if elem.is_sprite {
                        attrs.push(("firstFrame", "0"));
                    }

                    w.start_element("DOMSymbolInstance", &attrs)?;

                    w.start_element("matrix", &[])?;
                    // Transform
                    let m = elem.transform; // [a, b, c, d, tx, ty]
                    w.write_element(
                        "Matrix",
                        &[
                            ("a", &format_float(m[0])),
                            ("b", &format_float(m[1])),
                            ("c", &format_float(m[2])),
                            ("d", &format_float(m[3])),
                            ("tx", &format_float(m[4])),
                            ("ty", &format_float(m[5])),
                        ],
                        None,
                    )?;
                    w.end_element("matrix")?;

                    let c = elem.color;
                    if (c[0] - 1.0).abs() > 0.001
                        || (c[1] - 1.0).abs() > 0.001
                        || (c[2] - 1.0).abs() > 0.001
                        || (c[3] - 1.0).abs() > 0.001
                    {
                        w.start_element("color", &[])?;
                        w.write_element(
                            "Color",
                            &[
                                ("redMultiplier", &format_float(c[0])),
                                ("greenMultiplier", &format_float(c[1])),
                                ("blueMultiplier", &format_float(c[2])),
                                ("alphaMultiplier", &format_float(c[3])),
                            ],
                            None,
                        )?;
                        w.end_element("color")?;
                    }

                    w.end_element("DOMSymbolInstance")?;
                }

                w.end_element("elements")?;
                w.end_element("DOMFrame")?;
            }
        }

        w.end_element("frames")?;
        w.end_element("DOMLayer")?;
    }

    w.end_element("layers")?;
    w.end_element("DOMTimeline")?;
    w.end_element("timeline")?;
    w.end_element("DOMSymbolItem")?;

    Ok(())
}

struct DomFrameData {
    start_frame: usize,
    duration: usize,
    element: Option<ElementData>,
}

struct ElementData {
    state: Option<bool>,
    resource: i32,
    is_sprite: bool,
    transform: [f64; 6],
    color: [f64; 4],
    frame_start: usize,
    frame_duration: usize,
}

fn decode_frame_node_list(
    sprite: &SpriteInfo,
    _sub_sprites: &[SpriteInfo],
) -> HashMap<i32, Vec<DomFrameData>> {
    let mut frame_node_list: HashMap<i32, Vec<DomFrameData>> = HashMap::new();
    let mut model: HashMap<i32, ElementData> = HashMap::new();

    let total_frames = sprite.frame.len();

    // Layer 0 is just an empty timeline container to define length matching Sen parity
    frame_node_list.insert(
        0,
        vec![DomFrameData {
            start_frame: 0,
            duration: total_frames,
            element: None,
        }],
    );

    for (i, frame_info) in sprite.frame.iter().enumerate() {
        for remove in &frame_info.remove {
            if let Some(m) = model.get_mut(&remove.index) {
                m.state = Some(false);
            }
        }

        for append in &frame_info.append {
            let data = ElementData {
                state: None,
                resource: append.resource,
                is_sprite: append.sprite,
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                frame_start: i,
                frame_duration: i,
            };
            model.insert(append.index, data);

            let layer_idx = append.index + 1;
            frame_node_list.entry(layer_idx).or_default();

            if i > 0 {
                frame_node_list
                    .get_mut(&layer_idx)
                    .unwrap()
                    .push(DomFrameData {
                        start_frame: 0,
                        duration: i,
                        element: None,
                    });
            }
        }

        for change in &frame_info.change {
            if let Some(layer) = model.get_mut(&change.index) {
                layer.state = Some(true);
                layer.transform = variant_to_standard(&change.transform);
                if let Some(c) = &change.color {
                    if c[0] != 0.0 || c[1] != 0.0 {
                        layer.color = *c;
                    }
                }
            }
        }

        let mut keys_to_remove = Vec::new();

        for (&layer_index, layer) in model.iter_mut() {
            let node_list = frame_node_list.entry(layer_index + 1).or_default();

            if layer.state.is_some() {
                if let Some(last_node) = node_list.last_mut() {
                    last_node.duration = layer.frame_duration;
                }
            }

            if layer.state == Some(true) {
                node_list.push(DomFrameData {
                    start_frame: i,
                    duration: 0,
                    element: Some(ElementData {
                        state: None,
                        resource: layer.resource,
                        is_sprite: layer.is_sprite,
                        transform: layer.transform,
                        color: layer.color,
                        frame_start: layer.frame_start,
                        frame_duration: layer.frame_duration,
                    }),
                });
                layer.state = None;
                layer.frame_duration = 0;
            }

            if layer.state == Some(false) {
                keys_to_remove.push(layer_index);
            }

            layer.frame_duration += 1;
        }

        for k in keys_to_remove {
            model.remove(&k);
        }
    }

    // Close out remaining models
    let remaining_keys: Vec<i32> = model.keys().copied().collect();
    for layer_index in remaining_keys {
        if let Some(layer) = model.remove(&layer_index) {
            if let Some(node_list) = frame_node_list.get_mut(&(layer_index + 1)) {
                if let Some(last_node) = node_list.last_mut() {
                    last_node.duration = layer.frame_duration;
                }
            }
        }
    }

    frame_node_list
}

fn write_dom_document(pam: &PamInfo, path: &Path) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut w = XmlWriter::new(file);

    w.start_element(
        "DOMDocument",
        &[
            ("xmlns:xsi", XSI_NS),
            ("xmlns", XFL_NS),
            ("width", &pam.size[0].to_string()),
            ("height", &pam.size[1].to_string()),
            ("frameRate", &pam.frame_rate.to_string()),
            ("currentTimeline", "1"),
            ("xflVersion", "2.971"),
            ("creatorInfo", "Adobe Animate CC"),
            ("platform", "Windows"),
            ("versionInfo", "Saved by Animate Windows 19.0 build 326"),
            ("objectsSnapTo", "false"),
        ],
    )?;

    w.start_element("folders", &[])?;
    w.write_element(
        "DOMFolderItem",
        &[("name", "image"), ("isExpanded", "true")],
        None,
    )?;
    w.write_element(
        "DOMFolderItem",
        &[("name", "media"), ("isExpanded", "true")],
        None,
    )?;
    w.write_element(
        "DOMFolderItem",
        &[("name", "source"), ("isExpanded", "true")],
        None,
    )?;
    w.write_element(
        "DOMFolderItem",
        &[("name", "sprite"), ("isExpanded", "true")],
        None,
    )?;
    w.end_element("folders")?;

    w.start_element("media", &[])?;
    for image in &pam.image {
        let name = image.name.split('|').next().unwrap_or(&image.name);
        w.write_element(
            "DOMBitmapItem",
            &[
                ("name", &format!("media/{}", name)),
                ("href", &format!("media/{}.png", name)),
                ("bitmapDataHRef", &format!("media/{}.png", name)),
            ],
            None,
        )?;
    }
    w.end_element("media")?;

    w.start_element("symbols", &[])?;
    for i in 0..pam.image.len() {
        w.write_element(
            "Include",
            &[("href", &format!("source/source_{}.xml", i + 1))],
            None,
        )?;
        w.write_element(
            "Include",
            &[("href", &format!("image/image_{}.xml", i + 1))],
            None,
        )?;
    }
    for i in 0..pam.sprite.len() {
        w.write_element(
            "Include",
            &[("href", &format!("sprite/sprite_{}.xml", i + 1))],
            None,
        )?;
    }
    w.write_element("Include", &[("href", "LIBRARY/main.xml")], None)?;
    w.end_element("symbols")?;

    w.start_element("timelines", &[])?;
    w.start_element("DOMTimeline", &[("name", "animation")])?;
    w.start_element("layers", &[])?;

    // --- DOMLayer "flow" ---
    w.start_element("DOMLayer", &[("name", "flow")])?;
    w.start_element("frames", &[])?;
    let mut prev_flow = -1isize;
    for (frame_idx, frame) in pam.main_sprite.frame.iter().enumerate() {
        if frame.label.is_some() || frame.stop {
            if prev_flow + 1 < frame_idx as isize {
                w.write_element(
                    "DOMFrame",
                    &[
                        ("index", &(prev_flow + 1).to_string()),
                        (
                            "duration",
                            &(frame_idx as isize - (prev_flow + 1)).to_string(),
                        ),
                    ],
                    None,
                )?;
            }

            let mut attrs = vec![("index", frame_idx.to_string())];
            if let Some(lbl) = &frame.label {
                attrs.push(("name", lbl.clone()));
                attrs.push(("labelType", "name".to_string()));
            }
            let str_attrs: Vec<(&str, &str)> =
                attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();

            w.start_element("DOMFrame", &str_attrs)?;
            w.start_element("elements", &[])?;
            w.end_element("elements")?;

            if frame.stop {
                w.start_element("Actionscript", &[])?;
                w.start_element("script", &[])?;
                w.write_raw("<![CDATA[stop();]]>")?;
                w.end_element("script")?;
                w.end_element("Actionscript")?;
            }

            w.end_element("DOMFrame")?;
            prev_flow = frame_idx as isize;
        }
    }
    if prev_flow + 1 < pam.main_sprite.frame.len() as isize {
        w.write_element(
            "DOMFrame",
            &[
                ("index", &(prev_flow + 1).to_string()),
                (
                    "duration",
                    &(pam.main_sprite.frame.len() as isize - (prev_flow + 1)).to_string(),
                ),
            ],
            None,
        )?;
    }
    w.end_element("frames")?;
    w.end_element("DOMLayer")?;

    // --- DOMLayer "command" ---
    w.start_element("DOMLayer", &[("name", "command")])?;
    w.start_element("frames", &[])?;
    let mut prev_cmd = -1isize;
    for (frame_idx, frame) in pam.main_sprite.frame.iter().enumerate() {
        if !frame.command.is_empty() {
            if prev_cmd + 1 < frame_idx as isize {
                w.write_element(
                    "DOMFrame",
                    &[
                        ("index", &(prev_cmd + 1).to_string()),
                        (
                            "duration",
                            &(frame_idx as isize - (prev_cmd + 1)).to_string(),
                        ),
                    ],
                    None,
                )?;
            }

            w.start_element("DOMFrame", &[("index", &frame_idx.to_string())])?;
            w.start_element("Actionscript", &[])?;
            w.start_element("script", &[])?;

            let mut cdata = String::from("<![CDATA[");
            for cmd in &frame.command {
                cdata.push_str(&format!("fscommand(\"{}\", \"{}\");\n", cmd[0], cmd[1]));
            }
            cdata.push_str("]]>");
            w.write_raw(&cdata)?;

            w.end_element("script")?;
            w.end_element("Actionscript")?;
            w.end_element("DOMFrame")?;

            prev_cmd = frame_idx as isize;
        }
    }
    if prev_cmd + 1 < pam.main_sprite.frame.len() as isize {
        w.write_element(
            "DOMFrame",
            &[
                ("index", &(prev_cmd + 1).to_string()),
                (
                    "duration",
                    &(pam.main_sprite.frame.len() as isize - (prev_cmd + 1)).to_string(),
                ),
            ],
            None,
        )?;
    }
    w.end_element("frames")?;
    w.end_element("DOMLayer")?;

    // --- DOMLayer "sprite" ---
    w.start_element("DOMLayer", &[("name", "sprite")])?;
    w.start_element("frames", &[])?;
    w.start_element(
        "DOMFrame",
        &[
            ("index", "0"),
            ("duration", &pam.main_sprite.frame.len().to_string()),
        ],
    )?;
    w.start_element("elements", &[])?;
    w.write_element(
        "DOMSymbolInstance",
        &[
            ("libraryItemName", "main"),
            ("symbolType", "graphic"),
            ("loop", "loop"),
        ],
        None,
    )?;
    w.end_element("elements")?;
    w.end_element("DOMFrame")?;
    w.end_element("frames")?;
    w.end_element("DOMLayer")?;

    w.end_element("layers")?;
    w.end_element("DOMTimeline")?;
    w.end_element("timelines")?;

    w.end_element("DOMDocument")?;

    Ok(())
}

fn format_float(f: f64) -> String {
    format!("{:.6}", f)
}

fn format_matrix(m: &[f64]) -> [f64; 6] {
    if m.len() == 6 {
        [m[0], m[1], m[2], m[3], m[4], m[5]]
    } else if m.len() == 2 {
        [1.0, 0.0, 0.0, 1.0, m[0], m[1]]
    } else if m.len() == 3 {
        let cos = m[0].cos();
        let sin = m[0].sin();
        [cos, sin, -sin, cos, m[1], m[2]]
    } else {
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    }
}

fn variant_to_standard(transform: &[f64]) -> [f64; 6] {
    format_matrix(transform)
}
