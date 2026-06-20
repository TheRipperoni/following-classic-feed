use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum NotFoundErrorCode {
    #[serde(rename = "not_found_error")]
    NotFoundError,
    #[serde(rename = "undefined_endpoint")]
    UndefinedEndpoint,
    #[serde(rename = "unimplemented")]
    Unimplemented,
}

impl Display for NotFoundErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::NotFoundError => String::from("not_found_error"),
            Self::UndefinedEndpoint => String::from("undefined_endpoint"),
            Self::Unimplemented => String::from("unimplemented"),
        };
        write!(f, "{}", x)
    }
}
