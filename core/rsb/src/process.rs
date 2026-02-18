// use crate::error::Result as RsbResult;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::Rsb;
use crate::types::*;
use crate::writer::RsbWriter;

// use byteorder::{ReadBytesExt, WriteBytesExt};
use rsg::types::UnpackedFile;
use rsg::{pack_rsg, unpack_rsg};

pub fn unpack_rsb(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    let file = fs::File::open(input)?;
    let mut rsb = Rsb::open(file)?;

    let out_dir = match output {
        Some(p) => p.clone(),
        None => {
            let file_stem = input.file_stem().unwrap_or_default();
            PathBuf::from(file_stem)
        }
    };

    if !out_dir.exists() {
        fs::create_dir_all(&out_dir)?;
    }

    println!("Unpacking {:?} to {:?}", input, out_dir);

    // Read all metadata
    let rsg_infos = rsb.read_rsg_info()?;
    let composite_infos = rsb.read_composite_info()?;
    let ptx_infos = rsb.read_ptx_info()?;
    let _autopool_infos = rsb.read_autopool_info()?;

    println!("Found {} RSG packets.", rsg_infos.len());
    println!("Found {} Composite groups.", composite_infos.len());

    let mut group_list = Vec::new();
    let mut rsg_name_list = Vec::new();
    let mut processed_pool_indices = std::collections::HashSet::new();

    // Iterate Composites to drive unpacking (Parity with C#)
    for composite in &composite_infos {
        let mut sub_group_list = Vec::new();

        for packet_entry in &composite.packet_info {
            // Find RSG with pool_index == packet_index
            if let Some(rsg_info) = rsg_infos
                .iter()
                .find(|r| r.pool_index == packet_entry.packet_index)
            {
                if processed_pool_indices.contains(&rsg_info.pool_index) {
                    continue;
                }
                processed_pool_indices.insert(rsg_info.pool_index);

                rsg_name_list.push(rsg_info.name.clone());

                let packet_data = rsb.extract_packet(rsg_info)?;
                // Separate RSB/RSG: Just write the packet file
                let rsg_path = out_dir.join(&rsg_info.name);

                if !rsg_path.exists() {
                    if packet_data.is_empty() {
                        println!("  Packet {} is empty, skipping write.", rsg_info.name);
                    } else {
                        fs::write(&rsg_path, &packet_data)?;
                        println!("  Extracted packet: {}", rsg_info.name);
                    }
                }

                if !packet_data.is_empty() {
                    let mut reader = Cursor::new(&packet_data);

                    match unpack_rsg(&mut reader) {
                        Ok(unpacked_files) => {
                            let res_info_list: Vec<ManifestRes> = unpacked_files
                                .iter()
                                .map(|file| {
                                    // Match PTX info
                                    let mut ptx_info = None;
                                    let mut ptx_property = None;

                                    if let Some(extra) = &file.part1_info {
                                        let global_ptx_idx =
                                            rsg_info.ptx_before_number as usize + extra.id as usize;
                                        if let Some(global_ptx) = ptx_infos.get(global_ptx_idx) {
                                            ptx_info = Some(global_ptx.clone());
                                            ptx_property = Some(ManifestPtxProperty {
                                                format: global_ptx.format,
                                                pitch: global_ptx.pitch,
                                                alpha_size: global_ptx.alpha_size,
                                                alpha_format: global_ptx.alpha_format,
                                            });
                                        }
                                    }

                                    ManifestRes {
                                        path: file.path.clone(),
                                        ptx_info,
                                        ptx_property,
                                    }
                                })
                                .collect();

                            sub_group_list.push(ManifestSubgroup {
                                name_packet: rsg_info.name.clone(),
                                category: packet_entry.category.clone(),
                                packet_info: ManifestPacketInfo {
                                    version: 3,
                                    compression_flags: 0,
                                    res: res_info_list,
                                },
                            });
                        }
                        Err(e) => {
                            eprintln!("  Error parsing RSG {}: {:?}", rsg_info.name, e);
                        }
                    }
                }
            }
        }

        group_list.push(ManifestGroup {
            name: composite.name.clone(),
            is_composite: composite.is_composite,
            subgroup: sub_group_list,
        });
    }

    // Process leftover RSGs (orphans)
    let mut default_subgroups = Vec::new();
    for rsg_info in &rsg_infos {
        if processed_pool_indices.contains(&rsg_info.pool_index) {
            continue;
        }

        rsg_name_list.push(rsg_info.name.clone());

        let packet_data = rsb.extract_packet(rsg_info)?;
        let rsg_path = out_dir.join(&rsg_info.name);

        if !rsg_path.exists() {
            if packet_data.is_empty() {
                println!("  Packet {} is empty, skipping write.", rsg_info.name);
            } else {
                fs::write(&rsg_path, &packet_data)?;
                println!("  Extracted packet: {}", rsg_info.name);
            }
        }

        if !packet_data.is_empty() {
            let mut reader = Cursor::new(&packet_data);

            match unpack_rsg(&mut reader) {
                Ok(unpacked_files) => {
                    let res_info_list: Vec<ManifestRes> = unpacked_files
                        .iter()
                        .map(|file| {
                            let mut ptx_info = None;
                            let mut ptx_property = None;

                            if let Some(extra) = &file.part1_info {
                                let global_ptx_idx =
                                    rsg_info.ptx_before_number as usize + extra.id as usize;
                                if let Some(global_ptx) = ptx_infos.get(global_ptx_idx) {
                                    ptx_info = Some(global_ptx.clone());
                                    ptx_property = Some(ManifestPtxProperty {
                                        format: global_ptx.format,
                                        pitch: global_ptx.pitch,
                                        alpha_size: global_ptx.alpha_size,
                                        alpha_format: global_ptx.alpha_format,
                                    });
                                }
                            }

                            ManifestRes {
                                path: file.path.clone(),
                                ptx_info,
                                ptx_property,
                            }
                        })
                        .collect();

                    default_subgroups.push(ManifestSubgroup {
                        name_packet: rsg_info.name.clone(),
                        category: ["Default".to_string(), "".to_string()], // Default category
                        packet_info: ManifestPacketInfo {
                            version: 3,
                            compression_flags: 0,
                            res: res_info_list,
                        },
                    });
                }
                Err(e) => {
                    eprintln!("  Error parsing RSG {}: {:?}", rsg_info.name, e);
                }
            }
        }
    }

    if !default_subgroups.is_empty() {
        group_list.push(ManifestGroup {
            name: "Default".to_string(),
            is_composite: false,
            subgroup: default_subgroups,
        });
    }

    // Write ManifestInfo (rsb_manifest.json)
    let manifest_info = RsbManifest {
        version: rsb.header.version,
        ptx_info_size: rsb.header.ptx_info_each_length,
        path: RsbPathInfo {
            rsgs: rsg_name_list,
            packet_path: "packet".to_string(),
        },
        group: group_list,
    };

    let manifest_path = out_dir.join("rsb_manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_info)?,
    )?;

    println!("Unpack complete. Manifest written to {:?}", manifest_path);

    // Export description.json if version 3
    if rsb.header.version == 3 {
        let desc = rsb.read_resources_description(out_dir.to_str().unwrap_or("output"))?;
        let desc_path = out_dir.join("description.json");
        fs::write(&desc_path, serde_json::to_string_pretty(&desc)?)?;
        println!("Exported description.json to {:?}", desc_path);
    }

    Ok(())
}

