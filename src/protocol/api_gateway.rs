use crate::{
    protocol::{
        common::{Fill, Timestamp},
        marketdata_publisher::{Ticker, Trade},
        pagination::{TimeseriesPage, TimeseriesPagination},
    },
    types::{ApiKeyType, BboCandle, Candle, Instrument, Token},
    Side,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{formats::CommaSeparator, serde_as, StringWithSeparator};
use std::collections::HashMap;

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
pub struct ResetPasswordRequest {
    pub username: String,
    pub new_password: String,
    pub password_reset_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ResetPasswordResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateApiKeyRequest {
    pub username: String,
    pub password: String,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
    #[serde(default)]
    pub key_type: Option<ApiKeyType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ApiKeyInfo {
    pub api_key: String,
    pub key_type: ApiKeyType,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetApiKeysResponse {
    pub api_keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
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
pub struct AuthenticateRequest {
    #[serde(flatten)]
    pub auth: AuthenticationMethod,
    pub expiration_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(untagged)]
pub enum AuthenticationMethod {
    UsernamePassword {
        username: String,
        password: String,
        /// Optional 2FA code, if 2FA is enabled/required for the user.
        totp: Option<String>,
    },
    ApiKeySecret {
        api_key: String,
        api_secret: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AuthenticateResponse {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LoginRequest {
    #[serde(flatten)]
    pub auth: AuthenticationMethod,
    pub expiration_seconds: i32,
    /// Redirect URL to redirect to after successful login.
    pub redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WhoAmIResponse {
    pub id: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub enabled_2fa: bool,
    pub is_onboarded: bool,
    pub is_close_only: bool,
    pub is_frozen: bool,
    pub is_admin: bool,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub require_2fa: bool,
    pub fiat_deposit_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetCustomerResponse {
    pub business_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetInstrumentRequest {
    pub symbol: String,
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
pub struct GetTickerRequest {
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetTickerResponse {
    pub ticker: Ticker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetTickersResponse {
    pub tickers: Vec<Ticker>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTransactionsRequest {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    pub transaction_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Transaction {
    pub user_id: String,
    pub event_id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
    pub transaction_type: String,
    pub reference_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetTransactionsResponse {
    pub transactions: Vec<Transaction>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingTransaction {
    pub user_id: String,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_type: String,
    pub amount: Decimal,
    pub event_id: String,
    pub sequence_number: i32,
    pub reference_id: Option<String>,
    pub symbol: String,
    pub funding_rate: Decimal,
    pub funding_amount: Decimal,
    pub benchmark_price: Decimal,
    pub settlement_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFundingTransactionsResponse {
    pub funding_transactions: Vec<FundingTransaction>,
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
    pub user_id: String,
    pub symbol: String,
    pub signed_quantity: i64,
    pub signed_notional: Decimal,
    pub timestamp: DateTime<Utc>,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFillsRequest {
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFillsResponse {
    pub fills: Vec<Fill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AdminTrade {
    pub trade_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: u64,
    pub maker_user_id: String,
    pub taker_user_id: String,
    pub taker_side: Side,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetAdminTradesResponse {
    pub trades: Vec<AdminTrade>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SymbolRiskSnapshot {
    pub signed_quantity: i64,
    pub signed_notional: Decimal,
    pub average_price: Option<Decimal>,
    pub initial_margin_required_position: Decimal,
    pub initial_margin_required_open_orders: Decimal,
    pub initial_margin_required_total: Decimal,
    pub maintenance_margin_required: Decimal,
    pub unrealized_pnl: Decimal,
    pub liquidation_price: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UserRiskSnapshot {
    pub user_id: String,
    pub timestamp_ns: DateTime<Utc>,
    pub per_symbol: HashMap<String, SymbolRiskSnapshot>,
    pub initial_margin_required_for_positions: Decimal,
    pub initial_margin_required_for_open_orders: Decimal,
    pub initial_margin_required_total: Decimal,
    pub maintenance_margin_required: Decimal,
    pub unrealized_pnl: Decimal,
    pub equity: Decimal,
    pub initial_margin_available: Decimal,
    pub maintenance_margin_available: Decimal,
    pub balance_usd: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetRiskSnapshotResponse {
    pub risk_snapshot: UserRiskSnapshot,
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
        let req: AuthenticateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            AuthenticateRequest {
                auth: AuthenticationMethod::UsernamePassword {
                    username: "testuser".to_string(),
                    password: "password".to_string(),
                    totp: None,
                },
                expiration_seconds: 3600,
            }
        );

        let json = r#"
        {
            "api_key": "testapikey",
            "api_secret": "testsecret",
            "expiration_seconds": 3600
        }
        "#;
        let req: AuthenticateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            AuthenticateRequest {
                auth: AuthenticationMethod::ApiKeySecret {
                    api_key: "testapikey".to_string(),
                    api_secret: "testsecret".to_string(),
                },
                expiration_seconds: 3600,
            }
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTradesRequest {
    pub symbol: String,
    /// The maximum number of trades to return, up to 100 trades. Defaults to 10.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetTradesResponse {
    pub trades: Vec<Trade>,
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetCandleRequest {
    pub symbol: String,
    pub candle_width: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetCandleResponse {
    pub candle: Candle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetBboCandlesRequest {
    pub symbol: String,
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
    pub candle_width: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBboCandlesResponse {
    pub candles: Vec<BboCandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetBboCandleRequest {
    pub symbol: String,
    pub candle_width: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBboCandleResponse {
    pub candle: BboCandle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFundingRatesRequest {
    pub symbol: String,
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFundingRatesResponse {
    pub funding_rates: Vec<FundingRate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingRate {
    pub symbol: String,
    pub timestamp_ns: u64,
    pub funding_rate: Decimal,
    pub funding_amount: Decimal,
    pub benchmark_price: Decimal,
    pub settlement_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetAccountEquityHistoryRequest {
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
    /// Desired duration between returned points.
    pub resolution_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AccountEquityPoint {
    #[serde(rename = "t")]
    pub timestamp_ns: u64,
    #[serde(rename = "v")]
    pub equity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetAccountEquityHistoryResponse {
    pub data_points: Vec<AccountEquityPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SignupResponse {
    pub user_id: String,
}

/// Default orderbook depth level when not specified (Level 2: aggregated quantities)
pub const DEFAULT_BOOK_LEVEL: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetBookRequest {
    pub symbol: String,
    /// Orderbook depth level (2 or 3). Defaults to 2 if not specified.
    /// - 2: Returns aggregated quantity per price level
    /// - 3: Returns individual order quantities per price level
    pub level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBookResponse {
    pub book: GetBookResponseBook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBookResponseBook {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bids: Vec<GetBookResponseBookLevel>,
    #[serde(rename = "a")]
    pub offers: Vec<GetBookResponseBookLevel>,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetBookResponseBookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub total_quantity: u64,
    #[serde(rename = "o")]
    pub orders: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PreviewAggressiveLimitOrderRequest {
    pub symbol: String,
    pub quantity: u64,
    pub side: Side,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PreviewAggressiveLimitOrderResponse {
    pub limit_price: Option<Decimal>,
    pub vwap: Option<Decimal>,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,
}
