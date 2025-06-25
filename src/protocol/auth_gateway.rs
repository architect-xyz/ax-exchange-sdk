use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserTokenRequest {
    pub username: String,
    pub password: String,
    pub expiration_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserTokenResponse {
    pub token: String,
}
