use crate::{ReadReplicaConn, WriteDbConn};
use axum::extract::FromRef;
use identity::IdResolver;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stores temporary OAuth state values (state → (handle, pds_url))
#[derive(Clone, Default)]
pub struct OAuthStateStore(pub Arc<Mutex<HashMap<String, (String, String)>>>);

#[derive(Clone)]
pub struct AppState {
    pub read_db: ReadReplicaConn,
    pub write_db: WriteDbConn,
    pub id_resolver: IdResolver,
    pub oauth_state: OAuthStateStore,
}

impl FromRef<AppState> for ReadReplicaConn {
    fn from_ref(state: &AppState) -> Self {
        state.read_db.clone()
    }
}

impl FromRef<AppState> for WriteDbConn {
    fn from_ref(state: &AppState) -> Self {
        state.write_db.clone()
    }
}

impl FromRef<AppState> for IdResolver {
    fn from_ref(state: &AppState) -> Self {
        state.id_resolver.clone()
    }
}

impl FromRef<AppState> for OAuthStateStore {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_state.clone()
    }
}
