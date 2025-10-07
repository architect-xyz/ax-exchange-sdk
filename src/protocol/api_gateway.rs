use crate::types::{Instrument, Token};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateUserResponse {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DeleteUserRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DeleteUserResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ChangePasswordRequest {
    pub username: String,
    pub password: String,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ChangePasswordResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateApiKeyRequest {
    pub username: String,
    pub password: String,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetApiKeysRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetApiKeysResponse {
    pub api_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RevokeApiKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RevokeApiKeyResponse {
    pub message: String,
}

/// Exchange credentials for a bearer token.
///
/// Must provide exactly one of:
///
/// - `username` + `password`
/// - `api_key` + `secret`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserTokenRequest {
    #[serde(flatten)]
    pub auth: GetUserTokenAuthMethod,
    pub expiration_seconds: i32,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(untagged)]
pub enum GetUserTokenAuthMethod {
    UsernamePassword { username: String, password: String },
    ApiKeySecret { api_key: String, secret: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserTokenResponse {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RevokeUserTokenRequest {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RevokeUserTokenResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WhoAmIResponse {
    pub username: String,
    pub enabled_2fa: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DecodeTokenRequest {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DecodeTokenResponse {
    pub user_id: Uuid,
    pub username: String,
    pub is_admin_token: bool,
    pub can_place_orders: bool,
    pub enabled_2fa: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUsersResponse {
    pub users: Vec<GetUserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserResponse {
    pub id: Uuid,
    pub username: String,
    /// NB: will be deprecated soon; use is_onboarded, is_close_only, is_frozen instead
    pub is_valid: bool,
    pub is_onboarded: bool,
    pub is_close_only: bool,
    pub is_frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UpdateUserStatusRequest {
    pub is_onboarded: Option<bool>,
    pub is_close_only: Option<bool>,
    pub is_frozen: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserRiskProfilesResponse {
    pub user_risk_profiles: Vec<GetUserRiskProfileResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserRiskProfileResponse {
    pub user_id: Uuid,
    pub risk_score: String,
    pub compliance_risk_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(transparent)]
pub struct GetInstrumentResponse(pub Instrument);

impl GetInstrumentResponse {
    pub fn into_inner(self) -> Instrument {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetInstrumentsResponse {
    pub instruments: Vec<GetInstrumentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTransactionsRequest {
    pub transaction_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Transaction {
    pub user_id: Uuid,
    pub event_id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
    pub transaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetTransactionsResponse {
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Setup2faResponse {
    pub validate_token: String,
    /// The `uri` field contains a provisioning URI following the
    /// Google Authenticator format:
    ///
    /// `otpauth://totp/ADX:username?secret=BASE32SECRET&issuer=ADX&algorithm=SHA1&digits=6&period=30`
    ///
    /// This URI encodes all TOTP parameters and is typically displayed
    /// as a QR code for client apps to scan.
    pub uri: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Confirm2faRequest {
    pub validate_token: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Confirm2faResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Disable2faResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SandboxDepositRequest {
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SandboxWithdrawalRequest {
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetPositionsResponse {
    pub positions: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Position {
    pub user_id: Uuid,
    pub symbol: String,
    pub open_quantity: i64,
    pub open_notional: Decimal,
    pub timestamp: DateTime<Utc>,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFillsResponse {
    pub fills: Vec<Fill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Fill {
    pub execution_id: String,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBalancesResponse {
    pub balances: Vec<Balance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Balance {
    pub symbol: String,
    pub amount: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_token_request_serde() {
        let json = r#"
        {
            "username": "testuser",
            "password": "password",
            "expiration_seconds": 3600
        }
        "#;
        let req: GetUserTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            GetUserTokenRequest {
                auth: GetUserTokenAuthMethod::UsernamePassword {
                    username: "testuser".to_string(),
                    password: "password".to_string()
                },
                expiration_seconds: 3600,
                totp: None
            }
        );

        let json = r#"
        {
            "api_key": "testapikey",
            "secret": "testsecret",
            "expiration_seconds": 3600
        }
        "#;
        let req: GetUserTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            GetUserTokenRequest {
                auth: GetUserTokenAuthMethod::ApiKeySecret {
                    api_key: "testapikey".to_string(),
                    secret: "testsecret".to_string()
                },
                expiration_seconds: 3600,
                totp: None
            }
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetCandlesRequest {
    pub symbol: String,
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
    pub candle_width: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetCandlesResponse {
    pub candles: Vec<Candle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Candle {
    pub symbol: String,
    pub low: Decimal,
    pub high: Decimal,
    pub open: Decimal,
    pub close: Decimal,
    pub buy_volume: i64,
    pub sell_volume: i64,
    pub volume: i64,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetLastCandleRequest {
    pub symbol: String,
    pub candle_width: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetLastCandleResponse {
    pub candle: Candle,
}
