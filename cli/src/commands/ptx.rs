use anyhow::{Result, anyhow};
use clap::Subcommand;
use rsb::ptx::{PtxDecoder, PtxEncoder, PtxFormat};
use std::fs;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PtxCommands {
    /// Decode a PTX file into a PNG
    Decode {
        /// Input PTX file path
        input: PathBuf,
        /// Output PNG file path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Width of the texture (required if not derivable from header/size)
        #[arg(long)]
        width: Option<u32>,
        /// Height of the texture (required if not derivable from header/size)
        #[arg(long)]
        height: Option<u32>,
        /// Texture format ID (147, 30, etc.)
        #[arg(short, long)]
        format: i32,
        /// Optional alpha size for specific sub-formats
        #[arg(long)]
        alpha_size: Option<i32>,
        /// Optional alpha format override
        #[arg(long)]
        alpha_format: Option<i32>,
        /// Was it encoded with PowerVR? (Flips RGB/BGR for certain ETC1 formats)
        #[arg(long, default_value_t = false)]
        powervr: bool,
    },
    /// Encode a PNG or compatible image into a PTX file
    Encode {
        /// Input image file path
        input: PathBuf,
        /// Output PTX file path
        output: PathBuf,
        /// Target PTX Format name (e.g., Rgba8888, Etc1A8)
        #[arg(short, long)]
        format: String,
        /// Are we targeting PowerVR engines (changes element order in encoded block buffer)
        #[arg(long, default_value_t = false)]
        powervr: bool,
    },
}

pub fn handle(cmd: PtxCommands) -> Result<()> {
    match cmd {
        PtxCommands::Decode {
            input,
            output,
            width,
            height,
            format,
            alpha_size,
            alpha_format,
            powervr,
        } => {
            let data = fs::read(&input)?;

            // Assume width/height needs to be passed in for flat PTX files without a container providing them
            let w = width.ok_or_else(|| {
                anyhow!("Width must be provided for raw PTX decoding without RSG context")
            })?;
            let h = height.ok_or_else(|| {
                anyhow!("Height must be provided for raw PTX decoding without RSG context")
            })?;

            println!("Decoding PTX with Format ID: {}", format);
            let img = PtxDecoder::decode(&data, w, h, format, alpha_size, alpha_format, powervr)?;

            let out_path = output.unwrap_or_else(|| input.with_extension("png"));
            img.save(&out_path)?;
            println!("Decoded PTX saved to {:?}", out_path);
            Ok(())
        }
        PtxCommands::Encode {
            input,
            output,
            format,
            powervr,
        } => {
            let img = image::open(&input)?;

            // Map string format to enum
            let fmt = match format.to_lowercase().as_str() {
                "rgba8888" => PtxFormat::Rgba8888,
                "rgba4444" => PtxFormat::Rgba4444,
                "rgba5551" => PtxFormat::Rgba5551,
                "rgb565" => PtxFormat::Rgb565,
                "rgba4444block" => PtxFormat::Rgba4444Block,
                "rgba5551block" => PtxFormat::Rgba5551Block,
                "rgb565block" => PtxFormat::Rgb565Block,
                "etc1" => PtxFormat::Etc1,
                "etc1a8" => PtxFormat::Etc1A8,
                "etc1palette" => PtxFormat::Etc1Palette,
                "pvrtc4bpp" => PtxFormat::Pvrtc4BppRgba,
                _ => {
                    return Err(anyhow!(
                        "Unknown or unsupported PTX format string: {}",
                        format
                    ));
                }
            };

            println!("Encoding Image as {:?}", fmt);
            let ptx_data = PtxEncoder::encode(&img, fmt, powervr)?;
            fs::write(&output, ptx_data)?;
            println!("Encoded PTX saved to {:?}", output);
            Ok(())
        }
    }
}
