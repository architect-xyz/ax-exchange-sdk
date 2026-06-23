use crate::{
    Side,
    protocol::{
        common::{Fill, Timestamp},
        marketdata_publisher::{Ticker, Trade},
        pagination::{TimeseriesPage, TimeseriesPagination},
    },
    types::{ApiKeyType, BboCandle, Candle, Instrument, Token},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};
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
    #[serde(default)]
    pub allowed_ips: Option<Vec<String>>,
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
    pub allowed_ips: Option<Vec<String>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UpdateApiKeyAllowedIpsRequest {
    pub api_key: String,
    pub password: String,
    pub totp: Option<String>,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UpdateApiKeyAllowedIpsResponse {
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
    pub pseudonym: String,
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
    #[serde(default)]
    pub accounts: Vec<WhoAmIAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WhoAmIAccount {
    pub id: String,
    pub name: String,
    pub is_close_only: bool,
    pub is_frozen: bool,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub can_list: bool,
    pub can_read: bool,
    pub can_set_limits: bool,
    pub can_reduce_or_close: bool,
    pub can_trade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetCustomerResponse {
    pub business_name: Option<String>,
    pub doing_business_as: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum LeaderboardMetric {
    Volume,
}

impl LeaderboardMetric {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Volume => "volume",
        }
    }
}

impl std::fmt::Display for LeaderboardMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum LeaderboardCadence {
    Monthly,
}

impl LeaderboardCadence {
    pub fn all() -> &'static [Self] {
        &[Self::Monthly]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
        }
    }

    pub fn period_start(&self, dt: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Monthly => NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc()),
        }
    }

    pub fn next_period_start(&self, dt: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Monthly => {
                let d = if dt.month() == 12 {
                    NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1)
                };
                d.and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
            }
        }
    }

    pub fn crossed_boundary(&self, prev: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        match self {
            Self::Monthly => prev.month() != now.month() || prev.year() != now.year(),
        }
    }
}

