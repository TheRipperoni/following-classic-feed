extern crate url;

use crate::did::atproto_data::get_atproto_data;
use crate::did::did_resolver::DidResolver;
use crate::handle::HandleResolver;
use crate::types::{
    AtprotoData, DidCache, DidResolverOpts, HandleResolverOpts, IdentityResolverOpts,
};
use anyhow::Result;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct IdResolver {
    pub handle: HandleResolver,
    pub did: DidResolver,
}

impl IdResolver {
    pub fn new(opts: IdentityResolverOpts) -> Self {
        let IdentityResolverOpts {
            timeout,
            plc_url,
            did_cache,
            backup_nameservers,
        } = opts;
        let timeout = timeout.unwrap_or_else(|| Duration::from_millis(3000));
        let did_cache = did_cache.unwrap_or_else(|| DidCache {
            stale_ttl: Default::default(),
            max_ttl: Default::default(),
            cache: Default::default(),
        });

        Self {
            handle: HandleResolver::new(HandleResolverOpts {
                timeout: Some(timeout),
                backup_nameservers,
            }),
            did: DidResolver::new(DidResolverOpts {
                timeout: Some(timeout),
                plc_url,
                did_cache,
            }),
        }
    }

    pub async fn resolve_atproto_data(&mut self, did: String) -> Result<AtprotoData> {
        let doc = self.did.ensure_resolve(&did, None).await?;
        get_atproto_data(doc)
    }
}

pub mod common;
pub mod did;
pub mod errors;
pub mod handle;
pub mod types;

pub async fn determine_pds(did: &str) -> Result<String> {
    let plc_url = "https://plc.directory";
    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{0}/{1}",
            plc_url,
            crate::common::encode_uri_component(did)
        ))
        .header("Connection", "Keep-Alive")
        .header("Keep-Alive", "timeout=5, max=1000")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("DID not found: {}", did);
    }
    let doc: types::DidDocument = response.json().await?;
    if let Some(services) = doc.service {
        for service in services {
            if service.r#type == "AtprotoPersonalDataServer" {
                return Ok(service.service_endpoint);
            }
        }
    }
    anyhow::bail!("No PDS endpoint found for DID: {}", did);
}
