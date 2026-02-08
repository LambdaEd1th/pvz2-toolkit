use bnk::Bnk;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_final_verify_bnks() {
    let mut root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up from core/bnk/tests -> core/bnk -> core -> pvz2-toolkit
    root_path.push("../../test_output/final_verify");

    if !root_path.exists() {
        println!(
            "Skipping final_verify test: directory not found at {:?}",
            root_path
        );
        return;
    }

    println!("Scanning for BNK files in {:?}...", root_path);

    let mut bnk_files = Vec::new();
    collect_files(&root_path, "bnk", &mut bnk_files);

    if bnk_files.is_empty() {
        println!("No BNK files found in final_verify.");
        return;
    }

    println!("Found {} BNK files.", bnk_files.len());

    let mut successes = 0;
    let mut failures = 0;
    let mut errors = Vec::new();

    for path in &bnk_files {
        match fs::File::open(path) {
            Ok(file) => match Bnk::new(file) {
                Ok(_) => {
                    successes += 1;
                }
                Err(e) => {
                    failures += 1;
                    errors.push(format!(
                        "Failed to parse {:?}: {:?}",
                        path.file_name().unwrap_or_default(),
                        e
                    ));
                }
            },
            Err(e) => {
                failures += 1;
                errors.push(format!(
                    "Failed to open {:?}: {:?}",
                    path.file_name().unwrap_or_default(),
                    e
                ));
            }
        }
    }

    println!("Final Verify Results:");
    println!("  Total:     {}", bnk_files.len());
    println!("  Successes: {}", successes);
    println!("  Failures:  {}", failures);

    if !errors.is_empty() {
        println!("\nFailures details:");
        for err in errors {
            println!("  - {}", err);
        }
    }

    assert_eq!(
        failures, 0,
        "Some BNK files in final_verify failed to parse"
    );
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
