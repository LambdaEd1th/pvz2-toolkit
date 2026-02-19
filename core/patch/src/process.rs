use crate::{PatchError, Result};
use std::fs;
use std::path::Path;

use crate::{decode, encode, io::RsbPatchReader};

pub fn patch_create(source: &Path, target: &Path, output: &Path) -> Result<()> {
    let mut src_file = fs::File::open(source)?;
    let mut tgt_file = fs::File::open(target)?;
    let mut out_file = fs::File::create(output)?;

    println!(
        "Creating patch (Interleaved): {:?} -> {:?} = {:?}",
        source, target, output
    );
    encode(&mut src_file, &mut tgt_file, &mut out_file)?;
    println!("Patch created successfully.");
    Ok(())
}

pub fn patch_apply(source: &Path, patch: &Path, output: &Path) -> Result<()> {
    let mut src_file = fs::File::open(source)?;
    let mut patch_file = fs::File::open(patch)?;
    let mut out_file = fs::File::create(output)?;

    println!("Applying patch: {:?} + {:?} = {:?}", source, patch, output);
    decode(&mut src_file, &mut patch_file, &mut out_file)?;
    println!("Patch applied successfully.");
    Ok(())
}

pub fn patch_extract(input: &Path, output: &Path) -> Result<()> {
    let mut file = fs::File::open(input)?;
    let mut reader = RsbPatchReader::new(&mut file);

    let header = reader.read_header().map_err(|e| {
        PatchError::RsbPatch(crate::error::RsbPatchError::InvalidFile(e.to_string()))
    })?;

    println!("RSBPatch Header:");
    println!("  RSB Head Size: {}", header.rsb_head_size);
    println!("  RSG Number: {}", header.rsg_number);
    println!("  Need Patch: {}", header.rsb_need_patch);

    if !output.exists() {
        fs::create_dir_all(output)?;
    }

    if let Some(header_diff) = reader.extract_header_diff(&header).map_err(|e| {
        PatchError::RsbPatch(crate::error::RsbPatchError::InvalidFile(format!(
            "Failed to extract header diff: {}",
            e
        )))
    })? {
        let out_path = output.join("header.vcdiff");
        fs::write(&out_path, header_diff)?;
        println!("Extracted header diff to {:?}", out_path);
    }

    let mut count = 0;
    while let Some((info, _offset)) = reader.next_packet_info().map_err(|e| {
        PatchError::RsbPatch(crate::error::RsbPatchError::InvalidFile(format!(
            "Failed to read packet info: {}",
            e
        )))
    })? {
        if info.packet_patch_size > 0 {
            let diff_data = reader
                .extract_packet_diff(info.packet_patch_size)
                .map_err(|e| {
                    PatchError::RsbPatch(crate::error::RsbPatchError::InvalidFile(format!(
                        "Failed to extract packet diff for {}: {}",
                        info.packet_name, e
                    )))
                })?;

            let safe_name = info
                .packet_name
                .replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
            let out_path = output.join(format!("{}.vcdiff", safe_name));
            fs::write(&out_path, diff_data)?;
            println!(
                "Extracted packet diff: {} ({} bytes)",
                info.packet_name, info.packet_patch_size
            );
            count += 1;
        } else {
            // Skip 0 size
            let _ = reader.extract_packet_diff(0).map_err(|e| {
                PatchError::RsbPatch(crate::error::RsbPatchError::InvalidFile(format!(
                    "Failed to seek: {}",
                    e
                )))
            })?;
        }
    }

    println!("Extracted {} packet diffs to {:?}", count, output);
    Ok(())
}