pub fn pack_rsb(input: &Path, output: &Path) -> Result<()> {
    // Read Global Manifest
    let rsb_manifest_path = input.join("rsb_manifest.json");
    let rsb_manifest_content = fs::read_to_string(&rsb_manifest_path)?;
    let rsb_manifest: RsbManifest = serde_json::from_str(&rsb_manifest_content)?;

    struct PackedRsg {
        name: String,
        pool_index: i32,
        ptx_number: u32,
        ptx_before_number: u32,
        data: Vec<u8>,
        packet_head_info: Vec<u8>, // To store 32 bytes head info if available
    }

    let mut packed_rsgs = Vec::new();
    let mut all_files = Vec::new(); // Global file list
    let mut ptx_infos = Vec::new(); // Global PTX list (ordered by RSG -> File)
    let mut composite_infos = Vec::new();
    let mut autopool_infos = Vec::new();

    // Pre-load PTX info map: Path -> RsbPtxInfo
    let mut ptx_info_map: HashMap<String, RsbPtxInfo> = HashMap::new();
    for group in &rsb_manifest.group {
        for sub in &group.subgroup {
            for res in &sub.packet_info.res {
                if let Some(ptx) = &res.ptx_info {
                    let clean_key = res.path.replace('\\', "/");
                    ptx_info_map.insert(clean_key, ptx.clone());
                }
            }
        }
    }

    // We iterate `path.rsgs` to find packets to pack (linear list).
    for (current_pool_index, packet_name) in rsb_manifest.path.rsgs.iter().enumerate() {
        println!("Packing packet: {}", packet_name);
        let rsg_dir = input.join(packet_name);
        let manifest_path = rsg_dir.join("manifest.json");
        let mut rsg_data = Vec::new();
        let mut packet_head_info = vec![0u8; 32];

        let ptx_num;
        let mut current_rsg_ptx_infos: Vec<RsbPtxInfo> = Vec::new(); // Final list after sorting

        // Map path -> ID for this packet
        let mut resource_id_map: HashMap<String, u32> = HashMap::new();

        if manifest_path.exists() {
            let manifest_content = fs::read_to_string(&manifest_path)?;
            let unpacked_files: Vec<UnpackedFile> = serde_json::from_str(&manifest_content)?;

            let mut unpacked_files_for_pack = unpacked_files.clone();

            for file in &mut unpacked_files_for_pack {
                let clean_path = file.path.replace('\\', "/");
                let file_path = rsg_dir.join(&clean_path);
                file.data = fs::read(&file_path)?;

                // Add to global file list
                all_files.push(FileListInfo {
                    name_path: clean_path.clone(),
                    pool_index: current_pool_index as i32,
                });

                if let Some(part1) = &file.part1_info {
                    resource_id_map.insert(clean_path.clone(), part1.id);
                }
            }

            let mut cursor = Cursor::new(&mut rsg_data);
            pack_rsg(&mut cursor, &unpacked_files_for_pack, 4, 0)?;

            if rsg_data.len() >= 32 {
                packet_head_info.copy_from_slice(&rsg_data[..32]);
            }
        } else if input.join(packet_name).exists() {
            // Raw RSG file exists
            let rsg_path = input.join(packet_name);
            rsg_data = fs::read(&rsg_path)?;
            println!("  Using raw RSG file: {:?}", rsg_path);

            if rsg_data.len() >= 32 {
                packet_head_info.copy_from_slice(&rsg_data[..32]);
            }

            // Parse RSG to get IDs
            let mut cursor = Cursor::new(&rsg_data);
            match unpack_rsg(&mut cursor) {
                Ok(files) => {
                    for file in files {
                        if let Some(part1) = file.part1_info {
                            let clean_path = file.path.replace('\\', "/");
                            resource_id_map.insert(clean_path, part1.id);
                        }
                    }
                }
                Err(e) => {
                    println!("  Warning: Failed to parse raw RSG for IDs: {:?}", e);
                }
            }
        } else {
            println!(
                "  No manifest or raw file found for {}, packing empty.",
                packet_name
            );
        }

        // Collect PTX infos with IDs from Global Manifest
        struct PtxEntry {
            info: RsbPtxInfo,
            id: u32,
        }
        let mut collected_ptx_entries = Vec::new();

        if !rsg_data.is_empty() {
            for group in &rsb_manifest.group {
                let subgroups = &group.subgroup;

                for sub in subgroups {
                    if sub.name_packet == *packet_name {
                        // Sum up resources with ptx_info
                        for res in &sub.packet_info.res {
                            // Add to global file list for raw RSG mode
                            if !manifest_path.exists() {
                                // Only if not already added in manifest block
                                all_files.push(FileListInfo {
                                    name_path: res.path.clone(),
                                    pool_index: current_pool_index as i32,
                                });
                            }

                            if let Some(ptx_info) = &res.ptx_info {
                                let clean_key = res.path.replace('\\', "/");
                                let id = *resource_id_map.get(&clean_key).unwrap_or(&0);
                                collected_ptx_entries.push(PtxEntry {
                                    info: ptx_info.clone(),
                                    id,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort PTX entries by ID
        collected_ptx_entries.sort_by_key(|e| e.id);

        // Fill gaps and build final list
        if let Some(max_entry) = collected_ptx_entries.last() {
            let _max_id = max_entry.id;
            let mut final_entries = Vec::new(); // indices 0..max_id
            let mut current_id = 0;

            for entry in &collected_ptx_entries {
                while current_id < entry.id {
                    // Fill gap with dummy
                    final_entries.push(RsbPtxInfo::default());
                    current_id += 1;
                }
                final_entries.push(entry.info.clone());
                current_id += 1;
            }

            ptx_num = final_entries.len() as u32;
            current_rsg_ptx_infos = final_entries;
        } else {
            ptx_num = 0;
        }

        packed_rsgs.push(PackedRsg {
            name: packet_name.clone(),
            pool_index: current_pool_index as i32,
            ptx_number: ptx_num, // Updated count
            ptx_before_number: 0,
            data: rsg_data,
            packet_head_info,
        });

        // Append collected PTX infos
        ptx_infos.extend(current_rsg_ptx_infos);
    }

    // Calculate ptx_before_number
    let mut accum_ptx = 0;
    for rsg in &mut packed_rsgs {
        rsg.ptx_before_number = accum_ptx;
        accum_ptx += rsg.ptx_number;
    }

    // Build Composite Info
    for group in &rsb_manifest.group {
        let mut packet_info_list = Vec::new();
        for sub in &group.subgroup {
            // Find packet index
            if let Some(idx) = packed_rsgs.iter().position(|r| r.name == sub.name_packet) {
                packet_info_list.push(CompositePacketInfo {
                    packet_index: idx as i32,
                    category: sub.category.clone(),
                });
            }
        }

        composite_infos.push(CompositeInfo {
            name: group.name.clone(),
            is_composite: group.is_composite,
            packet_number: packet_info_list.len() as u32,
            packet_info: packet_info_list,
        });
    }

    // AutoPool Info - mirror RSG list for now
    for rsg in &packed_rsgs {
        autopool_infos.push(AutoPoolInfo {
            name: rsg.name.clone(),
            part0_size: 0, // values?
            part1_size: 0,
        });
    }

    // Write RSB
    let mut writer = fs::File::create(output)?;
    let mut rsb_writer = RsbWriter::new(&mut writer);

    // Header
    rsb_writer.write_header(&RsbHeader {
        version: rsb_manifest.version,
        ptx_info_each_length: rsb_manifest.ptx_info_size,
        ..Default::default()
    })?;

    let mut rsb_header_info = RsbHeader::default(); // To track offsets
    rsb_header_info.magic = *b"1bsr";
    rsb_header_info.version = rsb_manifest.version;

    // 1. File List
    let (file_list_begin, file_list_len) = rsb_writer.write_file_list(&all_files)?;
    rsb_header_info.file_list_begin_offset = file_list_begin;
    rsb_header_info.file_list_length = file_list_len;

    if rsb_manifest.version >= 4 {
        // V4 overlap hack: FileList starts at 112, overwriting end of Header reserve (112..120)
        rsb_writer.writer.seek(SeekFrom::Start(112))?;
    }
    let (file_begin, file_len) = rsb_writer.write_file_list(&all_files)?;
    rsb_header_info.file_list_begin_offset = file_begin;
    rsb_header_info.file_list_length = file_len;

    // Reserve RSG Info
    let rsg_info_begin = rsb_writer.writer.stream_position()? as u32;
    // Write empty bytes for RSG infos
    let rsg_count = packed_rsgs.len() as u32;
    let rsg_each_len = 204;
    rsb_writer
        .writer
        .write_all(&vec![0u8; (rsg_count * rsg_each_len) as usize])?;

    rsb_header_info.rsg_info_begin_offset = rsg_info_begin;
    rsb_header_info.rsg_info_each_length = rsg_each_len;
    rsb_header_info.rsg_number = rsg_count;

    // 3. Composite Info
    let (comp_begin, comp_each) = rsb_writer.write_composite_info(&composite_infos)?;
    rsb_header_info.composite_info_begin_offset = comp_begin;
    rsb_header_info.composite_info_each_length = comp_each;
    rsb_header_info.composite_number = composite_infos.len() as u32;

    rsb_header_info.part1_begin_offset = 0;
    rsb_header_info.part2_begin_offset = 0;
    rsb_header_info.part3_begin_offset = 0;

    rsb_header_info.packet_number = packed_rsgs.len() as u32;
    rsb_header_info.packet_info_begin_offset = rsg_info_begin;
    rsb_header_info.packet_info_each_length = rsg_each_len;

    // 4. AutoPool Info
    let (auto_begin, auto_each) = rsb_writer.write_autopool_info(&autopool_infos)?;
    rsb_header_info.autopool_info_begin_offset = auto_begin;
    rsb_header_info.autopool_info_each_length = auto_each;
    rsb_header_info.autopool_number = autopool_infos.len() as u32;

    // 5. PTX Info
    let ptx_begin = rsb_writer.write_ptx_info(&ptx_infos, rsb_manifest.ptx_info_size)?;
    rsb_header_info.ptx_info_begin_offset = ptx_begin;
    rsb_header_info.ptx_info_each_length = rsb_manifest.ptx_info_size;
    rsb_header_info.ptx_number = ptx_infos.len() as u32;

    // 6. Description
    let desc_path = input.join("description.json");
    if desc_path.exists() {
        println!("  Found description.json, writing ResourcesDescription...");
        let desc_content = fs::read_to_string(&desc_path)?;
        let desc: ResourcesDescription = serde_json::from_str(&desc_content)?;
        rsb_writer.write_resources_description(&desc, &mut rsb_header_info)?;
    }

    // Align
    fn align<W: Write + Seek>(w: &mut W) -> Result<()> {
        let pos = w.stream_position()?;
        if pos % 4096 != 0 {
            let pad = 4096 - (pos % 4096);
            w.write_all(&vec![0u8; pad as usize])?;
        }
        Ok(())
    }
    align(&mut rsb_writer.writer)?;

    // 7. Packets
    let mut updated_rsg_infos = Vec::new();
    let mut ptx_counts = Vec::new();

    for rsg in &packed_rsgs {
        let offset = rsb_writer.writer.stream_position()? as u32;
        rsb_writer.writer.write_all(&rsg.data)?;
        let length = rsg.data.len() as u32;
        align(&mut rsb_writer.writer)?;

        updated_rsg_infos.push(RsgInfo {
            name: rsg.name.clone(),
            rsg_offset: offset,
            rsg_length: length,
            pool_index: rsg.pool_index,
            packet_head_info: Some(rsg.packet_head_info.clone()),
            ptx_number: rsg.ptx_number,
            ptx_before_number: rsg.ptx_before_number,
        });
        ptx_counts.push((rsg.ptx_number, rsg.ptx_before_number));
    }
    let file_end = rsb_writer.writer.stream_position()? as u32;
    rsb_header_info.file_offset = file_end;

    // Rewind and write RSG Info
    rsb_writer
        .writer
        .seek(SeekFrom::Start(rsg_info_begin as u64))?;
    rsb_writer.write_rsg_info(&updated_rsg_infos, &ptx_counts)?;

    // Final Header Update
    rsb_writer.writer.seek(SeekFrom::Start(0))?;

    if rsb_manifest.version >= 4 {
        // Recover the overlapping FileList bytes
        rsb_writer.writer.flush()?;

        let mut f = fs::File::open(output)?;
        f.seek(SeekFrom::Start(112))?;
        let mut buf = [0u8; 8];
        f.read_exact(&mut buf)?;

        rsb_header_info.packet_info_begin_offset =
            u32::from_le_bytes(buf[0..4].try_into().unwrap());
        rsb_header_info.packet_info_each_length = u32::from_le_bytes(buf[4..8].try_into().unwrap());

        rsb_writer.writer.seek(SeekFrom::Start(0))?;
    }

    rsb_writer.write_header(&rsb_header_info)?;

    println!("Pack complete. Written to {:?}", output);

    Ok(())
}

pub fn unpack_rsg_batch(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Input is rsb_manifest.json
    let input_dir = input.parent().unwrap_or(Path::new("."));
    let out_dir = match output {
        Some(p) => p.clone(),
        None => input_dir.to_path_buf(),
    };

    println!("Reading manifest from {:?}", input);
    let content = fs::read_to_string(input)?;
    let manifest: RsbManifest = serde_json::from_str(&content)?;

    println!(
        "Processing {} RSG packets from manifest...",
        manifest.path.rsgs.len()
    );

    // Parallel processing would require rayon dependency.
    // To match original CLI behavior exactly, we should add rayon.
    // For now, doing sequential to avoid extra dependency if not strictly needed in core.
    // Or if performance is critical, add rayon to core/rsb dependencies.
    // The CLI implementation used: manifest.group.par_iter()
    // Here we iterate groups.

    for group in &manifest.group {
        for subgroup in &group.subgroup {
            let packet_name = &subgroup.name_packet;
            let packet_path = input_dir.join(packet_name);

            if !packet_path.exists() {
                continue;
            }

            // Output folder for this packet
            let packet_out_dir = out_dir.join(packet_name);
            if fs::create_dir_all(&packet_out_dir).is_err() {
                continue;
            }

            // Read packet data
            if let Ok(data) = fs::read(&packet_path) {
                let mut reader = Cursor::new(&data);
                if let Ok(unpacked_files) = unpack_rsg(&mut reader) {
                    // Write manifest.json for packing
                    let manifest_path = packet_out_dir.join("manifest.json");
                    if let Ok(json) = serde_json::to_string_pretty(&unpacked_files) {
                        let _ = fs::write(&manifest_path, json);
                    }

                    for file in unpacked_files {
                        let clean_path = file.path.replace('\\', "/");
                        let target_path = packet_out_dir.join(&clean_path);

                        if let Some(parent) = target_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::write(&target_path, &file.data);
                    }
                }
            }
        }
    }

    println!("RSG unpack complete.");
    Ok(())
}

pub fn pack_rsg_batch(input: &Path, output: &Path) -> Result<()> {
    // Expect manifest.json inside input folder
    let manifest_path = input.join("manifest.json");
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "manifest.json not found in input directory"
        ));
    }

    let manifest_content = fs::read_to_string(&manifest_path)?;
    let mut unpacked_files: Vec<UnpackedFile> = serde_json::from_str(&manifest_content)?;

    for file in &mut unpacked_files {
        let clean_path = file.path.replace('\\', "/");
        let file_path = input.join(clean_path);
        file.data = fs::read(&file_path)?;
    }

    let mut out_file = fs::File::create(output)?;
    pack_rsg(&mut out_file, &unpacked_files, 4, 0)?;

    println!("Packed RSG to {:?}", output);
    Ok(())
}
