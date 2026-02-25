use crate::error::Result;
use md5::{Digest, Md5};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{decode, encode};

pub fn smf_unpack(input: &Path, output: &Option<PathBuf>, use_64bit: bool) -> Result<()> {
    let mut file = fs::File::open(input)?;
    let decoded = decode(&mut file, use_64bit)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => {
            if input.extension().is_some_and(|e| e == "smf") {
                input.with_extension("")
            } else {
                input.with_extension("decoded")
            }
        }
    };

    fs::write(&out_path, decoded)?;
    println!("Unpacked SMF to {:?}", out_path);
    Ok(())
}

pub fn smf_pack(input: &Path, output: &Option<PathBuf>, use_64bit: bool) -> Result<()> {
    let data = fs::read(input)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => {
            // Append .smf. If existing extension, e.g. .rsb, becomes .rsb.smf
            let mut p = input.to_path_buf();
            if let Some(ext) = p.extension() {
                let mut ext = ext.to_os_string();
                ext.push(".smf");
                p.set_extension(ext);
            } else {
                p.set_extension("smf");
            }
            p
        }
    };

    let mut buffer = Vec::new();
    // Encode to buffer first to calculate MD5
    encode(&mut buffer, &data, use_64bit)?;

    fs::write(&out_path, &buffer)?;
    println!("Packed SMF to {:?}", out_path);

    // Calculate MD5 of the generated SMF file
    let mut hasher = Md5::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    let md5_hex = format!("{:X}", result); // Uppercase Hex

    // Generate .tag.smf filename
    // If output is zombie1.rsb.smf, tag is zombie1.rsb.tag.smf
    // simple string manipulation or with_extension
    let tag_path = if let Some(file_name) = out_path.file_name() {
        let mut name = file_name.to_string_lossy().into_owned();
        if name.ends_with(".smf") {
            // Replace .smf with .tag.smf
            name.truncate(name.len() - 4);
            name.push_str(".tag.smf");
            out_path.with_file_name(name)
        } else {
            // Just append
            let mut p = out_path.clone();
            if let Some(ext) = p.extension() {
                let mut ext = ext.to_os_string();
                ext.push(".tag.smf");
                p.set_extension(ext);
            } else {
                p.set_extension("tag.smf");
            }
            p
        }
    } else {
        out_path.with_extension("tag.smf")
    };

    let tag_content = format!("{}\r\n", md5_hex);
    fs::write(&tag_path, tag_content)?;
    println!("Generated Tag to {:?}", tag_path);

    Ok(())
}
