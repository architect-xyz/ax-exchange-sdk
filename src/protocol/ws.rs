use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request<T> {
    #[serde(rename = "rid")]
    pub request_id: i32,
    #[serde(flatten)]
    pub request: T,
}

#[derive(Serialize, Deserialize)]
pub struct Response<T> {
    #[serde(rename = "rid", alias = "request_id")]
    pub request_id: i32,
    #[serde(
        rename = "res",
        alias = "result",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub response: Option<T>,
    #[serde(
        rename = "err",
        alias = "error",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<Error>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    #[serde(rename = "msg", default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code: {}, message: {}",
            self.code,
            self.message.as_ref().unwrap_or(&"unknown error".to_string())
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Timestamp {
    pub ts: i32,
    pub tn: u32,
}

impl Timestamp {
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.ts as i64, self.tn)
    }
}
