//! API Protocol Types
//!
//! This module contains types used for API protocol communication (over-the-wire).

use crate::types::trading::Side;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Timestamp {
    pub ts: i32,
    pub tn: u32,
}

impl Timestamp {
    pub fn now() -> Self {
        let now = Utc::now();
        now.into()
    }

    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.ts as i64, self.tn)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self {
            ts: value.timestamp() as i32,
            tn: value.timestamp_subsec_nanos(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Fill {
    pub trade_id: String,
    pub order_id: Option<String>,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: u64,
    pub is_taker: bool,
    pub fee: Decimal,
    pub side: Side,
    pub realized_pnl: Option<Decimal>,
}
