use anyhow::{Context, Result};
use clap::Subcommand;
use pam::{convert_to_xfl, decode_pam, encode_pam, html::parse_html_pam};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum PamCommands {
    /// Decode PAM to JSON
    Decode {
        /// Input PAM file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Flash XFL scale resolution (default 1200)
        #[arg(short, long, default_value = "1200")]
        resolution: i32,
        /// Explicit output format: json, html, or xfl
        #[arg(short, long)]
        format: String,
    },
    /// Encode JSON/HTML to PAM
    Encode {
        /// Input JSON or HTML file
        input: PathBuf,
        /// Output PAM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Flash XFL scale resolution (default 1200)
        #[arg(short, long, default_value = "1200")]
        resolution: i32,
        /// Explicit input format: json, html, or xfl
        #[arg(short, long)]
        format: String,
    },
    /// Render PAM animation frames from JSON
    Render {
        /// Input PAM JSON file
        input: PathBuf,
        /// Directory containing extracted media elements (PNGs)
        #[arg(short, long)]
        media: PathBuf,
        /// Output directory for rendered frames
        #[arg(short, long)]
        output: PathBuf,
        /// Disable rendering of specific sprite indices (comma separated, e.g. '1,2,5')
        #[arg(short, long, value_delimiter = ',')]
        disable: Vec<i32>,
        /// Output format: 'png' (image sequence) or 'gif' (animated GIF)
        #[arg(short, long, default_value = "png")]
        format: String,
    },
}

pub fn handle(cmd: PamCommands) -> Result<()> {
    match cmd {
        PamCommands::Decode {
            input,
            output,
            resolution,
            format,
        } => pam_decode(&input, &output, resolution, &format),
        PamCommands::Encode {
            input,
            output,
            resolution,
            format,
        } => pam_encode(&input, &output, resolution, &format),
        PamCommands::Render {
            input,
            media,
            output,
            disable,
            format,
        } => pam_render(&input, &media, &output, disable, &format),
    }
}

pub fn pam_decode(
    input: &Path,
    output: &Option<PathBuf>,
    resolution: i32,
    format: &str,
) -> Result<()> {
    // Decode PAM -> JSON/HTML/XFL
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let pam_value = decode_pam(&mut file).context("Failed to decode PAM")?;

    let format_str = format.to_lowercase();

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
        pam::html::convert_to_html(&pam_value, &out_path)?;
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
    format: &str,
) -> Result<()> {
    // Encode JSON/HTML/XFL -> PAM
    let format_str = format.to_lowercase();

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
        pam::convert_from_xfl(&xfl_dir, resolution).context("Failed to parse XFL project")?
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
    let pam_info: pam::types::PamInfo =
        serde_json::from_str(&content).context("Failed to parse JSON")?;

    let format = match format_str.to_lowercase().as_str() {
        "png" => pam::render::RenderFormat::PngSequence,
        "gif" => pam::render::RenderFormat::Gif,
        _ => anyhow::bail!(
            "Unsupported format: {}. Valid formats: png, gif",
            format_str
        ),
    };

    let setting = pam::render::AnimationHelperSetting {
        disable_sprite: disable,
        format,
        ..Default::default()
    };

    pam::render::render_animation(&pam_info, output, media, &setting)?;
    println!("Rendered PAM animation to {:?}", output);
    Ok(())
}
