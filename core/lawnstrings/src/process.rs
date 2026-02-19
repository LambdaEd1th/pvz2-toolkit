use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    LawnStringsRoot,
    error::{LawnStringsError, Result},
    parse_lawn_strings, write_lawn_strings,
};

pub fn lawnstrings_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode LawnStrings -> JSON
    let content = fs::read_to_string(input).map_err(LawnStringsError::Io)?;
    let strings = parse_lawn_strings(&content)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    let json = serde_json::to_string_pretty(&strings).map_err(LawnStringsError::Json)?;
    fs::write(&out_path, json).map_err(LawnStringsError::Io)?;
    println!("Decoded LawnStrings to {:?}", out_path);
    Ok(())
}

pub fn lawnstrings_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> LawnStrings
    let content = fs::read_to_string(input).map_err(LawnStringsError::Io)?;
    let strings: LawnStringsRoot =
        serde_json::from_str(&content).map_err(LawnStringsError::Json)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("txt"), // LawnStrings are .txt usually, sometimes .st
    };

    let text_content = write_lawn_strings(&strings)?;
    fs::write(&out_path, text_content).map_err(LawnStringsError::Io)?;
    println!("Encoded LawnStrings to {:?}", out_path);
    Ok(())
}
