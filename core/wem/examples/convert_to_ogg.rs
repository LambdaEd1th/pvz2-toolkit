use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Seek};
use std::path::{Path, PathBuf};
use wem::wav::{get_wem_format, wem_to_wav};
use wem::{CodebookLibrary, WwiseRiffVorbis};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let dirs = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec!["test_output".to_string()]
    };

    let mut wem_files = Vec::new();
    for dir in &dirs {
        if Path::new(dir).exists() {
            find_wem_files(Path::new(dir), &mut wem_files)?;
        } else {
            println!("Warning: Directory not found: {}", dir);
        }
    }

    println!("Found {} .wem files.", wem_files.len());

    // Use embedded codebooks (aoTuV)
    let codebooks = CodebookLibrary::embedded_aotuv();

    let total = wem_files.len();
    let mut ogg_count = 0;
    let mut m4a_count = 0;
    let mut wav_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;

    for (i, wem_path) in wem_files.iter().enumerate() {
        if i % 100 == 0 {
            println!("Processing {}/{}...", i, total);
        }

        match convert_wem(wem_path, &codebooks) {
            Ok(format) => match format {
                "ogg" => ogg_count += 1,
                "m4a" => m4a_count += 1,
                "wav" => wav_count += 1,
                _ => {}
            },
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Unsupported format tag") {
                    skipped_count += 1;
                    // println!("Skipped {:?}: {}", wem_path, msg);
                } else {
                    failed_count += 1;
                    println!("Failed {:?}: {}", wem_path, e);
                }
            }
        }
    }

    println!("Conversion complete.");
    println!("OGG (Vorbis): {}", ogg_count);
    println!("M4A (AAC): {}", m4a_count);
    println!("WAV (ADPCM/PCM): {}", wav_count);
    println!("Skipped: {}", skipped_count);
    println!("Failed: {}", failed_count);

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
) -> Result<&'static str, Box<dyn std::error::Error>> {
    let mut input_file = File::open(input_path)?;

    // Check format
    let format_tag = get_wem_format(&mut input_file)?;
    input_file.seek(std::io::SeekFrom::Start(0))?;

    match format_tag {
        0xFFFF => {
            // Vorbis -> OGG
            let mut output_path = input_path.to_path_buf();
            output_path.set_extension("ogg");

            let mut output = BufWriter::new(File::create(&output_path)?);
            let reader = BufReader::new(input_file);

            let mut converter = WwiseRiffVorbis::new(reader, codebooks.clone())?;
            converter.generate_ogg(&mut output)?;
            Ok("ogg")
        }
        0xAAC0 => {
            // AAC -> M4A (Extract)
            let mut output_path = input_path.to_path_buf();
            output_path.set_extension("m4a");

            let mut output = BufWriter::new(File::create(&output_path)?);
            let reader = BufReader::new(input_file);
            wem_to_wav(reader, &mut output, codebooks)?;
            Ok("m4a")
        }

        0x8311 | 0x0001 | 0xFFFE => {
            // ADPCM/PCM -> WAV
            let mut output_path = input_path.to_path_buf();
            output_path.set_extension("wav");

            let mut output = BufWriter::new(File::create(&output_path)?);
            let reader = BufReader::new(input_file);
            wem_to_wav(reader, &mut output, codebooks)?;
            Ok("wav")
        }
        _ => Err(format!("Unsupported format tag: 0x{:04X}", format_tag).into()),
    }
}
