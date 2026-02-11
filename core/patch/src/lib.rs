use std::io::{self, Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("VCDiff error: {0}")]
    VCDiff(String),
}

pub type Result<T> = std::result::Result<T, PatchError>;

/// Encodes a patch (diff) between source (dictionary) and target.
/// Uses VCDiff format with interleaved instruction encoding.
pub fn encode<R1, R2, W>(source: &mut R1, target: &mut R2, output: &mut W) -> Result<()>
where
    R1: Read,
    R2: Read,
    W: Write,
{
    let mut source_data = Vec::new();
    source.read_to_end(&mut source_data)?;

    let mut target_data = Vec::new();
    target.read_to_end(&mut target_data)?;

    // vcdiff::encode(dictionary, target, format, ??)
    // Compiler indicated 4 arguments.
    // Try using vcdiff::FORMAT_INTERLEAVED constant.
    let patch = vcdiff::encode(
        &source_data,
        &target_data,
        vcdiff::FORMAT_INTERLEAVED,
        false,
    );

    // Check if it returns Result or Vec<u8>
    // Compiler error "no method named map_err found for struct Vec<u8>" implies it returns Vec<u8>.

    output.write_all(&patch)?;
    Ok(())
}

/// Decodes a patch, applying it to source (dictionary) to reconstruct target.
pub fn decode<R1, R2, W>(source: &mut R1, patch: &mut R2, output: &mut W) -> Result<()>
where
    R1: Read,
    R2: Read,
    W: Write,
{
    let mut source_data = Vec::new();
    source.read_to_end(&mut source_data)?;

    let mut patch_data = Vec::new();
    patch.read_to_end(&mut patch_data)?;

    // vcdiff::decode(dictionary, delta)
    // Compiler indicated 2 arguments.
    let target = vcdiff::decode(&source_data, &patch_data);

    output.write_all(&target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let source = b"Hello World";
        let target = b"Hello VCDiff";

        let mut patch_out = Vec::new();
        encode(&mut &source[..], &mut &target[..], &mut patch_out).expect("Encode failed");

        assert!(!patch_out.is_empty());

        let mut decoded_out = Vec::new();
        decode(&mut &source[..], &mut &patch_out[..], &mut decoded_out).expect("Decode failed");

        assert_eq!(decoded_out, target);
    }
}