impl std::fmt::Display for LeaderboardCadence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct LeaderboardRequest {
    pub metric: LeaderboardMetric,
    pub cadence: LeaderboardCadence,
    pub period_offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LeaderboardResponse {
    pub metric: LeaderboardMetric,
    pub cadence: LeaderboardCadence,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub entries: Vec<LeaderboardEntry>,
    pub your_entry: Option<LeaderboardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub pseudonym: String,
    pub score: Decimal,
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

/// Query parameters for `GET /transactions` (session-authenticated user), including optional
/// time range, sort direction, cursor, and page size.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTransactionsQueryParams {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
    #[serde(flatten)]
    pub request: GetTransactionsRequest,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Transaction {
    pub user_id: Option<String>,
    pub account_id: Option<String>,
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

/// A cash leg booked against a position-bearing account by a settlement
/// event. Covers perpetual funding-rate payments, daily mark-to-market on
/// any position-bearing contract, and the one-time final settlement at a
/// dated contract's expiration — discriminated by `transaction_type`.
///
/// `funding_rate`, `funding_amount`, and `benchmark_price` are populated
/// only for `Funding` and absent for the other kinds. `settlement_price`
/// and `amount` apply to every kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingTransaction {
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_type: SettlementKind,
    pub amount: Decimal,
    pub event_id: String,
    pub sequence_number: i32,
    pub reference_id: Option<String>,
    pub symbol: String,
    #[serde(default)]
    pub funding_rate: Option<Decimal>,
    /// Per-contract funding cash for this event — same for every user on
    /// the same `(symbol, timestamp)`. Multiply by signed position to
    /// reconstruct the per-user `amount`. (`amount` is the actual cash
    /// booked to *this* account; `funding_amount` is the per-contract
    /// quantity that drove the calculation.)
    #[serde(default)]
    pub funding_amount: Option<Decimal>,
    #[serde(default)]
    pub benchmark_price: Option<Decimal>,
    pub settlement_price: Decimal,
}

/// Discriminator for `FundingTransaction`. All variants are a cash leg
/// booked at a settlement price against a position; only the trigger
/// differs.
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, strum::Display, strum::EnumString, Serialize, Deserialize,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SettlementKind {
    /// Perpetual funding-rate payment. `funding_rate`, `funding_amount`,
    /// and `benchmark_price` are populated.
    Funding,
    /// Routine daily mark-to-market against the daily settlement price.
    /// Applies to perpetual and dated contracts alike.
    MarkToMarket,
    /// One-time final settlement at the expiration of a dated contract.
    FinalSettlement,
}

/// Query parameters for `GET /funding-transactions` (session-authenticated user),
/// including optional time range, sort direction, cursor, and page size.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFundingTransactionsQueryParams {
    pub symbol: Option<String>,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFundingTransactionsResponse {
    pub funding_transactions: Vec<FundingTransaction>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFundingTransactionsRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
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
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SandboxWithdrawalRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetPositionsRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetPositionsResponse {
    pub positions: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Position {
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub symbol: String,
    pub signed_quantity: i64,
    pub signed_notional: Decimal,
    pub timestamp: DateTime<Utc>,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFillsRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
    /// Optional symbol filter. If provided, only fills for this symbol will be returned.
    pub symbol: Option<String>,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFillsResponse {
    pub fills: Vec<Fill>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
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
    pub maker_account_id: String,
    pub taker_account_id: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd_borrow: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Balance {
    pub account_id: String,
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
    pub user_id: Option<String>,
    pub account_id: String,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetRiskSnapshotRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetVolumeRequest {
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetVolumeResponse {
    pub volume: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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

    #[test]
    fn test_monthly_period_start() {
        let c = LeaderboardCadence::Monthly;
        assert_eq!(
            c.period_start(Utc.with_ymd_and_hms(2025, 7, 15, 12, 30, 0).unwrap())
                .unwrap(),
            Utc.with_ymd_and_hms(2025, 7, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            c.period_start(Utc.with_ymd_and_hms(2025, 1, 1, 12, 30, 0).unwrap())
                .unwrap(),
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            c.period_start(Utc.with_ymd_and_hms(2025, 12, 31, 12, 30, 0).unwrap())
                .unwrap(),
            Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_monthly_next_period_start() {
        let c = LeaderboardCadence::Monthly;
        let jan = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            c.next_period_start(jan).unwrap(),
            Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap()
        );
        let dec = Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap();
        assert_eq!(
            c.next_period_start(dec).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_monthly_crossed_boundary() {
        let c = LeaderboardCadence::Monthly;
        let jan_15 = Utc.with_ymd_and_hms(2025, 1, 15, 12, 30, 0).unwrap();
        let jan_20 = Utc.with_ymd_and_hms(2025, 1, 20, 12, 30, 0).unwrap();
        let feb_1 = Utc.with_ymd_and_hms(2025, 2, 1, 12, 30, 0).unwrap();
        let jan_next_year = Utc.with_ymd_and_hms(2026, 1, 15, 12, 30, 0).unwrap();
        assert!(!c.crossed_boundary(jan_15, jan_20));
        assert!(c.crossed_boundary(jan_15, feb_1));
        assert!(c.crossed_boundary(jan_15, jan_next_year));
    }

    #[test]
    fn test_serde_roundtrip() {
        let metric = LeaderboardMetric::Volume;
        let json = serde_json::to_string(&metric).unwrap();
        assert_eq!(json, r#""volume""#);
        assert_eq!(
            serde_json::from_str::<LeaderboardMetric>(&json).unwrap(),
            metric
        );

        let cadence = LeaderboardCadence::Monthly;
        let json = serde_json::to_string(&cadence).unwrap();
        assert_eq!(json, r#""monthly""#);
        assert_eq!(
            serde_json::from_str::<LeaderboardCadence>(&json).unwrap(),
            cadence
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
    /// Instrument symbol (e.g. "XAU-PERP")
    pub symbol: String,
    /// Start of the time range (nanoseconds since epoch, inclusive)
    pub start_timestamp_ns: u64,
    /// End of the time range (nanoseconds since epoch, inclusive)
    pub end_timestamp_ns: u64,
    /// Candle width (e.g. "1s", "1m", "1h", "1d")
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
    /// Instrument symbol (e.g. "XAU-PERP")
    pub symbol: String,
    /// Candle width (e.g. "1s", "1m", "1h", "1d")
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetEstimatedFundingRateRequest {
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum EstimatedFundingRateStatus {
    Ready,
    SettlementPending,
    Unavailable,
}

/// Live estimated funding rate for a symbol, served verbatim from the cached
/// estimate the settlement runner publishes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetEstimatedFundingRateResponse {
    pub symbol: String,
    pub status: EstimatedFundingRateStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub funding_rate: Option<Decimal>,
    pub funding_amount: Option<Decimal>,
    pub benchmark_price: Option<Decimal>,
    pub settlement_price: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetAccountEquityHistoryRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
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
    pub account_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetIndexPricesRequest {
    pub symbol: String,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetIndexPricesResponse {
    pub index_prices: Vec<IndexPrice>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct IndexPrice {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
}
