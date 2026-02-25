use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{decode_pam, encode_pam, parser::parse_html_pam};

pub fn pam_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode PAM -> JSON
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let pam_value = decode_pam(&mut file).context("Failed to decode PAM")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    fs::write(
        &out_path,
        serde_json::to_string_pretty(&pam_value).context("Failed to serialize to JSON")?,
    )
    .context("Failed to write output file")?;
    println!("Decoded PAM to {:?}", out_path);
    Ok(())
}

pub fn pam_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON/HTML -> PAM
    let extension = input
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let content = fs::read_to_string(input).context("Failed to read input file")?;

    let pam_value = if extension == "html" {
        // Parse HTML to extract JSON
        parse_html_pam(&content).context("Failed to parse HTML PAM")?
    } else {
        // Parse JSON directly
        serde_json::from_str(&content).context("Failed to parse JSON")?
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
