use anyhow::Result;
use clap::Subcommand;
use rton::{RtonValue, from_reader, to_writer};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum RtonCommands {
    /// Decode RTON to JSON
    Decode {
        /// Input RTON file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
    /// Encode JSON to RTON
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output RTON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
    /// Encrypt RTON/File
    Encrypt {
        /// Input file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (required)
        #[arg(long)]
        seed: String,
    },
    /// Decrypt RTON/File
    Decrypt {
        /// Input file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (required)
        #[arg(long)]
        seed: String,
    },
}

pub fn handle(cmd: RtonCommands) -> Result<()> {
    match cmd {
        RtonCommands::Decode {
            input,
            output,
            seed,
        } => rton_decode(&input, &output, seed.as_deref()),
        RtonCommands::Encode {
            input,
            output,
            seed,
        } => rton_encode(&input, &output, seed.as_deref()),
        RtonCommands::Encrypt {
            input,
            output,
            seed,
        } => rton_encrypt_file(&input, &output, &seed),
        RtonCommands::Decrypt {
            input,
            output,
            seed,
        } => rton_decrypt_file(&input, &output, &seed),
    }
}

pub fn rton_decode(input: &Path, output: &Option<PathBuf>, seed: Option<&str>) -> Result<()> {
    // Decode RTON -> JSON (Default for .rton or others)
    let mut file = fs::File::open(input)?;
    let rton_value: RtonValue = from_reader(&mut file, seed)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    let json = serde_json::to_string_pretty(&rton_value)?;
    fs::write(&out_path, json)?;
    println!("Decoded RTON to {:?}", out_path);
    Ok(())
}

pub fn rton_encode(input: &Path, output: &Option<PathBuf>, seed: Option<&str>) -> Result<()> {
    // Encode JSON -> RTON
    let content = fs::read_to_string(input)?;
    let rton_value: RtonValue = serde_json::from_str(&content)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("rton"),
    };

    let mut file = fs::File::create(&out_path)?;
    to_writer(&mut file, &rton_value, seed)?;
    println!("Encoded RTON to {:?}", out_path);
    Ok(())
}

pub fn rton_encrypt_file(input: &Path, output: &Option<PathBuf>, seed: &str) -> Result<()> {
    // Encrypt raw file
    let data = fs::read(input)?;
    let encrypted = rton::crypto::encrypt_data(&data, seed)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("encrypted"),
    };

    fs::write(&out_path, encrypted)?;
    println!("Encrypted to {:?}", out_path);
    Ok(())
}

pub fn rton_decrypt_file(input: &Path, output: &Option<PathBuf>, seed: &str) -> Result<()> {
    // Decrypt raw file
    let data = fs::read(input)?;
    let decrypted = rton::crypto::decrypt_data(&data, seed)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("decrypted"),
    };

    fs::write(&out_path, decrypted)?;
    println!("Decrypted to {:?}", out_path);
    Ok(())
}
