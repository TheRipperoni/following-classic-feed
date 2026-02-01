use crate::models::JwtParts;
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose, Engine as _};
use rsky_crypto::verify::verify_signature;
use rsky_identity::IdResolver;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod extractors;

/**
 * Verifies a JSON Web Token (JWT) for a given service DID.
 *
 * @param jwtstr - The JWT string to verify.
 * @param service_did - The DID of the service to verify against.
 * @param resolver - The identity resolver for DID resolution.
 * @returns A Result containing the DID of the JWT issuer or an error.
 */
pub async fn verify_jwt(
    jwtstr: &str,
    service_did: &String,
    resolver: &mut IdResolver,
) -> Result<String> {
    let parts = jwtstr.split(".").collect::<Vec<_>>();

    if parts.len() != 3 {
        bail!("poorly formatted jwt");
    }

    let payload_bytes = general_purpose::STANDARD_NO_PAD
        .decode(parts[1])
        .map_err(|_| anyhow!("error decoding payload"))?;
    let payload_str =
        std::str::from_utf8(&payload_bytes).map_err(|_| anyhow!("error parsing payload"))?;
    let payload = serde_json::from_str::<JwtParts>(payload_str)
        .map_err(|_| anyhow!("error parsing payload"))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as u128;

    if now > payload.exp {
        bail!("jwt expired");
    }
    if service_did != &payload.aud {
        bail!("jwt audience does not match service did");
    }

    // Verify cryptographic signature
    let header_bytes = parts[0].as_bytes();
    let payload_bytes_raw = parts[1].as_bytes();
    let mut message = Vec::with_capacity(header_bytes.len() + 1 + payload_bytes_raw.len());
    message.extend_from_slice(header_bytes);
    message.push(b'.');
    message.extend_from_slice(payload_bytes_raw);

    let signature = general_purpose::STANDARD_NO_PAD
        .decode(parts[2])
        .map_err(|_| anyhow!("error decoding signature"))?;

    let did_doc = resolver
        .did
        .ensure_resolve(&payload.iss, None)
        .await
        .map_err(|e| anyhow!("failed to resolve issuer DID: {}", e))?;

    let verification_methods = did_doc
        .verification_method
        .ok_or_else(|| anyhow!("no verification methods found in DID document"))?;

    let mut verified = false;
    for method in verification_methods {
        if let Some(pub_key) = method.public_key_multibase {
            if let Ok(res) = verify_signature(&pub_key, &message, &signature, None) {
                if res {
                    verified = true;
                    break;
                }
            }
        }
    }

    if !verified {
        bail!("jwt signature verification failed");
    }

    serde_json::to_string(&payload).map_err(|_| anyhow!("error serializing payload"))
}
