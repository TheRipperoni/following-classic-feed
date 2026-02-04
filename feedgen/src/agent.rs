use crate::models::Follow;
use bsky_sdk::api::agent::Configure;
use bsky_sdk::api::com::atproto::repo::list_records::Record;
use bsky_sdk::api::types::string::{AtIdentifier, Nsid};
use bsky_sdk::api::types::Unknown;
use bsky_sdk::BskyAgent;
use ipld_core::ipld::Ipld;
use std::str::FromStr;

#[tracing::instrument]
pub async fn determine_pds(did: &str) -> String {
    match identity::determine_pds(did).await {
        Ok(pds) => pds,
        Err(e) => {
            tracing::error!("Error resolving PDS for {}: {}", did, e);
            "https://bsky.social".to_string() // Fallback
        }
    }
}

#[tracing::instrument(skip(agent))]
pub async fn get_follows(agent: &BskyAgent, did: &str) -> Vec<Follow> {
    use bsky_sdk::api::com::atproto::repo::list_records::{Parameters, ParametersData};
    let mut records: Vec<Record> = Vec::new();
    let mut follows = Vec::new();
    let mut cursor: Option<String> = None;

    let endpoint = determine_pds(did).await;
    agent.configure_endpoint(endpoint);

    match agent
        .api
        .com
        .atproto
        .repo
        .list_records(Parameters {
            data: ParametersData {
                collection: Nsid::new(String::from("app.bsky.graph.follow")).unwrap(),
                cursor,
                limit: None,
                repo: AtIdentifier::from_str(did).unwrap(),
                reverse: None,
            },
            extra_data: Ipld::Null,
        })
        .await
    {
        Ok(res) => {
            cursor = res.cursor.clone();
            records = res.records.clone();
        }
        Err(e) => {
            tracing::error!(
                "{}",
                format!(
                    "Error calling get following records: {x}",
                    x = e.to_string()
                )
            );
            cursor = None;
        }
    }
    while cursor.is_some() {
        match agent
            .api
            .com
            .atproto
            .repo
            .list_records(Parameters {
                data: ParametersData {
                    collection: Nsid::new(String::from("app.bsky.graph.follow")).unwrap(),
                    cursor,
                    limit: None,
                    repo: AtIdentifier::from_str(did).unwrap(),
                    reverse: None,
                },
                extra_data: Ipld::Null,
            })
            .await
        {
            Ok(mut res) => {
                cursor = res.cursor.clone();
                records.append(&mut res.records);
            }
            Err(e) => {
                tracing::error!(
                    "{}",
                    format!(
                        "Error calling get following records: {x}",
                        x = e.to_string()
                    )
                );
                cursor = None;
            }
        }
    }
    for record in records.iter_mut() {
        match record.value.clone() {
            Unknown::Object(obj) => {
                let obj_type = obj.get("$type");
                match obj_type {
                    None => {}
                    Some(x) => {
                        let follow_field: String = <Ipld as Clone>::clone(x)
                            .try_into()
                            .unwrap_or(String::from("no"));
                        if follow_field == "app.bsky.graph.follow" {
                            let subject: String;
                            let created_at: String;
                            match obj.get("subject") {
                                None => {
                                    panic!()
                                }
                                Some(x) => {
                                    subject = <Ipld as Clone>::clone(x).try_into().unwrap();
                                }
                            }
                            match obj.get("createdAt") {
                                None => {
                                    panic!()
                                }
                                Some(x) => {
                                    created_at = <Ipld as Clone>::clone(x).try_into().unwrap();
                                }
                            }

                            let new_follow = Follow {
                                uri: record.uri.clone(),
                                cid: record.cid.as_ref().to_string(),
                                author: did.to_string(),
                                subject,
                                created_at: created_at.clone(),
                                indexed_at: created_at,
                                prev: None,
                                sequence: None,
                            };

                            follows.push(new_follow);
                        }
                    }
                }
            }
            Unknown::Null => {}
            Unknown::Other(_) => {}
        }
    }
    follows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_determine_pds() {
        let did = "did:plc:z72i7hdynmk606qw7fm6zsk2"; // Example DID
        let pds = determine_pds(did).await;
        println!("PDS for {}: {}", did, pds);
        assert!(pds.starts_with("https://"));
    }
}

pub async fn get_agent() -> anyhow::Result<BskyAgent> {
    let agent: BskyAgent = BskyAgent::builder().build().await?;
    Ok(agent)
}
