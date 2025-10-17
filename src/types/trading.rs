//! Business Logic Types
//!
//! This module contains core business types for trading operations.

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentV0 {
    pub symbol: String,
    pub tick_size: Decimal,
    pub base_currency: String,
    pub multiplier: i32,
    pub minimum_trade_quantity: i32,
    pub description: String,
    pub product_id: String,
    pub state: String,
    pub price_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Instrument {
    pub symbol: String,
    pub state: InstrumentState,
    // Programmatic specification fields
    pub multiplier: Decimal,
    pub minimum_order_size: Decimal,
    pub tick_size: Decimal,
    pub quote_currency: String,
    pub price_band_lower_deviation_pct: Option<Decimal>,
    pub price_band_upper_deviation_pct: Option<Decimal>,
    pub finding_settlement_currency: String,
    pub funding_rate_cap_upper_pct: Option<Decimal>,
    pub funding_rate_cap_lower_pct: Option<Decimal>,
    pub maintenance_margin_pct: Decimal,
    pub initial_margin_pct: Decimal,
    // English language specification fields
    pub description: Option<String>,
    pub underlying_benchmark_price: Option<String>,
    pub contract_mark_price: Option<String>,
    pub contract_size: Option<String>,
    pub price_quotation: Option<String>,
    pub price_bands: Option<String>,
    pub funding_frequency: Option<String>,
    pub funding_calendar_schedule: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentState {
    /// The instrument is available to place orders and modify them
    /// before the opening, but no matching will occur until the open.
    ///
    /// At the open, crossing orders will be matched via Dutch auction.
    PreOpen,
    /// The instrument is open and is available for full trading.
    Open,
    /// The instrument has temporarily suspended trading. In this state,
    /// no orders can be placed or modified, but they can be cancelled.
    Suspended,
    /// The instrument has been delisted.  This state is terminal.
    Delisted,
    /// The instrument status is unknown.
    #[default]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PlaceOrder {
    pub symbol: String,
    pub side: Side,
    pub quantity: i32,
    pub price: Decimal,
    pub time_in_force: String,
    pub post_only: bool,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: String,
    pub user_id: Uuid,
    pub symbol: String,
    pub side: Side,
    pub quantity: i32,
    pub price: Decimal,
    pub time_in_force: String,
    pub tag: Option<String>,
    /// Timestamp when the order was received by the order gateway
    pub timestamp: DateTime<Utc>,
    pub order_state: OrderState,
    pub filled_quantity: i32,
    pub remaining_quantity: i32,
    /// Timestamp when the order state became terminal
    pub completion_time: Option<DateTime<Utc>>,
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
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl OrderState {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn is_open(&self) -> bool {
        match self {
            Self::Accepted | Self::PartiallyFilled => true,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
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

    pub fn from_char(s: &'static str) -> Result<Self> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub currency: String,
    pub available: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub average_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub mark_price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub fill_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
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
