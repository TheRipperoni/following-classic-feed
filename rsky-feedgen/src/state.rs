use crate::{ReadReplicaConn, WriteDbConn};
use axum::extract::FromRef;
use rsky_identity::IdResolver;

#[derive(Clone)]
pub struct AppState {
    pub read_db: ReadReplicaConn,
    pub write_db: WriteDbConn,
    pub id_resolver: IdResolver,
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
