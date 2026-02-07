use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use wem::{CodebookLibrary, WwiseRiffVorbis};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = vec![
        "test_output/final_verify/rsb",
        "test_output/final_verify/obb",
    ];

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

    for wem_path in wem_files {
        match convert_wem(&wem_path, &codebooks) {
            Ok(_) => {
                successes += 1;
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("bad codec id") || msg.contains("expected 0x42 fmt") {
                    skipped += 1;
                    // println!("Skipped (incompatible): {:?}", wem_path);
                } else {
                    failures += 1;
                    println!("Failed to convert {:?}: {}", wem_path, e);
                }
            }
        }
    }

    println!("Optimization verification complete.");
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
                && ext.to_string_lossy().to_lowercase() == "wem" {
                    files.push(path);
                }
        }
    }
    Ok(())
}

fn convert_wem(
    input_path: &Path,
    codebooks: &CodebookLibrary,
) -> Result<(), Box<dyn std::error::Error>> {
    let input = BufReader::new(File::open(input_path)?);
    let mut converter = WwiseRiffVorbis::new(input, codebooks.clone())?;

    let output_path = input_path.with_extension("ogg");
    let mut output = BufWriter::new(File::create(&output_path)?);

    converter.generate_ogg(&mut output)?;

    Ok(())
}
