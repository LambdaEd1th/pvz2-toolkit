use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use wem::CodebookLibrary;
use wem::wav::wem_to_wav;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.wem> <output.wav>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    println!("Converting {:?} to {:?}...", input_path, output_path);

    let input = BufReader::new(File::open(input_path)?);
    let mut output = BufWriter::new(File::create(output_path)?);

    let codebooks = CodebookLibrary::default_codebooks()?;

    wem_to_wav(input, &mut output, &codebooks)?;

    println!("Conversion successful!");

    Ok(())
}
