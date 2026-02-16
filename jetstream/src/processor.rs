use crate::jetstream::{read, JetstreamRepoMessage, Lexicon};
use crate::metrics::Metrics;
use crate::queue::{queue_create, queue_delete, update_cursor};
use lexicon::app::bsky::feed::like::Like;
use lexicon::app::bsky::feed::{Post, Repost};
use lexicon::app::bsky::graph::follow::Follow;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[tracing::instrument(skip(metrics))]
pub async fn process(
    message: String,
    client: &reqwest::Client,
    queue_path: &str,
    subscriber_path: &str,
    skip_cursor: bool,
    metrics: Arc<Metrics>,
) {
    metrics.messages_processed.fetch_add(1, Ordering::Relaxed);
    match read(&message) {
        Ok(body) => {
            let mut posts_to_delete = Vec::new();
            let mut posts_to_create = Vec::new();
            let mut reposts_to_delete = Vec::new();
            let mut reposts_to_create = Vec::new();
            let mut likes_to_delete = Vec::new();
            let mut likes_to_create = Vec::new();
            let mut follows_to_delete = Vec::new();
            let mut follows_to_create = Vec::new();

            match body {
                JetstreamRepoMessage::Commit(commit) => {
                    if commit.kind.is_empty() {
                        tracing::info!("Operations empty.");
                    }
                    // update stored the cursor every 20 events or so
                    if !skip_cursor && commit.time_us.rem_euclid(20) == 0 {
                        let cursor_endpoint = format!("{}/cursor", queue_path);
                        let resp = update_cursor(
                            cursor_endpoint,
                            subscriber_path.to_string(),
                            &commit.time_us,
                            client,
                        )
                        .await;
                        match resp {
                            Ok(()) => (),
                            Err(error) => {
                                tracing::error!("@LOG: Failed to update cursor: {error:?}")
                            }
                        };
                    }

                    match commit.commit.operation.as_str() {
                        "update" => {}
                        "create" => {
                            if let Some(cid) = commit.commit.cid {
                                match commit.commit.record {
                                    Some(Lexicon::AppBskyFeedPost(r)) => {
                                        let post: Box<Post> = r;
                                        let uri = String::from("at://")
                                            + commit.did.as_str()
                                            + "/app.bsky.feed.post/"
                                            + commit.commit.rkey.as_str();
                                        let create = crate::models::CreateOp {
                                            uri: uri.to_owned(),
                                            cid: cid.to_string(),
                                            author: commit.did.to_owned(),
                                            record: post,
                                        };
                                        posts_to_create.push(create);
                                        metrics.posts_created.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Some(Lexicon::AppBskyFeedRepost(r)) => {
                                        let repost: Repost = r;
                                        let uri = String::from("at://")
                                            + commit.did.as_str()
                                            + "/app.bsky.feed.repost/"
                                            + commit.commit.rkey.as_str();
                                        let create = crate::models::CreateOp {
                                            uri: uri.to_owned(),
                                            cid: cid.to_string(),
                                            author: commit.did.to_owned(),
                                            record: repost,
                                        };
                                        reposts_to_create.push(create);
                                        metrics.reposts_created.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Some(Lexicon::AppBskyFeedLike(r)) => {
                                        let like: Like = r;
                                        let uri = String::from("at://")
                                            + commit.did.as_str()
                                            + "/app.bsky.feed.like/"
                                            + commit.commit.rkey.as_str();
                                        let create = crate::models::CreateOp {
                                            uri: uri.to_owned(),
                                            cid: cid.to_string(),
                                            author: commit.did.to_owned(),
                                            record: like,
                                        };
                                        likes_to_create.push(create);
                                        metrics.likes_created.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Some(Lexicon::AppBskyFeedFollow(r)) => {
                                        let follow: Follow = r;
                                        let uri = String::from("at://")
                                            + commit.did.as_str()
                                            + "/app.bsky.graph.follow/"
                                            + commit.commit.rkey.as_str();
                                        let create = crate::models::CreateOp {
                                            uri: uri.to_owned(),
                                            cid: cid.to_string(),
                                            author: commit.did.to_owned(),
                                            record: follow,
                                        };
                                        follows_to_create.push(create);
                                        metrics.follows_created.fetch_add(1, Ordering::Relaxed);
                                    }
                                    _ => {}
                                }
                            } else {
                                tracing::warn!(
                                    "Create operation missing CID for DID: {}, rkey: {}",
                                    commit.did,
                                    commit.commit.rkey
                                );
                            }
                        }
                        "delete" => {
                            let collection = commit.commit.collection;
                            if collection == "app.bsky.feed.post" {
                                let uri = String::from("at://")
                                    + commit.did.as_str()
                                    + "/app.bsky.feed.post/"
                                    + commit.commit.rkey.as_str();
                                let delete = crate::models::DeleteOp {
                                    uri: uri.to_owned(),
                                };
                                posts_to_delete.push(delete);
                                metrics.posts_deleted.fetch_add(1, Ordering::Relaxed);
                            }
                            if collection == "app.bsky.feed.repost" {
                                let uri = String::from("at://")
                                    + commit.did.as_str()
                                    + "/app.bsky.feed.repost/"
                                    + commit.commit.rkey.as_str();
                                let delete = crate::models::DeleteOp {
                                    uri: uri.to_owned(),
                                };
                                reposts_to_delete.push(delete);
                                metrics.reposts_deleted.fetch_add(1, Ordering::Relaxed);
                            }
                            if collection == "app.bsky.feed.like" {
                                let uri = String::from("at://")
                                    + commit.did.as_str()
                                    + "/app.bsky.feed.like/"
                                    + commit.commit.rkey.as_str();
                                let delete = crate::models::DeleteOp {
                                    uri: uri.to_owned(),
                                };
                                likes_to_delete.push(delete);
                                metrics.likes_deleted.fetch_add(1, Ordering::Relaxed);
                            }
                            if collection == "app.bsky.graph.follow" {
                                let uri = String::from("at://")
                                    + commit.did.as_str()
                                    + "/app.bsky.graph.follow/"
                                    + commit.commit.rkey.as_str();
                                let delete = crate::models::DeleteOp {
                                    uri: uri.to_owned(),
                                };
                                follows_to_delete.push(delete);
                                metrics.follows_deleted.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        _ => {}
                    }
                }
                JetstreamRepoMessage::Account(account) => {
                    tracing::info!("Received account message: {:?}", account);
                }
                JetstreamRepoMessage::Identity(identity) => {
                    tracing::info!("Received identity message: {:?}", identity);
                }
            }

            if !posts_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", queue_path, "posts");
                let resp = queue_create(queue_endpoint, posts_to_create, client).await;
                match resp {
                    Ok(response) => tracing::info!("Records queued: {:?}", response.status()),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !posts_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", queue_path, "posts");
                let resp = queue_delete(queue_endpoint, posts_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !reposts_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", queue_path, "reposts");
                let resp = queue_create(queue_endpoint, reposts_to_create, client).await;
                match resp {
                    Ok(response) => tracing::info!("Records queued: {:?}", response.status()),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !reposts_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", queue_path, "reposts");
                let resp = queue_delete(queue_endpoint, reposts_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !likes_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", queue_path, "likes");
                let resp = queue_create(queue_endpoint, likes_to_create, client).await;
                match resp {
                    Ok(response) => tracing::info!("Records queued: {:?}", response.status()),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !likes_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", queue_path, "likes");
                let resp = queue_delete(queue_endpoint, likes_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !follows_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", queue_path, "follows");
                let resp = queue_create(queue_endpoint, follows_to_create, client).await;
                match resp {
                    Ok(response) => tracing::info!("Records queued: {:?}", response.status()),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !follows_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", queue_path, "follows");
                let resp = queue_delete(queue_endpoint, follows_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
        }
        Err(error) => {
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            tracing::error!(
                "@LOG: Error unwrapping message and header: {}",
                error.to_string()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_read_commit_create_like() -> Result<()> {
        let metrics = Arc::new(Metrics::new());
        let data = "{\"did\":\"did:plc:uhtptnlcrj4wrxfjfcanf34q\",\"time_us\":1731539977109649,\"kind\":\"commit\",\"commit\":{\"rev\":\"3lauicnwejh2f\",\"operation\":\"create\",\"collection\":\"app.bsky.feed.like\",\"rkey\":\"3lauicnw5op2f\",\"record\":{\"$type\":\"app.bsky.feed.like\",\"createdAt\":\"2024-11-13T23:19:36.449Z\",\"subject\":{\"cid\":\"bafyreigw5ufnkavdzcczl2dusa3bcnkckhi4tscp6qsrsmg76s3ckseney\",\"uri\":\"at://did:plc:6wthaiuqiys3y7eztkpsdam2/app.bsky.feed.post/3latjcehsho2n\"}},\"cid\":\"bafyreifsdaip3s5nm3hcz4fbgkxodnils75oi3rmqhipwtom34rxw4vwdi\"}}";
        let client = reqwest::Client::new();
        process(data.to_string(), &client, "http://localhost", "wss://localhost", true, metrics.clone()).await;
        
        assert_eq!(metrics.messages_processed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.likes_created.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_read_error_increments_metrics() -> Result<()> {
        let metrics = Arc::new(Metrics::new());
        let data = "invalid json";
        let client = reqwest::Client::new();
        process(data.to_string(), &client, "http://localhost", "wss://localhost", true, metrics.clone()).await;
        
        assert_eq!(metrics.messages_processed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 1);
        
        Ok(())
    }
}
