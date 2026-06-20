use crate::models::JwtParts;
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose, Engine as _};
use identity::IdResolver;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod extractors;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtHeader {
    #[serde(rename = "typ")]
    pub typ: Option<String>,
    pub alg: String,
}

pub async fn verify_jwt(
    jwtstr: &str,
    service_did: &String,
    id_resolver: Option<&mut IdResolver>,
) -> Result<String> {
    let parts = jwtstr.split(".").collect::<Vec<_>>();

    if parts.len() != 3 {
        bail!("poorly formatted jwt");
    }

    let header_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[0])
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(parts[0]))
        .map_err(|_| anyhow!("error decoding header"))?;
    let header_str =
        std::str::from_utf8(&header_bytes).map_err(|_| anyhow!("error parsing header"))?;
    let header = serde_json::from_str::<JwtHeader>(header_str)
        .map_err(|_| anyhow!("error parsing header"))?;

    if header.alg != "ES256K" && header.alg != "ES256" && header.alg != "HS256" {
        bail!("unsupported algorithm: {}", header.alg);
    }

    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .map_err(|_| anyhow!("error decoding payload"))?;
    let payload_str =
        std::str::from_utf8(&payload_bytes).map_err(|_| anyhow!("error parsing payload"))?;

    let payload = serde_json::from_str::<JwtParts>(payload_str)
        .map_err(|_| anyhow!("error parsing payload"))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("system time is before UNIX epoch"))?
        .as_secs() as u128;

    if now > payload.exp {
        bail!("jwt expired");
    }
    if service_did != &payload.aud {
        bail!("jwt audience does not match service did");
    }

    // Verify cryptographic signature
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[2])
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(parts[2]))
        .map_err(|_| anyhow!("error decoding signature"))?;

    match header.alg.as_str() {
        "ES256K" | "ES256" => {
            let signing_key = if payload.iss.starts_with("did:key:") {
                // did:key format is self-contained, use directly
                payload.iss.clone()
            } else {
                // Need to resolve DID to get the signing key
                match id_resolver {
                    Some(resolver) => {
                        let atproto_data = resolver
                            .resolve_atproto_data(payload.iss.clone())
                            .await
                            .map_err(|e| anyhow!("failed to resolve DID: {}", e))?;
                        atproto_data.signing_key
                    }
                    None => {
                        bail!(
                            "cannot verify ES256K/ES256 JWT: id_resolver required for non-did:key issuer"
                        );
                    }
                }
            };

            let digest = Sha256::digest(signing_input.as_bytes());
            let valid = crypto::verify::verify_signature(
                &signing_key,
                &digest,
                &sig_bytes,
                None,
            )
            .map_err(|e| anyhow!("signature verification error: {}", e))?;

            if !valid {
                bail!("jwt signature verification failed");
            }
        }
        "HS256" => {
            use hmac::{Hmac, Mac};
            type HmacSha256 = Hmac<Sha256>;

            let secret = std::env::var("JWT_SECRET")
                .map_err(|_| anyhow!("JWT_SECRET environment variable not set"))?;

            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|_| anyhow!("invalid HMAC key"))?;
            mac.update(signing_input.as_bytes());
            let expected = mac.finalize().into_bytes().to_vec();

            if sig_bytes != expected {
                bail!("jwt signature verification failed");
            }
        }
        _ => unreachable!(), // already validated above
    }

    serde_json::to_string(&payload).map_err(|_| anyhow!("error serializing payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::did::format_did_key;
    use secp256k1::{Message, Secp256k1, SecretKey};

    #[tokio::test]
    async fn test_verify_jwt_success() {
        let secp = Secp256k1::new();
        let secret_key =
            SecretKey::from_byte_array([0xcd; 32]).expect("32 bytes, within curve order");
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_bytes = public_key.serialize();

        let did = format_did_key("ES256K".to_string(), pubkey_bytes.to_vec()).unwrap();
        let service_did = "did:example:feedGenerator".to_string();

        let header = JwtHeader {
            typ: Some("JWT".to_string()),
            alg: "ES256K".to_string(),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u128;
        let payload = JwtParts {
            iss: did.clone(),
            aud: service_did.clone(),
            exp: now + 60,
        };

        let header_json = serde_json::to_string(&header).unwrap();
        let payload_json = serde_json::to_string(&payload).unwrap();

        let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(header_json);
        let payload_b64 = general_purpose::URL_SAFE_NO_PAD.encode(payload_json);

        let message_str = format!("{}.{}", header_b64, payload_b64);
        let message_bytes = message_str.as_bytes();

        let mut hasher = Sha256::new();
        hasher.update(message_bytes);
        let digest_bytes = hasher.finalize();
        let digest = Message::from_digest_slice(&digest_bytes).unwrap();

        let sig = secp.sign_ecdsa(digest, &secret_key);
        let sig_bytes = sig.serialize_compact();
        let sig_b64 = general_purpose::URL_SAFE_NO_PAD.encode(sig_bytes);

        let jwt = format!("{}.{}", message_str, sig_b64);

        let result = verify_jwt(&jwt, &service_did, None).await;
        assert!(result.is_ok());
        let verified_payload: JwtParts = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(verified_payload.iss, did);
    }

    #[tokio::test]
    async fn test_verify_jwt_invalid_alg() {
        let header = JwtHeader {
            typ: Some("JWT".to_string()),
            alg: "RS256".to_string(),
        };
        let payload = JwtParts {
            iss: "did:example:alice".to_string(),
            aud: "did:example:feedGenerator".to_string(),
            exp: 9999999999,
        };

        let header_b64 =
            general_purpose::STANDARD_NO_PAD.encode(serde_json::to_string(&header).unwrap());
        let payload_b64 =
            general_purpose::STANDARD_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
        let jwt = format!("{}.{}.sig", header_b64, payload_b64);

        let result = verify_jwt(&jwt, &"did:example:feedGenerator".to_string(), None).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unsupported algorithm: RS256"
        );
    }

    #[tokio::test]
    async fn test_verify_jwt_hs256_success() {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let secret = "test-secret";
        std::env::set_var("JWT_SECRET", secret);

        let service_did = "did:example:feedGenerator".to_string();
        let header = JwtHeader {
            typ: Some("JWT".to_string()),
            alg: "HS256".to_string(),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u128;
        let payload = JwtParts {
            iss: "did:example:alice".to_string(),
            aud: service_did.clone(),
            exp: now + 60,
        };

        let header_json = serde_json::to_string(&header).unwrap();
        let payload_json = serde_json::to_string(&payload).unwrap();

        let header_b64 = general_purpose::STANDARD_NO_PAD.encode(header_json);
        let payload_b64 = general_purpose::STANDARD_NO_PAD.encode(payload_json);

        let message_str = format!("{}.{}", header_b64, payload_b64);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message_str.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig_b64 = general_purpose::STANDARD_NO_PAD.encode(sig_bytes);

        let jwt = format!("{}.{}", message_str, sig_b64);

        let result = verify_jwt(&jwt, &service_did, None).await;
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let verified_payload: JwtParts = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(verified_payload.iss, "did:example:alice");
    }
}
