use anyhow::{Context, Result};
use clap::Parser;
use pvz2_resources::{
    ResInfo, ResourceGroup, convert_res_info_to_resource_group, convert_resource_group_to_res_info,
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub enum ResourcesCommands {
    /// Convert between grouped ResourceGroup and flat ResInfo formats
    Convert {
        /// Input JSON file (either res.json or resources.json format)
        #[arg(short, long)]
        input: PathBuf,
        /// Output JSON file
        #[arg(short, long)]
        output: PathBuf,
        /// Force output expand_path structure ("string" or "array") for flat -> grouped conversion or grouped -> flat.
        /// When omitted, defaults to "array" when generating flat.
        #[arg(long, default_value = "array")]
        expand_path: String,
    },
}

pub fn handle(cmd: ResourcesCommands) -> Result<()> {
    match cmd {
        ResourcesCommands::Convert {
            input,
            output,
            expand_path,
        } => {
            let json_str = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read input JSON: {:?}", input))?;

            // Try to parse as ResInfo first (flat)
            if let Ok(res_info) = serde_json::from_str::<ResInfo>(&json_str) {
                println!("Detected flat ResInfo structure. Converting to ResourceGroup...");
                let group = convert_res_info_to_resource_group(&res_info)
                    .context("Failed to convert ResInfo to ResourceGroup")?;

                let out_str = serde_json::to_string_pretty(&group)?;
                fs::write(&output, out_str)
                    .with_context(|| format!("Failed to write output JSON to {:?}", output))?;
                println!("Successfully wrote ResourceGroup to {:?}", output);
                return Ok(());
            }

            // Fallback: try parsing as ResourceGroup (hierarchical)
            if let Ok(group) = serde_json::from_str::<ResourceGroup>(&json_str) {
                println!("Detected hierarchical ResourceGroup structure. Converting to ResInfo...");
                let res_info = convert_resource_group_to_res_info(&group, &expand_path)
                    .context("Failed to convert ResourceGroup to ResInfo")?;

                let out_str = serde_json::to_string_pretty(&res_info)?;
                fs::write(&output, out_str)
                    .with_context(|| format!("Failed to write output JSON to {:?}", output))?;
                println!("Successfully wrote ResInfo to {:?}", output);
                return Ok(());
            }

            anyhow::bail!(
                "Input file format was not recognized as a valid ResInfo or ResourceGroup JSON."
            );
        }
    }
}
