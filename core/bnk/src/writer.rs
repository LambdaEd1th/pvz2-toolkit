use crate::error::BnkError;
use crate::types::*;
use byteorder::{LE, WriteBytesExt};
use std::io::{Seek, Write};
use utils::BinWriteExt;

type Result<T> = std::result::Result<T, BnkError>;

// Helper Utils
pub(crate) fn hex_space_to_bytes(hex_string: &str) -> Result<Vec<u8>> {
    let clean_string = hex_string.replace(' ', "");
    hex::decode(&clean_string).map_err(|e| BnkError::ParseError(format!("Invalid hex: {}", e)))
}

impl Bnk {
    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        // 1. Header (BKHD)
        let mut bkhd_data = Vec::new();
        self.header.write(&mut bkhd_data)?;

        writer.write_all(b"BKHD")?;
        writer.write_u32::<LE>(bkhd_data.len() as u32)?;
        writer.write_all(&bkhd_data)?;

        // 2. DIDX (Data Index)
        if !self.data_index.is_empty() {
            let mut didx_data = Vec::new();
            for entry in &self.data_index {
                entry.write(&mut didx_data)?;
            }
            writer.write_all(b"DIDX")?;
            writer.write_u32::<LE>(didx_data.len() as u32)?;
            writer.write_all(&didx_data)?;
        }

        // 3. INIT (Initialization)
        if let Some(init) = &self.initialization {
            let mut init_data = Vec::new();
            init_data.write_u32::<LE>(init.len() as u32)?;
            for entry in init {
                entry.write(&mut init_data)?;
            }
            writer.write_all(b"INIT")?;
            writer.write_u32::<LE>(init_data.len() as u32)?;
            writer.write_all(&init_data)?;
        }

        // 4. STMG (Game Sync/State Mgmt)
        if let Some(stmg) = &self.game_sync {
            let mut stmg_data = Vec::new();
            stmg.write(&mut stmg_data, self.header.version)?;
            writer.write_all(b"STMG")?;
            writer.write_u32::<LE>(stmg_data.len() as u32)?;
            writer.write_all(&stmg_data)?;
        }

        // 5. ENVS (Environments)
        if let Some(envs) = &self.environments {
            let mut envs_data = Vec::new();
            envs.write(&mut envs_data, self.header.version)?;
            writer.write_all(b"ENVS")?;
            writer.write_u32::<LE>(envs_data.len() as u32)?;
            writer.write_all(&envs_data)?;
        }

        // 6. HIRC (Hierarchy)
        if !self.hierarchy.is_empty() {
            let mut hirc_data = Vec::new();
            hirc_data.write_u32::<LE>(self.hierarchy.len() as u32)?;
            for obj in &self.hierarchy {
                obj.write(&mut hirc_data)?;
            }
            writer.write_all(b"HIRC")?;
            writer.write_u32::<LE>(hirc_data.len() as u32)?;
            writer.write_all(&hirc_data)?;
        }

        // 7. STID (String Mappings)
        if let Some(ref_data) = &self.reference {
            let mut stid_data = Vec::new();
            stid_data.write_u32::<LE>(ref_data.unknown_type)?;
            stid_data.write_u32::<LE>(ref_data.entries.len() as u32)?;
            for entry in &ref_data.entries {
                entry.write(&mut stid_data)?;
            }
            writer.write_all(b"STID")?;
            writer.write_u32::<LE>(stid_data.len() as u32)?;
            writer.write_all(&stid_data)?;
        }

        // 8. PLAT (Platform)
        if let Some(plat) = &self.platform {
            let mut plat_data = Vec::new();
            plat.write(&mut plat_data)?;
            writer.write_all(b"PLAT")?;
            writer.write_u32::<LE>(plat_data.len() as u32)?;
            writer.write_all(&plat_data)?;
        }

        // DATA chunk is NOT written here. It must be appended manually by the caller
        // because Bnk struct does not hold the raw audio data.

        Ok(())
    }
}

// Implement Write for sub-structs

impl BankHeader {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.version)?;
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.language)?;
        let expand_bytes = hex_space_to_bytes(&self.head_expand)?;
        writer.write_all(&expand_bytes)?;
        Ok(())
    }
}

