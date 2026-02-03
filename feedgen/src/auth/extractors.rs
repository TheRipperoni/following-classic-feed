use crate::models::{InternalErrorCode, InternalErrorMessageResponse};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    Json,
};
use identity::IdResolver;
use std::env;

#[derive(Debug)]
pub struct ApiKey(pub String);

#[derive(Debug)]
pub struct AccessToken(pub String);

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<InternalErrorMessageResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = env::var("API_KEY").map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::Unavailable),
                    message: Some("API Key not configured".to_string()),
                }),
            )
        })?;

        match parts.headers.get("X-KEY") {
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::Unavailable),
                    message: Some("Missing API Key".to_string()),
                }),
            )),
            Some(key) if key == token.as_str() => Ok(ApiKey(token)),
            Some(_) => Err((
                StatusCode::UNAUTHORIZED,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::Unavailable),
                    message: Some("Invalid API Key".to_string()),
                }),
            )),
        }
    }
}

impl<S> FromRequestParts<S> for AccessToken
where
    S: Send + Sync,
    IdResolver: FromRef<S>,
{
    type Rejection = (StatusCode, Json<InternalErrorMessageResponse>);

    #[tracing::instrument(skip(parts, state))]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let resolver = IdResolver::from_ref(state);

        match parts.headers.get(header::AUTHORIZATION) {
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::Unavailable),
                    message: Some("Missing Authorization header".to_string()),
                }),
            )),
            Some(token_header) => {
                let token = token_header.to_str().map_err(|_| {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(InternalErrorMessageResponse {
                            code: Some(InternalErrorCode::Unavailable),
                            message: Some("Invalid Authorization header".to_string()),
                        }),
                    )
                })?;
                if !token.starts_with("Bearer ") {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(InternalErrorMessageResponse {
                            code: Some(InternalErrorCode::Unavailable),
                            message: Some("Invalid token format".to_string()),
                        }),
                    ));
                }
                let jwtstr = &token[7..];
                let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or_default();
                let mut resolver_inner = resolver.clone();
                match crate::auth::verify_jwt(jwtstr, &service_did, &mut resolver_inner).await {
                    Ok(payload) => Ok(AccessToken(payload)),
                    Err(e) => Err((
                        StatusCode::UNAUTHORIZED,
                        Json(InternalErrorMessageResponse {
                            code: Some(InternalErrorCode::Unavailable),
                            message: Some(format!("Invalid token: {}", e)),
                        }),
                    )),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct OptionalAccessToken(pub Option<AccessToken>);

impl<S> FromRequestParts<S> for OptionalAccessToken
where
    S: Send + Sync,
    IdResolver: FromRef<S>,
{
    type Rejection = (StatusCode, Json<InternalErrorMessageResponse>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AccessToken::from_request_parts(parts, state).await {
            Ok(token) => Ok(OptionalAccessToken(Some(token))),
            Err((status, _)) if status == StatusCode::UNAUTHORIZED => Ok(OptionalAccessToken(None)),
            Err(e) => Err(e),
        }
    }
}
