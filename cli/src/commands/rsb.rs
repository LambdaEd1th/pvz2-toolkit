use anyhow::Result;
use clap::Subcommand;
use rsb::{
    Rsb,
    io::writer::RsbWriter,
    ptx::decoder::PtxDecoder,
    rsg::{pack_rsg, types::UnpackedFile, unpack_rsg},
    schema::types::*,
};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum RsbCommands {
    /// Unpack an RSB file
    Unpack {
        /// Input RSB file
        input: PathBuf,
        /// Output directory (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Is target platform PowerVR (iOS) -> PVRTC textures?
        #[arg(short, long)]
        powervr: bool,
    },
    /// Pack a directory into an RSB file
    Pack {
        /// Input directory containing rsb_manifest.json
        input: PathBuf,
        /// Output RSB file
        output: PathBuf,
        /// Is target platform PowerVR (iOS) -> PVRTC textures?
        #[arg(short, long)]
        powervr: bool,
        /// Apply Palette compression trick (experimental)
        #[arg(long)]
        use_palette: bool,
    },
}

pub fn handle(cmd: RsbCommands) -> Result<()> {
    match cmd {
        RsbCommands::Unpack {
            input,
            output,
            powervr,
        } => unpack_rsb(&input, &output, powervr),
        RsbCommands::Pack {
            input,
            output,
            powervr,
            use_palette,
        } => pack_rsb(&input, &output, powervr, use_palette),
    }
}

