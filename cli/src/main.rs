use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

use commands::{atlas, bnk, lawnstrings, newton, pam, patch, popfx, ptx, rsb, rsg, rton, smf, wem};

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
    /// RSG Operations (Unpack/Pack)
    #[command(subcommand)]
    Rsg(rsg::RsgCommands),
    /// BNK Operations (Unpack/Pack)
    #[command(subcommand)]
    Bnk(bnk::BnkCommands),
    /// WEM Operations (Convert/Pack)
    #[command(subcommand)]
    Wem(wem::WemCommands),
    /// PTX Operations (Convert)
    #[command(subcommand)]
    Ptx(ptx::PtxCommands),
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rsb(cmd) => rsb::handle(cmd),
        Commands::Rsg(cmd) => rsg::handle(cmd),
        Commands::Bnk(cmd) => bnk::handle(cmd),
        Commands::Wem(cmd) => wem::handle(cmd),
        Commands::Ptx(cmd) => ptx::handle(cmd),
        Commands::Rton(cmd) => rton::handle(cmd),
        Commands::Newton(cmd) => newton::handle(cmd),
        Commands::Pam(cmd) => pam::handle(cmd),
        Commands::LawnStrings(cmd) => lawnstrings::handle(cmd),
        Commands::Popfx(cmd) => popfx::handle(cmd),
        Commands::Patch(cmd) => patch::handle(cmd),
        Commands::Smf(cmd) => smf::handle(cmd),
        Commands::Atlas(cmd) => atlas::handle(cmd),
    }
}
