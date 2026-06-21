use crate::auth::extractors::AccessToken;
use crate::handlers::{FOLLOWING_CLASSIC, FOLLOWING_TRAD, MEDIA, MUTUALS};
use crate::models::{
    AlgoResponse, FollowingPreference, InternalErrorCode, InternalErrorMessageResponse, JwtParts,
    PostResult,
};
use crate::{apis, db, ReadReplicaConn, WriteDbConn};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct FeedSkeletonParams {
    pub feed: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

#[tracing::instrument(skip(connection))]
pub async fn index(
    State(connection): State<ReadReplicaConn>,
    Query(params): Query<FeedSkeletonParams>,
    jwt: AccessToken,
) -> Response {
    let did: String;
    let feed = params.feed.unwrap_or_default();
    match serde_json::from_str::<JwtParts>(&jwt.0) {
        Ok(jwt_obj) => {
            did = jwt_obj.iss;
            tracing::info!("Visit from {did}");
            tracing::info!("{}", jwt.0);
            match apis::add_visitor(
                did.clone(),
                jwt_obj.aud,
                feed.to_string(),
                connection.clone(),
            )
            .await
            {
                Ok(_) => tracing::info!("Visitor added"),
                Err(error) => tracing::error!("Failed to write visitor: {error}"),
            }
        }
        Err(e) => {
            tracing::error!(%e, "Failed to parse jwt string");
            return { StatusCode::BAD_REQUEST }.into_response();
        }
    }

    match feed.as_str() {
        _following_classic if FOLLOWING_CLASSIC == _following_classic => {
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
        _mutuals if MUTUALS == _mutuals => {
            if did.is_empty() {
                let internal_error = InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("No DID".to_string()),
                };
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(internal_error)).into_response();
            }
            match apis::get_posts_by_mutuals(
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FollowingPrefFetchResponse {
    pub did: String,
    pub preferences: Vec<FollowingPreference>,
}

pub async fn following_preferences_fetch(
    State(connection): State<WriteDbConn>,
    Query(params): Query<serde_json::Value>,
) -> Response {
    let did = match params.get("did").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("Missing required parameter: did".to_string()),
                }),
            )
                .into_response();
        }
    };
    let preferences = db::following_pref_fetch(did.clone(), connection).await;
    let response = FollowingPrefFetchResponse { did, preferences };
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
