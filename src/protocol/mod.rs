use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod api_gateway;
pub mod candle_server;
pub mod common;
pub mod marketdata_publisher;
pub mod order_gateway;
pub mod order_gateway_v2;
pub mod risk_manager;
pub mod settlement_engine;
pub mod ws;

/// Standard error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Service health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
}
