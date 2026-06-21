use crate::models::{InternalErrorCode, InternalErrorMessageResponse};
use crate::state::OAuthStateStore;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use base64::engine::Engine as _;
use base64::engine::general_purpose;
use identity::{determine_pds, IdResolver};
use rand::Rng;
use serde::Deserialize;

/// OAuth client metadata endpoint — required by the AT Protocol OAuth spec.
/// The client_id URL must point here.
pub async fn client_metadata() -> Response {
    let hostname = std::env::var("FEEDGEN_HOSTNAME").unwrap_or_else(|_| "localhost:8000".to_string());
    let protocol = if hostname.contains("localhost") || hostname.contains("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    let redirect_uri = format!("{protocol}://{hostname}/auth/bluesky/callback");

    Json(serde_json::json!({
        "client_id": format!("{protocol}://{hostname}/oauth/client-metadata.json"),
        "client_name": "rsky Feed Preferences",
        "redirect_uris": [redirect_uri],
        "scope": "atproto",
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "application_type": "web",
        "dpop_bound_access_tokens": true
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    handle: String,
}

/// Initiates the Bluesky OAuth login flow.
/// Resolves the handle → DID → PDS, then returns the PDS authorization URL.
#[tracing::instrument(skip(id_resolver, oauth_state))]
pub async fn login_bluesky(
    State(mut id_resolver): State<IdResolver>,
    State(oauth_state): State<OAuthStateStore>,
    Query(params): Query<LoginParams>,
) -> Response {
    let handle = params.handle.trim().to_lowercase();

    // Resolve handle → DID
    let did = match id_resolver.handle.resolve(&handle).await {
        Ok(Some(did)) => did,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some(format!("Could not resolve handle: {handle}")),
                }),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Error resolving handle {handle}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some(format!("Error resolving handle: {e}")),
                }),
            )
                .into_response();
        }
    };

    // Resolve DID → PDS URL
    let pds_url = match determine_pds(&did).await {
        Ok(url) => url.trim_end_matches('/').to_string(),
        Err(e) => {
            tracing::error!("Error determining PDS for DID {did}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some(format!("Error determining PDS: {e}")),
                }),
            )
                .into_response();
        }
    };

    // Generate a random state value for CSRF protection
    let state_value: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Store state temporarily
    {
        let mut store = oauth_state.0.lock().await;
        store.insert(state_value.clone(), (did, pds_url.clone()));
    }

    let hostname = std::env::var("FEEDGEN_HOSTNAME").unwrap_or_else(|_| "localhost:8000".to_string());
    let protocol = if hostname.contains("localhost") || hostname.contains("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    let client_id = format!("{protocol}://{hostname}/oauth/client-metadata.json");
    let redirect_uri = format!("{protocol}://{hostname}/auth/bluesky/callback");

    let authorize_url = format!(
        "{pds_url}/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope=atproto&state={state_value}"
    );

    Json(serde_json::json!({
        "authorize_url": authorize_url,
        "state": state_value,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
    iss: String,
}

/// Handles the OAuth callback from the PDS.
/// Exchanges the authorization code for tokens and issues a session JWT.
#[tracing::instrument(skip(oauth_state))]
pub async fn bluesky_callback(
    State(oauth_state): State<OAuthStateStore>,
    Query(params): Query<CallbackParams>,
) -> Response {
    // Validate and consume the OAuth state
    let (_did, pds_url) = {
        let mut store = oauth_state.0.lock().await;
        match store.remove(&params.state) {
            Some(val) => val,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(InternalErrorMessageResponse {
                        code: Some(InternalErrorCode::InternalError),
                        message: Some("Invalid or expired OAuth state".to_string()),
                    }),
                )
                    .into_response();
            }
        }
    };

    // Verify the iss matches the PDS we initiated with
    let pds_url_normalized = pds_url.trim_end_matches('/');
    let iss_normalized = params.iss.trim_end_matches('/');
    if iss_normalized != pds_url_normalized {
        tracing::warn!(
            "ISS mismatch: expected {pds_url_normalized}, got {iss_normalized}"
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some("Issuer mismatch".to_string()),
            }),
        )
            .into_response();
    }

    let hostname = std::env::var("FEEDGEN_HOSTNAME").unwrap_or_else(|_| "localhost:8000".to_string());
    let protocol = if hostname.contains("localhost") || hostname.contains("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    let client_id = format!("{protocol}://{hostname}/oauth/client-metadata.json");
    let redirect_uri = format!("{protocol}://{hostname}/auth/bluesky/callback");

    // Exchange authorization code for tokens at the PDS
    let token_url = format!("{pds_url}/oauth/token");
    let client = reqwest::Client::new();

    let token_params = [
        ("grant_type", "authorization_code"),
        ("code", &params.code),
        ("redirect_uri", &redirect_uri),
        ("client_id", &client_id),
    ];

    let token_response = match client
        .post(&token_url)
        .form(&token_params)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Token exchange request failed: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some(format!("Token exchange failed: {e}")),
                }),
            )
                .into_response();
        }
    };

    if !token_response.status().is_success() {
        let status_text = token_response.status().to_string();
        let body_text = token_response.text().await.unwrap_or_default();
        tracing::error!("Token exchange failed ({status_text}): {body_text}");
        return (
            StatusCode::BAD_GATEWAY,
            Json(InternalErrorMessageResponse {
                code: Some(InternalErrorCode::InternalError),
                message: Some(format!("Token exchange failed: {status_text}")),
            }),
        )
            .into_response();
    }

    // Parse token response — extract id_token for user identity
    let token_data: serde_json::Value = match token_response.json().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to parse token response: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("Failed to parse token response".to_string()),
                }),
            )
                .into_response();
        }
    };

    // The id_token is a JWT with sub = user DID
    // Also extract access_token for potential future use
    let _access_token = token_data["access_token"].as_str().unwrap_or_default();
    let id_token = match token_data["id_token"].as_str() {
        Some(t) => t.to_string(),
        None => {
            tracing::error!("No id_token in token response");
            return (
                StatusCode::BAD_GATEWAY,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("No id_token in response".to_string()),
                }),
            )
                .into_response();
        }
    };

    // Decode the id_token JWT to extract the issuer's DID (sub claim)
    let user_did = match extract_did_from_id_token(&id_token) {
        Some(did) => did.to_string(),
        None => {
            tracing::error!("Failed to extract DID from id_token");
            return (
                StatusCode::BAD_GATEWAY,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("Failed to extract identity from token".to_string()),
                }),
            )
                .into_response();
        }
    };

    // Issue a session JWT (HS256 signed with JWT_SECRET)
    let session_token = match issue_session_token(&user_did) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to issue session token: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorMessageResponse {
                    code: Some(InternalErrorCode::InternalError),
                    message: Some("Failed to create session".to_string()),
                }),
            )
                .into_response();
        }
    };

    // Redirect back to the frontend with the session token
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let redirect_url = format!("{frontend_url}?token={session_token}");

    Redirect::to(&redirect_url).into_response()
}

