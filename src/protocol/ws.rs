use anyhow::{anyhow, bail, Result};
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
    #[serde(
        rename = "rid",
        alias = "request_id",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub request_id: Option<i32>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

impl<T> Response<T> {
    pub fn ok(request_id: i32, response: T) -> Self {
        Self {
            request_id: Some(request_id),
            response: Some(response),
            error: None,
            data: None,
        }
    }

    pub fn error(request_id: Option<i32>, code: i32, message: Option<String>) -> Self {
        Self {
            request_id,
            response: None,
            error: Some(Error { code, message }),
            data: None,
        }
    }

    pub fn error_with_data(
        request_id: Option<i32>,
        code: i32,
        message: Option<String>,
        data: String,
    ) -> Self {
        Self {
            request_id,
            response: None,
            error: Some(Error { code, message }),
            data: Some(data),
        }
    }

    pub fn bad_request<S: Into<String>>(request_id: Option<i32>, message: Option<S>) -> Self {
        Self::error(request_id, 400, message.map(Into::into))
    }

    pub fn internal_server_error<S: Into<String>>(
        request_id: Option<i32>,
        message: Option<S>,
    ) -> Self {
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
