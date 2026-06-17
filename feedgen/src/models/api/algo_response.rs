use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AlgoResponse {
    #[serde(rename = "cursor")]
    pub cursor: Option<String>,
    #[serde(rename = "feed")]
    pub feed: Vec<crate::models::PostResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algo_response_empty_feed() {
        let resp = AlgoResponse {
            cursor: None,
            feed: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: AlgoResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
    }

    #[test]
    fn test_algo_response_with_cursor() {
        let resp = AlgoResponse {
            cursor: Some("next-page-cursor".to_string()),
            feed: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: AlgoResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
        assert_eq!(deserialized.cursor.unwrap(), "next-page-cursor");
    }
}
