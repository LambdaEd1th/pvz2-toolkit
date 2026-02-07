use bnk::Bnk;
use std::path::PathBuf;

#[test]
fn test_parse_bnk_from_output() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Navigate to test_output relative to core/bnk
    path.push("../../../rsb-rs/test_output/rsb_powervr/AudioCommon/SOUNDBANKS/GENERAL_ZOMBIE_INGAMESFX.BNK");

    if !path.exists() {
        println!(
            "Skipping test_parse_bnk_from_output: File not found at {:?}",
            path
        );
        return;
    }

    let file = std::fs::File::open(&path).expect("Failed to open BNK file");
    let result = Bnk::new(file);

    assert!(result.is_ok(), "Failed to parse BNK: {:?}", result.err());
    let bnk = result.unwrap();

    println!("Parsed BNK Header: {:?}", bnk.header);
    println!("Found {} entries", bnk.entries.len());

    assert!(bnk.header.id != 0, "BNK ID should not be zero");
    // Standard Wwise banks usually have at least some entries if they are SFX banks
    // GENERAL_ZOMBIE_INGAMESFX likely has embedded WEMs

    if !bnk.entries.is_empty() {
        println!("First entry: {:?}", bnk.entries[0]);
    }
}
