use crate::models::JwtParts;
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose, Engine as _};
use identity::IdResolver;
use serde_derive::{Deserialize, Serialize};
use sha2::Digest;
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
    resolver: &mut IdResolver,
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
        .expect("Time went backwards")
        .as_secs() as u128;

    if now > payload.exp {
        bail!("jwt expired");
    }
    if service_did != &payload.aud {
        bail!("jwt audience does not match service did");
    }

    // Verify cryptographic signature Omitted for now
    // let header_b64_raw = parts[0];
    // let payload_b64_raw = parts[1];
    // let message = format!("{}.{}", header_b64_raw, payload_b64_raw);
    // let message_bytes = message.as_bytes();
    //
    // let signature = general_purpose::URL_SAFE_NO_PAD
    //     .decode(parts[2])
    //     .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(parts[2]))
    //     .map_err(|_| anyhow!("error decoding signature"))?;
    //
    // if header.alg == "HS256" {
    //     use hmac::{Hmac, Mac};
    //     use sha2::Sha256;
    //     type HmacSha256 = Hmac<Sha256>;
    //
    //     let secret = std::env::var("JWT_SECRET")
    //         .or_else(|_| std::env::var("API_KEY"))
    //         .map_err(|_| anyhow!("JWT secret not configured"))?;
    //     let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
    //         .map_err(|_| anyhow!("invalid hmac secret length"))?;
    //     mac.update(message_bytes);
    //     if mac.verify_slice(&signature).is_ok() {
    //         return serde_json::to_string(&payload)
    //             .map_err(|_| anyhow!("error serializing payload"));
    //     } else {
    //         bail!("jwt signature verification failed");
    //     }
    // }
    //
    // let mut hasher = sha2::Sha256::new();
    // hasher.update(message_bytes);
    // let digest = hasher.finalize();
    //
    // let did_doc = resolver
    //     .did
    //     .ensure_resolve(&payload.iss, None)
    //     .await
    //     .map_err(|e| anyhow!("failed to resolve issuer DID: {}", e))?;
    //
    // let verification_methods = did_doc
    //     .verification_method
    //     .ok_or_else(|| anyhow!("no verification methods found in DID document"))?;
    //
    // let mut verified = false;
    // for method in verification_methods {
    //     if let Some(mut pub_key) = method.public_key_multibase {
    //         if !pub_key.starts_with("did:key:") {
    //             pub_key = format!("did:key:{}", pub_key);
    //         }
    //         println!("Verifying with PubKey: {}", pub_key);
    //         let res_raw =
    //             crypto::verify::verify_signature(&pub_key, message_bytes, &signature, None);
    //         println!("Result Raw: {:?}", res_raw);
    //         if let Ok(true) = res_raw {
    //             verified = true;
    //             break;
    //         }
    //         let res_digest = crypto::verify::verify_signature(&pub_key, &digest, &signature, None);
    //         println!("Result Digest: {:?}", res_digest);
    //         if let Ok(true) = res_digest {
    //             verified = true;
    //             break;
    //         }
    //     }
    // }
    //
    // if !verified {
    //     bail!("jwt signature verification failed");
    // }

    serde_json::to_string(&payload).map_err(|_| anyhow!("error serializing payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::did::format_did_key;
    use identity::types::{DidDocument, IdentityResolverOpts, VerificationMethod};
    use secp256k1::{Message, Secp256k1, SecretKey};

    #[tokio::test]
    async fn test_verify_jwt_success() {
        use sha2::{Digest, Sha256};
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[0xcd; 32]).expect("32 bytes, within curve order");
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

        let mut resolver = IdResolver::new(IdentityResolverOpts {
            timeout: None,
            plc_url: None,
            did_cache: None,
            backup_nameservers: None,
        });

        let did_doc = DidDocument {
            context: None,
            id: did.clone(),
            also_known_as: None,
            verification_method: Some(vec![VerificationMethod {
                id: format!("{}#key-1", did),
                r#type: "EcdsaSecp256k1VerificationKey2019".to_string(),
                controller: did.clone(),
                public_key_multibase: Some(did.clone()),
            }]),
            service: None,
        };

        resolver
            .did
            .cache
            .as_mut()
            .unwrap()
            .cache_did(did.clone(), did_doc)
            .await
            .unwrap();

        let result = verify_jwt(&jwt, &service_did, &mut resolver).await;
        if let Err(ref e) = result {
            println!("JWT: {}", jwt);
            println!("Error: {:?}", e);
        }
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

        let mut resolver = IdResolver::new(IdentityResolverOpts {
            timeout: None,
            plc_url: None,
            did_cache: None,
            backup_nameservers: None,
        });

        let result = verify_jwt(
            &jwt,
            &"did:example:feedGenerator".to_string(),
            &mut resolver,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unsupported algorithm: RS256"
        );
    }

    #[tokio::test]
    async fn test_verify_jwt_hs256_success() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
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

        let mut resolver = IdResolver::new(IdentityResolverOpts {
            timeout: None,
            plc_url: None,
            did_cache: None,
            backup_nameservers: None,
        });

        let result = verify_jwt(&jwt, &service_did, &mut resolver).await;
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let verified_payload: JwtParts = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(verified_payload.iss, "did:example:alice");
    }
}
