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
