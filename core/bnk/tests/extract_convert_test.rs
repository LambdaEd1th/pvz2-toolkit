use bnk::Bnk;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::process::Command;
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

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    println!("Extracting to temp dir: {:?}", temp_dir.path());

    let mut successes = 0;
    let mut failures = 0;

    for bnk_path in &bnk_files {
        let bnk_stem = bnk_path.file_stem().unwrap();
        let target_dir = temp_dir.path().join(bnk_stem);

        // 1. Extract BNK -> WEMs (to temp dir)
        let status = Command::new(&cli_path)
            .arg("bnk")
            .arg("unpack")
            .arg(bnk_path)
            .arg("--output")
            .arg(&target_dir)
            .current_dir(&root_dir)
            .status()
            .expect("Failed to execute bnk unpack");

        if !status.success() {
            println!("  Failed to extract {:?}", bnk_path);
            failures += 1;
            continue;
        }

        // 2. Find extracted WEMs
        // unpack_bnk creates output directory if specified
        let extract_dir = target_dir;

        if !extract_dir.exists() {
            // Empty BNK or similar
            continue;
        }

        let mut wem_files = Vec::new();
        collect_files(&extract_dir, "wem", &mut wem_files);

        // 3. Convert WEMs -> OGG
        for wem_path in wem_files {
            let status_wem = Command::new(&cli_path)
                .arg("wem")
                .arg("decode")
                .arg(&wem_path)
                // Output to same temp dir (implicit or explicit)
                // Default is same dir, which is fine since it's temp
                .current_dir(&root_dir)
                .status()
                .expect("Failed to execute wem decode");

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
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // We want to extract to a subdirectory "extracted"
    let extract_dir = output_dir.join("extracted");

    let status_extract = Command::new(&cli_path)
        .arg("bnk")
        .arg("unpack")
        .arg(&sample_bnk)
        .arg("--output")
        .arg(&extract_dir)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to extract BNK");
    assert!(status_extract.success(), "Failed to extract sample BNK");

    // 4. Repack it
    // The unpacked files are in extract_dir
    // JSON is at extract_dir/bank_header.json
    // WEMs are at extract_dir (files are directly there? No, loop in unpack_bnk joins out_dir.join(filename))
    // unpack_bnk writes WEMs to out_dir directly.

    let json_path = extract_dir.join("bank_header.json");

    // Check if WEMs are in extract_dir
    let wem_dir = extract_dir.clone();

    let repacked_bnk = output_dir.join("ZOMBOSS_MUSIC_REPACKED.BNK");

    let status_repack = Command::new(&cli_path)
        .arg("bnk")
        .arg("pack")
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
    let verify_dir = output_dir.join("verify");
    let status_verify = Command::new(&cli_path)
        .arg("bnk")
        .arg("unpack")
        .arg(&repacked_bnk)
        .arg("--output")
        .arg(&verify_dir)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to verify repacked BNK");

    assert!(status_verify.success(), "Failed to parse repacked BNK");

    println!("Repack round-trip successful!");
}

#[test]
fn test_pack_wem_round_trip() {
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
    // Note: We need to rely on existing extracted data OR extract it first.
    // Since we can't rely on order of test execution, we should probably check if we can extract one.
    // However, for now, let's assume if it exists we use it.
    // Wait, relying on files created by OTHER tests is bad practice.
    // But this test was pointing to `repack_test/ZOMBOSS_MUSIC` which was created by `test_repack_bnk_round_trip`!
    // This is flaky. I should make this test self-contained or skip if not available.
    // Let's try to extract ZOMBOSS_MUSIC to temp dir if not found?
    // Or just look for ANY wem in final_verify?

    // Let's try to find a WEM in `final_verify` itself (if unpacked previously?)
    // No, `final_verify` structure is `obb/...`.

    // Let's modify this test to check if `ZOMBOSS_MUSIC.BNK` exists, extract it to temp, then run test.
    let sample_bnk = final_verify_dir.join("obb/ZombossGlobalAudio/SOUNDBANKS/ZOMBOSS_MUSIC.BNK");
    if !sample_bnk.exists() {
        println!("Skipping pack-wem test: BNK source not found");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path(); // Use temp dir for output

    let extract_dir = output_dir.join("extracted");

    // Extract BNK to get a WEM
    let status_extract = Command::new(&cli_path)
        .arg("bnk")
        .arg("unpack")
        .arg(&sample_bnk)
        .arg("--output")
        .arg(&extract_dir)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to extract BNK");

    if !status_extract.success() {
        println!("Skipping pack-wem test: Failed to extract BNK source");
        return;
    }

    let wem_dir = extract_dir;

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
            println!("Skipping pack-wem test: No WEM files found in BNK");
            return;
        }
    };

    println!("Testing pack-wem with {:?}", sample_wem);

    // 3. WEM -> OGG
    let ogg_path = output_dir.join("temp.ogg");
    let status_convert = Command::new(&cli_path)
        .arg("wem")
        .arg("decode")
        .arg(&sample_wem)
        .arg("--output")
        .arg(&ogg_path)
        .current_dir(&root_dir)
        .status()
        .expect("Failed to convert WEM to OGG");

    assert!(status_convert.success(), "Failed to convert WEM to OGG");
    assert!(ogg_path.exists(), "OGG file not created");

    // 4. OGG -> WEM (Pack)
    let packed_wem_path = output_dir.join("packed.wem");
    let status_pack = Command::new(&cli_path)
        .arg("wem")
        .arg("encode")
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
        .arg("wem")
        .arg("decode")
        .arg(&packed_wem_path)
        .arg("--output")
        .arg(&verify_ogg_path)
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
    println!("Skipping pack-m4a test: No guaranteed M4A source available.");
}

#[test]
fn test_pack_wav_round_trip() {
    // 1. Setup paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let cli_path = root_dir.join("target/debug/pvz2-toolkit-cli");

    // Use temp dir
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let verify_dir = temp_dir.path().to_path_buf();

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
        .arg("wem")
        .arg("encode")
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
        .arg("wem")
        .arg("decode")
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
