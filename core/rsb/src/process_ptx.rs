use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::RsbManifest;

pub fn ptx_decode(input: &Path, output: &Option<PathBuf>, is_powervr: bool) -> Result<()> {
    // Input should be rsb_manifest.json
    let input_dir = input.parent().unwrap_or(Path::new("."));
    let out_dir = match output {
        Some(p) => p.clone(),
        None => input_dir.to_path_buf(),
    };

    println!("Reading manifest from {:?}", input);
    let content = fs::read_to_string(input).context("Failed to read manifest file")?;
    // Try to parse as RsbManifest
    let manifest: RsbManifest = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse rsb_manifest.json: {}", e))?;

    // Collect all resources with ptx_info
    let mut tasks = Vec::new();

    for group in manifest.group {
        for subgroup in group.subgroup {
            for res in subgroup.packet_info.res {
                if let Some(ptx_info) = res.ptx_info {
                    tasks.push((subgroup.name_packet.clone(), res.path.clone(), ptx_info));
                }
            }
        }
    }

    println!("Found {} PTX textures to decode.", tasks.len());

    // Process in parallel
    tasks.par_iter().for_each(|(subgroup_name, path, info)| {
        // Extracted files are in: {InputRoot}/{SubgroupName}/{Path}
        // Path in JSON uses backslashes, e.g. "IMAGES\\480\\..."

        let relative_path = path.replace("\\", "/");
        // Construct full input path
        let full_input_path = input_dir.join(subgroup_name).join(&relative_path);

        // Output PNG next to the PTX
        let full_output_path = out_dir
            .join(subgroup_name)
            .join(&relative_path)
            .with_extension("png");

        if !full_input_path.exists() {
            // println!("Skipping missing file: {:?}", full_input_path);
            return;
        }

        // Create parent dir
        if let Some(parent) = full_output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let width = info.width as u32;
        let height = info.height as u32;
        let format_code = info.format;

        if let Ok(data) = fs::read(&full_input_path) {
            match ptx::PtxDecoder::decode(
                &data,
                width,
                height,
                format_code,
                None,
                info.alpha_format,
                is_powervr,
            ) {
                Ok(img) => {
                    if let Err(e) = img.save(&full_output_path) {
                        println!("Failed to save {:?}: {:?}", full_output_path, e);
                    }
                }
                Err(e) => {
                    println!("Failed to decode {:?}: {:?}", full_input_path, e);
                }
            }
        }
    });

    println!("PTX decoding complete.");
    Ok(())
}

pub fn ptx_encode(
    input: &Path,
    output: &Option<PathBuf>,
    is_powervr: bool,
    use_palette: bool,
) -> Result<()> {
    // Input should be rsb_manifest.json
    let input_dir = input.parent().unwrap_or(Path::new("."));
    let out_dir = match output {
        Some(p) => p.clone(),
        None => input_dir.to_path_buf(),
    };

    println!("Reading manifest from {:?}", input);
    let content = fs::read_to_string(input).context("Failed to read manifest file")?;
    let manifest: RsbManifest = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse rsb_manifest.json: {}", e))?;

    // Collect all resources with ptx_info
    let mut tasks = Vec::new();

    for group in manifest.group {
        for subgroup in group.subgroup {
            for res in subgroup.packet_info.res {
                if let Some(ptx_info) = res.ptx_info {
                    tasks.push((subgroup.name_packet.clone(), res.path.clone(), ptx_info));
                }
            }
        }
    }

    println!("Found {} textures to encode.", tasks.len());

    tasks.par_iter().for_each(|(subgroup_name, path, info)| {
        let relative_path = path.replace("\\", "/");
        // Look for PNG first
        let png_path = input_dir
            .join(subgroup_name)
            .join(&relative_path)
            .with_extension("png");

        // Target PTX path
        let ptx_path = out_dir.join(subgroup_name).join(&relative_path);

        if !png_path.exists() {
            // println!("Skipping missing PNG: {:?}", png_path);
            return;
        }

        // Create parent dir
        if let Some(parent) = ptx_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(img) = image::open(&png_path) {
            let mut format = ptx::PtxFormat::from(info.format);

            // Apply Palette Override
            if use_palette && format == ptx::PtxFormat::Etc1A8 {
                format = ptx::PtxFormat::Etc1Palette;
            }

            // Handle PowerVR/iOS BGRA swap for Format 0 (RGBA8888)
            let mut img_to_encode = img;
            if is_powervr && format == ptx::PtxFormat::Rgba8888 {
                // Swap R and B channels
                let mut rgba = img_to_encode.to_rgba8();
                for pixel in rgba.pixels_mut() {
                    let r = pixel[0];
                    let b = pixel[2];
                    pixel[0] = b;
                    pixel[2] = r;
                }
                img_to_encode = image::DynamicImage::ImageRgba8(rgba);
            }

            match ptx::PtxEncoder::encode(&img_to_encode, format) {
                Ok(data) => {
                    if let Err(e) = fs::write(&ptx_path, data) {
                        println!("Failed to write {:?}: {:?}", ptx_path, e);
                    } else {
                        // println!("Encoded: {:?}", ptx_path);
                    }
                }
                Err(e) => {
                    println!("Failed to encode {:?}: {:?}", png_path, e);
                }
            }
        } else {
            println!("Failed to open image: {:?}", png_path);
        }
    });

    println!("PTX encoding complete.");
    Ok(())
}
