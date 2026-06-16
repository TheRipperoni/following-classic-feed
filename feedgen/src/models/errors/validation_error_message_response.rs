use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ValidationErrorMessageResponse {
    #[serde(rename = "code", skip_serializing_if = "Option::is_none")]
    pub code: Option<crate::models::ErrorCode>,
    #[serde(rename = "message", skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ValidationErrorMessageResponse {
    pub fn new() -> ValidationErrorMessageResponse {
        ValidationErrorMessageResponse {
            code: None,
            message: None,
        }
    }
}

impl fmt::Display for ValidationErrorMessageResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = "".to_owned();
        if let Some(error_message) = &self.message {
            message = error_message.clone();
        }
        write!(f, "validation_error: {}", message)
    }
}

impl Error for ValidationErrorMessageResponse {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ErrorCode;

    #[test]
    fn test_validation_error_message_response_default() {
        let resp = ValidationErrorMessageResponse::new();
        assert_eq!(resp.code, None);
        assert_eq!(resp.message, None);
    }

    #[test]
    fn test_validation_error_message_response_display() {
        let resp = ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("invalid input".to_string()),
        };
        assert_eq!(resp.to_string(), "validation_error: invalid input");
    }

    #[test]
    fn test_validation_error_message_response_display_empty_message() {
        let resp = ValidationErrorMessageResponse::new();
        assert_eq!(resp.to_string(), "validation_error: ");
    }

    #[test]
    fn test_validation_error_message_response_serde() {
        let resp = ValidationErrorMessageResponse {
            code: Some(ErrorCode::InvalidUser),
            message: Some("user not found".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: ValidationErrorMessageResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
    }
}
