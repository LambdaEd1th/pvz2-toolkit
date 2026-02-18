use clap::{Parser, Subcommand};
use std::path::PathBuf;

use atlas::split_atlas;
use bnk::process::{pack_bnk, unpack_bnk};
use lawnstrings::process::{lawnstrings_decode, lawnstrings_encode};
use newton::process::{newton_decode, newton_encode};
use pam::process::{pam_decode, pam_encode};
use patch::process::{patch_apply, patch_create, patch_extract};
use popfx::process::{popfx_decode, popfx_encode};
use rsb::process::{pack_rsb, unpack_rsb};
use rsb::process::{pack_rsg_batch, unpack_rsg_batch};
use rsb::process_ptx::{ptx_decode, ptx_encode};
use rton::process::{rton_decode, rton_encode};
use smf::process::{smf_pack, smf_unpack};
use wem::process::{wem_decode, wem_encode};

#[derive(Parser)]
#[command(name = "rsb-cli")]
#[command(about = "CLI for RSB/RSG resource files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// RSB Operations (Unpack/Pack)
    #[command(subcommand)]
    Rsb(RsbCommands),
    /// RSG Operations (Unpack/Pack)
    #[command(subcommand)]
    Rsg(RsgCommands),
    /// BNK Operations (Unpack/Pack)
    #[command(subcommand)]
    Bnk(BnkCommands),
    /// WEM Operations (Convert/Pack)
    #[command(subcommand)]
    Wem(WemCommands),
    /// PTX Operations (Convert)
    #[command(subcommand)]
    Ptx(PtxCommands),
    /// RTON Operations (Convert)
    #[command(subcommand)]
    Rton(RtonCommands),
    /// Newton Operations (Convert)
    #[command(subcommand)]
    Newton(NewtonCommands),
    /// PAM Operations (Convert)
    #[command(subcommand)]
    Pam(PamCommands),
    /// LawnStrings Operations (Convert)
    #[command(subcommand)]
    LawnStrings(LawnStringsCommands),
    /// POPFX Operations (Convert)
    #[command(subcommand)]
    Popfx(PopfxCommands),
    /// VCDiff Patch Operations (RSBPatch)
    #[command(subcommand)]
    Patch(PatchCommands),
    /// SMF Operations (PopCap Zlib)
    #[command(subcommand)]
    Smf(SmfCommands),
    /// Atlas Operations (Split)
    #[command(subcommand)]
    Atlas(AtlasCommand),
}

