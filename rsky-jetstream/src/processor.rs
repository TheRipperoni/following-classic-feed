use crate::jetstream::{read, JetstreamRepoMessage, Lexicon};
use crate::queue::{queue_create, queue_delete, update_cursor};
use rsky_lexicon::app::bsky::feed::like::Like;
use rsky_lexicon::app::bsky::feed::{Post, Repost};
use rsky_lexicon::app::bsky::graph::follow::Follow;
use std::env;

#[tracing::instrument]
pub async fn process(message: String, client: &reqwest::Client) {
    let default_queue_path =
        env::var("FEEDGEN_QUEUE_ENDPOINT").unwrap_or("http://127.0.0.1:8000".into());
    let default_subscriber_path = env::var("FEEDGEN_SUBSCRIPTION_ENDPOINT")
        .unwrap_or("wss://jetstream1.us-west.bsky.network".into());

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
                    // update stored cursor every 20 events or so
                    if commit.time_us.rem_euclid(20) == 0 {
                        let cursor_endpoint = format!("{}/cursor", default_queue_path);
                        let resp = update_cursor(
                            cursor_endpoint,
                            default_subscriber_path,
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
                            let cid = commit.commit.cid;
                            match commit.commit.record {
                                Some(Lexicon::AppBskyFeedPost(r)) => {
                                    let post: Box<Post> = r;
                                    let uri = String::from("at://")
                                        + commit.did.as_str()
                                        + "/app.bsky.feed.post/"
                                        + commit.commit.rkey.as_str();
                                    let create = crate::models::CreateOp {
                                        uri: uri.to_owned(),
                                        cid: cid.unwrap().to_string(),
                                        author: commit.did.to_owned(),
                                        record: post,
                                    };
                                    posts_to_create.push(create);
                                }
                                Some(Lexicon::AppBskyFeedRepost(r)) => {
                                    let repost: Repost = r;
                                    let uri = String::from("at://")
                                        + commit.did.as_str()
                                        + "/app.bsky.feed.repost/"
                                        + commit.commit.rkey.as_str();
                                    let create = crate::models::CreateOp {
                                        uri: uri.to_owned(),
                                        cid: cid.unwrap().to_string(),
                                        author: commit.did.to_owned(),
                                        record: repost,
                                    };
                                    reposts_to_create.push(create);
                                }
                                Some(Lexicon::AppBskyFeedLike(r)) => {
                                    let like: Like = r;
                                    let uri = String::from("at://")
                                        + commit.did.as_str()
                                        + "/app.bsky.feed.like/"
                                        + commit.commit.rkey.as_str();
                                    let create = crate::models::CreateOp {
                                        uri: uri.to_owned(),
                                        cid: cid.unwrap().to_string(),
                                        author: commit.did.to_owned(),
                                        record: like,
                                    };
                                    likes_to_create.push(create);
                                }
                                Some(Lexicon::AppBskyFeedFollow(r)) => {
                                    let follow: Follow = r;
                                    let uri = String::from("at://")
                                        + commit.did.as_str()
                                        + "/app.bsky.graph.follow/"
                                        + commit.commit.rkey.as_str();
                                    let create = crate::models::CreateOp {
                                        uri: uri.to_owned(),
                                        cid: cid.unwrap().to_string(),
                                        author: commit.did.to_owned(),
                                        record: follow,
                                    };
                                    follows_to_create.push(create);
                                }
                                _ => {}
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
                let queue_endpoint = format!("{}/queue/{}/create", default_queue_path, "posts");
                let resp = queue_create(queue_endpoint, posts_to_create, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !posts_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", default_queue_path, "posts");
                let resp = queue_delete(queue_endpoint, posts_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !reposts_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", default_queue_path, "reposts");
                let resp = queue_create(queue_endpoint, reposts_to_create, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !reposts_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", default_queue_path, "reposts");
                let resp = queue_delete(queue_endpoint, reposts_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !likes_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", default_queue_path, "likes");
                let resp = queue_create(queue_endpoint, likes_to_create, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !likes_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", default_queue_path, "likes");
                let resp = queue_delete(queue_endpoint, likes_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !follows_to_create.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/create", default_queue_path, "follows");
                let resp = queue_create(queue_endpoint, follows_to_create, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
            if !follows_to_delete.is_empty() {
                let queue_endpoint = format!("{}/queue/{}/delete", default_queue_path, "follows");
                let resp = queue_delete(queue_endpoint, follows_to_delete, client).await;
                match resp {
                    Ok(()) => (),
                    Err(error) => tracing::error!("Records failed to queue: {error:?}"),
                };
            }
        }
        Err(error) => tracing::error!(
            "@LOG: Error unwrapping message and header: {}",
            error.to_string()
        ),
    }
}
