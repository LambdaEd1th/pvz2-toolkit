use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    MResourceGroup, decode_newton, encode_newton,
    error::{NewtonError, Result},
};

pub fn newton_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode NTON -> XML
    let mut file = fs::File::open(input).map_err(NewtonError::Io)?;
    let root = decode_newton(&mut file)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("xml"),
    };

    let mut file = fs::File::create(&out_path).map_err(NewtonError::Io)?;
    serde_xml_rs::to_writer(&mut file, &root).map_err(NewtonError::Xml)?;
    println!("Decoded NTON to {:?}", out_path);
    Ok(())
}

pub fn newton_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode XML -> NTON
    let content = fs::read_to_string(input).map_err(NewtonError::Io)?;
    let root: MResourceGroup = serde_xml_rs::from_str(&content).map_err(NewtonError::Xml)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("nton"),
    };

    let mut file = fs::File::create(&out_path).map_err(NewtonError::Io)?;
    encode_newton(&root, &mut file)?;
    println!("Encoded NTON to {:?}", out_path);
    Ok(())
}
