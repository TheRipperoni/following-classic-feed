use crate::models::{
    DescribeFeedGenerator, FeedDescription, KnownService, NotFoundErrorCode,
    PathUnknownErrorMessageResponse, WellKnown,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use diesel::r2d2::R2D2Connection;
use std::env;

pub const FOLLOWING_TRAD: &str =
    "at://did:plc:khvyd3oiw46vif5gm7hijslk/app.bsky.feed.generator/following-trad";
pub const FOLLOWING_CLASSIC: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.generator/follow-orig";
pub const MEDIA: &str = "at://did:plc:nffcjkyymm3pzutbxobso2pa/app.bsky.feed.generator/media";
pub const MUTUALS: &str = "at://did:web:following.ripperoni.com/app.bsky.feed.generator/mutuals";

pub mod algo;
pub mod auth;
pub mod config;
pub mod queue;
pub mod stats;

pub use algo::*;
pub use auth::*;
pub use config::*;
pub use queue::*;
pub use stats::*;

pub async fn well_known() -> Response {
    let hostname = env::var("FEEDGEN_HOSTNAME").unwrap_or_default();
    let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or_default();

    if service_did.starts_with("did:") {
        let known_service = KnownService {
            id: "#bsky_fg".to_string(),
            r#type: "BskyFeedGenerator".into(),
            service_endpoint: format!("https://{}", hostname),
        };
        let result = WellKnown {
            context: vec!["https://www.w3.org/ns/did/v1".into()],
            id: service_did,
            service: vec![known_service],
        };
        Json(result).into_response()
    } else {
        let path_error = PathUnknownErrorMessageResponse {
            code: Some(NotFoundErrorCode::NotFoundError),
            message: Some("Not Found".to_string()),
        };
        (StatusCode::NOT_FOUND, Json(path_error)).into_response()
    }
}

/// Health check endpoint for the feed generator service.
/// Returns 200 OK if the service is running and can reach the database.
#[tracing::instrument(skip(connection))]
pub async fn health_check(State(connection): State<crate::ReadReplicaConn>) -> Response {
    // Attempt to get a connection from the pool and verify it's alive
    let db_ok = match connection.0.get().await {
        Ok(conn) => conn.interact(|c| c.ping().is_ok()).await.unwrap_or(false),
        Err(_) => false,
    };

    if db_ok {
        Json(serde_json::json!({
            "status": "ok",
            "service": "feedgen"
        }))
        .into_response()
    } else {
        let status = StatusCode::SERVICE_UNAVAILABLE;
        (
            status,
            Json(serde_json::json!({
                "status": "error",
                "service": "feedgen",
                "message": "Database connection failed"
            })),
        )
            .into_response()
    }
}

/// Implements the `app.bsky.feed.describeFeedGenerator` XRPC endpoint.
/// Returns the service DID and a list of available feed URIs.
pub async fn describe_feed_generator() -> Response {
    let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or_default();

    let feeds = vec![
        FeedDescription {
            uri: FOLLOWING_CLASSIC.to_string(),
        },
        FeedDescription {
            uri: FOLLOWING_TRAD.to_string(),
        },
        FeedDescription {
            uri: MEDIA.to_string(),
        },
        FeedDescription {
            uri: MUTUALS.to_string(),
        },
    ];

    let response = DescribeFeedGenerator {
        did: service_did,
        feeds,
    };
    Json(response).into_response()
}
