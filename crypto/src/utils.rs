use crate::constants::{BASE58_MULTIBASE_PREFIX, DID_KEY_PREFIX};
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
    if !multikey.starts_with(BASE58_MULTIBASE_PREFIX) {
        bail!("Incorrect prefix for multikey: {multikey}")
    }
    let encoded = &multikey[BASE58_MULTIBASE_PREFIX.len()..];
    let (_, decoded) = multibase::decode(format!("{}{}", BASE58_MULTIBASE_PREFIX, encoded))?;
    Ok(decoded)
}

pub fn has_prefix(bytes: &[u8], prefix: &Vec<u8>) -> bool {
    *prefix == bytes[0..prefix.len()]
}
