//! Business Logic Types
//!
//! This module contains core business types for trading operations.

use super::days_of_week::DaysOfWeek;
use super::funding_rate_schedule::FundingRateSchedule;
use crate::{ClientOrderId, OrderId};
use anyhow::{Error, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use strum::VariantArray;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentV0 {
    pub symbol: String,
    pub tick_size: Decimal,
    pub base_currency: String,
    pub multiplier: i32,
    pub minimum_trade_quantity: u64,
    pub description: String,
    pub product_id: String,
    pub state: String,
    pub price_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Instrument {
    pub symbol: String,
    /// Absolute expiration time for dated contracts. `None` for perpetuals.
    /// Presence of a value is the discriminator between dated and perpetual contracts.
    #[serde(default)]
    pub expiration: Option<DateTime<Utc>>,
    // Programmatic specification fields
    pub multiplier: Decimal,
    pub price_scale: i64,
    pub minimum_order_size: Decimal,
    pub tick_size: Decimal,
    pub quote_currency: String,
    pub price_band_lower_deviation_pct: Option<Decimal>,
    pub price_band_upper_deviation_pct: Option<Decimal>,
    pub funding_settlement_currency: String,
    pub funding_rate_cap_upper_pct: Option<Decimal>,
    pub funding_rate_cap_lower_pct: Option<Decimal>,
    pub maintenance_margin_pct: Decimal,
    pub initial_margin_pct: Decimal,
    pub category: InstrumentCategory,
    // English language specification fields
    pub description: Option<String>,
    pub underlying_benchmark_price: Option<String>,
    pub contract_mark_price: Option<String>,
    pub contract_size: Option<String>,
    pub price_quotation: Option<String>,
    pub price_bands: Option<String>,
    pub funding_schedule_time_description: Option<String>,
    pub funding_schedule_calendar_description: Option<String>,
    pub funding_schedule: Option<FundingRateSchedule>,
    pub trading_schedule: Option<TradingSchedule>,
    /// Whether a live index feed is configured for this instrument, so an
    /// intraday funding-rate estimate can be produced. When `false`, the
    /// estimated-funding endpoint reports the symbol as unsupported and
    /// clients should not surface an estimate for it.
    #[serde(default)]
    pub estimated_funding_supported: bool,
    #[cfg_attr(feature = "utoipa", schema(value_type = Object))]
    pub additional_product_specs: Option<HashMap<String, String>>,
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

/// Return the canonical integer multiplier used to scale prices for `tick_size`.
///
/// This is the *reduced* multiplier (`denominator / gcd`), not `10^decimals`;
/// e.g. `0.05 -> 20`. The two conventions coincide for `1×10⁻ⁿ` ticks,
/// and diverge only for fractional-mantissa ticks (`0.05`, `0.25`).
/// See `validate_price_scale` and the tests below.
pub fn price_scale_from_tick_size(tick_size: Decimal) -> Result<i64> {
    if tick_size <= Decimal::ZERO {
        bail!("tick_size must be positive, got {tick_size}");
    }

    let tick_size = tick_size.normalize();
    let numerator = tick_size.mantissa().unsigned_abs();
    let denominator = 10_u128.pow(tick_size.scale());
    let price_scale = denominator / gcd_u128(numerator, denominator);

    i64::try_from(price_scale)
        .map_err(|_| anyhow!("price scale {price_scale} for tick_size {tick_size} overflows i64"))
}

/// Validate that `price_scale` is the canonical multiplier for `tick_size`.
///
/// Callers gate the error on their own `strict` flag: strict callers fail
/// fast on a mismatch, lenient callers log and keep the instrument.
/// Only fires on a fractional-mantissa tick — none exist in live markets today.
pub fn validate_price_scale(symbol: &str, tick_size: Decimal, price_scale: i64) -> Result<()> {
    if price_scale <= 0 {
        bail!("instrument {symbol} has non-positive price_scale {price_scale}");
    }

    let expected = price_scale_from_tick_size(tick_size)?;
    if price_scale != expected {
        bail!(
            "instrument {symbol} has price_scale {price_scale}, expected {expected} for tick_size {tick_size}"
        );
    }
    Ok(())
}

#[derive(
    Copy, Clone, Debug, Eq, PartialEq, strum::Display, strum::EnumString, Serialize, Deserialize,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum InstrumentCategory {
    Compute,
    Treasuries,
    Energy,
    Fx,
    Equities,
    Metals,
    EnergyEtfs,
}

/// Trading schedule for an instrument, containing multiple trading hour segments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TradingSchedule {
    pub segments: Vec<TradingHoursSegment>,
}

/// A single trading hours segment with specific days, times, and state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TradingHoursSegment {
    /// Days of the week (1=Monday, 2=Tuesday, ..., 7=Sunday)
    pub days_of_week: DaysOfWeek,
    /// Time of day when this segment starts
    pub time_of_day: TimeOfDay,
    /// Duration of this segment in seconds
    pub duration_seconds: u64,
    /// Trading state during this segment
    pub state: InstrumentState,
    /// Whether to hide market data during this segment
    pub hide_market_data: bool,
    /// Whether to expire all orders during this segment
    pub expire_all_orders: bool,
}

/// Time of day representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TimeOfDay {
    pub hours: u8,
    pub minutes: u8,
    #[serde(default)]
    pub seconds: u8,
}

