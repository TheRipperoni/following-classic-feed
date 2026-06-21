use crate::constants::PLUGINS;
use crate::did::parse_did_key;
use crate::types::VerifyOptions;
use anyhow::{bail, Result};

pub fn verify_signature(
    did_key: &String,
    data: &[u8],
    sig: &[u8],
    opts: Option<VerifyOptions>,
) -> Result<bool> {
    let parsed = parse_did_key(did_key)?;
    let plugin = PLUGINS.into_iter().find(|p| *p.jwt_alg == parsed.jwt_alg);
    match plugin {
        None => bail!("Unsupported signature alg: {0}", parsed.jwt_alg),
        Some(plugin) => (plugin.verify_signature)(did_key, data, sig, opts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_invalid_did_format() {
        let result = verify_signature(&"not-a-did-key".to_string(), b"data", b"sig", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Incorrect prefix"));
    }

    #[test]
    fn test_verify_signature_unsupported_jwt_alg() {
        // did:key with a base58btc multikey that doesn't match any known plugin prefix
        let result = verify_signature(
            &"did:key:z6MkhaXgBZDqoKtUfP1YqYbVJmZGzZ8zZ8zZ8zZ8zZ8zZ8zZ8zZ8zZ8".to_string(),
            b"data",
            b"sig",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature_empty_data() {
        // A valid-looking did:key structure that will fail at key extraction
        let result = verify_signature(&"did:key:z".to_string(), b"", b"", None);
        assert!(result.is_err());
    }
}
