#[derive(Debug, Serialize, Deserialize)]
pub struct WellKnown {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "service")]
    pub service: Vec<crate::models::KnownService>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KnownService;

    #[test]
    fn test_well_known_serde() {
        let wk = WellKnown {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: "did:web:example.com".to_string(),
            service: vec![
                KnownService {
                    id: "#atproto_feedgen".to_string(),
                    r#type: "AtprotoFeedGenerator".to_string(),
                    service_endpoint: "https://example.com".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&wk).unwrap();
        let deserialized: WellKnown = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "did:web:example.com");
        assert_eq!(deserialized.context.len(), 1);
        assert_eq!(deserialized.service.len(), 1);
        assert_eq!(deserialized.service[0].r#type, "AtprotoFeedGenerator");
    }
}
