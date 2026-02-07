use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: inspect_aac <file.wem>");
        return Ok(());
    }

    let mut file = File::open(&args[1])?;
    let mut buffer = [0u8; 12];
    file.read_exact(&mut buffer)?;

    if &buffer[0..4] != b"RIFF" && &buffer[0..4] != b"RIFX" {
        println!("Not a RIFF/RIFX file");
        return Ok(());
    }

    let big_endian = &buffer[0..4] == b"RIFX";
    // Skip size (4 bytes)
    if &buffer[8..12] != b"WAVE" {
        println!("Not a WAVE file");
        // return Ok(()); // Continue anyway, XWMA etc might differ but WEM usually WAVE
    }

    println!("Endianness: {}", if big_endian { "Big" } else { "Little" });

    // Iterate chunks
    let mut offset = 12u64;
    loop {
        if file.seek(SeekFrom::Start(offset)).is_err() {
            break;
        }

        let mut chunk_header = [0u8; 8];
        if file.read_exact(&mut chunk_header).is_err() {
            break;
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = if big_endian {
            u32::from_be_bytes(chunk_header[4..8].try_into().unwrap())
        } else {
            u32::from_le_bytes(chunk_header[4..8].try_into().unwrap())
        };

        println!(
            "Chunk: {:?} Size: {}",
            String::from_utf8_lossy(chunk_id),
            chunk_size
        );

        if chunk_id == b"data" {
            println!("Found data chunk at offset {}", offset + 8);
            let mut data_start = [0u8; 16];
            if file.read_exact(&mut data_start).is_ok() {
                println!("Data start: {:02X?}", data_start);
            }
            break;
        } else if chunk_id == b"fmt " {
            println!("Found fmt chunk at offset {}", offset + 8);
            let mut fmt_data = vec![0u8; chunk_size as usize];
            if file.read_exact(&mut fmt_data).is_ok() {
                let tag = if big_endian {
                    u16::from_be_bytes(fmt_data[0..2].try_into().unwrap())
                } else {
                    u16::from_le_bytes(fmt_data[0..2].try_into().unwrap())
                };
                println!("Format Tag: 0x{:04X}", tag);
            }
        }

        offset += 8 + chunk_size as u64;
        // Padding byte if odd size? RIFF usually does, but Wwise might not.
        // vgmstream source says "chunks are even-aligned and don't need to add padding byte, unlike real RIFFs"
        // But let's check.
    }

    Ok(())
}
