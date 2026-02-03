use crate::auth::extractors::{ApiKey, OptionalAccessToken};
use crate::models::{
    AlgoResponse, CreateRequest, DeleteRequest, FollowingPreference, InternalErrorCode,
    InternalErrorMessageResponse, JanitorConfig, JwtParts, KnownService, NotFoundErrorCode,
    PathUnknownErrorMessageResponse, PostResult, SubState, UserFeedPreference, WellKnown,
};
use crate::{apis, db, ReadReplicaConn, WriteDbConn};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_derive::{Deserialize, Serialize};
use std::env;

const FOLLOWING_TRAD: &str =
    "at://did:plc:khvyd3oiw46vif5gm7hijslk/app.bsky.feed.generator/following-trad";
const FOLLOWING_CLASSIC: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.generator/follow-orig";
const MEDIA: &str = "at://did:plc:nffcjkyymm3pzutbxobso2pa/app.bsky.feed.generator/media";

#[derive(Deserialize, Debug)]
pub struct FeedSkeletonParams {
    pub feed: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn index(
    State(connection): State<ReadReplicaConn>,
    Query(params): Query<FeedSkeletonParams>,
    OptionalAccessToken(token): OptionalAccessToken,
) -> Response {
    let mut did = String::from("did:plc:jipcqdf3d36yhk3dzvjbkh6y");
    let feed = params.feed.unwrap_or_default();
    if let Some(jwt) = token {
        match serde_json::from_str::<JwtParts>(&jwt.0) {
            Ok(jwt_obj) => {
                did = jwt_obj.iss;
                tracing::info!("Visit from {did}");
                tracing::info!("{}", jwt.0);
                match apis::add_visitor(did.clone(), jwt_obj.aud, feed.to_string()) {
                    Ok(_) => (),
                    Err(_) => tracing::error!("Failed to write visitor"),
                }
            }
            Err(e) => {
                tracing::error!(%e, "Failed to parse jwt string")
            }
        }
    } else {
        let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or("".into());
        match apis::add_visitor("anonymous".into(), service_did, feed.to_string()) {
            Ok(_) => (),
            Err(_) => tracing::error!("Failed to write anonymous visitor"),
        }
    }
    match feed.as_str() {
        _following_classic if FOLLOWING_CLASSIC == _following_classic => {
            if did.is_empty() {
                let internal_error = InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("No DID".to_string()),
                };
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response();
            }
            match apis::get_posts_by_user_feed(
                did,
                params.limit,
                params.cursor.as_deref(),
                connection,
            )
            .await
            {
                Ok(response) => Json(response).into_response(),
                Err(error) => {
                    tracing::error!("Internal Error: {error}");
                    let internal_error = InternalErrorMessageResponse {
                        code: Some(InternalErrorCode::InternalError),
                        message: Some(error.to_string()),
                    };
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
                }
            }
        }
        _following_trad if FOLLOWING_TRAD == _following_trad => {
            let mut post_results = Vec::new();
            let post_result = PostResult {
                post: String::from(
                    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l4pi6irzsg2m",
                ),
                reason: None,
            };
            post_results.push(post_result);
            let response = AlgoResponse {
                cursor: Some(String::from("none")),
                feed: post_results,
            };
            Json(response).into_response()
        }
        _following_media if MEDIA == _following_media => {
            if did.is_empty() {
                let internal_error = InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("No DID".to_string()),
                };
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response();
            }
            match apis::get_posts_by_following_media(
                did,
                params.limit,
                params.cursor.as_deref(),
                connection,
            )
            .await
            {
                Ok(response) => Json(response).into_response(),
                Err(error) => {
                    tracing::error!("Internal Error: {error}");
                    let internal_error = InternalErrorMessageResponse {
                        code: Some(InternalErrorCode::InternalError),
                        message: Some(error.to_string()),
                    };
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
                }
            }
        }
        _ => {
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::Unavailable),
                message: Some("Unsupported feed".to_string()),
            };
            (StatusCode::BAD_REQUEST, Json(internal_error)).into_response()
        }
    }
}

