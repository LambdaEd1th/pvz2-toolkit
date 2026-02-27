use crate::error::{NewtonError, Result};
use crate::types::{
    MResourceGroup, MSubgroupWrapper, ResourceType, ShellSubgroupData, SubgroupWrapper,
};
use byteorder::{LE, ReadBytesExt};
use std::io::Read;

pub fn decode_newton(mut reader: impl Read) -> Result<MResourceGroup> {
    let slot_count = reader.read_u32::<LE>()?;
    let groups_count = reader.read_u32::<LE>()?;
    let mut groups = Vec::with_capacity(groups_count as usize);

    for _ in 0..groups_count {
        let group_type_byte = reader.read_u8()?;
        let group_type = match group_type_byte {
            1 => "composite",
            2 => "simple",
            _ => {
                return Err(NewtonError::DeserializationError(format!(
                    "Unknown group type: {}",
                    group_type_byte
                )));
            }
        }
        .to_string();

        let res_val = reader.read_u32::<LE>()?;
        let res = if res_val != 0 {
            Some(res_val.to_string())
        } else {
            None
        };

        let subgroups_count = reader.read_u32::<LE>()?;
        let resources_count = reader.read_u32::<LE>()?;

        let version = reader.read_u8()?;
        if version != 1 {
            return Err(NewtonError::DeserializationError(format!(
                "Unknown version number: {}",
                version
            )));
        }

        let group_has_parent = reader.read_u8()?;
        let id = read_string(&mut reader)?;

        let parent = if group_has_parent != 0 {
            Some(read_string(&mut reader)?)
        } else {
            None
        };

        let mut subgroups = None;
        let mut resources = None;

        if group_type == "composite" {
            if resources_count != 0 {
                return Err(NewtonError::DeserializationError(
                    "Composite group cannot have resources".into(),
                ));
            }
            let mut sub_list = Vec::with_capacity(subgroups_count as usize);
            for _ in 0..subgroups_count {
                let sub_res_val = reader.read_u32::<LE>()?;
                let sub_res = if sub_res_val != 0 {
                    Some(sub_res_val.to_string())
                } else {
                    None
                };
                let sub_id = read_string(&mut reader)?;
                sub_list.push(SubgroupWrapper {
                    id: sub_id,
                    res: sub_res,
                });
            }
            subgroups = Some(sub_list);
        } else if group_type == "simple" {
            if subgroups_count != 0 {
                return Err(NewtonError::DeserializationError(
                    "Simple group cannot have subgroups".into(),
                ));
            }
            let mut res_list = Vec::with_capacity(resources_count as usize);
            for _ in 0..resources_count {
                let res_type_byte = reader.read_u8()?;
                let res_type_enum = ResourceType::from_u8(res_type_byte)?;
                let res_type_str = match res_type_enum {
                    ResourceType::Image => "Image",
                    ResourceType::PopAnim => "PopAnim",
                    ResourceType::SoundBank => "SoundBank",
                    ResourceType::File => "File",
                    ResourceType::PrimeFont => "PrimeFont",
                    ResourceType::RenderEffect => "RenderEffect",
                    ResourceType::DecodedSoundBank => "DecodedSoundBank",
                }
                .to_string();

                let slot = reader.read_u32::<LE>()?;
                let width = reader.read_u32::<LE>()?;
                let height = reader.read_u32::<LE>()?;
                let x = reader.read_i32::<LE>()?;
                let y = reader.read_i32::<LE>()?;
                let ax = reader.read_u32::<LE>()?;
                let ay = reader.read_u32::<LE>()?;
                let aw = reader.read_u32::<LE>()?;
                let ah = reader.read_u32::<LE>()?;
                let cols = reader.read_u32::<LE>()?;
                let rows = reader.read_u32::<LE>()?;
                let is_atlas = reader.read_u8()? != 0;

                // Logic from C# "is_sprite"
                let is_sprite = aw != 0 && ah != 0;

                let r_wrapper_slot = slot;
                let r_wrapper_width = if width != 0 { Some(width) } else { None };
                let r_wrapper_height = if height != 0 { Some(height) } else { None };
                let r_wrapper_x = if x != 2147483647 && x != 0 {
                    Some(x)
                } else {
                    None
                };
                let r_wrapper_y = if y != 2147483647 && y != 0 {
                    Some(y)
                } else {
                    None
                };

                let r_wrapper_ax = if is_sprite { Some(ax) } else { None };
                let r_wrapper_ay = if is_sprite { Some(ay) } else { None };

                let r_wrapper_aw = if aw != 0 { Some(aw) } else { None };
                let r_wrapper_ah = if ah != 0 { Some(ah) } else { None };

                let r_wrapper_cols = if cols != 1 { Some(cols) } else { None };
                let r_wrapper_rows = if rows != 1 { Some(rows) } else { None };

                reader.read_u8()?; // skip
                reader.read_u8()?; // skip
                let resource_has_parent = reader.read_u8()?;

                let id = read_string(&mut reader)?;
                let path = read_string(&mut reader)?;

                let parent = if resource_has_parent != 0 {
                    Some(read_string(&mut reader)?)
                } else {
                    None
                };

                let mut resource_x = MSubgroupWrapper {
                    res_type: res_type_str,
                    slot: r_wrapper_slot,
                    id,
                    path,
                    width: r_wrapper_width,
                    height: r_wrapper_height,
                    x: r_wrapper_x,
                    y: r_wrapper_y,
                    ax: r_wrapper_ax,
                    ay: r_wrapper_ay,
                    aw: r_wrapper_aw,
                    ah: r_wrapper_ah,
                    cols: r_wrapper_cols,
                    rows: r_wrapper_rows,
                    atlas: None,
                    runtime: None,
                    parent,
                    srcpath: None,
                    force_original_vector_symbol_size: None,
                };

                match res_type_enum {
                    ResourceType::PopAnim => {
                        resource_x.force_original_vector_symbol_size = Some(true);
                    }
                    ResourceType::RenderEffect => {
                        let path_str = resource_x.path.clone();
                        resource_x.srcpath = Some(format!("res\\common\\{}", path_str));
                        // C# logic
                    }
                    _ => {
                        if is_atlas {
                            resource_x.atlas = Some(true);
                            resource_x.runtime = Some(true);
                        } else {
                            resource_x.atlas = None;
                            resource_x.runtime = None;
                        }
                    }
                }
                res_list.push(resource_x);
            }
            resources = Some(res_list);
        }

        groups.push(ShellSubgroupData {
            id,
            res,
            group_type,
            parent,
            subgroups,
            resources,
        });
    }

    Ok(MResourceGroup { slot_count, groups })
}

pub(crate) fn read_string(reader: &mut impl Read) -> Result<String> {
    let len = reader.read_u32::<LE>()?;
    if len > 0 {
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    } else {
        Ok(String::new())
    }
}
