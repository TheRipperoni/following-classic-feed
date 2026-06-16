use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not resolve DID: `{0}`")]
    DidNotFoundError(String),
    #[error("Poorly formatted DID: `{0}`")]
    PoorlyFormattedDidError(String),
    #[error("Unsupported DID method: `{0}`")]
    UnsupportedDidMethodError(String),
    #[error("Poorly formatted DID Document: `{0:#?}`")]
    PoorlyFormattedDidDocumentError(Value),
    #[error("Unsupported did:web paths: `{0}`")]
    UnsupportedDidWebPathError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_not_found_error_display() {
        let err = Error::DidNotFoundError("did:plc:test123".to_string());
        assert_eq!(
            err.to_string(),
            "Could not resolve DID: `did:plc:test123`"
        );
    }

    #[test]
    fn test_poorly_formatted_did_error_display() {
        let err = Error::PoorlyFormattedDidError("invalid".to_string());
        assert_eq!(
            err.to_string(),
            "Poorly formatted DID: `invalid`"
        );
    }

    #[test]
    fn test_unsupported_did_method_error_display() {
        let err = Error::UnsupportedDidMethodError("did:eth:123".to_string());
        assert_eq!(
            err.to_string(),
            "Unsupported DID method: `did:eth:123`"
        );
    }

    #[test]
    fn test_unsupported_did_web_path_error_display() {
        let err = Error::UnsupportedDidWebPathError("did:web:example.com:path".to_string());
        assert_eq!(
            err.to_string(),
            "Unsupported did:web paths: `did:web:example.com:path`"
        );
    }

    #[test]
    fn test_error_is_debug() {
        let err = Error::DidNotFoundError("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("DidNotFoundError"));
    }
}
