use crate::error::{Result, RsbError};
use crate::file_list::read_file_list;
use crate::rsg::types::{Part0Info, Part1Extra, Part1Info, RsgPayload, UnpackedFile};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Seek, SeekFrom, Write};

pub fn unpack_rsg(reader: &mut (impl Read + Seek)) -> Result<Vec<UnpackedFile>> {
    let start_pos = reader.stream_position()?;

    // Read Header
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"pgsr" {
        return Err(RsbError::InvalidMagic(
            "pgsr".to_string(),
            String::from_utf8_lossy(&magic).to_string(),
        ));
    }

    let version = reader.read_u32::<LE>()?;
    if version != 3 && version != 4 {
        return Err(RsbError::InvalidVersion(version));
    }
    reader.read_u64::<LE>()?; // Skip 8 bytes

    let flags = reader.read_u32::<LE>()?;

    // Read rest of header
    let _file_offset = reader.read_u32::<LE>()?;
    let part0_offset = reader.read_u32::<LE>()?;
    let part0_zlib = reader.read_u32::<LE>()?;
    let part0_size = reader.read_u32::<LE>()?;
    reader.read_u32::<LE>()?; // Skip 4
    let part1_offset = reader.read_u32::<LE>()?;
    let part1_zlib = reader.read_u32::<LE>()?;
    let part1_size = reader.read_u32::<LE>()?;
    reader.read_u64::<LE>()?; // Skip 20 bytes
    reader.read_u64::<LE>()?;
    reader.read_u32::<LE>()?;

    let file_list_length = reader.read_u32::<LE>()?;
    let file_list_offset = reader.read_u32::<LE>()?;

    let files = read_file_list::<RsgPayload, _>(
        reader,
        start_pos + file_list_offset as u64,
        file_list_length as u64,
    )?;

    // Helper to get data
    let mut outputs = Vec::new();

    // Read Data Blobs
    // Part0 Data
    let part0_data = read_packet_data(
        reader,
        start_pos,
        part0_offset as u64,
        part0_size as u64,
        part0_zlib as u64,
        flags,
        false,
    )?;

    // Part1 Data (Atlas/Textures)
    let part1_data = read_packet_data(
        reader,
        start_pos,
        part1_offset as u64,
        part1_size as u64,
        part1_zlib as u64,
        flags,
        true,
    )?;

    for (path, payload) in files {
        let (data_slice, info_offset, info_size, is_part1, part1_info) = match payload {
            RsgPayload::Part0(info) => (&part0_data, info.offset, info.size, false, None),
            RsgPayload::Part1(info) => (
                &part1_data,
                info.offset,
                info.size,
                true,
                Some(Part1Extra {
                    id: info.id,
                    width: info.width,
                    height: info.height,
                }),
            ),
        };

        let end_idx = info_offset as usize + info_size as usize;
        if end_idx <= data_slice.len() {
            let file_bytes = data_slice[info_offset as usize..end_idx].to_vec();
            outputs.push(UnpackedFile {
                path,
                data: file_bytes,
                is_part1,
                part1_info,
            });
        } else {
            // OOB
            eprintln!("Warning: OOB read for {}", path);
        }
    }

    Ok(outputs)
}

