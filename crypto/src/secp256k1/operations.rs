use crate::constants::SECP256K1_DID_PREFIX;
use crate::types::VerifyOptions;
use crate::utils::{extract_multikey, extract_prefixed_bytes, has_prefix};
use anyhow::{bail, Result};
use secp256k1::{ecdsa, Message, PublicKey, Secp256k1};

pub fn verify_did_sig(
    did: &String,
    data: &[u8],
    sig: &[u8],
    opts: Option<VerifyOptions>,
) -> Result<bool> {
    let prefixed_bytes = extract_prefixed_bytes(extract_multikey(did)?)?;
    if !has_prefix(&prefixed_bytes, &SECP256K1_DID_PREFIX) {
        bail!("Not a secp256k1 did:key: {did}");
    }
    let key_bytes = &prefixed_bytes[SECP256K1_DID_PREFIX.len()..];
    verify_sig(key_bytes, data, sig, opts)
}

pub fn verify_sig(
    public_key: &[u8],
    data: &[u8],
    sig: &[u8],
    opts: Option<VerifyOptions>,
) -> Result<bool> {
    let allow_malleable = match opts {
        Some(opts) if opts.allow_malleable_sig.is_some() => opts.allow_malleable_sig.unwrap(),
        _ => false,
    };
    let is_compact = is_compact_format(sig);
    if !allow_malleable && !is_compact {
        return Ok(false);
    }
    let secp = Secp256k1::verification_only();
    let public_key = PublicKey::from_slice(public_key)?;

    let data = Message::from_digest_slice(data)?;
    let sig = match is_compact {
        true => ecdsa::Signature::from_compact(sig)?,
        false => ecdsa::Signature::from_der(sig)?,
    };
    Ok(secp.verify_ecdsa(data, &sig, &public_key).is_ok())
}

pub fn is_compact_format(sig: &[u8]) -> bool {
    match ecdsa::Signature::from_compact(sig) {
        Ok(parsed) => parsed.serialize_compact() == sig,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_compact_format_valid_64_bytes() {
        // A valid 64-byte ECDSA signature (r || s, 32 bytes each)
        let sig = [0u8; 64];
        assert!(is_compact_format(&sig));
    }

    #[test]
    fn test_is_compact_format_invalid_length() {
        assert!(!is_compact_format(&[0u8; 32]));
        assert!(!is_compact_format(&[0u8; 63]));
        assert!(!is_compact_format(&[0u8; 65]));
        assert!(!is_compact_format(&[]));
    }

    #[test]
    fn test_is_compact_format_random_bytes() {
        let sig = [0xabu8; 64];
        // Random bytes are valid from_compact perspective
        // (compact sig is just two 32-byte scalars)
        let result = is_compact_format(&sig);
        assert!(result);
    }

    #[test]
    fn test_is_compact_format_der_format() {
        // DER-encoded signature starts with 0x30, not a valid compact sig
        let der_sig = [
            0x30, 0x44, 0x02, 0x20, 0x5a, 0x5a, 0x5a, 0x5a, 0x02, 0x20, 0x5a, 0x5a, 0x5a, 0x5a,
        ];
        assert!(!is_compact_format(&der_sig));
    }

    #[test]
    fn test_verify_sig_invalid_key() {
        // Should not panic with empty key
        let _result = verify_sig(&[], b"data", b"sig", None);
    }

    #[test]
    fn test_verify_sig_invalid_key_length() {
        // Should not panic with short key
        let _result = verify_sig(&[0u8; 10], b"data", b"sig", None);
    }

    #[test]
    fn test_verify_did_sig_invalid_did() {
        let result = verify_did_sig(&"did:key:invalid".to_string(), b"data", b"sig", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_did_sig_non_secp256k1_did() {
        // did:key with unsupported prefix
        let result = verify_did_sig(&"did:web:example.com".to_string(), b"data", b"sig", None);
        assert!(result.is_err());
    }
}