#[derive(Subcommand)]
enum AtlasCommand {
    /// Split an Atlas into individual sprites
    Split {
        /// Input Atlas JSON file
        json_path: PathBuf,
        /// Input Image file (optional, defaults to json name + .png)
        #[arg(short, long)]
        image: Option<PathBuf>,
        /// Output directory (optional, defaults to json name + .sprite/media)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum PatchCommands {
    /// Create a patch from Source to Target
    Create {
        /// Source file (Dictionary/Original)
        source: PathBuf,
        /// Target file (New/Modified)
        target: PathBuf,
        /// Output Patch file
        output: PathBuf,
    },
    /// Apply a patch to Source to get Target
    Apply {
        /// Source file (Dictionary/Original)
        source: PathBuf,
        /// Patch file
        patch: PathBuf,
        /// Output Target file
        output: PathBuf,
    },
    /// Extract VCDiff patches from an RSBPatch file (PBSR)
    Extract {
        /// Input RSBPatch file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory for extracted .vcdiff files
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum SmfCommands {
    /// Unpack (Decompress) a .smf file
    Unpack {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
        /// Output file (optional, defaults to input without .smf extension or .decoded)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Use 64-bit variant (16-byte header)
        #[arg(long)]
        use_64bit: bool,
    },
    /// Pack (Compress) a file into .smf format
    Pack {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
        /// Output file (optional, defaults to input + .smf)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Use 64-bit variant (16-byte header)
        #[arg(long)]
        use_64bit: bool,
    },
}

#[derive(Subcommand)]
enum RsbCommands {
    /// Unpack an RSB file
    Unpack {
        /// Input RSB file path
        input: PathBuf,
        /// Output directory (optional, defaults to file name stem)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack a directory into an RSB file
    Pack {
        /// Input directory (containing rsb_manifest.json)
        input: PathBuf,
        /// Output RSB file
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum RsgCommands {
    /// Unpack RSG packets from rsb_manifest.json (or single RSG file)
    Unpack {
        /// Input rsb_manifest.json or .rsa/rsg file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack RSG packet from folder/config
    Pack {
        /// Input directory or config
        #[arg(short, long)]
        input: PathBuf,
        /// Output RSG file
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum BnkCommands {
    /// Unpack BNK file (Binary -> JSON + WEM extraction)
    Unpack {
        /// Input file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Disable WEM extraction
        #[arg(long)]
        no_extract: bool,
    },
    /// Pack a BNK file from JSON and WEM files
    Pack {
        /// Input JSON file (generated by unpack-bnk)
        #[arg(short = 'j', long)]
        json: PathBuf,
        /// Directory containing WEM files
        #[arg(short = 'w', long)]
        wems: PathBuf,
        /// Output BNK file
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum WemCommands {
    /// Decode WEM to WAV/OGG/M4A
    Decode {
        /// Input WEM file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Path to codebooks.bin (for Vorbis)
        #[arg(short, long)]
        codebooks: Option<String>,
        /// Inline codebooks into OGG (for Vorbis)
        #[arg(long)]
        inline_codebooks: bool,
    },
    /// Encode WAV/OGG to WEM
    Encode {
        /// Input WAV/OGG file
        input: PathBuf,
        /// Output WEM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Force ADPCM encoding (for WAV input)
        #[arg(short, long)]
        adpcm: bool,
    },
}

#[derive(Subcommand)]
enum PtxCommands {
    /// Decode PTX to PNG (Batch from manifest)
    Decode {
        /// Input rsb_manifest.json file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory (optional, defaults to input dir)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Treat RGBA8888 (Format 0) as PowerVR/iOS format (BGRA) instead of default (RGBA)
        #[arg(long)]
        powervr: bool,
    },
    /// Encode PNG to PTX (Batch from manifest)
    Encode {
        /// Input rsb_manifest.json file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory (optional, defaults to input dir)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Force PowerVR/iOS format (BGRA for Format 0)
        #[arg(long)]
        powervr: bool,
        /// Use Palette Alpha (4bpp with 16-color header) for ETC1A8 (Format 147)
        #[arg(long)]
        palette: bool,
    },
}

#[derive(Subcommand)]
enum RtonCommands {
    /// Decode RTON to JSON
    Decode {
        /// Input RTON file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
    /// Encode JSON to RTON
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output RTON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
}

#[derive(Subcommand)]
enum NewtonCommands {
    /// Decode Newton to XML
    Decode {
        /// Input Newton file
        input: PathBuf,
        /// Output XML file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode XML to Newton
    Encode {
        /// Input XML file
        input: PathBuf,
        /// Output Newton file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum PamCommands {
    /// Decode PAM to JSON
    Decode {
        /// Input PAM file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON/HTML to PAM
    Encode {
        /// Input JSON or HTML file
        input: PathBuf,
        /// Output PAM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum LawnStringsCommands {
    /// Decode LawnStrings to JSON
    Decode {
        /// Input LawnStrings file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to LawnStrings
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output LawnStrings file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum PopfxCommands {
    /// Decode Popfx to JSON
    Decode {
        /// Input Popfx file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to Popfx
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output Popfx file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Rsb(cmd) => match cmd {
            RsbCommands::Unpack { input, output } => unpack_rsb(input, output)?,
            RsbCommands::Pack { input, output } => pack_rsb(input, output)?,
        },
        Commands::Rsg(cmd) => match cmd {
            RsgCommands::Unpack { input, output } => unpack_rsg_batch(input, output)?,
            RsgCommands::Pack { input, output } => pack_rsg_batch(input, output)?,
        },
        Commands::Bnk(cmd) => match cmd {
            BnkCommands::Unpack {
                input,
                output,
                no_extract,
            } => unpack_bnk(input, output, *no_extract)?,
            BnkCommands::Pack { json, wems, output } => pack_bnk(json, wems, output)?,
        },
        Commands::Wem(cmd) => match cmd {
            WemCommands::Decode {
                input,
                output,
                codebooks,
                inline_codebooks,
            } => wem_decode(input, output, codebooks, *inline_codebooks)?,
            WemCommands::Encode {
                input,
                output,
                adpcm,
            } => wem_encode(input, output, *adpcm)?,
        },
        Commands::Ptx(cmd) => match cmd {
            PtxCommands::Decode {
                input,
                output,
                powervr,
            } => ptx_decode(input, output, *powervr)?,
            PtxCommands::Encode {
                input,
                output,
                powervr,
                palette,
            } => ptx_encode(input, output, *powervr, *palette)?,
        },
        Commands::Rton(cmd) => match cmd {
            RtonCommands::Decode {
                input,
                output,
                seed,
            } => rton_decode(input, output, seed.as_deref())?,
            RtonCommands::Encode {
                input,
                output,
                seed,
            } => rton_encode(input, output, seed.as_deref())?,
        },
        Commands::Newton(cmd) => match cmd {
            NewtonCommands::Decode { input, output } => newton_decode(input, output)?,
            NewtonCommands::Encode { input, output } => newton_encode(input, output)?,
        },
        Commands::Pam(cmd) => match cmd {
            PamCommands::Decode { input, output } => pam_decode(input, output)?,
            PamCommands::Encode { input, output } => pam_encode(input, output)?,
        },
        Commands::LawnStrings(cmd) => match cmd {
            LawnStringsCommands::Decode { input, output } => lawnstrings_decode(input, output)?,
            LawnStringsCommands::Encode { input, output } => lawnstrings_encode(input, output)?,
        },
        Commands::Popfx(cmd) => match cmd {
            PopfxCommands::Decode { input, output } => popfx_decode(input, output)?,
            PopfxCommands::Encode { input, output } => popfx_encode(input, output)?,
        },
        Commands::Patch(cmd) => match cmd {
            PatchCommands::Create {
                source,
                target,
                output,
            } => patch_create(source, target, output)?,
            PatchCommands::Apply {
                source,
                patch,
                output,
            } => patch_apply(source, patch, output)?,
            PatchCommands::Extract { input, output } => patch_extract(input, output)?,
        },
        Commands::Smf(cmd) => match cmd {
            SmfCommands::Unpack {
                input,
                output,
                use_64bit,
            } => smf_unpack(input, output, *use_64bit)?,
            SmfCommands::Pack {
                input,
                output,
                use_64bit,
            } => smf_pack(input, output, *use_64bit)?,
        },
        Commands::Atlas(cmd) => match cmd {
            AtlasCommand::Split {
                json_path,
                image,
                output,
            } => split_atlas(json_path, image.as_deref(), output.as_deref())?,
        },
    }

    Ok(())
}
