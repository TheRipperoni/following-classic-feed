use crate::types::{AtprotoData, DidDocument};
use anyhow::{anyhow, Result};
use crypto::constants::{P256_JWT_ALG, SECP256K1_JWT_ALG};
use crypto::did::{format_did_key, parse_multikey};
use crypto::multibase::multibase_to_bytes;

#[derive(Clone)]
pub struct VerificationMaterial {
    pub r#type: String,
    pub public_key_multibase: String,
}

pub fn get_did_key_from_multibase(key: VerificationMaterial) -> Result<Option<String>> {
    let key_bytes = multibase_to_bytes(key.public_key_multibase.clone())?;
    let did_key = match key.r#type.as_str() {
        "EcdsaSecp256r1VerificationKey2019" => {
            Some(format_did_key(P256_JWT_ALG.to_string(), key_bytes)?)
        }
        "EcdsaSecp256k1VerificationKey2019" => {
            Some(format_did_key(SECP256K1_JWT_ALG.to_string(), key_bytes)?)
        }
        "Multikey" => {
            let parsed = parse_multikey(key.public_key_multibase)?;
            Some(format_did_key(parsed.jwt_alg, parsed.key_bytes)?)
        }
        _ => None,
    };
    Ok(did_key)
}

pub fn get_atproto_data(doc: DidDocument) -> Result<AtprotoData> {
    let did = doc.id.clone();
    let mut pds = None;
    let mut signing_key = None;
    let handle = doc
        .also_known_as
        .as_ref()
        .and_then(|aka| aka.first())
        .map(|h| h.trim_start_matches("at://").to_string())
        .ok_or_else(|| anyhow!("No handle found in DID document"))?;

    if let Some(services) = doc.service {
        for service in services {
            if service.r#type == "AtprotoPersonalDataServer" {
                pds = Some(service.service_endpoint);
                break;
            }
        }
    }

    if let Some(vms) = doc.verification_method {
        for vm in vms {
            if vm.id == format!("{}#atproto", did) || vm.id == "#atproto" {
                if let Some(pk) = vm.public_key_multibase {
                    signing_key = get_did_key_from_multibase(VerificationMaterial {
                        r#type: vm.r#type,
                        public_key_multibase: pk,
                    })?;
                }
                break;
            }
        }
    }

    Ok(AtprotoData {
        did,
        signing_key: signing_key.ok_or_else(|| anyhow!("No signing key found"))?,
        handle,
        pds: pds.ok_or_else(|| anyhow!("No PDS endpoint found"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DidDocument, Service};

    #[test]
    fn test_get_did_key_from_multibase_unsupported_type() {
        let material = VerificationMaterial {
            r#type: "SomeUnsupportedKeyType".to_string(),
            // Valid base58btc multibase with minimal content
            public_key_multibase: "z".to_string(),
        };
        let result = get_did_key_from_multibase(material);
        // multibase_to_bytes runs before type check, so 'z' decodes to empty bytes Ok
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_did_key_from_multibase_invalid_multibase() {
        let material = VerificationMaterial {
            r#type: "EcdsaSecp256k1VerificationKey2019".to_string(),
            public_key_multibase: "invalid!".to_string(),
        };
        let result = get_did_key_from_multibase(material);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_atproto_data_missing_handle() {
        let doc = DidDocument {
            context: None,
            id: "did:plc:test".to_string(),
            also_known_as: None,
            verification_method: None,
            service: None,
        };
        let result = get_atproto_data(doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No handle"));
    }

    #[test]
    fn test_get_atproto_data_missing_services_and_key() {
        let doc = DidDocument {
            context: None,
            id: "did:plc:test".to_string(),
            also_known_as: Some(vec!["at://alice.com".to_string()]),
            verification_method: None,
            service: None,
        };
        let result = get_atproto_data(doc);
        // signing_key is checked before pds in the final tuple, so first error is about signing key
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No signing key") || err.contains("No PDS endpoint"));
    }

    #[test]
    fn test_get_atproto_data_missing_verification_method() {
        let doc = DidDocument {
            context: None,
            id: "did:plc:test".to_string(),
            also_known_as: Some(vec!["at://alice.com".to_string()]),
            verification_method: None,
            service: Some(vec![Service {
                id: "#pds".to_string(),
                r#type: "AtprotoPersonalDataServer".to_string(),
                service_endpoint: "https://pds.example.com".to_string(),
            }]),
        };
        let result = get_atproto_data(doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No signing key"));
    }

    #[test]
    fn test_get_atproto_data_handle_without_at_prefix() {
        let doc = DidDocument {
            context: None,
            id: "did:plc:test".to_string(),
            also_known_as: Some(vec!["alice.com".to_string()]),
            verification_method: None,
            service: None,
        };
        let result = get_atproto_data(doc);
        // handle will be "alice.com" (trim_start_matches only removes "at://" prefix)
        assert!(result.is_err()); // still fails due to missing PDS/key
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No PDS endpoint") || err.contains("No signing key"));
    }

    #[test]
    fn test_get_did_key_from_multibase_empty_multibase() {
        let material = VerificationMaterial {
            r#type: "EcdsaSecp256k1VerificationKey2019".to_string(),
            public_key_multibase: "".to_string(),
        };
        let result = get_did_key_from_multibase(material);
        assert!(result.is_err());
    }
}