impl TimeOfDay {
    pub fn validate(&self) -> Result<()> {
        if self.hours > 23 || self.minutes > 59 || self.seconds > 59 {
            bail!(
                "invalid time_of_day: {:02}:{:02}:{:02}",
                self.hours,
                self.minutes,
                self.seconds
            );
        }
        Ok(())
    }
}

#[derive(Default, Debug, strum::Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentState {
    /// The instrument is closed. No trading can occur, nor can orders
    /// even be cancelled in this state.
    ClosedFrozen,

    /// The instrument is available to place orders and modify them
    /// before the opening, but no matching will occur until the open.
    ///
    /// At the open, crossing orders will be matched via Dutch auction.
    PreOpen,

    /// The instrument is open and is available for full trading.
    Open,

    /// The instrument has suspended trading. In this state, no orders
    /// can be placed or modified, but they can be cancelled.
    Closed,

    /// The instrument has been delisted.  This state is terminal.
    Delisted,

    /// The instrument has halted trading. This state is similar to the suspended state in that no orders can be placed or modified, but orders cannot be cancelled unlike the suspended state which allows cancellation.
    Halted,

    /// The instrument is available to place orders and modify them just as in pre open, but no matching will occur until this state has been exited.
    MatchAndCloseAuction,

    /// The instrument status is unknown.
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrder {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub side: Side,
    #[serde(rename = "q")]
    pub quantity: u64,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "tif")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "po")]
    pub post_only: bool,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(rename = "cid", skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<ClientOrderId>,
    #[serde(rename = "st")]
    pub self_trade_prevention: SelfTradeBehavior,
    /// Optional account ID. If omitted, default (primary) user account is used.
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeInForce {
    #[serde(rename = "GTC")]
    GoodTillCanceled,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
    #[serde(rename = "DAY")]
    Day,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: String,
    pub account_id: String,
    pub symbol: String,
    pub side: Side,
    pub quantity: u64,
    pub price: Decimal,
    pub time_in_force: TimeInForce,
    pub tag: Option<String>,
    pub clord_id: Option<ClientOrderId>,
    #[serde(default)]
    pub post_only: bool,
    /// Timestamp when the order was received by the order gateway
    pub timestamp: DateTime<Utc>,
    pub order_state: OrderState,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,
    /// Timestamp when the order state became terminal
    pub completion_time: Option<DateTime<Utc>>,
    /// Reason for rejection if order_state is Rejected
    pub reject_reason: Option<OrderRejectReason>,
    /// Additional message for rejection if order_state is Rejected
    pub reject_message: Option<String>,
}

impl Order {
    /// Check if this is a liquidation order
    pub fn is_liquidation(&self) -> bool {
        self.order_id.is_liquidation()
    }
}

#[derive(
    Debug, Default, derive_more::Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum SelfTradeBehavior {
    /// Cancel the incoming aggressor order; resting orders remain on the book.
    #[default]
    #[serde(alias = "xi")]
    CancelIncoming,
    /// Cancel resting orders that would self-match; allow the aggressor.
    #[serde(alias = "xr")]
    CancelResting,
    /// Cancel both resting orders and the incoming aggressor.
    #[serde(alias = "xb")]
    CancelBoth,
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Side {
    #[serde(rename = "B")]
    Buy,
    #[serde(rename = "S")]
    Sell,
}

impl Side {
    pub fn as_char(&self) -> &'static str {
        match self {
            Self::Buy => "B",
            Self::Sell => "S",
        }
    }

    pub fn from_char(s: &str) -> Result<Self> {
        let t = match s {
            "B" => Self::Buy,
            "S" => Self::Sell,
            other => bail!("unknown side: {other}"),
        };
        Ok(t)
    }

    pub fn position_sign(&self) -> i8 {
        match self {
            Self::Buy => 1,
            Self::Sell => -1,
        }
    }

    pub fn flip(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum OrderState {
    #[strum(serialize = "PENDING")]
    #[serde(rename = "PENDING")]
    Pending,
    #[strum(serialize = "ACCEPTED")]
    #[serde(rename = "ACCEPTED")]
    Accepted,
    #[strum(serialize = "PARTIALLY_FILLED")]
    #[serde(rename = "PARTIALLY_FILLED")]
    PartiallyFilled,
    #[strum(serialize = "FILLED")]
    #[serde(rename = "FILLED")]
    Filled,
    #[strum(serialize = "CANCELED")]
    #[serde(rename = "CANCELED")]
    Canceled,
    #[strum(serialize = "REJECTED")]
    #[serde(rename = "REJECTED")]
    Rejected,
    #[strum(serialize = "EXPIRED")]
    #[serde(rename = "EXPIRED")]
    Expired,
    #[strum(serialize = "REPLACED")]
    #[serde(rename = "REPLACED")]
    Replaced,
    #[strum(serialize = "DONE_FOR_DAY")]
    #[serde(rename = "DONE_FOR_DAY")]
    DoneForDay,
    #[strum(serialize = "UNKNOWN")]
    #[serde(rename = "UNKNOWN", other)]
    Unknown,
}

impl OrderState {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn is_open(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Accepted | Self::PartiallyFilled => true,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Canceled
            | Self::Filled
            | Self::Rejected
            | Self::Replaced
            | Self::DoneForDay
            | Self::Expired => true,
            _ => false,
        }
    }

    pub fn can_transition_to(&self, next_state: &Self) -> bool {
        match self {
            Self::Pending => matches!(
                next_state,
                Self::Pending
                    | Self::Accepted
                    | Self::Rejected
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            Self::Accepted => matches!(
                next_state,
                Self::Accepted
                    | Self::PartiallyFilled
                    | Self::Filled
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            Self::PartiallyFilled => matches!(
                next_state,
                Self::PartiallyFilled
                    | Self::Filled
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            _ => false, // terminal states
        }
    }

    /// Check if the order can be canceled
    pub fn can_be_canceled(&self) -> bool {
        matches!(self, Self::Pending | Self::Accepted | Self::PartiallyFilled)
    }

    /// Check if the order can be replaced
    pub fn can_be_replaced(&self) -> bool {
        matches!(self, Self::Accepted | Self::PartiallyFilled)
    }

    /// Canonical single-character representation
    pub fn as_char(&self) -> &'static str {
        match self {
            Self::Pending => "P",
            Self::Accepted => "A",
            Self::PartiallyFilled => "D",
            Self::Filled => "F",
            Self::Canceled => "X",
            Self::Rejected => "R",
            Self::Expired => "E",
            Self::Replaced => "K",
            Self::DoneForDay => "Z",
            Self::Unknown => "?",
        }
    }

    pub fn from_char(s: &str) -> Result<Self> {
        let t = match s {
            "P" => Self::Pending,
            "A" => Self::Accepted,
            "D" => Self::PartiallyFilled,
            "F" => Self::Filled,
            "X" => Self::Canceled,
            "R" => Self::Rejected,
            "E" => Self::Expired,
            "K" => Self::Replaced,
            "Z" => Self::DoneForDay,
            "?" => Self::Unknown,
            other => bail!("unknown order state: {other}"),
        };
        Ok(t)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderRejectReason {
    /// User in close-only mode attempting non-closing order
    CloseOnly,
    /// Initial margin required exceeds available
    InsufficientMargin,
    /// User has too many open orders
    MaxOpenOrdersExceeded,
    /// Unknown or invalid symbol
    UnknownSymbol,
    /// Exchange is closed
    ExchangeClosed,
    /// Incorrect or invalid quantity
    IncorrectQuantity,
    /// Invalid price increment
    InvalidPriceIncrement,
    /// Incorrect order type
    IncorrectOrderType,
    /// Price is outside allowed bounds
    PriceOutOfBounds,
    /// No liquidity available
    NoLiquidity,
    /// Insufficient credit limit
    InsufficientCreditLimit,
    /// Original order was canceled or filled while a cancel-replace was pending
    OriginalOrderTerminated,
    /// Client order ID is already in use by another open order
    DuplicateClientOrderId,
    /// Unknown or unrecognized reject reason
    #[serde(other)]
    Unknown,
}

impl OrderRejectReason {
    /// Human-readable description of the reject reason.
    pub fn message(&self) -> &'static str {
        match self {
            Self::CloseOnly => "account is in close-only mode",
            Self::InsufficientMargin => "insufficient margin for order",
            Self::MaxOpenOrdersExceeded => "too many open orders on this account",
            Self::UnknownSymbol => "symbol not found",
            Self::ExchangeClosed => "exchange is closed",
            Self::IncorrectQuantity => "order quantity is invalid",
            Self::InvalidPriceIncrement => "price uses more precision than the minimum tick size",
            Self::IncorrectOrderType => "order type is not allowed for this instrument",
            Self::PriceOutOfBounds => "price is outside the allowed band",
            Self::NoLiquidity => "no liquidity available to fill this order",
            Self::InsufficientCreditLimit => "insufficient buying power",
            Self::OriginalOrderTerminated => "original order is no longer active",
            Self::DuplicateClientOrderId => "duplicate client order ID",
            Self::Unknown => "order rejected",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub currency: String,
    pub available: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub signed_quantity: i64,
    pub average_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub mark_price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Candle {
    pub symbol: String,
    #[serde(rename = "ts")]
    #[serde_as(as = "serde_with::TimestampSeconds")]
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub buy_volume: u64,
    pub sell_volume: u64,
    pub volume: u64,
    pub width: CandleWidth,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct BboCandle {
    /// Instrument symbol (e.g. "XAU-PERP")
    pub symbol: String,
    /// Start timestamp of the candle interval (epoch seconds)
    #[serde(rename = "ts")]
    #[serde_as(as = "serde_with::TimestampSeconds")]
    pub timestamp: DateTime<Utc>,
    /// Best bid price at the start of the interval
    pub bid_open: Option<Decimal>,
    /// Highest best bid price during the interval
    pub bid_high: Option<Decimal>,
    /// Lowest best bid price during the interval
    pub bid_low: Option<Decimal>,
    /// Best bid price at the end of the interval
    pub bid_close: Option<Decimal>,
    /// Best ask price at the start of the interval
    pub ask_open: Option<Decimal>,
    /// Highest best ask price during the interval
    pub ask_high: Option<Decimal>,
    /// Lowest best ask price during the interval
    pub ask_low: Option<Decimal>,
    /// Best ask price at the end of the interval
    pub ask_close: Option<Decimal>,
    /// Mid-price ((bid + ask) / 2) at the start of the interval
    pub mid_open: Option<Decimal>,
    /// Highest mid-price during the interval
    pub mid_high: Option<Decimal>,
    /// Lowest mid-price during the interval
    pub mid_low: Option<Decimal>,
    /// Mid-price at the end of the interval
    pub mid_close: Option<Decimal>,
    /// Duration of the candle interval
    pub width: CandleWidth,
}

// TODO: re-examine the shape of this type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterest {
    pub symbol: String,
    pub data: Vec<OpenInterestData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestData {
    pub timestamp: DateTime<Utc>,
    pub open_interest: Decimal,
}

// TODO: re-examine the name of this type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistory {
    pub symbol: String,
    pub funding_amount: Decimal,
    pub net_position: i32,
    pub timestamp: DateTime<Utc>,
    pub funding_rate: Decimal,
}

// TODO: reconsider where this type lives; cash management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

// TODO: reconsider where this type lives; cash management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

#[derive(
    Copy,
    Clone,
    VariantArray,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum CandleWidth {
    #[serde(rename = "1s")]
    #[display("1s")]
    OneSecond,
    #[serde(rename = "5s")]
    #[display("5s")]
    FiveSecond,
    #[serde(rename = "1m")]
    #[display("1m")]
    OneMinute,
    #[serde(rename = "5m")]
    #[display("5m")]
    FiveMinute,
    #[serde(rename = "15m")]
    #[display("15m")]
    FifteenMinute,
    #[serde(rename = "1h")]
    #[display("1h")]
    OneHour,
    #[serde(rename = "1d")]
    #[display("1d")]
    OneDay,
}

impl std::str::FromStr for CandleWidth {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "1s" => Ok(Self::OneSecond),
            "5s" => Ok(Self::FiveSecond),
            "1m" => Ok(Self::OneMinute),
            "5m" => Ok(Self::FiveMinute),
            "15m" => Ok(Self::FifteenMinute),
            "1h" => Ok(Self::OneHour),
            "1d" => Ok(Self::OneDay),
            _ => Err(anyhow!("unrecognized candle width: '{s}'")),
        }
    }
}

impl CandleWidth {
    /// Get the closed interval of nanosecond timestamps containing `instant`
    /// that form the candle of this width.
    pub fn to_nanosec_window(&self, instant: u64) -> (u64, u64) {
        let ns_in_sec = 1_000_000_000;
        let nanosec = match self {
            CandleWidth::OneSecond => ns_in_sec,
            CandleWidth::FiveSecond => ns_in_sec * 5,
            CandleWidth::OneMinute => ns_in_sec * 60,
            CandleWidth::FiveMinute => ns_in_sec * 60 * 5,
            CandleWidth::FifteenMinute => ns_in_sec * 60 * 15,
            CandleWidth::OneHour => ns_in_sec * 60 * 60,
            CandleWidth::OneDay => ns_in_sec * 60 * 60 * 24,
        };

        let start = instant - (instant % nanosec);
        let end = start + nanosec - 1;

        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 20250916T19:19:39.100Z - Arbitrary time
    const TIME_1: u64 = 1758050379100000000;

    // 20250916T16:29:43.500Z - Arbitrary time
    const TIME_2: u64 = 1758040183500000000;

    // 20250916T00:00:00.000Z - Exact midnight UTC
    const MIDNIGHT_UTC: u64 = 1757980800000000000;

    // 20250916T23:59:59.999999999Z - Last nanosecond of the day
    const END_OF_DAY: u64 = 1758067199999999999;

    // 20250916T12:00:00.000Z - Exact noon
    const NOON: u64 = 1758024000000000000;

    // 20250916T11:59:59.999999999Z - Last nanosecond before noon
    const JUST_BEFORE_NOON: u64 = 1758023999999999999;

    // 20250916T15:30:00.000Z - Exact half hour
    const HALF_HOUR: u64 = 1758036600000000000;

    // 20250916T15:30:05.000Z - Exact 5-second boundary
    const FIVE_SEC_BOUNDARY: u64 = 1758036605000000000;

    // 20250916T15:30:04.999999999Z - Just before 5-second boundary
    const JUST_BEFORE_FIVE_SEC: u64 = 1758036604999999999;

    #[test]
    fn one_second_candle_window() {
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050379000000000);
        assert_eq!(end, 1758050379999999999);
    }

    #[test]
    fn one_second_exact_boundary() {
        // Exactly on the second boundary
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758024000999999999);
    }

    #[test]
    fn one_second_last_nanosecond() {
        // Last nanosecond of a second
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023999000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn five_second_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        // The 5-second window containing this instant is 19:19:35 to 19:19:39.999999999
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050375000000000);
        assert_eq!(end, 1758050379999999999);
    }

    #[test]
    fn five_second_exact_boundary() {
        // Exactly on a 5-second boundary (15:30:05)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(FIVE_SEC_BOUNDARY);
        assert_eq!(start, 1758036605000000000);
        assert_eq!(end, 1758036609999999999);
    }

    #[test]
    fn five_second_just_before_boundary() {
        // Just before a 5-second boundary (15:30:04.999999999)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(JUST_BEFORE_FIVE_SEC);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036604999999999);
    }

    #[test]
    fn five_second_at_minute_boundary() {
        // At exact minute boundary (should align to :00 to :04.999999999)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036604999999999);
    }

    #[test]
    fn one_minute_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050340000000000);
        assert_eq!(end, 1758050399999999999);
    }

    #[test]
    fn one_minute_exact_boundary() {
        // Exactly on minute boundary
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036659999999999);
    }

    #[test]
    fn one_minute_last_nanosecond() {
        // Last nanosecond before noon (11:59:59.999999999)
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023940000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn fifteen_minute_candle_window() {
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(TIME_2);
        assert_eq!(start, 1758039300000000000);
        assert_eq!(end, 1758040199999999999);
    }

    #[test]
    fn fifteen_minute_at_half_hour() {
        // 15:30 should be in the 15:30-15:44:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758037499999999999);
    }

    #[test]
    fn fifteen_minute_at_noon() {
        // Noon should be in the 12:00-12:14:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758024899999999999);
    }

    #[test]
    fn fifteen_minute_just_before_noon() {
        // 11:59:59.999999999 should be in the 11:45-11:59:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023100000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn one_hour_candle_window() {
        // Using TIME_2: 20250916T16:29:43.500Z
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(TIME_2);
        assert_eq!(start, 1758038400000000000);
        assert_eq!(end, 1758041999999999999);
    }

    #[test]
    fn one_hour_exact_boundary() {
        // Noon should be exactly at hour boundary
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758027599999999999);
    }

    #[test]
    fn one_hour_last_nanosecond_before() {
        // Last nanosecond before noon
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758020400000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn one_hour_at_midnight() {
        // Midnight UTC
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(MIDNIGHT_UTC);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1757984399999999999);
    }

    #[test]
    fn one_day_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(TIME_1);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_at_midnight() {
        // Exactly at midnight UTC
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(MIDNIGHT_UTC);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_end_of_day() {
        // Last nanosecond of the day
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(END_OF_DAY);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_at_noon() {
        // Noon should still be in the same day window
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(NOON);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn boundaries_are_inclusive_and_continuous() {
        // Verify that consecutive windows are continuous with no gaps
        let time = NOON;

        // Check that end of one second + 1 nanosecond = start of next second
        let (_, end1) = CandleWidth::OneSecond.to_nanosec_window(time);
        let (start2, _) = CandleWidth::OneSecond.to_nanosec_window(end1 + 1);
        assert_eq!(end1 + 1, start2);

        // Check that end of one minute + 1 nanosecond = start of next minute
        let (_, end1) = CandleWidth::OneMinute.to_nanosec_window(time);
        let (start2, _) = CandleWidth::OneMinute.to_nanosec_window(end1 + 1);
        assert_eq!(end1 + 1, start2);
    }

    #[test]
    fn window_widths_are_correct() {
        // Verify window widths are exactly what we expect
        let time = NOON;

        // One second = 1_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(time);
        assert_eq!(end - start + 1, 1_000_000_000);

        // Five seconds = 5_000_000_000 nanoseconds
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(time);
        assert_eq!(end - start + 1, 5_000_000_000);

        // One minute = 60_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(time);
        assert_eq!(end - start + 1, 60_000_000_000);

        // Fifteen minutes = 900_000_000_000 nanoseconds
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(time);
        assert_eq!(end - start + 1, 900_000_000_000);

        // One hour = 3_600_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(time);
        assert_eq!(end - start + 1, 3_600_000_000_000);

        // One day = 86_400_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(time);
        assert_eq!(end - start + 1, 86_400_000_000_000);
    }

    #[test]
    fn test_trading_schedule_serde_roundtrip() {
        let schedule = TradingSchedule {
            segments: vec![
                TradingHoursSegment {
                    days_of_week: DaysOfWeek::weekdays(),
                    time_of_day: TimeOfDay {
                        hours: 9,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 3600,
                    state: InstrumentState::PreOpen,
                    hide_market_data: false,
                    expire_all_orders: false,
                },
                TradingHoursSegment {
                    days_of_week: DaysOfWeek::weekdays(),
                    time_of_day: TimeOfDay {
                        hours: 10,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 21600,
                    state: InstrumentState::Open,
                    hide_market_data: false,
                    expire_all_orders: false,
                },
            ],
        };

        insta::assert_json_snapshot!(schedule, @r#"
        {
          "segments": [
            {
              "days_of_week": [
                1,
                2,
                3,
                4,
                5
              ],
              "time_of_day": {
                "hours": 9,
                "minutes": 30,
                "seconds": 0
              },
              "duration_seconds": 3600,
              "state": "PRE_OPEN",
              "hide_market_data": false,
              "expire_all_orders": false
            },
            {
              "days_of_week": [
                1,
                2,
                3,
                4,
                5
              ],
              "time_of_day": {
                "hours": 10,
                "minutes": 30,
                "seconds": 0
              },
              "duration_seconds": 21600,
              "state": "OPEN",
              "hide_market_data": false,
              "expire_all_orders": false
            }
          ]
        }
        "#);

        let json = serde_json::to_string(&schedule).unwrap();
        let deserialized: TradingSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.segments.len(), 2);
        assert_eq!(
            deserialized.segments[0].days_of_week,
            DaysOfWeek::weekdays()
        );
        assert_eq!(deserialized.segments[0].state, InstrumentState::PreOpen);
        assert_eq!(deserialized.segments[1].state, InstrumentState::Open);
    }

    #[test]
    fn test_trading_schedule_deserialization() {
        let json = r#"{
            "segments": [
                {
                    "days_of_week": [1, 2, 3, 4, 5],
                    "time_of_day": {"hours": 9, "minutes": 30, "seconds": 0},
                    "duration_seconds": 1800,
                    "state": "PRE_OPEN",
                    "hide_market_data": false,
                    "expire_all_orders": false
                },
                {
                    "days_of_week": [1, 2, 3, 4, 5],
                    "time_of_day": {"hours": 10, "minutes": 0, "seconds": 0},
                    "duration_seconds": 21600,
                    "state": "OPEN",
                    "hide_market_data": false,
                    "expire_all_orders": false
                }
            ]
        }"#;

        let schedule: TradingSchedule = serde_json::from_str(json).unwrap();

        assert_eq!(schedule.segments.len(), 2);

        let preopen = &schedule.segments[0];
        assert_eq!(preopen.days_of_week, DaysOfWeek::weekdays());
        assert_eq!(preopen.time_of_day.hours, 9);
        assert_eq!(preopen.time_of_day.minutes, 30);
        assert_eq!(preopen.duration_seconds, 1800);
        assert_eq!(preopen.state, InstrumentState::PreOpen);

        let open = &schedule.segments[1];
        assert_eq!(open.time_of_day.hours, 10);
        assert_eq!(open.time_of_day.minutes, 0);
        assert_eq!(open.duration_seconds, 21600);
        assert_eq!(open.state, InstrumentState::Open);
    }

    #[test]
    fn test_instrument_state_serialization() {
        assert_eq!(
            serde_json::to_string(&InstrumentState::ClosedFrozen).unwrap(),
            r#""CLOSED_FROZEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::PreOpen).unwrap(),
            r#""PRE_OPEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Open).unwrap(),
            r#""OPEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Closed).unwrap(),
            r#""CLOSED""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Delisted).unwrap(),
            r#""DELISTED""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Unknown).unwrap(),
            r#""UNKNOWN""#
        );
    }

    #[test]
    fn test_instrument_with_trading_schedule_serde_roundtrip() {
        let instrument = Instrument {
            symbol: "TEST-PERP".to_string(),
            expiration: None,
            multiplier: rust_decimal::Decimal::ONE,
            price_scale: 10000,
            minimum_order_size: rust_decimal::Decimal::ONE,
            tick_size: rust_decimal::Decimal::new(1, 4), // 0.0001
            quote_currency: "USD".to_string(),
            price_band_lower_deviation_pct: Some(rust_decimal::Decimal::new(-5, 0)),
            price_band_upper_deviation_pct: Some(rust_decimal::Decimal::new(5, 0)),
            funding_settlement_currency: "USD".to_string(),
            funding_rate_cap_upper_pct: Some(rust_decimal::Decimal::new(1, 0)),
            funding_rate_cap_lower_pct: Some(rust_decimal::Decimal::new(-1, 0)),
            maintenance_margin_pct: rust_decimal::Decimal::new(4, 0),
            initial_margin_pct: rust_decimal::Decimal::new(8, 0),
            category: InstrumentCategory::Fx,
            description: Some("Test Perpetual Future".to_string()),
            underlying_benchmark_price: None,
            contract_mark_price: None,
            contract_size: None,
            price_quotation: None,
            price_bands: None,
            funding_schedule_time_description: None,
            funding_schedule_calendar_description: None,
            funding_schedule: None,
            trading_schedule: Some(TradingSchedule {
                segments: vec![TradingHoursSegment {
                    days_of_week: DaysOfWeek::weekdays(),
                    time_of_day: TimeOfDay {
                        hours: 9,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 1800,
                    state: InstrumentState::PreOpen,
                    hide_market_data: false,
                    expire_all_orders: false,
                }],
            }),
            estimated_funding_supported: false,
            additional_product_specs: None,
        };

        insta::assert_json_snapshot!(instrument, @r#"
        {
          "symbol": "TEST-PERP",
          "expiration": null,
          "multiplier": "1",
          "price_scale": 10000,
          "minimum_order_size": "1",
          "tick_size": "0.0001",
          "quote_currency": "USD",
          "price_band_lower_deviation_pct": "-5",
          "price_band_upper_deviation_pct": "5",
          "funding_settlement_currency": "USD",
          "funding_rate_cap_upper_pct": "1",
          "funding_rate_cap_lower_pct": "-1",
          "maintenance_margin_pct": "4",
          "initial_margin_pct": "8",
          "category": "fx",
          "description": "Test Perpetual Future",
          "underlying_benchmark_price": null,
          "contract_mark_price": null,
          "contract_size": null,
          "price_quotation": null,
          "price_bands": null,
          "funding_schedule_time_description": null,
          "funding_schedule_calendar_description": null,
          "funding_schedule": null,
          "trading_schedule": {
            "segments": [
              {
                "days_of_week": [
                  1,
                  2,
                  3,
                  4,
                  5
                ],
                "time_of_day": {
                  "hours": 9,
                  "minutes": 30,
                  "seconds": 0
                },
                "duration_seconds": 1800,
                "state": "PRE_OPEN",
                "hide_market_data": false,
                "expire_all_orders": false
              }
            ]
          },
          "estimated_funding_supported": false,
          "additional_product_specs": null
        }
        "#);

        let json = serde_json::to_string(&instrument).unwrap();
        let deserialized: Instrument = serde_json::from_str(&json).unwrap();
        assert!(deserialized.trading_schedule.is_some());
        assert_eq!(deserialized.trading_schedule.unwrap().segments.len(), 1);
    }

    fn dec(s: &str) -> rust_decimal::Decimal {
        s.parse().expect("valid decimal")
    }

    #[test]
    fn price_scale_from_tick_size_uses_multiplier_semantics() {
        // `1×10⁻ⁿ` ticks: reduced multiplier == `10^decimals`.
        // The only tick shapes seen in live markets (verified 20/20:
        // JPYUSD 1e-6, equities 0.01, XAU 0.1).
        assert_eq!(price_scale_from_tick_size(dec("1")).unwrap(), 1);
        assert_eq!(price_scale_from_tick_size(dec("0.001")).unwrap(), 1000);
        assert_eq!(
            price_scale_from_tick_size(dec("0.000001")).unwrap(),
            1_000_000
        );

        // Integer ticks coarser than 1: still scale 1 (prices are integers).
        assert_eq!(price_scale_from_tick_size(dec("5")).unwrap(), 1);
        assert_eq!(price_scale_from_tick_size(dec("10")).unwrap(), 1);

        // Fractional-mantissa ticks: reduced form diverges from `10^decimals`
        // (0.5->2 not 10). None exist in prod; see the divergence test below.
        assert_eq!(price_scale_from_tick_size(dec("0.5")).unwrap(), 2);
        assert_eq!(price_scale_from_tick_size(dec("0.25")).unwrap(), 4);
    }

    #[test]
    fn price_scale_validation_rejects_non_positive_values() {
        assert!(price_scale_from_tick_size(dec("0")).is_err());
        assert!(price_scale_from_tick_size(dec("-0.01")).is_err());
        assert!(validate_price_scale("BAD-SCALE", dec("0.01"), 0).is_err());
        assert!(validate_price_scale("BAD-TICK", dec("0"), 100).is_err());
    }

    #[test]
    fn price_scale_validation_rejects_mismatched_scale() {
        // Live-prod shapes: validation passes (EURUSD 0.0001->10000, QQQ 0.01->100).
        validate_price_scale("EURUSD-PERP", dec("0.0001"), 10000).unwrap();
        validate_price_scale("QQQ-PERP", dec("0.01"), 100).unwrap();
        assert!(validate_price_scale("EURUSD-PERP", dec("0.0001"), 5).is_err());
    }

    #[test]
    fn price_scale_validation_diverges_for_fractional_ticks() {
        // Latent footgun: for a fractional-mantissa tick,
        // the likely `10^decimals` price_scale (0.05 -> 100)
        // ≠ the reduced multiplier (20), so validation rejects it.
        // None exist in prod; strict=true fails fast,
        // strict=false logs and keeps it.
        // Pinned so the divergence is a deliberate choice.
        validate_price_scale("FRACTIONAL-PERP", dec("0.05"), 20).unwrap();
        assert!(validate_price_scale("FRACTIONAL-PERP", dec("0.05"), 100).is_err());
    }
}
