use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: analyze_ogg <input.ogg>");
        return Ok(());
    }
    let input_path = &args[1];
    let mut file = File::open(input_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    println!("Analyzing OGG size: {} bytes", data.len());

    let mut position = 0;
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

            let mut current_packet = Vec::new();
            for &seg_len in &segment_table {
                let len = seg_len as usize;
                if body_offset + len > data.len() {
                    println!("Error: segment goes past EOF");
                    break;
                }
                current_packet.extend_from_slice(&data[body_offset..body_offset + len]);
                body_offset += len;

                if seg_len < 255 {
                    packet_idx += 1;
                    println!("  Packet {} ({} bytes):", packet_idx, current_packet.len());
                    if !current_packet.is_empty() {
                        let packet_type = current_packet[0];
                        print!("    Type: {}", packet_type);
                        if packet_type == 1 {
                            print!(" (Identification)");
                        } else if packet_type == 3 {
                            print!(" (Comment)");
                        } else if packet_type == 5 {
                            print!(" (Setup)");
                        }
                        println!();

                        if current_packet.len() >= 7
                            && (packet_type == 1 || packet_type == 3 || packet_type == 5)
                        {
                            if &current_packet[1..7] == b"vorbis" {
                                println!("    Signature: vorbis (Valid)");
                            } else {
                                println!("    Signature: {:?} (Invalid)", &current_packet[1..7]);
                            }
                        }

                        println!(
                            "    Last byte: {:02x} (Framing bit: {})",
                            current_packet[current_packet.len() - 1],
                            current_packet[current_packet.len() - 1] & 1
                        );

                        if true {
                            println!("    Packet HEX DUMP:");
                            for (i, b) in current_packet.iter().enumerate() {
                                if i % 16 == 0 {
                                    print!("\n      {:04x}: ", i);
                                }
                                print!("{:02x} ", b);
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
