use crate::auth::extractors::ApiKey;
use crate::models::{
    InternalErrorCode, InternalErrorMessageResponse, JanitorConfig, SubState, UserFeedPreference,
};
use crate::{apis, db, ReadReplicaConn, WriteDbConn};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[tracing::instrument(skip(connection))]
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

#[tracing::instrument(skip(connection))]
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

#[tracing::instrument(skip(connection))]
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

#[tracing::instrument(skip(connection))]
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

pub async fn user_config(
    State(connection): State<WriteDbConn>,
    Query(params): Query<serde_json::Value>,
    _token: ApiKey,
) -> Response {
    let did = params["did"].as_str().unwrap_or_default();
    let response = db::user_config_fetch(did.to_string(), connection).await;
    Json(response).into_response()
}