/// Returns the current user's DID from the session token.
#[tracing::instrument]
pub async fn me(token: crate::auth::extractors::SessionToken) -> Response {
    Json(serde_json::json!({
        "did": token.did,
    }))
    .into_response()
}

/// Decodes the payload of an id_token JWT and extracts the `sub` field as the DID.
fn extract_did_from_id_token(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let payload_str = std::str::from_utf8(&payload_bytes).ok()?;
    let payload: serde_json::Value = serde_json::from_str(payload_str).ok()?;
    payload["sub"].as_str().map(|s| s.to_string())
}

/// Issues a signed session JWT (HS256) containing the user's DID.
fn issue_session_token(user_did: &str) -> anyhow::Result<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::time::{SystemTime, UNIX_EPOCH};

    type HmacSha256 = Hmac<Sha256>;

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET environment variable not set"))?;

    let header = serde_json::json!({
        "typ": "JWT",
        "alg": "HS256"
    });

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as u128;

    let payload = serde_json::json!({
        "sub": user_did,
        "iat": now,
        "exp": now + 86400, // 24 hours
    });

    let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_string(&header)?);
    let payload_b64 = general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload)?);
    let signing_input = format!("{header_b64}.{payload_b64}");

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(signing_input.as_bytes());
    let sig_bytes = mac.finalize().into_bytes().to_vec();
    let sig_b64 = general_purpose::URL_SAFE_NO_PAD.encode(sig_bytes);

    Ok(format!("{signing_input}.{sig_b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_did_from_id_token() {
        // Build a minimal valid id_token using HS256
        let header = serde_json::json!({"typ": "JWT", "alg": "HS256"});
        let payload = serde_json::json!({
            "sub": "did:plc:testuser123",
            "iss": "https://bsky.social",
            "aud": "http://localhost:8000/oauth/client-metadata.json",
            "exp": 9999999999u64,
            "iat": 1000000000u64,
        });

        use base64::engine::Engine as _;
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&payload).unwrap());
        let id_token = format!("{header_b64}.{payload_b64}.fakesignature");

        let result = extract_did_from_id_token(&id_token);
        assert_eq!(result, Some("did:plc:testuser123".to_string()));
    }

    #[test]
    fn test_issue_session_token_and_extract() {
        let did = "did:plc:testuser";
        std::env::set_var("JWT_SECRET", "test-secret");

        let token = issue_session_token(did).unwrap();
        assert!(token.split('.').count() == 3);

        // Verify we can decode the payload
        let parts: Vec<&str> = token.split('.').collect();
        let payload_bytes =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        let payload_str = std::str::from_utf8(&payload_bytes).unwrap();
        let payload: serde_json::Value = serde_json::from_str(payload_str).unwrap();
        assert_eq!(payload["sub"], did);
        assert!(payload["exp"].as_u64().unwrap() > 0);
    }
}
