use anyhow::{Context, Result};
use clap::Subcommand;
use pam::{convert_to_fla, decode_pam, encode_pam};
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
        /// FLA scale resolution (default 1200)
        #[arg(short, long, default_value = "1200")]
        resolution: i32,
        /// Explicit input format: json or fla
        #[arg(short, long)]
        format: String,
    },
    /// Encode JSON/FLA to PAM
    Encode {
        /// Input JSON or FLA directory
        input: PathBuf,
        /// Output PAM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// FLA scale resolution (default 1200)
        #[arg(short, long, default_value = "1200")]
        resolution: i32,
        /// Explicit input format: json or fla
        #[arg(short, long)]
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
    }
}

pub fn pam_decode(
    input: &Path,
    output: &Option<PathBuf>,
    resolution: i32,
    format: &str,
) -> Result<()> {
    // Decode PAM -> JSON/FLA
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let pam_value = decode_pam(&mut file).context("Failed to decode PAM")?;

    let format_str = format.to_lowercase();

    if format_str == "fla" {
        let out_path = match output {
            Some(p) => p.clone(),
            None => input.with_extension("fla"),
        };
        convert_to_fla(&pam_value, &out_path, resolution)
            .context("Failed to generate FLA file")?;
        println!("Decoded PAM to FLA file at {:?}", out_path);
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
    // Encode JSON/FLA -> PAM
    let format_str = format.to_lowercase();

    let pam_value = if format_str == "json" {
        let content = fs::read_to_string(input).context("Failed to read input file")?;
        serde_json::from_str(&content).context("Failed to parse JSON")?
    } else if format_str == "fla" {
        pam::convert_from_fla(input, resolution).context("Failed to parse FLA file")?
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
