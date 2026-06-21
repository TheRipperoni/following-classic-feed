use crate::constants::P256_DID_PREFIX;
use crate::types::VerifyOptions;
use crate::utils::{extract_multikey, extract_prefixed_bytes, has_prefix};
use anyhow::{bail, Result};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};

pub fn verify_did_sig(
    did: &String,
    data: &[u8],
    sig: &[u8],
    opts: Option<VerifyOptions>,
) -> Result<bool> {
    let prefixed_bytes = extract_prefixed_bytes(extract_multikey(did)?)?;
    if !has_prefix(&prefixed_bytes, &P256_DID_PREFIX) {
        bail!("Not a P-256 did:key: {did}");
    }
    let key_bytes = &prefixed_bytes[P256_DID_PREFIX.len()..];
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
    if !allow_malleable && !is_compact_format(sig) {
        return Ok(false);
    }
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key)?;
    let signature = Signature::try_from(sig)?;
    Ok(verifying_key.verify(data, &signature).is_ok())
}

pub fn is_compact_format(sig: &[u8]) -> bool {
    let mut parsed = match Signature::try_from(sig) {
        Ok(res) => res,
        Err(_) => return false,
    };
    parsed = match parsed.normalize_s() {
        Some(res) => res,
        None => return false,
    };
    parsed.to_vec() == *sig
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_compact_format_parseable_only() {
        // A validly-parseable 64-byte sequence (r || s, 32 bytes each)
        let sig = [0xabu8; 64];
        // Just ensure the function doesn't panic
        let _result = is_compact_format(&sig);
    }

    #[test]
    fn test_is_compact_format_invalid_length() {
        assert!(!is_compact_format(&[0u8; 32]));
        assert!(!is_compact_format(&[0u8; 63]));
        assert!(!is_compact_format(&[0u8; 65]));
        assert!(!is_compact_format(&[]));
    }

    #[test]
    fn test_is_compact_format_high_s_not_normalized() {
        // s > n/2 should be normalized to s' = n - s,
        // so the original sig won't match normalized form
        let mut sig = [0xffu8; 64];
        sig[32] = 0x80; // Ensure high s (high bit set)
        let result = is_compact_format(&sig);
        // If s was normalized, the original won't match
        // either way this shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_is_compact_format_random_bytes() {
        let sig = [0xabu8; 64];
        let result = is_compact_format(&sig);
        // Random bytes might or might not be valid, this shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_verify_sig_invalid_key() {
        // Should not panic with empty key
        let _result = verify_sig(&[], b"data", b"sig", None);
    }

    #[test]
    fn test_verify_sig_invalid_key_length() {
        // Should not panic with short key
        let _result = verify_sig(&[0u8; 5], b"data", b"sig", None);
    }

    #[test]
    fn test_verify_did_sig_invalid_did() {
        let result = verify_did_sig(&"not-a-did".to_string(), b"data", b"sig", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_did_sig_non_p256_did() {
        let result = verify_did_sig(&"did:web:example.com".to_string(), b"data", b"sig", None);
        assert!(result.is_err());
    }
}
