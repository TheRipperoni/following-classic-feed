use crate::models::{KnownService, NotFoundErrorCode, PathUnknownErrorMessageResponse, WellKnown};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::env;

const FOLLOWING_TRAD: &str =
    "at://did:plc:khvyd3oiw46vif5gm7hijslk/app.bsky.feed.generator/following-trad";
const FOLLOWING_CLASSIC: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.generator/follow-orig";
const MEDIA: &str = "at://did:plc:nffcjkyymm3pzutbxobso2pa/app.bsky.feed.generator/media";

pub mod algo;
pub mod config;
pub mod queue;
pub mod stats;

pub use algo::*;
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
