#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct KnownService {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}
