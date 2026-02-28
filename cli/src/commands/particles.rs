use anyhow::Result;
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use particles::trail::{decode_trail, encode_trail};
use particles::xml;
use particles::{ParticlesVersion, decode, encode};

#[derive(Subcommand)]
pub enum ParticlesCommands {
    /// Decode a Particles binary file to PopCap XML
    Decode {
        #[arg(required = true, help = "Input Particles file")]
        input: PathBuf,
        #[arg(required = true, help = "Output XML file")]
        output: PathBuf,
    },
    /// Encode a PopCap XML file to a Particles binary file
    Encode {
        #[arg(required = true, help = "Input XML file")]
        input: PathBuf,
        #[arg(required = true, help = "Output Particles file")]
        output: PathBuf,
        #[arg(long, default_value = "pc", help = "Version: pc, phone32, phone64")]
        version: String,
    },
    /// Decode a Trail (.trail.compiled) file to PopCap XML
    DecodeTrail {
        #[arg(required = true, help = "Input Trail file")]
        input: PathBuf,
        #[arg(required = true, help = "Output XML file")]
        output: PathBuf,
    },
    /// Encode a PopCap XML file to a Trail (.trail.compiled) file
    EncodeTrail {
        #[arg(required = true, help = "Input XML file")]
        input: PathBuf,
        #[arg(required = true, help = "Output Trail file")]
        output: PathBuf,
    },
}

pub fn handle(cmd: ParticlesCommands) -> Result<()> {
    match cmd {
        ParticlesCommands::Decode { input, output } => {
            let data = fs::read(&input)?;
            let particles = decode(&data)?;
            let xml_str = xml::format_particles_xml(&particles)?;
            fs::write(&output, xml_str)?;
            println!("Decoded {} → {}", input.display(), output.display());
        }
        ParticlesCommands::Encode {
            input,
            output,
            version,
        } => {
            let xml_str = fs::read_to_string(&input)?;
            let particles = xml::parse_particles_xml(&xml_str)?;
            let ver = match version.to_lowercase().as_str() {
                "pc" => ParticlesVersion::PC,
                "phone32" => ParticlesVersion::Phone32,
                "phone64" => ParticlesVersion::Phone64,
                _ => anyhow::bail!("Invalid version, must be pc, phone32, or phone64"),
            };
            let out_data = encode(&particles, ver)?;
            fs::write(&output, out_data)?;
            println!(
                "Encoded {} → {} ({:?})",
                input.display(),
                output.display(),
                ver
            );
        }
        ParticlesCommands::DecodeTrail { input, output } => {
            let data = fs::read(&input)?;
            let trail = decode_trail(&data)?;
            let xml_str = xml::format_trail_xml(&trail)?;
            fs::write(&output, xml_str)?;
            println!("Decoded trail {} → {}", input.display(), output.display());
        }
        ParticlesCommands::EncodeTrail { input, output } => {
            let xml_str = fs::read_to_string(&input)?;
            let trail = xml::parse_trail_xml(&xml_str)?;
            let out_data = encode_trail(&trail)?;
            fs::write(&output, out_data)?;
            println!("Encoded trail {} → {}", input.display(), output.display());
        }
    }
    Ok(())
}
