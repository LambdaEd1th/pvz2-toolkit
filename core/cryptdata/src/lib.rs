pub mod error;

pub use error::{CryptDataError, Result};

/// Magic bytes: `CRYPT_RES\n\0`
const MAGIC: &[u8] = &[
    0x43, 0x52, 0x59, 0x50, 0x54, 0x5F, 0x52, 0x45, 0x53, 0x0A, 0x00,
];

/// The number of leading bytes that are XOR-encrypted (0x100 = 256).
const ENCRYPT_SIZE: usize = 0x100;

/// Minimum encrypted file size: MAGIC(11) + size_field(8) + ENCRYPT_SIZE(256) = 275.
/// The C# code checks `sen.length >= 0x112` (274) for the decrypt path,
/// but the threshold for encryption is `size >= 0x100` (256).
const MIN_ENCRYPTED_PAYLOAD: usize = MAGIC.len() + 8; // 19 bytes before payload

/// Encrypt raw data with the given key string.
///
/// Produces: `MAGIC` + `original_size (i64 LE)` + XOR'd first 256 bytes + remaining bytes.
pub fn encrypt(data: &[u8], key: &str) -> Vec<u8> {
    let code = key.as_bytes();
    let size = data.len();

    let mut out = Vec::with_capacity(MAGIC.len() + 8 + size);
    out.extend_from_slice(MAGIC);

    // Write original size as i64 LE
    out.extend_from_slice(&(size as i64).to_le_bytes());

    if size >= ENCRYPT_SIZE {
        let mut index = 0usize;
        let key_len = code.len();
        for i in 0..ENCRYPT_SIZE {
            out.push(data[i] ^ code[index]);
            index = (index + 1) % key_len;
        }
        out.extend_from_slice(&data[ENCRYPT_SIZE..]);
    } else {
        out.extend_from_slice(data);
    }

    out
}

/// Decrypt a CRYPT_RES encrypted blob back to raw data.
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < MAGIC.len() {
        return Err(CryptDataError::InvalidMagic);
    }

    if &data[..MAGIC.len()] != MAGIC {
        return Err(CryptDataError::InvalidMagic);
    }

    Err(CryptDataError::MissingKey)
}

/// Decrypt a CRYPT_RES encrypted blob with the given key string.
pub fn decrypt_with_key(data: &[u8], key: &str) -> Result<Vec<u8>> {
    if data.len() < MAGIC.len() {
        return Err(CryptDataError::InvalidMagic);
    }

    if &data[..MAGIC.len()] != MAGIC {
        return Err(CryptDataError::InvalidMagic);
    }

    let code = key.as_bytes();
    let offset = MAGIC.len();

    if data.len() < offset + 8 {
        return Err(CryptDataError::TooShort);
    }

    let size_bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    let original_size = i64::from_le_bytes(size_bytes) as usize;

    let payload_start = offset + 8;
    let payload = &data[payload_start..];

    let mut out = Vec::with_capacity(original_size);

    // C# checks: `sen.length >= 0x112` which is MAGIC(11) + size(8) + 0x100(256) + 1 = 276.
    // We use a slightly cleaner check: total file length >= MIN_ENCRYPTED_PAYLOAD + ENCRYPT_SIZE.
    if data.len() >= MIN_ENCRYPTED_PAYLOAD + ENCRYPT_SIZE {
        let mut index = 0usize;
        let key_len = code.len();
        for i in 0..ENCRYPT_SIZE {
            out.push(payload[i] ^ code[index]);
            index = (index + 1) % key_len;
        }
        if payload.len() > ENCRYPT_SIZE {
            out.extend_from_slice(&payload[ENCRYPT_SIZE..]);
        }
    } else {
        out.extend_from_slice(payload);
    }

    // Trim to original size if needed
    out.truncate(original_size);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_small_data() {
        let key = "hello_key";
        let data = b"small";
        let encrypted = encrypt(data, key);
        let decrypted = decrypt_with_key(&encrypted, key).expect("decrypt failed");
        assert_eq!(&decrypted, data);
    }

    #[test]
    fn test_roundtrip_large_data() {
        let key = "my_secret_key";
        let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        let encrypted = encrypt(&data, key);
        let decrypted = decrypt_with_key(&encrypted, key).expect("decrypt failed");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_invalid_magic() {
        let result = decrypt_with_key(b"NOT_CRYPT", "key");
        assert!(result.is_err());
    }
}
