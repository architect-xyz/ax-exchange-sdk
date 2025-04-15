//! Websocket RPC/subscription protocol. Loosely JSON-RPC.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<T> {
    pub id: i32,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T, E> {
    pub id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error<E>>,
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
