use bnk::Bnk;
use std::fs;
use std::io::Seek;
use std::path::{Path, PathBuf};
use wem::CodebookLibrary;
use wem::WwiseRiffVorbis;

#[test]
fn test_bulk_extract_and_convert() {
    // Locate the CLI binary (assuming cargo build/test has run)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // core/bnk
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let cli_path = root_dir.join("target/debug/pvz2-toolkit-cli");

    // Ensure CLI is built
    let status_build = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("pvz2-toolkit-cli")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to build CLI");
    assert!(status_build.success(), "Failed to build CLI binary");

    let final_verify_dir = root_dir.join("test_output/final_verify");
    if !final_verify_dir.exists() {
        println!(
            "Skipping bulk extraction: directory not found at {:?}",
            final_verify_dir
        );
        return;
    }

    println!("Scanning for BNK files in {:?}...", final_verify_dir);
    let mut bnk_files = Vec::new();
    collect_files(&final_verify_dir, "bnk", &mut bnk_files);

    if bnk_files.is_empty() {
        println!("No BNK files found.");
        return;
    }

    println!(
        "Found {} BNK files. Starting extraction & conversion...",
        bnk_files.len()
    );

    let mut successes = 0;
    let mut failures = 0;

    for bnk_path in &bnk_files {
        // 1. Extract BNK -> WEMs
        let status = Command::new(&cli_path)
            .arg("unpack-bnk")
            .arg(bnk_path)
            .current_dir(&root_dir)
            .status()
            .expect("Failed to execute unpack-bnk");

        if !status.success() {
            println!("  Failed to extract {:?}", bnk_path);
            failures += 1;
            continue;
        }

        // 2. Find extracted WEMs
        let bnk_stem = bnk_path.file_stem().unwrap();
        let extract_dir = bnk_path.parent().unwrap().join(bnk_stem);

        if !extract_dir.exists() {
            // Empty BNK or similar
            continue;
        }

        let mut wem_files = Vec::new();
        collect_files(&extract_dir, "wem", &mut wem_files);

        // 3. Convert WEMs -> OGG
        for wem_path in wem_files {
            let status_wem = Command::new(&cli_path)
                .arg("convert-wem")
                .arg(&wem_path)
                .arg("--original") // OGG repacketization
                .current_dir(&root_dir)
                .status()
                .expect("Failed to execute convert-wem");

            if !status_wem.success() {
                println!(
                    "  Failed to convert WEM {:?}",
                    wem_path.file_name().unwrap()
                );
                failures += 1;
            }
        }
        successes += 1;
    }

    println!("Batch Processing Complete.");
    println!("  BNKs Processed: {}", successes + failures);
    println!("  Failures (BNK extraction): {}", failures);
}

use std::process::Command; // Ensure this import exists or add it

#[test]
fn test_find_truncated_wem() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let final_verify_dir = root_dir.join("test_output/final_verify");

    if !final_verify_dir.exists() {
        return;
    }

    // Embed default codebooks for test
    let codebooks = CodebookLibrary::embedded_aotuv();

    println!("Scanning for BNK files...");
    let mut bnk_files = Vec::new();
    collect_files(&final_verify_dir, "bnk", &mut bnk_files);

    let mut truncated_files = Vec::new();

    for bnk_path in bnk_files {
        let file = match fs::File::open(&bnk_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        // Parse BNK
        let mut bnk = match Bnk::new(file) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Re-open file for extraction or seek
        let mut file = fs::File::open(&bnk_path).unwrap();

        if let Some(data_start) = bnk.data_chunk_offset {
            for entry in bnk.data_index {
                if entry.size == 0 {
                    continue;
                }

                let wem_offset = data_start + entry.offset as u64;
                if file.seek(std::io::SeekFrom::Start(wem_offset)).is_err() {
                    continue;
                }

                let mut wem_data = vec![0u8; entry.size as usize];
                if file.read_exact(&mut wem_data).is_err() {
                    continue;
                }

                // Try to parse WEM header logic (check if it's truncated RIFF)
                let cursor = std::io::Cursor::new(&wem_data);
                let reader = std::io::BufReader::new(cursor);

                // Only check Vorbis repacking for "original" mode which triggered the error
                // We don't need full convert, just WwiseRiffVorbis::new
                match WwiseRiffVorbis::new(reader, codebooks.clone()) {
                    Ok(_) => {} // Valid
                    Err(e) => {
                        let err_str = format!("{:?}", e);
                        if err_str.contains("RIFF truncated") {
                            truncated_files.push((
                                bnk_path.file_name().unwrap().to_string_lossy().to_string(),
                                entry.id,
                            ));
                        }
                    }
                }
            }
        }
    }

    if !truncated_files.is_empty() {
        println!("\nFound {} truncated WEM files:", truncated_files.len());
        for (bnk, id) in truncated_files {
            println!("  [{}] {}.wem", bnk, id);
        }
    }
}

fn collect_files(dir: &Path, extension: &str, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, extension, results);
            } else if let Some(ext) = path.extension() {
                if ext.to_string_lossy().eq_ignore_ascii_case(extension) {
                    results.push(path);
                }
            }
        }
    }
}

use std::io::Read;

#[test]
fn test_repack_bnk_round_trip() {
    // 1. Setup paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let cli_path = root_dir.join("target/debug/pvz2-toolkit-cli");
    let final_verify_dir = root_dir.join("test_output/final_verify");

    // Ensure CLI is built
    let status_build = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("pvz2-toolkit-cli")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to build CLI");
    assert!(status_build.success(), "Failed to build CLI");

    // 2. Pick a sample BNK (ZOMBOSS_MUSIC.BNK is good, has embedded media)
    let sample_bnk = final_verify_dir.join("obb/ZombossGlobalAudio/SOUNDBANKS/ZOMBOSS_MUSIC.BNK");
    if !sample_bnk.exists() {
        println!(
            "Skipping repack test: sample BNK not found at {:?}",
            sample_bnk
        );
        return;
    }

    // 3. Extract it first (to get JSON and WEMs)
    let output_dir = final_verify_dir.join("repack_test");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).unwrap();
    }
    fs::create_dir_all(&output_dir).unwrap();

    let json_path = output_dir.join("ZOMBOSS_MUSIC.json");

    let status_extract = Command::new(&cli_path)
        .arg("unpack-bnk")
        .arg(&sample_bnk)
        .arg("--output")
        .arg(&json_path)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to extract BNK");
    assert!(status_extract.success(), "Failed to extract sample BNK");

    // 4. Repack it
    // The extracted WEMs are in output_dir/ZOMBOSS_MUSIC
    let wem_dir = output_dir.join("ZOMBOSS_MUSIC");
    let repacked_bnk = output_dir.join("ZOMBOSS_MUSIC_REPACKED.BNK");

    let status_repack = Command::new(&cli_path)
        .arg("repack-bnk")
        .arg("--json")
        .arg(&json_path)
        .arg("--wems")
        .arg(&wem_dir)
        .arg("--output")
        .arg(&repacked_bnk)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to repack BNK");

    assert!(status_repack.success(), "Failed to repack BNK");

    // 5. Verify Structure (Size might differ due to padding/order, but should be parsable)
    // Try to extract the repacked BNK to verify it's valid
    let repacked_json = output_dir.join("ZOMBOSS_MUSIC_REPACKED.json");
    let status_verify = Command::new(&cli_path)
        .arg("unpack-bnk")
        .arg(&repacked_bnk)
        .arg("--output")
        .arg(&repacked_json)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to verify repacked BNK");

    assert!(status_verify.success(), "Failed to parse repacked BNK");

    println!("Repack round-trip successful!");
}
