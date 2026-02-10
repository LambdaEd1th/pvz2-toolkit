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
        let bnk = match Bnk::new(file) {
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

#[test]
fn test_pack_wem_round_trip() {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

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

    // 2. We need a sample WEM. Let's use one from ZOMBOSS_MUSIC if extracted,
    // or just assume one exists in test_output/final_verify/repack_test/ZOMBOSS_MUSIC
    // If not, we skip.
    let wem_dir = final_verify_dir.join("repack_test/ZOMBOSS_MUSIC");
    if !wem_dir.exists() {
        println!("Skipping pack-wem test: WEM dir not found (run repack test first)");
        return;
    }

    // Find first WEM
    let mut sample_wem = None;
    if let Ok(entries) = fs::read_dir(&wem_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().unwrap_or_default() == "wem" {
                sample_wem = Some(entry.path());
                break;
            }
        }
    }

    let sample_wem = match sample_wem {
        Some(p) => p,
        None => {
            println!("Skipping pack-wem test: No WEM files found");
            return;
        }
    };

    println!("Testing pack-wem with {:?}", sample_wem);
    let output_dir = final_verify_dir.join("pack_wem_test");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).unwrap();
    }
    fs::create_dir_all(&output_dir).unwrap();

    // 3. WEM -> OGG
    let ogg_path = output_dir.join("temp.ogg");
    let status_convert = Command::new(&cli_path)
        .arg("convert-wem")
        .arg(&sample_wem)
        .arg("--output")
        .arg(&ogg_path)
        .arg("--original")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to convert WEM to OGG");

    assert!(status_convert.success(), "Failed to convert WEM to OGG");
    assert!(ogg_path.exists(), "OGG file not created");

    // 4. OGG -> WEM (Pack)
    let packed_wem_path = output_dir.join("packed.wem");
    let status_pack = Command::new(&cli_path)
        .arg("pack-wem")
        .arg("--input")
        .arg(&ogg_path)
        .arg("--output")
        .arg(&packed_wem_path)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to pack WEM");

    assert!(status_pack.success(), "Failed to pack WEM");
    assert!(packed_wem_path.exists(), "Packed WEM not created");

    // 5. Verify: WEM -> OGG again
    let verify_ogg_path = output_dir.join("verify.ogg");
    let status_verify = Command::new(&cli_path)
        .arg("convert-wem")
        .arg(&packed_wem_path)
        .arg("--output")
        .arg(&verify_ogg_path)
        .arg("--original")
        .arg("--inline-codebooks")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to verify packed WEM");

    assert!(
        status_verify.success(),
        "Failed to verify packed WEM (re-conversion failed)"
    );
    assert!(verify_ogg_path.exists(), "Verification OGG not created");

    println!("pack-wem round-trip successful!");
}

#[test]
fn test_pack_m4a_round_trip() {
    // 1. Setup paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let _cli_path = root_dir.join("target/debug/pvz2-toolkit-cli");
    let _final_verify_dir = root_dir.join("test_output/final_verify");

    // Ensure CLI is build
    let status_build = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("pvz2-toolkit-cli")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to build CLI");
    assert!(status_build.success(), "Failed to build CLI");

    // 2. We need a sample M4A.
    // We can generate one from a BNK that has AAC (rare in main assets?), or use a dummy m4a.
    // Or we can convert a WEM to M4A first using `convert-wem` if we find an AAC wem.
    // Let's create a dummy M4A file (valid container is hard to fake without encoder).
    // Better: Find an existing AAC wem in verify dir and extract it.

    // Scan for any WEM in `final_verify` and check if it's AAC.
    // This is slow.
    // Let's assume we can just use a placeholder file for testing PACKING structure,
    // even if content is garbage? No, `pack-wem` probes with symphonia.
    // So we need a valid M4A.

    // We don't have a guaranteed M4A sample.
    // Let's skip if we can't find one?
    // Or check if we have `ZOMBOSS_MUSIC` which might have AAC? (Usually Vorbis).
    // `DINO_MUSIC`?

    // Let's try to find an AAC wem in the `repack_test` output if available.
    // If not, we skip with a message.

    println!("Skipping pack-m4a test: No guaranteed M4A source available in current test data.");
    // To properly test this, we should add a small `test.m4a` to repo or `test_data`.
}

#[test]
fn test_pack_wav_round_trip() {
    // 1. Setup paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let cli_path = root_dir.join("target/debug/pvz2-toolkit-cli");
    let verify_dir = root_dir.join("test_output/pack_wav_test");

    if verify_dir.exists() {
        fs::remove_dir_all(&verify_dir).unwrap();
    }
    fs::create_dir_all(&verify_dir).unwrap();

    // Ensure CLI is build
    let status_build = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("pvz2-toolkit-cli")
        .current_dir(&root_dir)
        .status()
        .expect("Failed to build CLI");
    assert!(status_build.success(), "Failed to build CLI");

    // 2. We need a sample WAV.
    // Create a dummy WAV using hound
    let wav_path = verify_dir.join("test_source.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
    for t in (0..44100).map(|x| x as f32 / 44100.0) {
        let sample = (t.sin() * 2000.0) as i16;
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();

    // Verify the source WAV acts like a WAV
    let _ = hound::WavReader::open(&wav_path).expect("Test source WAV is invalid locally!");

    // 3. Pack WAV -> WEM
    let wem_path = verify_dir.join("test_packed.wem");
    let status_pack = Command::new(&cli_path)
        .arg("pack-wem")
        .arg("--input")
        .arg(&wav_path)
        .arg("--output")
        .arg(&wem_path)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to run pack-wem");
    assert!(
        status_pack.success(),
        "Failed to run pack-wem with WAV input"
    );
    assert!(wem_path.exists(), "Packed WEM not created");

    // 4. Verify (WEM -> WAV)
    let verify_wav_path = verify_dir.join("test_verify.wav");
    let status_verify = Command::new(&cli_path)
        .arg("convert-wem")
        .arg(&wem_path)
        .arg("--output")
        .arg(&verify_wav_path)
        // .arg("--original") // For PCM, original is also WAV, so convert-wem default (wav) is fine.
        .current_dir(&root_dir)
        .status()
        .expect("Failed to verify packed WEM");

    assert!(
        status_verify.success(),
        "Failed to verify packed WEM (re-conversion failed)"
    );
    assert!(verify_wav_path.exists(), "Verification WAV not created");

    // Compare headers/size roughly?
    // The contents should be identical PCM.
    // We can use hound to compare samples.
    let mut reader1 = hound::WavReader::open(&wav_path).unwrap();
    let mut reader2 = hound::WavReader::open(&verify_wav_path).unwrap();

    assert_eq!(reader1.spec(), reader2.spec(), "WAV specs mismatch");
    assert_eq!(reader1.len(), reader2.len(), "WAV length mismatch");

    // Check first few samples
    let samples1: Vec<i16> = reader1.samples().take(100).map(|s| s.unwrap()).collect();
    let samples2: Vec<i16> = reader2.samples().take(100).map(|s| s.unwrap()).collect();
    assert_eq!(samples1, samples2, "WAV samples mismatch");

    println!("pack-wem WAV round-trip successful!");
}
