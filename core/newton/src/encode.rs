use crate::error::{NewtonError, Result};
use crate::types::MResourceGroup;
use byteorder::{LE, WriteBytesExt};
use std::io::Write;

pub fn encode_newton(resource: &MResourceGroup, mut writer: impl Write) -> Result<()> {
    writer.write_u32::<LE>(resource.slot_count)?;
    writer.write_u32::<LE>(resource.groups.len() as u32)?;

    for group in &resource.groups {
        match group.group_type.as_str() {
            "composite" => writer.write_u8(1)?,
            "simple" => writer.write_u8(2)?,
            _ => {
                return Err(NewtonError::DeserializationError(format!(
                    "Unknown group type: {}",
                    group.group_type
                )));
            }
        }

        let subgroups_count = group.subgroups.as_ref().map_or(0, |v| v.len() as u32);
        let resources_count = group.resources.as_ref().map_or(0, |v| v.len() as u32);

        let res_val = if let Some(r) = &group.res {
            r.parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        writer.write_u32::<LE>(res_val)?;
        writer.write_u32::<LE>(subgroups_count)?;
        writer.write_u32::<LE>(resources_count)?;

        writer.write_u8(1)?; // version

        if group.parent.is_some() {
            writer.write_u8(1)?;
        } else {
            writer.write_u8(0)?;
        }

        write_string(&mut writer, &group.id)?;
        if let Some(p) = &group.parent {
            write_string(&mut writer, p)?;
        }

        if group.group_type == "composite"
            && let Some(subs) = &group.subgroups
        {
            for sub in subs {
                let sub_res_val = if let Some(r) = &sub.res {
                    r.parse::<u32>().unwrap_or(0)
                } else {
                    0
                };
                writer.write_u32::<LE>(sub_res_val)?;
                write_string(&mut writer, &sub.id)?;
            }
        }

        if group.group_type == "simple"
            && let Some(res_list) = &group.resources
        {
            for res in res_list {
                let type_byte = match res.res_type.as_str() {
                    "Image" => 1,
                    "PopAnim" => 2,
                    "SoundBank" => 3,
                    "File" => 4,
                    "PrimeFont" => 5,
                    "RenderEffect" => 6,
                    "DecodedSoundBank" => 7,
                    _ => {
                        return Err(NewtonError::DeserializationError(format!(
                            "Unknown resource type: {}",
                            res.res_type
                        )));
                    }
                };
                writer.write_u8(type_byte)?;
                writer.write_u32::<LE>(res.slot)?;

                writer.write_u32::<LE>(res.width.unwrap_or(0))?;
                writer.write_u32::<LE>(res.height.unwrap_or(0))?;

                if let Some(x) = res.x {
                    writer.write_i32::<LE>(x)?;
                } else if res.aw.unwrap_or(0) != 0 && res.ah.unwrap_or(0) != 0 {
                    writer.write_i32::<LE>(0)?;
                } else {
                    writer.write_i32::<LE>(0x7FFFFFFF)?;
                }

                if let Some(y) = res.y {
                    writer.write_i32::<LE>(y)?;
                } else if res.aw.unwrap_or(0) != 0 && res.ah.unwrap_or(0) != 0 {
                    writer.write_i32::<LE>(0)?;
                } else {
                    writer.write_i32::<LE>(0x7FFFFFFF)?;
                }

                writer.write_u32::<LE>(res.ax.unwrap_or(0))?;
                writer.write_u32::<LE>(res.ay.unwrap_or(0))?;
                writer.write_u32::<LE>(res.aw.unwrap_or(0))?;
                writer.write_u32::<LE>(res.ah.unwrap_or(0))?;

                writer.write_u32::<LE>(res.cols.unwrap_or(1))?;
                writer.write_u32::<LE>(res.rows.unwrap_or(1))?;

                if res.atlas.unwrap_or(false) {
                    writer.write_u8(1)?;
                } else {
                    writer.write_u8(0)?;
                }

                writer.write_u8(1)?;
                writer.write_u8(1)?;

                if res.parent.is_some() {
                    writer.write_u8(1)?;
                } else {
                    writer.write_u8(0)?;
                }

                write_string(&mut writer, &res.id)?;
                write_string(&mut writer, &res.path)?;

                if let Some(p) = &res.parent {
                    write_string(&mut writer, p)?;
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn write_string(writer: &mut impl Write, s: &str) -> Result<()> {
    writer.write_u32::<LE>(s.len() as u32)?;
    if !s.is_empty() {
        writer.write_all(s.as_bytes())?;
    }
    Ok(())
}