pub fn pack_rsb(input: &Path, output: &Path, is_powervr: bool, use_palette: bool) -> Result<()> {
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

        let mut unpacked_files_for_pack = Vec::new();
        for group in &rsb_manifest.group {
            for sub in &group.subgroup {
                if sub.name_packet == *packet_name {
                    for res in &sub.packet_info.res {
                        let clean_path = res.path.replace('\\', "/");
                        let file_path = input.join(packet_name).join(&clean_path);

                        let mut data = Vec::new();
                        if file_path
                            .extension()
                            .unwrap_or_default()
                            .eq_ignore_ascii_case("ptx")
                        {
                            let png_path = file_path.with_extension("png");
                            if png_path.exists() {
                                // Encode PNG back to PTX
                                if let Ok(img) = image::open(&png_path) {
                                    let ptx_fmt =
                                        res.ptx_info.as_ref().map(|p| p.format).unwrap_or(0);
                                    let mut format = rsb::ptx::types::PtxFormat::from(ptx_fmt);

                                    // Apply Palette Override
                                    if use_palette && format == rsb::ptx::types::PtxFormat::Etc1A8 {
                                        format = rsb::ptx::types::PtxFormat::Etc1Palette;
                                    }

                                    if let Ok(encoded) = rsb::ptx::encoder::PtxEncoder::encode(
                                        &img, format, is_powervr,
                                    ) {
                                        data = encoded;
                                    }
                                }
                            } else if file_path.exists() {
                                data = fs::read(&file_path).unwrap_or_default();
                            }
                        } else if file_path.exists() {
                            data = fs::read(&file_path).unwrap_or_default();
                        }

                        if !data.is_empty() {
                            unpacked_files_for_pack.push(UnpackedFile {
                                path: res.path.clone(),
                                data,
                                is_part1: res.part1_info.is_some(),
                                part1_info: res.part1_info.clone(),
                            });

                            all_files.push(FileListInfo {
                                name_path: clean_path.clone(),
                                pool_index: current_pool_index as i32,
                            });

                            if let Some(part1) = &res.part1_info {
                                resource_id_map.insert(clean_path.clone(), part1.id);
                            }
                        }
                    }
                }
            }
        }

        if !unpacked_files_for_pack.is_empty() {
            let mut cursor = std::io::Cursor::new(&mut rsg_data);
            pack_rsg(&mut cursor, &unpacked_files_for_pack, 4, 0)?;

            if rsg_data.len() >= 32 {
                packet_head_info.copy_from_slice(&rsg_data[..32]);
            }
        } else {
            println!("  No files found for {}, packing empty.", packet_name);
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

    let mut rsb_header_info = RsbHeader {
        magic: *b"1bsr",
        version: rsb_manifest.version,
        ..Default::default()
    }; // To track offsets

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

pub fn unpack_rsb(input: &Path, output: &Option<PathBuf>, is_powervr: bool) -> Result<()> {
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

                                    // Extract to disk
                                    let clean_path = file.path.replace('\\', "/");
                                    let packet_out_dir = out_dir.join(&rsg_info.name);
                                    let out_file_path = packet_out_dir.join(&clean_path);
                                    if let Some(parent) = out_file_path.parent() {
                                        let _ = fs::create_dir_all(parent);
                                    }
                                    if let Err(e) = fs::write(&out_file_path, &file.data) {
                                        eprintln!(
                                            "Failed to write {}: {:?}",
                                            out_file_path.display(),
                                            e
                                        );
                                    }

                                    // Decode PTX if applicable
                                    if out_file_path
                                        .extension()
                                        .unwrap_or_default()
                                        .eq_ignore_ascii_case("ptx")
                                        && let Some(ptx) = &ptx_info
                                    {
                                        // Primarily use dimensions from the RSG packet internal properties, fallback to global canvas size
                                        let mut width = ptx.width as u32;
                                        let mut height = ptx.height as u32;

                                        if let Some(p1) = file
                                            .part1_info
                                            .as_ref()
                                            .filter(|p| p.width > 0 && p.height > 0)
                                        {
                                            width = p1.width;
                                            height = p1.height;
                                        }

                                        if width > 0 && height > 0 {
                                            match PtxDecoder::decode(
                                                &file.data,
                                                width,
                                                height,
                                                ptx.format,
                                                ptx.alpha_size,
                                                ptx.alpha_format,
                                                is_powervr,
                                            ) {
                                                Ok(img) => {
                                                    let png_path =
                                                        out_file_path.with_extension("png");
                                                    if let Err(e) = img.save(&png_path) {
                                                        eprintln!(
                                                            "Failed to save PNG {}: {:?}",
                                                            png_path.display(),
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => eprintln!(
                                                    "Failed to decode PTX {}: {:?}",
                                                    out_file_path.display(),
                                                    e
                                                ),
                                            }
                                        }
                                    }

                                    ManifestRes {
                                        path: file.path.clone(),
                                        part1_info: file.part1_info.clone(),
                                        ptx_info,
                                        ptx_property,
                                    }
                                })
                                .collect();

                            // Write manifest.json to mimic Sen's rsg unpack state
                            let packet_out_dir = out_dir.join(&rsg_info.name);
                            let manifest_path = packet_out_dir.join("manifest.json");
                            if let Ok(json) = serde_json::to_string_pretty(&unpacked_files) {
                                let _ = fs::write(&manifest_path, json);
                            }

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

                            // Extract to disk
                            let clean_path = file.path.replace('\\', "/");
                            let packet_out_dir = out_dir.join(&rsg_info.name);
                            let out_file_path = packet_out_dir.join(&clean_path);
                            if let Some(parent) = out_file_path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            if let Err(e) = fs::write(&out_file_path, &file.data) {
                                eprintln!("Failed to write {}: {:?}", out_file_path.display(), e);
                            }

                            // Decode PTX if applicable
                            if out_file_path
                                .extension()
                                .unwrap_or_default()
                                .eq_ignore_ascii_case("ptx")
                                && let Some(ptx) = &ptx_info
                            {
                                // Primarily use dimensions from the RSG packet internal properties, fallback to global canvas size
                                let mut width = ptx.width as u32;
                                let mut height = ptx.height as u32;

                                if let Some(p1) = file
                                    .part1_info
                                    .as_ref()
                                    .filter(|p| p.width > 0 && p.height > 0)
                                {
                                    width = p1.width;
                                    height = p1.height;
                                }

                                if width > 0 && height > 0 {
                                    match PtxDecoder::decode(
                                        &file.data,
                                        width,
                                        height,
                                        ptx.format,
                                        ptx.alpha_size,
                                        ptx.alpha_format,
                                        is_powervr,
                                    ) {
                                        Ok(img) => {
                                            let png_path = out_file_path.with_extension("png");
                                            if let Err(e) = img.save(&png_path) {
                                                eprintln!(
                                                    "Failed to save PNG {}: {:?}",
                                                    png_path.display(),
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => eprintln!(
                                            "Failed to decode PTX {}: {:?}",
                                            out_file_path.display(),
                                            e
                                        ),
                                    }
                                }
                            }

                            ManifestRes {
                                path: file.path.clone(),
                                part1_info: file.part1_info.clone(),
                                ptx_info,
                                ptx_property,
                            }
                        })
                        .collect();

                    // Write manifest.json to mimic Sen's rsg unpack state
                    let packet_out_dir = out_dir.join(&rsg_info.name);
                    let manifest_path = packet_out_dir.join("manifest.json");
                    if let Ok(json) = serde_json::to_string_pretty(&unpacked_files) {
                        let _ = fs::write(&manifest_path, json);
                    }

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
