pub mod error;

pub use error::{CompiledTextError, Result};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use simple_rijndael::impls::RijndaelCbc;
use simple_rijndael::paddings::ZeroPadding;
use std::io::{Read, Write};

/// PopCap Zlib magic: `0xDEADFED4`
const POPCAP_ZLIB_MAGIC: u32 = 0xDEADFED4;

/// Rijndael block size used by PvZ (192 bits = 24 bytes).
const RIJNDAEL_BLOCK_SIZE: usize = 24;

// ─── Rijndael helpers (same as rton/crypto.rs) ───

fn derive_key_iv(seed: &str) -> (Vec<u8>, Vec<u8>) {
    let digest = md5::compute(seed).0;
    let hex_string = hex::encode(digest);
    let hex_bytes = hex_string.as_bytes();
    let key = hex_bytes.to_vec(); // 32 bytes
    let iv = hex_bytes[4..28].to_vec(); // 24 bytes
    (key, iv)
}

fn rijndael_encrypt(data: &[u8], seed: &str) -> Result<Vec<u8>> {
    let (key, iv) = derive_key_iv(seed);
    let cipher = RijndaelCbc::<ZeroPadding>::new(&key, RIJNDAEL_BLOCK_SIZE)
        .map_err(|e| CompiledTextError::Cipher(format!("{:?}", e)))?;
    cipher
        .encrypt(&iv, data.to_vec())
        .map_err(|e| CompiledTextError::Cipher(format!("{:?}", e)))
}

fn rijndael_decrypt(data: &[u8], seed: &str) -> Result<Vec<u8>> {
    let (key, iv) = derive_key_iv(seed);
    let cipher = RijndaelCbc::<ZeroPadding>::new(&key, RIJNDAEL_BLOCK_SIZE)
        .map_err(|e| CompiledTextError::Cipher(format!("{:?}", e)))?;
    cipher
        .decrypt(&iv, data.to_vec())
        .map_err(|e| CompiledTextError::Cipher(format!("{:?}", e)))
}

// ─── PopCap Zlib helpers ───

/// Compress with PopCap Zlib header: magic(4) + [padding(4) if 64-bit] + size(4) + [padding(4) if 64-bit] + zlib_data
fn popcap_zlib_compress(data: &[u8], use_64bit: bool) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    buf.write_u32::<LE>(POPCAP_ZLIB_MAGIC)?;
    if use_64bit {
        buf.write_u32::<LE>(0)?; // padding
    }
    buf.write_u32::<LE>(data.len() as u32)?;
    if use_64bit {
        buf.write_u32::<LE>(0)?; // padding
    }
    let mut encoder = ZlibEncoder::new(&mut buf, Compression::best());
    encoder.write_all(data)?;
    encoder.finish()?;
    Ok(buf)
}

/// Decompress a PopCap Zlib blob, stripping the header.
fn popcap_zlib_decompress(data: &[u8], use_64bit: bool) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(data);
    let magic = cursor.read_u32::<LE>()?;
    if magic != POPCAP_ZLIB_MAGIC {
        return Err(CompiledTextError::InvalidZlibMagic);
    }
    // Skip header: 32-bit variant = 8 bytes total, 64-bit = 16 bytes total.
    let header_size = if use_64bit { 16 } else { 8 };
    let zlib_data = &data[header_size..];
    let mut decoder = ZlibDecoder::new(zlib_data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

// ─── Public API ───

/// Decode (decrypt) a Compiled Text file.
///
/// Pipeline: Base64 decode → Rijndael decrypt → PopCap Zlib decompress.
pub fn decode(data: &[u8], encryption_key: &str, use_64bit: bool) -> Result<Vec<u8>> {
    let cipher_bytes = BASE64.decode(data)?;
    let compressed = rijndael_decrypt(&cipher_bytes, encryption_key)?;
    popcap_zlib_decompress(&compressed, use_64bit)
}

/// Encode (encrypt) a Compiled Text file.
///
/// Pipeline: PopCap Zlib compress → Rijndael encrypt → Base64 encode.
pub fn encode(data: &[u8], encryption_key: &str, use_64bit: bool) -> Result<Vec<u8>> {
    let compressed = popcap_zlib_compress(data, use_64bit)?;
    let encrypted = rijndael_encrypt(&compressed, encryption_key)?;
    let b64 = BASE64.encode(&encrypted);
    Ok(b64.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_32bit() {
        let key = "AS-23DSRFG-209JH0";
        let original = b"Hello, CompiledText 32-bit!";
        let encoded = encode(original, key, false).expect("encode failed");
        let decoded = decode(&encoded, key, false).expect("decode failed");
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_roundtrip_64bit() {
        let key = "MY-64BIT-KEY-HERE";
        let original = b"Hello, CompiledText 64-bit variant!";
        let encoded = encode(original, key, true).expect("encode failed");
        let decoded = decode(&encoded, key, true).expect("decode failed");
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_popcap_zlib_roundtrip() {
        let data = b"Some test data for PopCap Zlib compression";
        let compressed = popcap_zlib_compress(data, false).unwrap();
        let decompressed = popcap_zlib_decompress(&compressed, false).unwrap();
        assert_eq!(&decompressed, data);
    }
}
