use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use wem::wav::wem_to_wav;
use wem::{CodebookLibrary, WwiseRiffVorbis};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = vec!["test_output"];

    let mut wem_files = Vec::new();
    for dir in dirs {
        if Path::new(dir).exists() {
            find_wem_files(Path::new(dir), &mut wem_files)?;
        } else {
            println!("Warning: Directory not found: {}", dir);
        }
    }

    println!("Found {} .wem files.", wem_files.len());

    let mut successes = 0;
    let mut failures = 0;
    let mut skipped = 0;

    // Use embedded codebooks (aoTuV)
    let codebooks = CodebookLibrary::embedded_aotuv();

    let total = wem_files.len();
    for (i, wem_path) in wem_files.iter().enumerate() {
        if i % 100 == 0 {
            println!("Processing {}/{}...", i, total);
        }
        match convert_wem_to_wav(wem_path, &codebooks) {
            Ok(_) => {
                successes += 1;
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Unsupported format tag") {
                    skipped += 1;
                    println!("Skipped {:?}: {}", wem_path, msg);
                } else if msg.contains("bad codec id") || msg.contains("expected 0x42 fmt") {
                    skipped += 1;
                } else {
                    failures += 1;
                    println!("Failed to convert {:?}: {}", wem_path, e);

                    // Dump bad OGG for inspection
                    if let Ok(file) = File::open(wem_path) {
                        let reader = BufReader::new(file);
                        if let Ok(mut converter) = WwiseRiffVorbis::new(reader, codebooks.clone())
                            && let Ok(mut out) = File::create("bad.ogg")
                        {
                            let _ = converter.generate_ogg(&mut out);
                            println!("Dumped bad.ogg");
                        }
                    }
                    // break; // Don't stop on failure so we can see full results
                }
            }
        }
    }

    println!("WAV Conversion verification complete.");
    println!("Successes: {}", successes);
    println!("Skipped (incompatible): {}", skipped);
    println!("Failures: {}", failures);

    if failures > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn find_wem_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_wem_files(&path, files)?;
            } else if let Some(ext) = path.extension()
                && ext.to_string_lossy().to_lowercase() == "wem"
            {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn convert_wem_to_wav(
    input_path: &Path,
    codebooks: &CodebookLibrary,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = File::open(input_path)?;

    let extension = "wav";
    let mut output_path = input_path.to_path_buf();
    output_path.set_extension(extension);

    let input = BufReader::new(input_file);
    let mut output = BufWriter::new(File::create(&output_path)?);

    wem_to_wav(input, &mut output, codebooks)?;

    Ok(())
}
