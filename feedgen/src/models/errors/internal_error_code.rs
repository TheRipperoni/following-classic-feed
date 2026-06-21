use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum InternalErrorCode {
    #[serde(rename = "no_internal_error")]
    NoInternalError,
    #[serde(rename = "internal_error")]
    InternalError,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "deadline_exceeded")]
    DeadlineExceeded,
    #[serde(rename = "already_exists")]
    AlreadyExists,
    #[serde(rename = "resource_exhausted")]
    ResourceExhausted,
    #[serde(rename = "failed_precondition")]
    FailedPrecondition,
    #[serde(rename = "aborted")]
    Aborted,
    #[serde(rename = "out_of_range")]
    OutOfRange,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "data_loss")]
    DataLoss,
}

impl Display for InternalErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::NoInternalError => String::from("no_internal_error"),
            Self::InternalError => String::from("internal_error"),
            Self::Cancelled => String::from("cancelled"),
            Self::DeadlineExceeded => String::from("deadline_exceeded"),
            Self::AlreadyExists => String::from("already_exists"),
            Self::ResourceExhausted => String::from("resource_exhausted"),
            Self::FailedPrecondition => String::from("failed_precondition"),
            Self::Aborted => String::from("aborted"),
            Self::OutOfRange => String::from("out_of_range"),
            Self::Unavailable => String::from("unavailable"),
            Self::DataLoss => String::from("data_loss"),
        };
        write!(f, "{}", x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_error_code_display() {
        assert_eq!(
            InternalErrorCode::NoInternalError.to_string(),
            "no_internal_error"
        );
        assert_eq!(
            InternalErrorCode::InternalError.to_string(),
            "internal_error"
        );
        assert_eq!(InternalErrorCode::DataLoss.to_string(), "data_loss");
    }

    #[test]
    fn test_internal_error_code_serde_roundtrip() {
        for code in &[
            InternalErrorCode::NoInternalError,
            InternalErrorCode::InternalError,
            InternalErrorCode::Cancelled,
            InternalErrorCode::AlreadyExists,
            InternalErrorCode::Unavailable,
        ] {
            let json = serde_json::to_string(code).unwrap();
            let deserialized: InternalErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(*code, deserialized);
        }
    }

    #[test]
    fn test_internal_error_code_deserialize() {
        let result: InternalErrorCode = serde_json::from_str(r#""cancelled""#).unwrap();
        assert_eq!(result, InternalErrorCode::Cancelled);
    }
}
