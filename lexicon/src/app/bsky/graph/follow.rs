/// Record declaring a social 'follow' relationship of another account.
use serde::{Deserialize, Serialize};
/// Duplicate follows will be ignored by the AppView.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "$type")]
#[serde(rename = "app.bsky.graph.follow")]
#[serde(rename_all = "camelCase")]
pub struct Follow {
    pub created_at: String,
    pub subject: String,
}