impl DidxEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.offset)?;
        writer.write_u32::<LE>(self.size)?;
        Ok(())
    }
}

impl InitEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_null_term_string(&self.name)?;
        Ok(())
    }
}

impl GameSync {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        let vol_bytes = hex_space_to_bytes(&self.volume_threshold)?;
        writer.write_all(&vol_bytes)?;

        let voice_bytes = hex_space_to_bytes(&self.max_voice_instances)?;
        writer.write_all(&voice_bytes)?;

        if version >= 140 {
            writer.write_u16::<LE>(self.unknown_type_1)?;
        }

        writer.write_u32::<LE>(self.stage_group.len() as u32)?;
        for s in &self.stage_group {
            s.write(writer)?;
        }

        writer.write_u32::<LE>(self.switch_group.len() as u32)?;
        for s in &self.switch_group {
            s.write(writer, version)?;
        }

        writer.write_u32::<LE>(self.game_parameter.len() as u32)?;
        for p in &self.game_parameter {
            p.write(writer, version)?;
        }

        if version >= 140 {
            writer.write_u32::<LE>(self.unknown_type_2)?;
        }

        Ok(())
    }
}

impl StageGroup {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        let def_trans = hex_space_to_bytes(&self.data.default_transition_time)?;
        writer.write_all(&def_trans)?;

        writer.write_u32::<LE>(self.data.custom_transition.len() as u32)?;
        for c in &self.data.custom_transition {
            let b = hex_space_to_bytes(c)?;
            writer.write_all(&b)?;
        }
        Ok(())
    }
}

impl SwitchGroup {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.data.parameter)?;

        if version >= 112 {
            writer.write_u8(self.data.parameter_category)?;
        }

        writer.write_u32::<LE>(self.data.point.len() as u32)?;
        for p in &self.data.point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }
        Ok(())
    }
}

impl GameParameter {
    fn write<W: Write>(&self, writer: &mut W, _version: u32) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        // Size is implicitly handled by the hex string length,
        // but parsing logic hardcoded 17 or 4 based on version.
        // We trust the hex string in JSON to have correct length.
        let b = hex_space_to_bytes(&self.data)?;
        writer.write_all(&b)?;
        Ok(())
    }
}

impl Environments {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        self.obstruction.write(writer, version)?;
        self.occlusion.write(writer, version)?;
        Ok(())
    }
}

impl EnvironmentItem {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        // Volume
        let vol_val = hex_space_to_bytes(&self.volume.volume_value)?;
        writer.write_all(&vol_val)?;
        writer.write_u16::<LE>(self.volume.volume_point.len() as u16)?;
        for p in &self.volume.volume_point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }

        // Low Pass
        let lp_val = hex_space_to_bytes(&self.low_pass_filter.value)?;
        writer.write_all(&lp_val)?;
        writer.write_u16::<LE>(self.low_pass_filter.point.len() as u16)?;
        for p in &self.low_pass_filter.point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }

        // High Pass
        if version >= 112 {
            if let Some(hp) = &self.high_pass_filter {
                let hp_val = hex_space_to_bytes(&hp.value)?;
                writer.write_all(&hp_val)?;
                writer.write_u16::<LE>(hp.point.len() as u16)?;
                for p in &hp.point {
                    let b = hex_space_to_bytes(p)?;
                    writer.write_all(&b)?;
                }
            } else {
                writer.write_all(&[0u8; 2])?;
                writer.write_u16::<LE>(0)?;
            }
        }
        Ok(())
    }
}

impl HircObject {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.obj_type)?;
        let data_bytes = hex_space_to_bytes(&self.data)?;
        let length = data_bytes.len() as u32 + 4; // +4 for ID
        writer.write_u32::<LE>(length)?;
        writer.write_u32::<LE>(self.id)?;
        writer.write_all(&data_bytes)?;
        Ok(())
    }
}

impl ReferenceEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u8(self.name.len() as u8)?;
        writer.write_all(self.name.as_bytes())?;
        Ok(())
    }
}

impl PlatformSetting {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_null_term_string(&self.platform)?;
        Ok(())
    }
}
