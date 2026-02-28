use anyhow::Result;
use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum PakCommands {
    /// Unpack a .pak archive into a directory
    Unpack {
        /// Input .pak file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory (defaults to input stem)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack a directory into a .pak archive
    Pack {
        /// Input directory
        #[arg(short, long)]
        input: PathBuf,
        /// Output .pak file (defaults to input + .pak)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Platform: PC, Xbox360 (default: PC)
        #[arg(long, default_value = "PC")]
        platform: String,
        /// Use Windows path separators (backslash)
        #[arg(long)]
        windows_path: bool,
        /// Use Zlib compression
        #[arg(long)]
        zlib: bool,
    },
}

pub fn handle(cmd: PakCommands) -> Result<()> {
    match cmd {
        PakCommands::Unpack { input, output } => pak_unpack(&input, &output),
        PakCommands::Pack {
            input,
            output,
            platform,
            windows_path,
            zlib,
        } => pak_pack(&input, &output, &platform, windows_path, zlib),
    }
}

fn pak_unpack(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    let file = fs::File::open(input)?;
    let (info, records) = pak::unpack(file)?;

    let out_dir = match output {
        Some(p) => p.clone(),
        None => input.with_extension(""),
    };

    fs::create_dir_all(&out_dir)?;

    // Write info.json
    let info_json = serde_json::to_string_pretty(&info)?;
    fs::write(out_dir.join("pak_info.json"), info_json)?;

    for record in records {
        let normalized = record.path.replace('\\', "/");
        let file_path = out_dir.join(&normalized);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, &record.data)?;
    }

    println!(
        "Unpacked PAK to {:?} ({} platform)",
        out_dir, info.pak_platform
    );
    Ok(())
}

fn pak_pack(
    input: &Path,
    output: &Option<PathBuf>,
    platform: &str,
    windows_path: bool,
    zlib: bool,
) -> Result<()> {
    let out_path = match output {
        Some(p) => p.clone(),
        None => {
            let mut p = input.to_path_buf();
            p.set_extension("pak");
            p
        }
    };

    let info = pak::PakInfo {
        pak_platform: platform.to_string(),
        pak_use_windows_path_separate: windows_path,
        pak_use_zlib_compress: zlib,
    };

    // Collect all files from the input directory
    let mut records = Vec::new();
    let prefix_len = input.to_string_lossy().len() + 1; // +1 for separator

    for entry in walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let full_path = entry.path();
            let rel = &full_path.to_string_lossy()[prefix_len..];
            // Skip pak_info.json
            if rel == "pak_info.json" {
                continue;
            }
            let data = fs::read(full_path)?;
            records.push(pak::PakRecord {
                path: rel.to_string(),
                data,
            });
        }
    }

    let mut out_buf = Vec::new();
    pak::pack(&mut out_buf, &info, &records)?;
    fs::write(&out_path, out_buf)?;

    println!(
        "Packed PAK to {:?} ({}, {} files)",
        out_path,
        platform,
        records.len()
    );
    Ok(())
}
