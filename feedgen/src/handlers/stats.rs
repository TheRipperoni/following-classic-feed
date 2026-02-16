use crate::models::{InternalErrorCode, InternalErrorMessageResponse};
use crate::{apis, ReadReplicaConn};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Get usage statistics for the feed generation service.
#[tracing::instrument(skip(connection))]
pub async fn get_usage_stats(State(connection): State<ReadReplicaConn>) -> Response {
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

/// Get visitors for the feed generation service.
#[tracing::instrument(skip(connection))]
pub async fn get_visitors(State(connection): State<ReadReplicaConn>) -> Response {
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
