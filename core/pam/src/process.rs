use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{convert_to_xfl, decode_pam, encode_pam, html::parse_html_pam};

pub fn pam_decode(
    input: &Path,
    output: &Option<PathBuf>,
    resolution: i32,
    format: Option<&str>,
) -> Result<()> {
    // Decode PAM -> JSON/HTML/XFL
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let pam_value = decode_pam(&mut file).context("Failed to decode PAM")?;

    let format_str = format
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| match output {
            Some(p) => {
                let path_str = p.to_string_lossy().to_lowercase();
                if path_str.ends_with(".xfl")
                    || path_str.ends_with("\\xfl")
                    || path_str.ends_with("/xfl")
                    || p.extension().is_none()
                {
                    "xfl".to_string()
                } else if path_str.ends_with(".html") {
                    "html".to_string()
                } else {
                    "json".to_string()
                }
            }
            None => "json".to_string(),
        });

    if format_str == "xfl" {
        let out_dir = match output {
            Some(p) => p.clone(),
            None => input.with_extension("xfl"),
        };
        convert_to_xfl(&pam_value, &out_dir, resolution)
            .context("Failed to generate XFL project")?;
        println!("Decoded PAM to XFL project at {:?}", out_dir);
    } else if format_str == "html" {
        let out_path = match output {
            Some(p) => p.clone(),
            None => input.with_extension("html"),
        };
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).context("Failed to create output directory")?;
        }
        crate::html::convert_to_html(&pam_value, &out_path)?;
        println!("Decoded PAM to HTML at {:?}", out_path);
    } else {
        let out_path = match output {
            Some(p) => p.clone(),
            None => input.with_extension("json"),
        };

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).context("Failed to create output directory")?;
        }

        fs::write(
            &out_path,
            serde_json::to_string_pretty(&pam_value).context("Failed to serialize to JSON")?,
        )
        .context("Failed to write output file")?;
        println!("Decoded PAM to {:?}", out_path);
    }

    Ok(())
}

pub fn pam_encode(
    input: &Path,
    output: &Option<PathBuf>,
    resolution: i32,
    format: Option<&str>,
) -> Result<()> {
    // Encode JSON/HTML/XFL -> PAM
    let format_str = format.map(|s| s.to_lowercase()).unwrap_or_else(|| {
        let ext = input
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if input.is_dir() || ext == "xfl" || ext == "xml" {
            "xfl".to_string()
        } else if ext == "html" {
            "html".to_string()
        } else {
            "json".to_string()
        }
    });

    let pam_value = if format_str == "html" {
        let content = fs::read_to_string(input).context("Failed to read input file")?;
        parse_html_pam(&content).context("Failed to parse HTML PAM")?
    } else if format_str == "json" {
        let content = fs::read_to_string(input).context("Failed to read input file")?;
        serde_json::from_str(&content).context("Failed to parse JSON")?
    } else if format_str == "xfl" {
        // Assume XFL directory or DOMDocument.xml
        let xfl_dir = if input.is_dir() {
            input.to_path_buf()
        } else {
            input.parent().unwrap_or(Path::new("")).to_path_buf()
        };
        crate::convert_from_xfl(&xfl_dir, resolution).context("Failed to parse XFL project")?
    } else {
        anyhow::bail!("Unsupported input format for pam encode: {}", format_str);
    };

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("pam"),
    };

    let mut file = fs::File::create(&out_path).context("Failed to create output file")?;
    encode_pam(&pam_value, &mut file).context("Failed to encode PAM")?;
    println!("Encoded PAM to {:?}", out_path);
    Ok(())
}

pub fn pam_render(
    input: &Path,
    media: &Path,
    output: &Path,
    disable: Vec<i32>,
    format_str: &str,
) -> Result<()> {
    let content = fs::read_to_string(input).context("Failed to read input PAM JSON")?;
    let pam_info: crate::types::PamInfo =
        serde_json::from_str(&content).context("Failed to parse JSON")?;

    let format = match format_str.to_lowercase().as_str() {
        "png" => crate::render::RenderFormat::PngSequence,
        "gif" => crate::render::RenderFormat::Gif,
        _ => anyhow::bail!(
            "Unsupported format: {}. Valid formats: png, gif",
            format_str
        ),
    };

    let setting = crate::render::AnimationHelperSetting {
        disable_sprite: disable,
        format,
        ..Default::default()
    };

    crate::render::render_animation(&pam_info, output, media, &setting)?;
    println!("Rendered PAM animation to {:?}", output);
    Ok(())
}
