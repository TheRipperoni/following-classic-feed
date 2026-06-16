#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct InternalErrorMessageResponse {
    #[serde(rename = "code", skip_serializing_if = "Option::is_none")]
    pub code: Option<crate::models::InternalErrorCode>,
    #[serde(rename = "message", skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl InternalErrorMessageResponse {
    pub fn new() -> InternalErrorMessageResponse {
        InternalErrorMessageResponse {
            code: None,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InternalErrorCode;

    #[test]
    fn test_internal_error_message_response_default() {
        let resp = InternalErrorMessageResponse::new();
        assert_eq!(resp.code, None);
        assert_eq!(resp.message, None);
    }

    #[test]
    fn test_internal_error_message_response_serde() {
        let resp = InternalErrorMessageResponse {
            code: Some(InternalErrorCode::Unavailable),
            message: Some("service unavailable".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: InternalErrorMessageResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
    }

    #[test]
    fn test_internal_error_message_response_serde_empty() {
        let resp = InternalErrorMessageResponse::new();
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: InternalErrorMessageResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
    }
}
