use crate::{
    Side,
    protocol::{
        common::{Fill, Timestamp},
        marketdata_publisher::{Ticker, Trade},
        pagination::{
            LimitOffsetPage, LimitOffsetPagination, TimeseriesPage, TimeseriesPagination,
        },
        sort::SortFields,
    },
    types::{ApiKeyPermissions, BboCandle, Candle, Instrument, PerpetualSpecs, Token},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateApiKeyRequest {
    #[serde(default)]
    pub allowed_ips: Option<Vec<String>>,
    /// Accounts the key may act on. Each must be one the caller has access to.
    /// When omitted, the key covers every account the caller can access. When
    /// provided, the key is restricted to exactly those accounts.
    #[serde(default)]
    pub account_ids: Option<Vec<String>>,
    /// Permissions to grant the key, intersected per request with the caller's
    /// current permissions on the requested account (so a key can never
    /// out-rank its owner). When omitted, the key is granted full permissions
    /// and behaves as the user.
    #[serde(default)]
    pub permissions: Option<ApiKeyPermissions>,
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub allowed_ips: Option<Vec<String>>,
    /// Accounts this key may act on. `null` means every account the owner can
    /// access; otherwise the key is restricted to exactly these accounts.
    pub account_ids: Option<Vec<String>>,
    /// The key's granted permissions.
    pub permissions: ApiKeyPermissions,
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
    #[serde(default)]
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UpdateApiKeyAllowedIpsResponse {
    pub message: String,
}

/// Exchange an API key and secret for a bearer token.
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
    ApiKeySecret { api_key: String, api_secret: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AuthenticateResponse {
    pub token: Token,
}

/// Log in with a Clerk session token, exchanging it for an AX session token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ClerkLoginRequest {
    /// Session JWT obtained from Clerk after completing sign-in.
    pub clerk_token: String,
    pub expiration_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WhoAmIResponse {
    pub id: String,
    pub username: String,
    pub pseudonym: String,
    pub created_at: DateTime<Utc>,
    pub is_onboarded: bool,
    pub is_frozen: bool,
    pub is_admin: bool,
    pub require_2fa: bool,
    pub fiat_deposit_code: String,
    #[serde(default)]
    pub accounts: Vec<WhoAmIAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WhoAmIAccount {
    pub id: String,
    /// Optional, owner-facing nickname for the account.
    pub name: Option<String>,
    pub is_close_only: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(transparent)]
pub struct GetInstrumentResponse(pub Instrument);

/// Strips perpetual-only fields when the instrument is not a perpetual.
impl Serialize for GetInstrumentResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.is_perpetual() {
            return self.0.serialize(serializer);
        }

        let mut value = serde_json::to_value(&self.0).map_err(serde::ser::Error::custom)?;
        let object = value.as_object_mut().ok_or_else(|| {
            serde::ser::Error::custom("instrument did not serialize as a JSON object")
        })?;
        for field in PerpetualSpecs::FIELD_NAMES {
            object.remove(field);
        }
        value.serialize(serializer)
    }
}

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
    #[serde(flatten)]
    pub page: LimitOffsetPage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTickersQueryParams {
    #[serde(flatten)]
    pub pagination: LimitOffsetPagination,
    #[serde(default)]
    pub sort: SortFields,
}

/// Kinds of ledger entry that can appear as a `Transaction::transaction_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    /// Funds credited into the account.
    Deposit,
    /// Funds debited out of the account.
    Withdrawal,
    /// Periodic funding payment on a perpetual position.
    Funding,
    /// Trading fee.
    Fee,
    /// Realized profit and loss on a position.
    Pnl,
    /// Funds credited to the account from the lending pool.
    LendingCredit,
    /// Funds debited from the account to the lending pool.
    LendingDebit,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetTransactionsRequest {
    /// Transaction types to include, as a single comma-separated value
    /// (`transaction_types=deposit,withdrawal`). Repeating the parameter is
    /// rejected; an empty value includes every type. Values not listed here
    /// match nothing rather than erroring.
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[cfg_attr(feature = "utoipa", schema(value_type = Vec<TransactionType>))]
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
    pub account_id: String,
    pub event_id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
    /// The kind of ledger entry; one of the [`TransactionType`] values.
    // Kept as `String` (not the enum) so an unrecognized value deserializes
    // rather than failing; `value_type` documents the closed set in the spec.
    #[cfg_attr(feature = "utoipa", schema(value_type = TransactionType))]
    pub transaction_type: String,
    pub reference_id: Option<String>,
    /// Actor of record — the user who initiated the transaction. Present only
    /// for directly-initiated kinds (`deposit`, `withdrawal`); `None` for
    /// order-derived and system-generated transactions.
    pub initiated_by_user_id: Option<String>,
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
    pub account_id: String,
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
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
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

/// A ledger view of a position: every field is derived from the fill stream
/// alone (no price feed), so it only changes when a fill occurs. For
/// mark-dependent valuation (USD notional, unrealized PnL), use the risk
/// snapshot endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Position {
    pub account_id: String,
    pub symbol: String,
    pub signed_quantity: i64,
    /// Signed cost basis (Σ fill `price * quantity`), unmultiplied — in
    /// contract price units, not USD. Multiply by the instrument's contract
    /// multiplier for the USD cost basis.
    pub signed_cost_basis: Decimal,
    /// **Deprecated:** renamed to `signed_cost_basis` (same value). Retained for
    /// backward compatibility; will be removed in a future release.
    #[deprecated(note = "use `signed_cost_basis`")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetBalancesRequest {
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
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
    /// Signed cost basis (Σ fill `price * quantity`), unmultiplied — in
    /// contract price units, not USD. Multiply by the instrument's contract
    /// multiplier for the USD cost basis.
    pub signed_cost_basis: Decimal,
    /// **Deprecated:** renamed to `signed_cost_basis` (same value). Retained for
    /// backward compatibility; will be removed in a future release.
    #[deprecated(note = "use `signed_cost_basis`")]
    pub signed_notional: Decimal,
    /// Mark price this snapshot was priced at — the input to every USD figure
    /// in this row. `None` for snapshots predating this field.
    #[serde(default)]
    pub mark_price: Option<Decimal>,
    /// Signed USD exposure at the mark: `signed_quantity * mark_price *
    /// contract multiplier`. `None` for snapshots predating this field.
    #[serde(default)]
    pub signed_usd_notional: Option<Decimal>,
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
pub struct AccountRiskSnapshot {
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
    pub risk_snapshot: AccountRiskSnapshot,
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

    fn instrument_with_funding(expiration: Option<&str>, populated: bool) -> Instrument {
        serde_json::from_value(serde_json::json!({
            "symbol": if expiration.is_some() { "XAU-2026-SEP" } else { "XAU-PERP" },
            "product": "XAU",
            "expiration": expiration,
            "multiplier": "1",
            "price_scale": 1,
            "minimum_order_size": "1",
            "tick_size": "0.1",
            "quote_currency": "USD",
            "price_band_lower_deviation_pct": null,
            "price_band_upper_deviation_pct": null,
            "funding_settlement_currency": "USD",
            "funding_rate_cap_upper_pct": populated.then_some("1"),
            "funding_rate_cap_lower_pct": populated.then_some("-1"),
            "maintenance_margin_pct": "4",
            "initial_margin_pct": "8",
            "category": "metals",
            "description": null,
            "underlying_benchmark_price": null,
            "contract_mark_price": null,
            "contract_size": null,
            "price_quotation": null,
            "price_bands": null,
            "funding_schedule_time_description": populated.then_some("every eight hours"),
            "funding_schedule_calendar_description": populated.then_some("every day"),
            "funding_schedule": populated.then_some(serde_json::json!({
                "timezone": "UTC",
                "times": [],
                "exceptions": []
            })),
            "trading_schedule": null,
            "estimated_funding_supported": populated,
            "additional_product_specs": null
        }))
        .unwrap()
    }

    fn funding_field_names(value: &serde_json::Value) -> Vec<&str> {
        let mut fields = value
            .as_object()
            .unwrap()
            .keys()
            .filter(|field| field.starts_with("funding_"))
            .map(String::as_str)
            .collect::<Vec<_>>();
        fields.sort_unstable();
        fields
    }

    #[test]
    fn instrument_normalizes_perpetual_specs_from_expiration() {
        let perpetual = instrument_with_funding(None, true);
        let specs = perpetual.perpetual_specs().unwrap();

        assert_eq!(specs.funding_settlement_currency, "USD");
        assert_eq!(specs.funding_rate_cap_upper_pct, Some(Decimal::ONE));
        assert_eq!(
            specs.funding_rate_cap_lower_pct,
            Some(Decimal::NEGATIVE_ONE)
        );
        assert_eq!(
            specs.funding_schedule_time_description.as_deref(),
            Some("every eight hours")
        );
        assert_eq!(
            specs.funding_schedule_calendar_description.as_deref(),
            Some("every day")
        );
        assert!(specs.funding_schedule.is_some());

        let dated = instrument_with_funding(Some("2026-09-30T00:00:00Z"), true);
        assert!(dated.perpetual_specs().is_none());
    }

    #[test]
    fn perpetual_instrument_response_preserves_funding_keys() {
        let response = GetInstrumentResponse(instrument_with_funding(None, false));
        let json = serde_json::to_value(response).unwrap();
        let object = json.as_object().unwrap();

        assert_eq!(
            funding_field_names(&json),
            vec![
                "funding_rate_cap_lower_pct",
                "funding_rate_cap_upper_pct",
                "funding_schedule",
                "funding_schedule_calendar_description",
                "funding_schedule_time_description",
                "funding_settlement_currency",
            ]
        );
        assert_eq!(object["funding_settlement_currency"], "USD");
        assert!(object["funding_rate_cap_upper_pct"].is_null());
        assert!(object["funding_rate_cap_lower_pct"].is_null());
        assert!(object["funding_schedule_time_description"].is_null());
        assert!(object["funding_schedule_calendar_description"].is_null());
        assert!(object["funding_schedule"].is_null());
    }

    #[test]
    fn dated_instrument_response_omits_funding_keys() {
        let response =
            GetInstrumentResponse(instrument_with_funding(Some("2026-09-30T00:00:00Z"), true));
        let json = serde_json::to_value(response).unwrap();
        let object = json.as_object().unwrap();

        assert!(funding_field_names(&json).is_empty());
        assert_eq!(object["estimated_funding_supported"], true);
    }

    #[test]
    fn instrument_response_deserializes_omitted_dated_funding_fields() {
        let mut json =
            serde_json::to_value(instrument_with_funding(Some("2026-09-30T00:00:00Z"), true))
                .unwrap();
        let object = json.as_object_mut().unwrap();
        object.retain(|field, _| !field.starts_with("funding_"));

        let response: GetInstrumentResponse = serde_json::from_value(json).unwrap();

        assert!(response.0.funding_settlement_currency.is_empty());
        assert!(response.0.funding_rate_cap_upper_pct.is_none());
        assert!(response.0.funding_rate_cap_lower_pct.is_none());
        assert!(response.0.funding_schedule_time_description.is_none());
        assert!(response.0.funding_schedule_calendar_description.is_none());
        assert!(response.0.funding_schedule.is_none());
    }

    #[test]
    fn instrument_list_serializes_mixed_contract_funding_shapes() {
        let response = GetInstrumentsResponse {
            instruments: vec![
                GetInstrumentResponse(instrument_with_funding(None, false)),
                GetInstrumentResponse(instrument_with_funding(Some("2026-09-30T00:00:00Z"), true)),
            ],
        };
        let json = serde_json::to_value(response).unwrap();
        let instruments = json["instruments"].as_array().unwrap();

        assert_eq!(funding_field_names(&instruments[0]).len(), 6);
        assert!(funding_field_names(&instruments[1]).is_empty());
    }

    #[test]
    fn test_get_user_token_request_serde() {
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
    fn test_get_funding_rates_request_query_params() {
        let req: GetFundingRatesRequest = serde_urlencoded::from_str(
            "symbol=EURUSD-PERP&start_timestamp_ns=1&end_timestamp_ns=2&limit=2&sort_ts=asc",
        )
        .unwrap();
        assert_eq!(req.symbol, "EURUSD-PERP");
        assert_eq!(req.timeseries.range.start_timestamp_ns, Some(1));
        assert_eq!(req.timeseries.range.end_timestamp_ns, Some(2));
        assert_eq!(req.timeseries.pagination.limit, Some(2));

        // legacy query string (no pagination params) still parses
        let legacy: GetFundingRatesRequest = serde_urlencoded::from_str(
            "symbol=EURUSD-PERP&start_timestamp_ns=10&end_timestamp_ns=20",
        )
        .unwrap();
        assert_eq!(legacy.timeseries.pagination.limit, None);
        assert_eq!(legacy.timeseries.pagination.cursor, None);
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

/// Query parameters for `GET /funding-rates`, including required `symbol` plus
/// optional time range, sort direction, cursor, and page size.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFundingRatesRequest {
    pub symbol: String,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFundingRatesResponse {
    pub funding_rates: Vec<FundingRate>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingRate {
    pub symbol: String,
    pub timestamp_ns: u64,
    pub funding_rate: Decimal,
    #[serde(default)]
    pub funding_amount: Option<Decimal>,
    #[serde(default)]
    pub benchmark_price: Option<Decimal>,
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

/// Query parameters for `GET /funding-slots`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetFundingSlotsRequest {
    pub symbol: String,
    /// Trading date, interpreted in the symbol's funding schedule timezone.
    /// Defaults to the current date there.
    pub date: Option<NaiveDate>,
}

/// How a symbol's funding accrues over a trading day.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FundingVariant {
    /// A single funding settlement at the daily close.
    DailyClose,
    /// Funding settles in fixed intraday slots, each charging its share of
    /// the day's TWAP premium.
    IntradayTwap,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FundingSlotStatus {
    /// The slot has settled; values are the settled ones.
    Realized,
    /// The slot is in the future; values are derived from the current
    /// estimated funding rate, when one is available.
    Projected,
    /// The slot was passed over (insufficient benchmark or mark data in its
    /// interval); no funding was charged.
    Skipped,
    /// The slot has elapsed but its settlement has not been recorded yet.
    Pending,
}

/// One funding slot of a trading day.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FundingSlot {
    /// 1-based position within the day's schedule.
    pub index: u32,
    pub funding_time: DateTime<Utc>,
    pub status: FundingSlotStatus,
    pub mark_twap: Option<Decimal>,
    pub underlying_twap: Option<Decimal>,
    /// Premium of the mark TWAP over the underlying TWAP, in basis points.
    pub premium_bps: Option<Decimal>,
    /// The slot's funding rate in basis points; positive means longs pay
    /// shorts.
    pub funding_rate_bps: Option<Decimal>,
    /// True when the rate was clamped by the symbol's funding rate cap.
    pub capped: bool,
    /// Why the slot was skipped; present only on skipped slots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A full trading day of funding slots — realized, projected, skipped, and
/// pending — with running totals. One surface for both funding variants:
/// `daily_close` symbols report a single slot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetFundingSlotsResponse {
    pub symbol: String,
    pub date: NaiveDate,
    /// IANA name of the funding schedule's timezone.
    pub timezone: String,
    pub variant: FundingVariant,
    /// Number of funding slots scheduled on `date`; 0 on holidays and
    /// weekends.
    pub interval_count: u32,
    /// Per-slot cap on the funding rate in basis points, if configured.
    pub cap_bps: Option<Decimal>,
    pub slots: Vec<FundingSlot>,
    /// Sum of realized slot rates so far, in basis points.
    pub realized_sum_bps: Decimal,
    /// Projected end-of-day total in basis points: realized so far plus the
    /// projection for the remaining slots.
    pub projected_eod_bps: Decimal,
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
pub struct GetUnderlyingPricesRequest {
    pub symbol: String,
    #[serde(flatten)]
    pub timeseries: TimeseriesPagination,
}

#[deprecated(note = "use GetUnderlyingPricesRequest")]
pub type GetIndexPricesRequest = GetUnderlyingPricesRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUnderlyingPricesResponse {
    pub underlying_prices: Vec<UnderlyingPrice>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

// TODO: deprecated, use GetUnderlyingPrices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetIndexPricesResponse {
    pub index_prices: Vec<UnderlyingPrice>,
    #[serde(flatten)]
    pub page: TimeseriesPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UnderlyingPrice {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
}

#[deprecated(note = "use UnderlyingPrice")]
pub type IndexPrice = UnderlyingPrice;
