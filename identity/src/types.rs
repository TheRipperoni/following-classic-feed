use crate::common::{DAY, HOUR};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DidDocument {
    #[serde(rename = "@context")]
    pub context: Option<Vec<String>>,
    pub id: String,
    #[serde(rename = "alsoKnownAs")]
    pub also_known_as: Option<Vec<String>>,
    #[serde(rename = "verificationMethod")]
    pub verification_method: Option<Vec<VerificationMethod>>,
    pub service: Option<Vec<Service>>,
}

pub struct IdentityResolverOpts {
    pub timeout: Option<Duration>,
    pub plc_url: Option<String>,
    pub did_cache: Option<DidCache>,
    pub backup_nameservers: Option<Vec<String>>,
}

pub struct HandleResolverOpts {
    pub timeout: Option<Duration>,
    pub backup_nameservers: Option<Vec<String>>,
}

pub struct DidResolverOpts {
    pub timeout: Option<Duration>,
    pub plc_url: Option<String>,
    pub did_cache: DidCache,
}

#[derive(Debug)]
pub struct AtprotoData {
    pub did: String,
    pub signing_key: String,
    pub handle: String,
    pub pds: String,
}

pub struct CacheResult {
    pub did: String,
    pub doc: DidDocument,
    pub updated_at: u128,
    pub stale: bool,
    pub expired: bool,
}

#[derive(Clone, Debug)]
pub struct CacheVal {
    pub doc: DidDocument,
    pub updated_at: u128,
}

/// MemoryCache implementation of DidCache
#[derive(Clone, Debug)]
pub struct DidCache {
    pub stale_ttl: Duration,
    pub max_ttl: Duration,
    pub cache: BTreeMap<String, CacheVal>,
}

impl DidCache {
    pub fn new(stale_ttl: Option<Duration>, max_ttl: Option<Duration>) -> Self {
        Self {
            stale_ttl: stale_ttl.unwrap_or_else(|| Duration::new(HOUR as u64, 0)),
            max_ttl: max_ttl.unwrap_or_else(|| Duration::new(DAY as u64, 0)),
            cache: BTreeMap::new(),
        }
    }

