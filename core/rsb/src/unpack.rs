use crate::Rsb;
use crate::error::Result;
use crate::ptx::decoder::PtxDecoder;
use crate::rsg::unpack_rsg;
use crate::types::*;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

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
