use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct KnownService {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_service_serde() {
        let svc = KnownService {
            id: "#atproto_feedgen".to_string(),
            r#type: "AtprotoFeedGenerator".to_string(),
            service_endpoint: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&svc).unwrap();
        let deserialized: KnownService = serde_json::from_str(&json).unwrap();
        assert_eq!(svc, deserialized);
    }

    #[test]
    fn test_known_service_deserialize() {
        let json = "{\"id\":\"#bsky_fg\",\"type\":\"AtprotoFeedGenerator\",\"serviceEndpoint\":\"https://feed.example.com\"}";
        let svc: KnownService = serde_json::from_str(json).unwrap();
        assert_eq!(svc.id, "#bsky_fg");
        assert_eq!(svc.r#type, "AtprotoFeedGenerator");
        assert_eq!(svc.service_endpoint, "https://feed.example.com");
    }
}
