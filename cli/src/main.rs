use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

use commands::{
    atlas, bnk, lawnstrings, newton, pak, pam, particles, patch, popfx, ptx, reanim, resources,
    rsb, rsg, rton, smf, wem,
};

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
    Rsb(rsb::RsbCommands),
    /// Isolated RSG Operations (Unpack/Pack single packets)
    #[command(subcommand)]
    Rsg(rsg::RsgCommands),
    /// PTX Operations (Decode/Encode Images)
    #[command(subcommand)]
    Ptx(ptx::PtxCommands),
    /// BNK Operations (Unpack/Pack)
    #[command(subcommand)]
    Bnk(bnk::BnkCommands),
    /// WEM Operations (Convert/Pack)
    #[command(subcommand)]
    Wem(wem::WemCommands),
    /// RTON Operations (Convert)
    #[command(subcommand)]
    Rton(rton::RtonCommands),
    /// Newton Operations (Convert)
    #[command(subcommand)]
    Newton(newton::NewtonCommands),
    /// PAM Operations (Convert)
    #[command(subcommand)]
    Pam(pam::PamCommands),
    /// LawnStrings Operations (Convert)
    #[command(subcommand)]
    LawnStrings(lawnstrings::LawnStringsCommands),
    /// POPFX Operations (Convert)
    #[command(subcommand)]
    Popfx(popfx::PopfxCommands),
    /// VCDiff Patch Operations (RSBPatch)
    #[command(subcommand)]
    Patch(patch::PatchCommands),
    /// SMF Operations (PopCap Zlib)
    #[command(subcommand)]
    Smf(smf::SmfCommands),
    /// Atlas Operations (Split)
    #[command(subcommand)]
    Atlas(atlas::AtlasCommand),
    /// Resources Operations (Convert res.json to resources.json and vice versa)
    #[command(subcommand)]
    Resources(resources::ResourcesCommands),
    /// PAK Operations (Unpack/Pack PvZ PAK archives)
    #[command(subcommand)]
    Pak(pak::PakCommands),
    /// Reanim Operations (Decode/Encode Animations)
    #[command(subcommand)]
    Reanim(reanim::ReanimCommands),
    /// Particles Operations (Decode/Encode Particles)
    #[command(subcommand)]
    Particles(particles::ParticlesCommands),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rsb(cmd) => rsb::handle(cmd),
        Commands::Rsg(cmd) => rsg::handle(cmd),
        Commands::Ptx(cmd) => ptx::handle(cmd),
        Commands::Bnk(cmd) => bnk::handle(cmd),
        Commands::Wem(cmd) => wem::handle(cmd),
        Commands::Rton(cmd) => rton::handle(cmd),
        Commands::Newton(cmd) => newton::handle(cmd),
        Commands::Pam(cmd) => pam::handle(cmd),
        Commands::LawnStrings(cmd) => lawnstrings::handle(cmd),
        Commands::Popfx(cmd) => popfx::handle(cmd),
        Commands::Patch(cmd) => patch::handle(cmd),
        Commands::Smf(cmd) => smf::handle(cmd),
        Commands::Atlas(cmd) => atlas::handle(cmd),
        Commands::Resources(cmd) => resources::handle(cmd),
        Commands::Pak(cmd) => pak::handle(cmd),
        Commands::Reanim(cmd) => reanim::handle(cmd),
        Commands::Particles(cmd) => particles::handle(cmd),
    }
}
