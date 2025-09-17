use serde::{Deserialize, Serialize};

pub mod api_gateway;
pub mod auth_gateway;
pub mod marketdata_publisher;
pub mod order_gateway;
pub mod ws;

/// Generate service health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Standard error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
