use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct JwtParts {
    pub iss: String,
    pub aud: String,
    pub exp: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_parts_serialization() {
        let jwt = JwtParts {
            iss: "did:plc:123".to_string(),
            aud: "did:web:example.com".to_string(),
            exp: 1234567890,
        };
        let serialized = serde_json::to_string(&jwt).unwrap();
        let deserialized: JwtParts = serde_json::from_str(&serialized).unwrap();
        assert_eq!(jwt, deserialized);
    }

    #[test]
    fn test_jwt_parts_deserialization() {
        let json = r#"{"iss":"did:plc:123","aud":"did:web:example.com","exp":1234567890}"#;
        let deserialized: JwtParts = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.iss, "did:plc:123");
        assert_eq!(deserialized.aud, "did:web:example.com");
        assert_eq!(deserialized.exp, 1234567890);
    }
}
