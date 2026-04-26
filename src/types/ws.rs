//! Common WebSocket types shared across all WS clients.

use futures::future::BoxFuture;
use std::sync::Arc;
use thiserror::Error;
use yawc::Frame;

/// A type-erased async function that returns a fresh auth token.
pub type TokenRefreshFn =
    Arc<dyn Fn() -> BoxFuture<'static, anyhow::Result<arcstr::ArcStr>> + Send + Sync>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Exited,
}

pub enum InternalCommand {
    Send(Frame),
    Close,
}

#[derive(Error, Debug)]
pub enum WsClientError {
    #[error("WebSocket error: {0}")]
    WebsocketError(#[from] yawc::WebSocketError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Transport error: {0}")]
    Transport(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Invalid URL scheme")]
    InvalidScheme,
    #[error("Server error {code}: {message}")]
    ServerError { code: i32, message: String },
}
