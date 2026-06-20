use anyhow::Result;
use urlencoding::{decode, encode};

pub const SECOND: i32 = 1000;
pub const MINUTE: i32 = SECOND * 60;
pub const HOUR: i32 = MINUTE * 60;
pub const DAY: i32 = HOUR * 24;

pub fn encode_uri_component(input: &str) -> String {
    encode(input).to_string()
}
pub fn decode_uri_component(input: &str) -> Result<String> {
    Ok(decode(input)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_uri_component() {
        let result = encode_uri_component("hello world");
        assert_eq!(result, "hello%20world");
    }

    #[test]
    fn test_encode_uri_component_special_chars() {
        let result = encode_uri_component("a=b&c=d");
        assert_eq!(result, "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_encode_uri_component_already_encoded() {
        let result = encode_uri_component("hello%20world");
        assert_eq!(result, "hello%2520world");
    }

    #[test]
    fn test_decode_uri_component() {
        let result = decode_uri_component("hello%20world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_decode_uri_component_multiple() {
        let result = decode_uri_component("a%3Db%26c%3Dd").unwrap();
        assert_eq!(result, "a=b&c=d");
    }

    #[test]
    fn test_decode_uri_component_no_encoding() {
        let result = decode_uri_component("plaintext").unwrap();
        assert_eq!(result, "plaintext");
    }

    #[test]
    fn test_decode_uri_component_empty() {
        let result = decode_uri_component("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = "some text with spaces & special chars = test?";
        let encoded = encode_uri_component(original);
        let decoded = decode_uri_component(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
