use anyhow::{bail, Result};
use secp256k1::PublicKey;

pub fn compress_pubkey(pubkey_bytes: Vec<u8>) -> Result<Vec<u8>> {
    let point = PublicKey::from_slice(pubkey_bytes.as_slice())?.serialize();
    Ok(point.to_vec())
}

pub fn decompress_pubkey(compressed: Vec<u8>) -> Result<Vec<u8>> {
    if compressed.len() != 33 {
        bail!("Expected 33 byte compress pubkey")
    }
    let point = PublicKey::from_slice(compressed.as_slice())?.serialize_uncompressed();
    Ok(point.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a known uncompressed secp256k1 public key for testing.
    /// This is derived from the secret key 0xcd repeated 32 times.
    fn known_uncompressed_pubkey() -> Vec<u8> {
        let secp = secp256k1::Secp256k1::new();
        let secret_key = secp256k1::SecretKey::from_byte_array([0xcd; 32]).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        public_key.serialize_uncompressed().to_vec()
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
        let compressed = compress_pubkey(uncompressed).unwrap();
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
