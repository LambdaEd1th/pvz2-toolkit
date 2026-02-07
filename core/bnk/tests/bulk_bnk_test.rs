use bnk::Bnk;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_bulk_parse_test_output_bnks() {
    let mut root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_path.push("../../../rsb-rs/test_output");

    if !root_path.exists() {
        println!(
            "Skipping bulk test: test_output not found at {:?}",
            root_path
        );
        return;
    }

    println!("Scanning for BNK files in {:?}...", root_path);

    let mut bnk_files = Vec::new();
    collect_files(&root_path, "bnk", &mut bnk_files);

    println!("Found {} BNK files.", bnk_files.len());

    let mut successes = 0;
    let mut failures = 0;
    let mut errors = Vec::new();

    for path in &bnk_files {
        match fs::File::open(path) {
            Ok(file) => {
                match Bnk::new(file) {
                    Ok(bnk) => {
                        successes += 1;
                        if bnk.entries.is_empty() {
                            // Warn but not error?
                            // println!("Warning: {:?} parsed but has 0 entries", path.file_name());
                        }
                    }
                    Err(e) => {
                        failures += 1;
                        errors.push(format!("Failed to parse {:?}: {:?}", path.file_name(), e));
                    }
                }
            }
            Err(e) => {
                failures += 1;
                errors.push(format!("Failed to open {:?}: {:?}", path.file_name(), e));
            }
        }
    }

    println!("Bulk BNK Parse Results:");
    println!("  Successes: {}", successes);
    println!("  Failures:  {}", failures);

    if !errors.is_empty() {
        println!("\nFailures details (first 10):");
        for err in errors.iter().take(10) {
            println!("  - {}", err);
        }
    }

    assert_eq!(failures, 0, "Some BNK files failed to parse");
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
