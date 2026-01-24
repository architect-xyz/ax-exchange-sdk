//! API Protocol Types
//!
//! This module contains types used for API protocol communication (over-the-wire).

pub const DEFAULT_PAGINATION_LIMIT: u32 = 100;

use crate::types::trading::Side;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Pagination parameters for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: T,
    pub metadata: Option<PaginationMetadata>,
}

// TODO: this struct is incoherent
/// Response metadata for pagination and counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub total_count: Option<u64>,
    pub total: Option<u64>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// TODO: rename to HistoricalRangeParams and delete HistoryParams
// and HistoryResponse as a bad abstraction; instead we ought to
// have individual requests compose DateRangeParams and
// PaginationParams as appropriate.
/// Date range parameters for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// History query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryParams {
    pub symbol: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub pagination: Option<PaginationParams>,
    pub date_range: Option<DateRangeParams>,
    pub filters: Option<HashMap<String, String>>,
}

/// Generic history response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
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
    pub quantity: i64,
    pub abs_quantity: u64,
    pub is_taker: bool,
    pub fee: Decimal,
    pub side: Side,
}
