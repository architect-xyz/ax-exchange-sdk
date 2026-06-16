//! Pagination params and responses for API endpoints.
//!
//! Provides three modes of pagination:
//!
//! - Limit/offset: `LimitOffsetPagination` returning `LimitOffsetPage`
//! - Cursor: `CursorPagination` returning `CursorPage`
//! - Timeseries: `TimeseriesPagination` returning `TimeseriesPage`

use crate::protocol::{sort::SortDirection, time_range::TimeRangeNs};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};

pub const DEFAULT_PAGE_SIZE: u32 = 100;

/// Simple limit/offset paging.
///
/// Set `limit` for page size and `offset` to skip results. Both are
/// optional and default to `DEFAULT_PAGE_SIZE` and 0.
#[serde_as]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", into_params(parameter_in = Query))]
pub struct LimitOffsetPagination {
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub limit: Option<u32>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub offset: Option<u32>,
}

impl LimitOffsetPagination {
    pub fn resolve(&self) -> (u32, u32) {
        (
            self.limit.unwrap_or(DEFAULT_PAGE_SIZE),
            self.offset.unwrap_or(0),
        )
    }
}

/// Page metadata for limit/offset paged responses.
///
/// This is intended for response bodies (not query params).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LimitOffsetPage {
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

/// Cursor-based paging.
///
/// Pass `cursor` from a previous response to get the next page. Set `limit` to control
/// page size (accepts number or string).
#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", into_params(parameter_in = Query))]
pub struct CursorPagination {
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

/// Page metadata for cursor-paged responses.
///
/// This is intended for response bodies (not query params).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CursorPage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,
}

/// Alias of [`CursorPage`] for timeseries responses (same JSON shape).
pub type TimeseriesPage = CursorPage;

/// Query timeseries data with time range, sort, and paging.
///
/// Combines time filtering, sort direction, and cursor paging. Set time bounds and
/// control result order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", into_params(parameter_in = Query))]
pub struct TimeseriesPagination {
    #[serde(flatten)]
    pub range: TimeRangeNs,
    /// Timestamp sort direction (defaults to `desc`).
    pub sort_ts: Option<SortDirection>,
    #[serde(flatten)]
    pub pagination: CursorPagination,
}

impl TimeseriesPagination {
    pub fn validate(&self) -> Result<()> {
        self.range.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- limit/offset tests --

    #[derive(Debug, Deserialize)]
    struct LimitOffsetQuery {
        #[serde(flatten)]
        pagination: LimitOffsetPagination,
    }

    #[test]
    fn limit_offset_query_params_decode_as_numbers() {
        let parsed: LimitOffsetQuery = serde_urlencoded::from_str("limit=10&offset=20").unwrap();
        assert_eq!(parsed.pagination.limit, Some(10));
        assert_eq!(parsed.pagination.offset, Some(20));
    }

    #[test]
    fn limit_offset_query_params_empty_is_none() {
        let parsed: LimitOffsetQuery = serde_urlencoded::from_str("").unwrap();
        assert_eq!(parsed.pagination.limit, None);
        assert_eq!(parsed.pagination.offset, None);
    }

    #[test]
    fn limit_offset_query_params_invalid_number_errors() {
        let err = serde_urlencoded::from_str::<LimitOffsetQuery>("limit=abc").unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn limit_offset_json_accepts_number_or_string() {
        let from_number: LimitOffsetPagination =
            serde_json::from_str(r#"{ "limit": 10 }"#).unwrap();
        let from_string: LimitOffsetPagination =
            serde_json::from_str(r#"{ "limit": "10" }"#).unwrap();
        assert_eq!(from_number.limit, Some(10));
        assert_eq!(from_string.limit, Some(10));
    }

    #[test]
    fn limit_offset_json_invalid_string_errors() {
        let err =
            serde_json::from_str::<LimitOffsetPagination>(r#"{ "limit": "abc" }"#).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn resolve_defaults() {
        let p = LimitOffsetPagination::default();
        assert_eq!(p.resolve(), (DEFAULT_PAGE_SIZE, 0));
    }

    #[test]
    fn resolve_explicit_values() {
        let p = LimitOffsetPagination {
            limit: Some(25),
            offset: Some(50),
        };
        assert_eq!(p.resolve(), (25, 50));
    }

    // -- cursor tests --

    #[derive(Debug, Deserialize)]
    struct CursorQuery {
        #[serde(flatten)]
        pagination: CursorPagination,
    }

    #[test]
    fn cursor_query_params_decode_limit_and_cursor() {
        let parsed: CursorQuery = serde_urlencoded::from_str("limit=10&cursor=abc").unwrap();
        assert_eq!(parsed.pagination.limit, Some(10));
        assert_eq!(parsed.pagination.cursor.as_deref(), Some("abc"));
    }

    #[test]
    fn cursor_query_params_empty_is_none() {
        let parsed: CursorQuery = serde_urlencoded::from_str("").unwrap();
        assert_eq!(parsed.pagination.limit, None);
        assert_eq!(parsed.pagination.cursor, None);
    }

    #[test]
    fn cursor_json_accepts_number_or_string() {
        let from_number: CursorPagination = serde_json::from_str(r#"{ "limit": 10 }"#).unwrap();
        let from_string: CursorPagination = serde_json::from_str(r#"{ "limit": "10" }"#).unwrap();
        assert_eq!(from_number.limit, Some(10));
        assert_eq!(from_string.limit, Some(10));
    }
}