pub fn pack_rsg<W: Write + Seek>(
    writer: &mut W,
    files: &[UnpackedFile],
    version: u32,
    flags: u32,
) -> Result<()> {
    let start_pos = writer.stream_position()?;

    let mut part0_buffer = Vec::new();
    let mut part1_buffer = Vec::new();

    let mut part0_items: Vec<(usize, u32, u32)> = Vec::new(); // (file_index, offset, size)
    let mut part1_items: Vec<(usize, u32, u32)> = Vec::new(); // (file_index, offset, size)

    fn get_padding(len: usize) -> usize {
        if len.is_multiple_of(4096) {
            0
        } else {
            4096 - (len % 4096)
        }
    }

    for (i, file) in files.iter().enumerate() {
        let data = &file.data;
        let padding = get_padding(data.len());

        if file.is_part1 {
            let offset = part1_buffer.len() as u32;
            part1_buffer.extend_from_slice(data);
            part1_buffer.extend(std::iter::repeat_n(0, padding));
            part1_items.push((i, offset, data.len() as u32));
        } else {
            let offset = part0_buffer.len() as u32;
            part0_buffer.extend_from_slice(data);
            part0_buffer.extend(std::iter::repeat_n(0, padding));
            part0_items.push((i, offset, data.len() as u32));
        }
    }

    let mut part0_lookup = std::collections::HashMap::new();
    for (idx, off, sz) in part0_items {
        part0_lookup.insert(idx, (off, sz));
    }
    let mut part1_lookup = std::collections::HashMap::new();
    for (idx, off, sz) in part1_items {
        part1_lookup.insert(idx, (off, sz));
    }

    let mut file_list_entries: Vec<(String, RsgPayload)> = Vec::with_capacity(files.len());

    for (i, file) in files.iter().enumerate() {
        let payload = if file.is_part1 {
            let (off, sz) = part1_lookup[&i];
            let info = file.part1_info.as_ref().ok_or(RsbError::InvalidMagic(
                "Missing Part1 Info".into(),
                "".into(),
            ))?;
            RsgPayload::Part1(Part1Info {
                offset: off,
                size: sz,
                id: info.id,
                width: info.width,
                height: info.height,
            })
        } else {
            let (off, sz) = part0_lookup[&i];
            RsgPayload::Part0(Part0Info {
                offset: off,
                size: sz,
            })
        };
        file_list_entries.push((file.path.clone(), payload));
    }

    writer.write_all(b"pgsr")?;
    writer.write_u32::<LE>(version)?;
    writer.write_u64::<LE>(0)?;
    writer.write_u32::<LE>(flags)?;

    let _header_offsets_pos = writer.stream_position()?;
    writer.write_all(&[0u8; 96])?;

    let file_list_offset = (writer.stream_position()? - start_pos) as u32;

    let file_list_begin = writer.stream_position()?;
    use crate::file_list::write_file_list;
    write_file_list(writer, file_list_begin, &file_list_entries)?;
    let file_list_end = writer.stream_position()?;
    let file_list_len = (file_list_end - file_list_begin) as u32;

    fn align_file<W: Write + Seek>(w: &mut W) -> Result<()> {
        let pos = w.stream_position()?;
        if pos % 4096 != 0 {
            let pad = 4096 - (pos % 4096);
            w.write_all(&vec![0u8; pad as usize])?;
        }
        Ok(())
    }
    align_file(writer)?;

    let part0_start_offset = (writer.stream_position()? - start_pos) as u32;
    let mut part0_zlib_len = part0_buffer.len() as u32;
    let part0_final_len = part0_buffer.len() as u32;

    if flags >= 2 && !part0_buffer.is_empty() {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&part0_buffer)?;
        let compressed = e.finish()?;
        writer.write_all(&compressed)?;
        part0_zlib_len = compressed.len() as u32;
        let pad = get_padding(compressed.len());
        writer.write_all(&vec![0u8; pad])?;
        part0_zlib_len += pad as u32;
    } else {
        writer.write_all(&part0_buffer)?;
    }

    let part1_start_offset = (writer.stream_position()? - start_pos) as u32;
    let mut part1_zlib_len = part1_buffer.len() as u32;
    let part1_final_len = part1_buffer.len() as u32;

    if flags >= 2 && !part1_buffer.is_empty() {
        if flags == 0 || flags == 2 {
            writer.write_all(&part1_buffer)?;
        } else {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
            e.write_all(&part1_buffer)?;
            let compressed = e.finish()?;
            writer.write_all(&compressed)?;
            part1_zlib_len = compressed.len() as u32;
            let pad = get_padding(compressed.len());
            writer.write_all(&vec![0u8; pad])?;
            part1_zlib_len += pad as u32;
        }
    } else {
        writer.write_all(&part1_buffer)?;
    }

    let end_pos = writer.stream_position()?;
    writer.seek(SeekFrom::Start(start_pos + 20))?;

    let total_size = (end_pos - start_pos) as u32;
    writer.write_u32::<LE>(total_size)?;

    writer.write_u32::<LE>(part0_start_offset)?;
    writer.write_u32::<LE>(part0_zlib_len)?;
    writer.write_u32::<LE>(part0_final_len)?;

    writer.write_u32::<LE>(0)?;

    writer.write_u32::<LE>(part1_start_offset)?;
    writer.write_u32::<LE>(part1_zlib_len)?;
    writer.write_u32::<LE>(part1_final_len)?;

    writer.write_all(&[0u8; 20])?;

    writer.write_u32::<LE>(file_list_len)?;
    writer.write_u32::<LE>(file_list_offset)?;

    writer.seek(SeekFrom::Start(end_pos))?;
    Ok(())
}

fn read_packet_data(
    reader: &mut (impl Read + Seek),
    start_pos: u64,
    offset: u64,
    size: u64,
    z_size: u64,
    flags: u32,
    is_atlas: bool,
) -> Result<Vec<u8>> {
    if size == 0 {
        return Ok(Vec::new());
    }

    let read_offset = start_pos + offset;
    reader.seek(SeekFrom::Start(read_offset))?;

    let mut raw_data = vec![0u8; z_size as usize];
    reader.read_exact(&mut raw_data)?;

    // Zlib check logic
    // Unused strict check omitted or simplified

    let is_zlib_header = |b: &[u8]| -> bool {
        if b.len() < 2 {
            return false;
        }
        b[0] == 0x78 && (b[1] == 0x01 || b[1] == 0x5E || b[1] == 0x9C || b[1] == 0xDA)
    };

    let actually_zlib = if is_atlas {
        if flags == 0 || flags == 2 {
            // Check if it LOOKS like zlib despite flags
            is_zlib_header(&raw_data)
        } else {
            true
        }
    } else if flags < 2 {
        is_zlib_header(&raw_data)
    } else {
        true
    };

    if actually_zlib {
        let mut d = ZlibDecoder::new(&raw_data[..]);
        let mut out = Vec::new(); // should reserve size?
        d.read_to_end(&mut out).map_err(|_| RsbError::Zlib)?;
        Ok(out)
    } else {
        // If size != z_size, and not zlib, what is it?
        // Should verify size matches.
        Ok(raw_data)
    }
}
