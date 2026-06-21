use anyhow::{bail, Result};
use p256::ecdsa::VerifyingKey;

pub fn compress_pubkey(pubkey_bytes: Vec<u8>) -> Result<Vec<u8>> {
    let point = VerifyingKey::from_sec1_bytes(pubkey_bytes.as_slice())?.to_encoded_point(true);
    Ok(point.as_bytes().to_vec())
}

pub fn decompress_pubkey(compressed: Vec<u8>) -> Result<Vec<u8>> {
    if compressed.len() != 33 {
        bail!("Expected 33 byte compress pubkey")
    }
    let point = VerifyingKey::from_sec1_bytes(compressed.as_slice())?.to_encoded_point(false);
    Ok(point.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A known valid P-256 uncompressed public key (65 bytes: 0x04 || x || y)
    fn known_uncompressed_pubkey() -> Vec<u8> {
        vec![
            0x04, 0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63,
            0xa4, 0x40, 0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39,
            0x45, 0xd8, 0x98, 0xc2, 0x96, 0x4f, 0xe3, 0x42, 0xe2, 0xfe, 0x1a, 0x7f, 0x9b, 0x8e,
            0xe7, 0xeb, 0x4a, 0x7c, 0x0f, 0x9e, 0x16, 0x2b, 0xce, 0x33, 0x57, 0x6b, 0x31, 0x5e,
            0xce, 0xcb, 0xb6, 0x40, 0x68, 0x37, 0xbf, 0x51, 0xf5,
        ]
    }

    #[test]
    fn test_compress_pubkey_length() {
        let uncompressed = known_uncompressed_pubkey();
        assert_eq!(uncompressed.len(), 65);

        let compressed = compress_pubkey(uncompressed).unwrap();
        assert_eq!(compressed.len(), 33);
    }

    #[test]
    fn test_decompress_pubkey_length() {
        let uncompressed = known_uncompressed_pubkey();
        let compressed = compress_pubkey(uncompressed.clone()).unwrap();
        assert_eq!(compressed.len(), 33);

        let decompressed = decompress_pubkey(compressed).unwrap();
        assert_eq!(decompressed.len(), 65);
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = known_uncompressed_pubkey();
        let compressed = compress_pubkey(original.clone()).unwrap();
        let decompressed = decompress_pubkey(compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_pubkey_invalid_length() {
        let result = compress_pubkey(vec![0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compress_pubkey_all_zeros() {
        let result = compress_pubkey(vec![0u8; 65]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_pubkey_wrong_length() {
        let result = decompress_pubkey(vec![0u8; 32]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("33 byte"));

        let result = decompress_pubkey(vec![0u8; 34]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("33 byte"));
    }

    #[test]
    fn test_decompress_pubkey_empty() {
        let result = decompress_pubkey(vec![]);
        assert!(result.is_err());
    }
}
