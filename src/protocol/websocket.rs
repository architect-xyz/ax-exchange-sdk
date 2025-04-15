//! Websocket RPC/subscription protocol. Loosely JSON-RPC.

use serde::{Deserialize, Serialize};

pub trait Rpc {
    const METHOD_NAME: &'static str;
}

#[macro_export]
macro_rules! websocket_rpc {
    ($req:ident, $method_name:literal) => {
        impl crate::protocol::websocket::Rpc for $req {
            const METHOD_NAME: &'static str = $method_name;
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<T: ?Sized> {
    pub id: i32,
    pub method: String,
    // CR alee: making this Option<T> is tricky
    pub params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T, E> {
    pub id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error<E>>,
}

impl<T, E> Response<T, E> {
    pub fn internal_error(id: i32, reason: String) -> Self {
        Self {
            id: Some(id),
            result: None,
            error: Some(Error {
                code: ErrorCode::InternalError as i32,
                message: Some(reason),
                data: None,
            }),
        }
    }

    pub fn invalid_params(id: i32, reason: String) -> Self {
        Self {
            id: Some(id),
            result: None,
            error: Some(Error {
                code: ErrorCode::InvalidParams as i32,
                message: Some(reason),
                data: None,
            }),
        }
    }

    pub fn method_not_found(id: i32) -> Self {
        Self {
            id: Some(id),
            result: None,
            error: Some(Error {
                code: ErrorCode::MethodNotFound as i32,
                message: Some("method not found".to_string()),
                data: None,
            }),
        }
    }

    pub fn parse_error() -> Self {
        Self {
            id: None,
            result: None,
            error: Some(Error {
                code: ErrorCode::ParseError as i32,
                message: Some("parse error".to_string()),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error<E> {
    pub code: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<E>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// Invalid JSON was received by the server.
    /// An error occurred on the server while parsing the JSON text.
    ParseError = -32700,
    /// The JSON sent is not a valid Request object.
    InvalidRequest = -32600,
    /// The method does not exist / is not available.
    MethodNotFound = -32601,
    /// Invalid method parameter(s).
    InvalidParams = -32602,
    /// Internal JSON-RPC error.
    InternalError = -32603,
}
