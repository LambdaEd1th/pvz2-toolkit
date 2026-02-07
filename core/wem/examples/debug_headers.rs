use std::env;
use std::fs::File;
use std::io::Cursor;
use wem::{CodebookLibrary, WwiseRiffVorbis};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: debug_headers <input.wem>");
        return Ok(());
    }
    let input_path = &args[1];
    let file = File::open(input_path)?;

    // Load codebooks (assuming internal codebooks are used or default)
    // For this debug we try default
    let codebooks = CodebookLibrary::default_codebooks()?;

    let mut converter = WwiseRiffVorbis::new(file, codebooks)?;
    let mut ogg_buffer = Vec::new();
    converter.generate_ogg(&mut ogg_buffer)?;

    println!("Generated OGG size: {} bytes", ogg_buffer.len());

    // Manually parse Ogg pages to find packets
    let cursor = Cursor::new(ogg_buffer);
    let mut position = 0;

    // Using ogg crate would be easier but we don't have it.
    // We can use lewton's OggStreamReader to get packets if new() didn't fail.
    // Since new() fails, we use ogg_packet_reader from lewton?
    // lewton depends on `ogg` crate usually? No, it has `inside_ogg` module.

    // Let's just manually scan for "vorbis" strings and dump surrounding bytes?
    // Or just look for OggS headers.

    let data = cursor.get_ref();

    // Helper to find "OggS"
    while position < data.len() - 4 {
        if &data[position..position + 4] == b"OggS" {
            println!("Found OggS at offset {}", position);
            let version = data[position + 4];
            let flags = data[position + 5];
            let _granule = &data[position + 6..position + 14];
            let _serial = &data[position + 14..position + 18];
            let _seq = &data[position + 18..position + 22];
            let _crc = &data[position + 22..position + 26];
            let segments = data[position + 26] as usize;

            println!("  Version: {}", version);
            println!("  Flags: {:02x}", flags);
            println!("  Segments: {}", segments);

            let mut segment_table = Vec::new();
            for i in 0..segments {
                segment_table.push(data[position + 27 + i]);
            }

            let mut body_offset = position + 27 + segments;
            let mut packet_idx = 0;

            // Reconstruct packets
            let mut current_packet = Vec::new();
            for &seg_len in &segment_table {
                let len = seg_len as usize;
                current_packet.extend_from_slice(&data[body_offset..body_offset + len]);
                body_offset += len;

                if seg_len < 255 {
                    // Packet finished
                    packet_idx += 1;
                    println!("  Packet {} ({} bytes):", packet_idx, current_packet.len());
                    if current_packet.len() >= 7 {
                        let packet_type = current_packet[0];
                        let id = &current_packet[1..7];
                        println!("    Type: {}", packet_type);
                        println!("    ID: {:?} ({})", id, String::from_utf8_lossy(id));

                        // Dump last byte (framing bit)
                        println!(
                            "    Last byte: {:02x}",
                            current_packet[current_packet.len() - 1]
                        );

                        if packet_type == 5 {
                            println!("    Packet 5 HEX:");
                            for (i, b) in current_packet.iter().enumerate() {
                                if i % 32 == 0 {
                                    print!("\n      ");
                                }
                                print!("{:02x}", b);
                            }
                            println!();
                        }
                    } else {
                        println!("    Too short");
                    }
                    current_packet.clear();
                }
            }

            position = body_offset;
        } else {
            position += 1;
        }
    }

    Ok(())
}
