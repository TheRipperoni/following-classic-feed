use crate::constants::DID_KEY_PREFIX;
use anyhow::{bail, Result};
use multibase;

pub fn extract_multikey(did: &String) -> Result<String> {
    if !did.starts_with(DID_KEY_PREFIX) {
        bail!("Incorrect prefix for did:key: {did}")
    }
    let multikey = &did[DID_KEY_PREFIX.len()..];
    if multikey.is_empty() {
        bail!("Empty multikey in did:key: {did}")
    }
    Ok(multikey.to_string())
}

pub fn extract_prefixed_bytes(multikey: String) -> Result<Vec<u8>> {
    // The multikey is in multibase format (e.g. 'z' prefix for base58btc).
    // multibase::decode strips the prefix and decodes the remaining data.
    let (_, decoded) = multibase::decode(multikey)?;
    Ok(decoded)
}

pub fn has_prefix(bytes: &[u8], prefix: &[u8]) -> bool {
    if bytes.len() < prefix.len() {
        return false;
    }
    prefix == &bytes[0..prefix.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DID_KEY_PREFIX, P256_DID_PREFIX, SECP256K1_DID_PREFIX};

    #[test]
    fn test_extract_multikey_valid() {
        let result = extract_multikey(&"did:key:zabc123".to_string()).unwrap();
        assert_eq!(result, "zabc123");
    }

    #[test]
    fn test_extract_multikey_invalid_prefix() {
        let err = extract_multikey(&"did:web:example.com".to_string()).unwrap_err();
        assert!(err.to_string().contains("Incorrect prefix"));
    }

    #[test]
    fn test_extract_multikey_empty() {
        let err = extract_multikey(&format!("{DID_KEY_PREFIX}")).unwrap_err();
        assert!(err.to_string().contains("Empty multikey"));
    }

    #[test]
    fn test_extract_prefixed_bytes_valid_hex() {
        // 'f' is the multibase prefix for hex/base16
        let result = extract_prefixed_bytes("f001122".to_string()).unwrap();
        assert_eq!(result, vec![0x00, 0x11, 0x22]);
    }

    #[test]
    fn test_extract_prefixed_bytes_invalid_multibase() {
        let err = extract_prefixed_bytes("!!!invalid".to_string());
        assert!(err.is_err());
    }

    #[test]
    fn test_has_prefix_match() {
        let bytes = vec![0xe7, 0x01, 0x01, 0x02, 0x03];
        assert!(has_prefix(&bytes, &SECP256K1_DID_PREFIX));
    }

    #[test]
    fn test_has_prefix_no_match() {
        let bytes = vec![0x01, 0x02, 0x03];
        assert!(!has_prefix(&bytes, &SECP256K1_DID_PREFIX));
    }

    #[test]
    fn test_has_prefix_shorter_than_prefix() {
        let bytes = vec![0xe7];
        assert!(!has_prefix(&bytes, &SECP256K1_DID_PREFIX));
    }

    #[test]
    fn test_has_prefix_p256_match() {
        let bytes = vec![0x80, 0x24, 0x01, 0x02];
        assert!(has_prefix(&bytes, &P256_DID_PREFIX));
    }

    #[test]
    fn test_has_prefix_empty_bytes() {
        assert!(!has_prefix(&[], &SECP256K1_DID_PREFIX));
    }
}
