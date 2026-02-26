use crate::error::Result;
use crate::rsg::types::UnpackedFile;
use crate::rsg::{pack_rsg, unpack_rsg};
use crate::schema::types::*;

use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub fn unpack_rsg_batch(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Check for RSG magic to support single file unpacking
    let mut file = fs::File::open(input)?;
    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_ok() && magic == *b"pgsr" {
        println!("Detected single RSG file: {:?}", input);
        let out_dir = match output {
            Some(p) => p.clone(),
            None => {
                let stem = input.file_stem().unwrap_or_default();
                input.parent().unwrap_or(Path::new(".")).join(stem)
            }
        };

        if !out_dir.exists() {
            fs::create_dir_all(&out_dir)?;
        }

        // Rewind and unpack
        file.seek(SeekFrom::Start(0))?;
        let mut reader = std::io::BufReader::new(file);

        let unpacked_files = unpack_rsg(&mut reader)?;

        // Write manifest.json for packing
        let manifest_path = out_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(&unpacked_files)?;
        fs::write(&manifest_path, json)?;

        for file in unpacked_files {
            let clean_path = file.path.replace('\\', "/");
            let target_path = out_dir.join(&clean_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target_path, &file.data)?;
        }

        println!("RSG unpack complete.");
        return Ok(());
    }

    // Input is rsb_manifest.json
    let input_dir = input.parent().unwrap_or(Path::new("."));
    let out_dir = match output {
        Some(p) => p.clone(),
        None => input_dir.to_path_buf(),
    };

    println!("Reading manifest from {:?}", input);
    // Re-read file as string since we consumed 4 bytes
    let content = fs::read_to_string(input)?;
    let manifest: RsbManifest = serde_json::from_str(&content)?;

    println!(
        "Processing {} RSG packets from manifest...",
        manifest.path.rsgs.len()
    );

    // Parallel processing using rayon
    use rayon::prelude::*;

    // Collect all tasks first to allow parallel iteration
    let mut tasks = Vec::new();
    for group in &manifest.group {
        for subgroup in &group.subgroup {
            tasks.push(&subgroup.name_packet);
        }
    }

    // Process packets in parallel
    tasks.par_iter().for_each(|packet_name| {
        let packet_path = input_dir.join(packet_name);

        if !packet_path.exists() {
            return;
        }

        // Output folder for this packet
        let packet_out_dir = out_dir.join(packet_name);
        if fs::create_dir_all(&packet_out_dir).is_err() {
            return;
        }

        // Read packet data
        if let Ok(data) = fs::read(&packet_path) {
            let mut reader = Cursor::new(&data);
            if let Ok(unpacked_files) = unpack_rsg(&mut reader) {
                // Write manifest.json for packing
                let manifest_path = packet_out_dir.join("manifest.json");
                if let Ok(json) = serde_json::to_string_pretty(&unpacked_files) {
                    let _ = fs::write(&manifest_path, json);
                }

                for file in unpacked_files {
                    let clean_path = file.path.replace('\\', "/");
                    let target_path = packet_out_dir.join(&clean_path);

                    if let Some(parent) = target_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&target_path, &file.data);
                }
            }
        }
    });

    println!("RSG unpack complete.");
    Ok(())
}

pub fn pack_rsg_batch(input: &Path, output: &Path) -> Result<()> {
    // Check for manifest.json inside input folder
    let manifest_path = input.join("manifest.json");

    let unpacked_files = if manifest_path.exists() {
        println!("Using manifest: {:?}", manifest_path);
        let manifest_content = fs::read_to_string(&manifest_path)?;
        let mut files: Vec<UnpackedFile> = serde_json::from_str(&manifest_content)?;

        for file in &mut files {
            let clean_path = file.path.replace('\\', "/");
            let file_path = input.join(clean_path);
            file.data = fs::read(&file_path)?;
        }
        files
    } else {
        println!("No manifest.json found. Scanning directory: {:?}", input);
        let mut files: Vec<UnpackedFile> = Vec::new(); // Fix type inference
        let walker = walkdir::WalkDir::new(input).into_iter();

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                // Skip hidden files or system files if necessary?
                // For now, include everything except maybe .DS_Store if we wanted to be picky,
                // but let's just include all files.

                let relative_path = path.strip_prefix(input).unwrap();
                let path_str = relative_path.to_string_lossy().replace('\\', "/");

                if path_str.ends_with(".ptx") {
                    println!(
                        "Warning: Packing .ptx file as Part0 (Data). It may not work if the game expects Part1 (Resource)."
                    );
                }

                let data = fs::read(path)?;
                files.push(UnpackedFile {
                    path: path_str,
                    data,
                    is_part1: false,
                    part1_info: None,
                });
            }
        }
        // Sort for deterministic output
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files
    };

    let mut out_file = fs::File::create(output)?;
    pack_rsg(&mut out_file, &unpacked_files, 4, 0)?;

    println!("Packed RSG to {:?}", output);
    Ok(())
}
