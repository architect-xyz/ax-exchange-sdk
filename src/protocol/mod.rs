use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod api_gateway;
pub mod candle_server;
pub mod common;
pub mod marketdata_publisher;
pub mod order_gateway;
pub mod pagination;
pub mod settlement_engine;
pub mod sort;
pub mod time_range;
pub mod ws;

/// Standard error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error: String,
}

/// Service health response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
