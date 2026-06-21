use crate::constants::{DID_KEY_PREFIX, PLUGINS};
use crate::utils::{extract_multikey, extract_prefixed_bytes, has_prefix};
use anyhow::{bail, Result};
use multibase::{encode, Base};

#[derive(Clone, Debug)]
pub struct ParsedMultikey {
    pub jwt_alg: String,
    pub key_bytes: Vec<u8>,
}

pub fn parse_multikey(multikey: String) -> Result<ParsedMultikey> {
    let prefixed_bytes = extract_prefixed_bytes(multikey)?;
    let plugin = PLUGINS
        .into_iter()
        .find(|p| has_prefix(&prefixed_bytes, &p.prefix));
    if let Some(plugin) = plugin {
        let key_bytes = (plugin.decompress_pubkey)(prefixed_bytes[plugin.prefix.len()..].to_vec())?;
        Ok(ParsedMultikey {
            jwt_alg: plugin.jwt_alg.to_string(),
            key_bytes,
        })
    } else {
        bail!("Unsupported key type")
    }
}

pub fn format_multikey(jwt_alg: String, key_bytes: Vec<u8>) -> Result<String> {
    let plugin = PLUGINS.into_iter().find(|p| *p.jwt_alg == jwt_alg);
    if let Some(plugin) = plugin {
        let prefixed_bytes: Vec<u8> =
            [plugin.prefix.to_vec(), (plugin.compress_pubkey)(key_bytes)?].concat();
        // NOTE: multibase::encode already prepends the base58btc prefix ('z')
        Ok(encode(Base::Base58Btc, prefixed_bytes))
    } else {
        bail!("Unsupported key type")
    }
}

pub fn parse_did_key(did: &String) -> Result<ParsedMultikey> {
    let multikey = extract_multikey(did)?;
    parse_multikey(multikey)
}

pub fn format_did_key(jwt_alg: String, key_bytes: Vec<u8>) -> Result<String> {
    Ok([
        DID_KEY_PREFIX,
        format_multikey(jwt_alg, key_bytes)?.as_str(),
    ]
    .concat())
}

#[cfg(test)]
mod did_tests {
    use super::*;
    #[test]
    fn test_secp256k1_roundtrip() {
        let secp = secp256k1::Secp256k1::new();
        let secret_key = secp256k1::SecretKey::from_byte_array([0xcd; 32]).unwrap();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_bytes = public_key.serialize();
        assert_eq!(pubkey_bytes.len(), 33);

        let did = format_did_key("ES256K".to_string(), pubkey_bytes.to_vec()).unwrap();
        assert!(did.starts_with("did:key:z"));

        let result = parse_did_key(&did).unwrap();
        assert_eq!(result.jwt_alg, "ES256K");
        assert_eq!(result.key_bytes.len(), 65); // uncompressed secp256k1
    }

    #[test]
    fn test_p256_roundtrip() {
        // Known P-256 uncompressed public key (65 bytes: 0x04 || x || y)
        let raw: Vec<u8> = vec![
            0x04, 0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63,
            0xa4, 0x40, 0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39,
            0x45, 0xd8, 0x98, 0xc2, 0x96, 0x4f, 0xe3, 0x42, 0xe2, 0xfe, 0x1a, 0x7f, 0x9b, 0x8e,
            0xe7, 0xeb, 0x4a, 0x7c, 0x0f, 0x9e, 0x16, 0x2b, 0xce, 0x33, 0x57, 0x6b, 0x31, 0x5e,
            0xce, 0xcb, 0xb6, 0x40, 0x68, 0x37, 0xbf, 0x51, 0xf5,
        ];
        assert_eq!(raw.len(), 65);

        let did = format_did_key("ES256".to_string(), raw.to_vec()).unwrap();
        assert!(did.starts_with("did:key:z"));

        let result = parse_did_key(&did).unwrap();
        assert_eq!(result.jwt_alg, "ES256");
    }

    #[test]
    fn test_parse_did_key_invalid_prefix() {
        let err = parse_did_key(&"did:web:example.com".to_string()).unwrap_err();
        assert!(err.to_string().contains("Incorrect prefix"));
    }

    #[test]
    fn test_parse_did_key_empty_multikey() {
        let err = parse_did_key(&"did:key:".to_string()).unwrap_err();
        assert!(err.to_string().contains("Empty multikey"));
    }

    #[test]
    fn test_format_did_key_unsupported_jwt_alg() {
        let err = format_did_key("UNSUPPORTED".to_string(), vec![0u8; 33]).unwrap_err();
        assert!(err.to_string().contains("Unsupported key type"));
    }
}