    pub async fn cache_did(&mut self, did: String, doc: DidDocument) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("timestamp in micros since UNIX epoch")
            .as_micros();
        self.cache.insert(
            did,
            CacheVal {
                doc,
                updated_at: now,
            },
        );
        Ok(())
    }

    pub async fn refresh_cache<Fut>(&mut self, did: String, get_doc: impl Fn() -> Fut) -> Result<()>
    where
        Fut: Future<Output = Result<Option<DidDocument>>>,
    {
        match get_doc().await? {
            None => Ok(()),
            Some(doc) => self.cache_did(did, doc).await,
        }
    }

    pub fn check_cache(&self, did: String) -> Result<Option<CacheResult>> {
        match self.cache.get(&did) {
            None => Ok(None),
            Some(val) => {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("timestamp in micros since UNIX epoch")
                    .as_micros();
                let expired = now > val.updated_at + self.max_ttl.as_micros();
                let stale = now > val.updated_at + self.stale_ttl.as_micros();
                let CacheVal { doc, updated_at } = val.clone();
                Ok(Some(CacheResult {
                    did,
                    doc,
                    updated_at,
                    stale,
                    expired,
                }))
            }
        }
    }

    pub fn clear_entry(&mut self, did: String) -> Result<()> {
        self.cache.remove(&did);
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_did_cache_new() {
        let cache = DidCache::new(None, None);
        // Note: common::HOUR = 1000 * 60 * 60 = 3_600_000 (milliseconds-based constant)
        // This is passed to Duration::new(secs, nanos), so the effective duration
        // is 3_600_000 seconds ~ 41.67 days, matching the current implementation.
        assert_eq!(cache.stale_ttl, Duration::new(3_600_000, 0));
        assert_eq!(cache.max_ttl, Duration::new(86_400_000, 0));
    }

    #[test]
    fn test_did_cache_new_custom_ttl() {
        let cache = DidCache::new(
            Some(Duration::new(60, 0)),  // 1 min stale
            Some(Duration::new(600, 0)), // 10 min max
        );
        assert_eq!(cache.stale_ttl, Duration::new(60, 0));
        assert_eq!(cache.max_ttl, Duration::new(600, 0));
    }

    #[tokio::test]
    async fn test_did_cache_cache_and_check() {
        let mut cache = DidCache::new(Some(Duration::new(3600, 0)), Some(Duration::new(86400, 0)));
        let did = "did:plc:test123".to_string();
        let doc = DidDocument {
            context: Some(vec!["https://www.w3.org/ns/did/v1".to_string()]),
            id: did.clone(),
            also_known_as: Some(vec!["at://alice.com".to_string()]),
            verification_method: None,
            service: None,
        };

        cache.cache_did(did.clone(), doc.clone()).await.unwrap();
        let result = cache.check_cache(did.clone()).unwrap();
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.did, did);
        assert_eq!(cached.doc.id, "did:plc:test123");
        assert!(!cached.expired);
    }

    #[tokio::test]
    async fn test_did_cache_check_missing() {
        let cache = DidCache::new(None, None);
        let result = cache
            .check_cache("did:plc:nonexistent".to_string())
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_did_cache_clear_entry() {
        let mut cache = DidCache::new(None, None);
        cache
            .cache_did(
                "did:plc:test1".to_string(),
                DidDocument {
                    context: None,
                    id: "did:plc:test1".to_string(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();
        cache
            .cache_did(
                "did:plc:test2".to_string(),
                DidDocument {
                    context: None,
                    id: "did:plc:test2".to_string(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();

        assert!(cache
            .check_cache("did:plc:test1".to_string())
            .unwrap()
            .is_some());
        cache.clear_entry("did:plc:test1".to_string()).unwrap();
        assert!(cache
            .check_cache("did:plc:test1".to_string())
            .unwrap()
            .is_none());
        assert!(cache
            .check_cache("did:plc:test2".to_string())
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_did_cache_clear_all() {
        let mut cache = DidCache::new(None, None);
        cache
            .cache_did(
                "did:plc:a".to_string(),
                DidDocument {
                    context: None,
                    id: "did:plc:a".to_string(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();
        cache
            .cache_did(
                "did:plc:b".to_string(),
                DidDocument {
                    context: None,
                    id: "did:plc:b".to_string(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();
        cache.clear().unwrap();
        assert!(cache
            .check_cache("did:plc:a".to_string())
            .unwrap()
            .is_none());
        assert!(cache
            .check_cache("did:plc:b".to_string())
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_did_cache_refresh() {
        let mut cache = DidCache::new(None, None);
        let did = "did:plc:test".to_string();

        // Initial cache
        cache
            .cache_did(
                did.clone(),
                DidDocument {
                    context: None,
                    id: did.clone(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();

        // Refresh with new doc
        cache
            .refresh_cache(did.clone(), || async {
                Ok(Some(DidDocument {
                    context: None,
                    id: "did:plc:test".to_string(),
                    also_known_as: Some(vec!["at://newhandle.com".to_string()]),
                    verification_method: None,
                    service: None,
                }))
            })
            .await
            .unwrap();

        let result = cache.check_cache(did.clone()).unwrap().unwrap();
        assert_eq!(
            result.doc.also_known_as,
            Some(vec!["at://newhandle.com".to_string()])
        );
    }

    #[tokio::test]
    async fn test_did_cache_refresh_none() {
        let mut cache = DidCache::new(None, None);
        let did = "did:plc:test".to_string();
        cache
            .cache_did(
                did.clone(),
                DidDocument {
                    context: None,
                    id: did.clone(),
                    also_known_as: None,
                    verification_method: None,
                    service: None,
                },
            )
            .await
            .unwrap();

        // Refresh returning None should not clear the entry
        cache
            .refresh_cache(did.clone(), || async { Ok(None) })
            .await
            .unwrap();
        assert!(cache.check_cache(did.clone()).unwrap().is_some());
    }

    #[test]
    fn test_did_cache_expiry() {
        let mut cache = DidCache::new(
            Some(Duration::from_secs(0)), // stale immediately
            Some(Duration::from_secs(0)), // expired immediately
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            cache
                .cache_did(
                    "did:plc:expired".to_string(),
                    DidDocument {
                        context: None,
                        id: "did:plc:expired".to_string(),
                        also_known_as: None,
                        verification_method: None,
                        service: None,
                    },
                )
                .await
                .unwrap();
        });
        let result = cache
            .check_cache("did:plc:expired".to_string())
            .unwrap()
            .unwrap();
        assert!(result.stale);
        assert!(result.expired);
    }
}
