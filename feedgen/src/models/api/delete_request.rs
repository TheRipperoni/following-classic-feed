use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    #[serde(rename = "uri")]
    pub uri: String,
}