pub async fn update_cursor(
    State(connection): State<WriteDbConn>,
    Query(params): Query<serde_json::Value>,
    _token: ApiKey,
    Json(new_cursor): Json<SubState>,
) -> Response {
    let service = params["service"].as_str().unwrap_or_default();
    match apis::update_cursor(service.to_string(), new_cursor.cursor, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn get_cursor(
    State(connection): State<ReadReplicaConn>,
    Query(params): Query<serde_json::Value>,
    _token: ApiKey,
) -> Response {
    let service = params["service"].as_str().unwrap_or_default();
    match apis::get_cursor(service.to_string(), connection).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!(
                "Internal Error: {}",
                error.message.clone().unwrap_or_default()
            );
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

#[tracing::instrument(skip(connection))]
pub async fn queue_creation(
    State(connection): State<WriteDbConn>,
    Path(lex): Path<String>,
    _token: ApiKey,
    Json(body): Json<Vec<CreateRequest>>,
) -> Response {
    tracing::info!("Queue creation request received");
    match apis::queue_creation(lex, body, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn user_config(
    State(connection): State<WriteDbConn>,
    Query(params): Query<serde_json::Value>,
    _token: ApiKey,
) -> Response {
    let did = params["did"].as_str().unwrap_or_default();
    let response = db::user_config_fetch(did.to_string(), connection).await;
    Json(response).into_response()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FollowingPrefFetchResponse {
    pub did: String,
    pub preferences: Vec<FollowingPreference>,
}

pub async fn following_preferences_fetch(
    State(connection): State<WriteDbConn>,
    Query(params): Query<serde_json::Value>,
) -> Response {
    let did = params["did"].as_str().unwrap_or_default();
    let preferences = db::following_pref_fetch(did.to_string(), connection).await;
    let response = FollowingPrefFetchResponse {
        did: did.to_string(),
        preferences,
    };
    Json(response).into_response()
}

pub async fn following_preferences_update(
    State(connection): State<WriteDbConn>,
    Json(body): Json<FollowingPreference>,
) -> Response {
    match db::following_pref_update(body, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn update_user_config(
    State(connection): State<WriteDbConn>,
    _token: ApiKey,
    Json(body): Json<UserFeedPreference>,
) -> Response {
    match db::user_config_update(body, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn queue_deletion(
    State(connection): State<WriteDbConn>,
    Path(lex): Path<String>,
    _token: ApiKey,
    Json(body): Json<Vec<DeleteRequest>>,
) -> Response {
    match apis::queue_deletion(lex, body, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn well_known() -> Response {
    let hostname = env::var("FEEDGEN_HOSTNAME").unwrap_or_default();
    let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or_default();

    if service_did.starts_with("did:") {
        let known_service = KnownService {
            id: format!("{}#bsky_fg", service_did),
            r#type: "AtprotoFeedGenerator".into(),
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

pub async fn get_usage_stats(
    State(connection): State<ReadReplicaConn>,
    _token: ApiKey,
) -> Response {
    match apis::get_usage_stats(connection).await {
        Ok(stats) => Json(stats).into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn get_visitors(State(connection): State<ReadReplicaConn>, _token: ApiKey) -> Response {
    match apis::get_visitors(connection).await {
        Ok(visitors) => Json(visitors).into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}

pub async fn get_janitor_config(
    State(connection): State<ReadReplicaConn>,
    _token: ApiKey,
) -> Response {
    match apis::get_janitor_config(connection).await {
        Ok(config) => Json(config).into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

pub async fn update_janitor_config(
    State(connection): State<WriteDbConn>,
    _token: ApiKey,
    Json(body): Json<JanitorConfig>,
) -> Response {
    match apis::update_janitor_config(body, connection).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => {
            tracing::error!("Internal Error: {error}");
            let internal_error = InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(error),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response()
        }
    }
}
