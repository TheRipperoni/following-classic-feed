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
