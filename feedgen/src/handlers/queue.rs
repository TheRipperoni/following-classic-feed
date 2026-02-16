use crate::auth::extractors::ApiKey;
use crate::models::{
    CreateRequest, DeleteRequest, InternalErrorCode, InternalErrorMessageResponse,
};
use crate::{apis, WriteDbConn};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Queues post creation requests for processing.
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

/// Queues post deletion requests for processing.
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
