use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::types::*;

pub fn convert_from_xfl(input_dir: &Path, _resolution: i32) -> Result<PamInfo> {
    let mut pam_info = PamInfo {
        version: 5,
        frame_rate: 30,
        position: [0.0, 0.0],
        size: [0.0, 0.0],
        image: vec![],
        sprite: vec![],
        main_sprite: SpriteInfo {
            name: Some("main_sprite".to_string()),
            description: None,
            frame_rate: 30.0,
            work_area: [0, 0],
            frame: vec![],
        },
    };

    let doc_path = input_dir.join("DOMDocument.xml");
    let doc_str =
        fs::read_to_string(&doc_path).with_context(|| format!("Failed to read {:?}", doc_path))?;

    let mut reader = Reader::from_str(&doc_str);
    reader.config_mut().trim_text(true);

    let mut inside_animation = false;
    let mut current_layer_name = String::new();
    let mut current_frame_index = 0;

    let stop_regex = Regex::new(r"stop\(\)").unwrap();
    let fs_regex = Regex::new(r#"fscommand\("([^"]+)"(?:,\s*"([^"]*)")?\)"#).unwrap();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"DOMDocument" => {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"width" {
                            pam_info.size[0] =
                                String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                        } else if attr.key.as_ref() == b"height" {
                            pam_info.size[1] =
                                String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                        } else if attr.key.as_ref() == b"frameRate" {
                            pam_info.frame_rate =
                                String::from_utf8_lossy(&attr.value).parse().unwrap_or(30);
                        }
                    }
                }
                b"DOMTimeline" => {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"name"
                            && String::from_utf8_lossy(&attr.value) == "animation"
                        {
                            inside_animation = true;
                        }
                    }
                }
                b"DOMLayer" if inside_animation => {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"name" {
                            current_layer_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
                b"DOMFrame" if inside_animation => {
                    if current_layer_name == "flow" || current_layer_name == "command" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"index" {
                                current_frame_index =
                                    String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                                if current_frame_index >= pam_info.main_sprite.frame.len() {
                                    pam_info
                                        .main_sprite
                                        .frame
                                        .resize_with(current_frame_index + 1, Default::default);
                                }
                            } else if attr.key.as_ref() == b"name" && current_layer_name == "flow" {
                                pam_info.main_sprite.frame[current_frame_index].label =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"DOMTimeline" {
                    inside_animation = false;
                }
            }
            Ok(Event::Text(e)) if inside_animation => {
                let text = String::from_utf8_lossy(&e.into_inner()).into_owned();
                if current_layer_name == "flow" && stop_regex.is_match(&text) {
                    pam_info.main_sprite.frame[current_frame_index].stop = true;
                } else if current_layer_name == "command" {
                    for caps in fs_regex.captures_iter(&text) {
                        let cmd = caps.get(1).unwrap().as_str().to_string();
                        let arg = caps.get(2).map_or("", |m| m.as_str()).to_string();
                        pam_info.main_sprite.frame[current_frame_index]
                            .command
                            .push([cmd, arg]);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("Error parsing DOMDocument XML: {:?}", e),
            _ => {}
        }
    }

    let mut id_to_name: HashMap<i32, String> = HashMap::new();
    let mut id_to_size: HashMap<i32, [i32; 2]> = HashMap::new();
    let dim_regex = Regex::new(r"_(\d+)x(\d+)(_\d+)?$").unwrap();

    let source_dir = input_dir.join("LIBRARY").join("source");
    if source_dir.exists() {
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with("source_") {
                    if let Ok(idx) = stem[7..].parse::<i32>() {
                        let xml_str = fs::read_to_string(&path)?;
                        let mut rd = Reader::from_str(&xml_str);
                        rd.config_mut().trim_text(true);
                        loop {
                            match rd.read_event() {
                                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                                    if e.name().as_ref() == b"DOMBitmapInstance" {
                                        for attr in e.attributes() {
                                            let attr = attr?;
                                            if attr.key.as_ref() == b"libraryItemName" {
                                                let name = String::from_utf8_lossy(&attr.value)
                                                    .replace("media/", "");
                                                id_to_name.insert(idx, name.clone());

                                                let mut size = [0, 0];
                                                if let Some(caps) = dim_regex.captures(&name) {
                                                    size[0] = caps
                                                        .get(1)
                                                        .unwrap()
                                                        .as_str()
                                                        .parse()
                                                        .unwrap_or(0);
                                                    size[1] = caps
                                                        .get(2)
                                                        .unwrap()
                                                        .as_str()
                                                        .parse()
                                                        .unwrap_or(0);
                                                }
                                                id_to_size.insert(idx, size);
                                            }
                                        }
                                    }
                                }
                                Ok(Event::Eof) => break,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let mut image_keys: Vec<i32> = id_to_name.keys().copied().collect();
    image_keys.sort();

    for &idx in &image_keys {
        let name = id_to_name[&idx].clone();
        let size = id_to_size[&idx];
        let mut transform = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

        let img_path = input_dir
            .join("LIBRARY")
            .join("image")
            .join(format!("image_{}.xml", idx));
        if img_path.exists() {
            let xml_str = fs::read_to_string(&img_path)?;
            let mut rd = Reader::from_str(&xml_str);
            rd.config_mut().trim_text(true);
            loop {
                match rd.read_event() {
                    Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                        if e.name().as_ref() == b"Matrix" {
                            for attr in e.attributes() {
                                let attr = attr.unwrap();
                                let v = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                                match attr.key.as_ref() {
                                    b"a" => transform[0] = v,
                                    b"b" => transform[1] = v,
                                    b"c" => transform[2] = v,
                                    b"d" => transform[3] = v,
                                    b"tx" => transform[4] = v,
                                    b"ty" => transform[5] = v,
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => {}
                }
            }
        }

        pam_info.image.push(ImageInfo {
            name,
            size,
            transform,
        });
    }

    let sprite_dir = input_dir.join("LIBRARY").join("sprite");
    let mut sprite_files = Vec::new();
    if sprite_dir.exists() {
        for entry in fs::read_dir(sprite_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with("sprite_") {
                    if let Ok(idx) = stem[7..].parse::<i32>() {
                        sprite_files.push((idx, path));
                    }
                }
            }
        }
    }
    sprite_files.sort_by_key(|k| k.0);

    for (_, path) in sprite_files {
        let sp = parse_sprite_document(&path)?;
        pam_info.sprite.push(sp);
    }

    let main_path = input_dir.join("LIBRARY").join("main.xml");
    if main_path.exists() {
        let mut main_sp = parse_sprite_document(&main_path)?;
        main_sp.name = Some("main_sprite".to_string());

        let frames_len = pam_info.main_sprite.frame.len().max(main_sp.frame.len());
        pam_info
            .main_sprite
            .frame
            .resize_with(frames_len, Default::default);
        main_sp.frame.resize_with(frames_len, Default::default);

        for i in 0..frames_len {
            pam_info.main_sprite.frame[i].remove = std::mem::take(&mut main_sp.frame[i].remove);
            pam_info.main_sprite.frame[i].append = std::mem::take(&mut main_sp.frame[i].append);
            pam_info.main_sprite.frame[i].change = std::mem::take(&mut main_sp.frame[i].change);
        }
    }

    Ok(pam_info)
}

fn parse_sprite_document(path: &Path) -> Result<SpriteInfo> {
    let xml_str = fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&xml_str);
    reader.config_mut().trim_text(true);

    let mut sp = SpriteInfo {
        name: None,
        description: None,
        frame_rate: 30.0,
        work_area: [0, 0],
        frame: Vec::new(),
    };

    let mut total_frames = 0;

    #[derive(Clone, PartialEq, Debug)]
    struct ElementState {
        resource: i32,
        transform: Vec<f64>,
        color: Vec<f64>,
        first_frame: Option<i32>,
    }

    let mut state_map: HashMap<i32, Vec<Option<ElementState>>> = HashMap::new();

    let mut current_z_index: Option<i32> = None;
    let mut current_start_idx = 0;
    let mut current_duration = 1;
    let mut current_state: Option<ElementState> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"DOMLayer" => {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"name" {
                            let name_str = String::from_utf8_lossy(&attr.value);
                            current_z_index = name_str.parse().ok();
                        }
                    }
                }
                b"DOMFrame" => {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"index" {
                            current_start_idx =
                                String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                        } else if attr.key.as_ref() == b"duration" {
                            current_duration =
                                String::from_utf8_lossy(&attr.value).parse().unwrap_or(1);
                        }
                    }
                    total_frames = total_frames.max(current_start_idx + current_duration);
                }
                b"DOMSymbolInstance" => {
                    let mut resource_id = -1;
                    let mut first_frame = None;
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"libraryItemName" {
                            let lib_item = String::from_utf8_lossy(&attr.value);
                            if lib_item.starts_with("image/image_") {
                                resource_id =
                                    lib_item["image/image_".len()..].parse::<i32>().unwrap_or(1)
                                        - 1;
                            } else if lib_item.starts_with("sprite/sprite_") {
                                resource_id = lib_item["sprite/sprite_".len()..]
                                    .parse::<i32>()
                                    .unwrap_or(1)
                                    - 1
                                    + 10000;
                            }
                        } else if attr.key.as_ref() == b"firstFrame" {
                            first_frame =
                                Some(String::from_utf8_lossy(&attr.value).parse().unwrap_or(0));
                        }
                    }
                    if resource_id >= 0 {
                        current_state = Some(ElementState {
                            resource: resource_id,
                            transform: vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                            color: vec![1.0, 1.0, 1.0, 1.0],
                            first_frame,
                        });
                    }
                }
                b"Matrix" => {
                    if let Some(state) = &mut current_state {
                        for attr in e.attributes() {
                            let attr = attr?;
                            let val = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                            match attr.key.as_ref() {
                                b"a" => state.transform[0] = val,
                                b"b" => state.transform[1] = val,
                                b"c" => state.transform[2] = val,
                                b"d" => state.transform[3] = val,
                                b"tx" => state.transform[4] = val,
                                b"ty" => state.transform[5] = val,
                                _ => {}
                            }
                        }
                    }
                }
                b"Color" => {
                    if let Some(state) = &mut current_state {
                        let mut rgba_mult = [1.0, 1.0, 1.0, 1.0];
                        let mut rgba_offset = vec![0.0, 0.0, 0.0, 0.0];
                        let mut has_offset = false;
                        for attr in e.attributes() {
                            let attr = attr?;
                            let val = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                            match attr.key.as_ref() {
                                b"redMultiplier" => rgba_mult[0] = val,
                                b"greenMultiplier" => rgba_mult[1] = val,
                                b"blueMultiplier" => rgba_mult[2] = val,
                                b"alphaMultiplier" => rgba_mult[3] = val,
                                b"redOffset" => {
                                    rgba_offset[0] = val;
                                    has_offset = true;
                                }
                                b"greenOffset" => {
                                    rgba_offset[1] = val;
                                    has_offset = true;
                                }
                                b"blueOffset" => {
                                    rgba_offset[2] = val;
                                    has_offset = true;
                                }
                                b"alphaOffset" => {
                                    rgba_offset[3] = val;
                                    has_offset = true;
                                }
                                _ => {}
                            }
                        }
                        state.color = vec![rgba_mult[0], rgba_mult[1], rgba_mult[2], rgba_mult[3]];
                        if has_offset {
                            state.color.extend(rgba_offset);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"DOMFrame" => {
                    if let Some(z) = current_z_index {
                        let tl = state_map.entry(z).or_insert_with(|| Vec::new());
                        let end_idx = current_start_idx + current_duration;
                        if tl.len() < end_idx {
                            tl.resize(end_idx, None);
                        }
                        if let Some(state) = &current_state {
                            for t in current_start_idx..end_idx {
                                tl[t] = Some(state.clone());
                            }
                        }
                    }
                    current_state = None;
                }
                b"DOMLayer" => {
                    current_z_index = None;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break, // Ignore errors, handle cleanly instead of panicking on bad XML nodes
            _ => {}
        }
    }

    sp.frame.resize_with(total_frames, Default::default);

    let mut z_keys: Vec<i32> = state_map.keys().copied().collect();
    z_keys.sort();

    for &z in &z_keys {
        let tl = &state_map[&z];
        let mut prev_state: Option<ElementState> = None;
        let mut virtual_prev: Option<ElementState> = None;

        for t in 0..total_frames {
            let curr = if t < tl.len() { tl[t].clone() } else { None };

            match (&prev_state, &curr) {
                (Some(_), None) => {
                    sp.frame[t].remove.push(RemovesInfo { index: z });
                    virtual_prev = None;
                }
                (None, Some(c)) => {
                    sp.frame[t].append.push(AddsInfo {
                        index: z,
                        resource: c.resource % 10000,
                        name: None,
                        sprite: c.resource >= 10000,
                        additive: false,
                        preload_frame: 0,
                        time_scale: 1.0,
                    });
                    virtual_prev = Some(ElementState {
                        resource: c.resource,
                        transform: vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                        color: vec![1.0, 1.0, 1.0, 1.0],
                        first_frame: None,
                    });
                }
                (Some(p), Some(c)) => {
                    if p.resource != c.resource {
                        sp.frame[t].remove.push(RemovesInfo { index: z });
                        sp.frame[t].append.push(AddsInfo {
                            index: z,
                            resource: c.resource % 10000,
                            name: None,
                            sprite: c.resource >= 10000,
                            additive: false,
                            preload_frame: 0,
                            time_scale: 1.0,
                        });
                        virtual_prev = Some(ElementState {
                            resource: c.resource,
                            transform: vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                            color: vec![1.0, 1.0, 1.0, 1.0],
                            first_frame: None,
                        });
                    }
                }
                _ => {}
            }

            if let Some(c) = &curr {
                let vp = virtual_prev.unwrap();
                let transform_change = vp.transform != c.transform;
                let color_change = vp.color != c.color;
                let frame_change = vp.first_frame != c.first_frame;

                if transform_change || color_change || frame_change {
                    let mut color_arr = None;
                    if c.color != vec![1.0, 1.0, 1.0, 1.0] {
                        let r = *c.color.get(0).unwrap_or(&1.0);
                        let g = *c.color.get(1).unwrap_or(&1.0);
                        let b = *c.color.get(2).unwrap_or(&1.0);
                        let a = *c.color.get(3).unwrap_or(&1.0);
                        color_arr = Some([r, g, b, a]);
                    }
                    sp.frame[t].change.push(MovesInfo {
                        index: z,
                        transform: c.transform.clone(),
                        color: color_arr,
                        source_rectangle: None,
                        sprite_frame_number: c.first_frame.unwrap_or(0),
                    });
                }

                virtual_prev = Some(c.clone());
            }

            prev_state = curr;
        }
    }

    Ok(sp)
}
