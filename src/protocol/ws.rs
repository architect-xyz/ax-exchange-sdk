use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(derive_more::Deref, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Request<T> {
    #[serde(rename = "rid", alias = "request_id")]
    pub request_id: i32,
    #[serde(flatten)]
    #[deref]
    pub request: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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

impl<T> Response<T> {
    pub fn ok(request_id: i32, response: T) -> Self {
        Self {
            request_id,
            response: Some(response),
            error: None,
        }
    }

    pub fn error(request_id: i32, code: i32, message: Option<String>) -> Self {
        Self {
            request_id,
            response: None,
            error: Some(Error { code, message }),
        }
    }

    pub fn bad_request<S: Into<String>>(request_id: i32, message: Option<S>) -> Self {
        Self::error(request_id, 400, message.map(Into::into))
    }

    pub fn internal_server_error<S: Into<String>>(request_id: i32, message: Option<S>) -> Self {
        Self::error(request_id, 500, message.map(Into::into))
    }

    pub fn into_inner(self) -> Result<T> {
        if let Some(e) = self.error {
            Err(anyhow!(e))
        } else if let Some(inner) = self.response {
            Ok(inner)
        } else {
            bail!("malformed response");
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
            self.message
                .as_ref()
                .unwrap_or(&"unknown error".to_string())
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Timestamp {
    pub ts: i32,
    pub tn: u32,
}

impl Timestamp {
    pub fn now() -> Self {
        let now = Utc::now();
        now.into()
    }

    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.ts as i64, self.tn)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self {
            ts: value.timestamp() as i32,
            tn: value.timestamp_subsec_nanos() as u32,
        }
    }
}
