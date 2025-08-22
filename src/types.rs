use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Instrument {
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

#[derive(Debug, Clone)]
pub struct PlaceOrder {
    pub symbol: String,
    pub side: String,
    pub quantity: i32,
    pub price: Decimal,
    pub time_in_force: String,
    pub post_only: bool,
    pub tag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: String,
    pub username: String,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: i32,
    pub filled_quantity: i32,
    pub remaining_quantity: i32,
    pub order_state: String,
    pub side: String,
    pub time_in_force: String,
    pub timestamp: DateTime<Utc>,
    pub tag: Option<String>,
}

// REST API Types for Order Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOrderRequest {
    pub username: String,
    pub symbol: String,
    pub price: String,
    pub quantity: i64,
    pub side: String,
    pub time_in_force: String,
    pub post_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOrderResponse {
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    pub username: String,
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersResponse {
    pub orders: Vec<RestOrderMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllResponse {
    pub successful_cancellations: Vec<String>,
    pub failed_cancellations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestOrderMessage {
    pub order_id: String,
    pub username: String,
    pub symbol: String,
    pub price: String,
    pub quantity: i64,
    pub executed_quantity: i64,
    pub traded_quantity: i64, // Alias for executed_quantity
    pub average_executed_price: Option<String>,
    pub remaining_quantity: i64,
    pub state: String,
    pub side: String,
    pub time_in_force: String,
    pub insert_time: String,
    pub insert_epoch_seconds: i64,
    pub insert_epoch_nanos: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

// Account Gateway Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub symbol: String,
    pub balance: String,
    pub locked_balance: String,
    pub realized_pnl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub position: i64,
    pub maintenance_margin: String,
    pub buy_quantity: i64,
    pub sell_quantity: i64,
    pub buy_notional: String,
    pub sell_notional: String,
    pub realized_pnl: String,
    pub unrealized_pnl: String,
    pub mark_price: String,
}

// Common pagination and filtering types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryParams {
    pub pagination: Option<PaginationParams>,
    pub date_range: Option<DateRangeParams>,
    pub filters: Option<HashMap<String, String>>,
}

// Standard response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub total: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// Account Gateway extended types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub user_id: i32,
    pub username: String,
    pub is_onboarded: bool,
    pub risk_score: String,
    pub compliance_risk_approved: bool,
    pub is_frozen: bool,
    pub is_close_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterest {
    pub symbol: String,
    pub open_interest: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestData {
    pub total_open_interest: Decimal,
    pub by_symbol: HashMap<String, Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub execution_id: String,
    pub market: String,
    pub timestamp: String,
    pub price: Decimal,
    pub quantity: i32,
    pub side: String,
    pub aggressor: bool,
    pub commission: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistory {
    pub symbol: String,
    pub funding_amount: Decimal,
    pub net_position: i32,
    pub timestamp: DateTime<Utc>,
    pub funding_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRequest {
    pub username: String,
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositResponse {
    pub deposit_id: String,
    pub status: String,
    pub expected_confirmation_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawRequest {
    pub username: String,
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawResponse {
    pub withdrawal_id: String,
    pub status: String,
    pub expected_confirmation_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidateRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidateResponse {
    pub successful_cancellations: Vec<String>,
    pub failed_cancellations: Vec<String>,
    pub successful_liquidations: Vec<String>,
    pub failed_liquidations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminResponse {
    pub deposits: Decimal,
    pub withdrawals: Decimal,
    pub commissions: Decimal,
    pub trading_volume: Decimal,
    pub deposits_count: i32,
    pub withdrawals_count: i32,
    pub users: Vec<String>,
    pub open_interest: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingVolumeStats {
    pub username: String,
    pub total_volume: Decimal,
    pub volume_by_symbol: HashMap<String, Decimal>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositStats {
    pub total_deposits: Decimal,
    pub deposits_count: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalStats {
    pub total_withdrawals: Decimal,
    pub withdrawals_count: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStats {
    pub total_users: u64,
    pub active_users: u64,
    pub total_volume: Decimal,
    pub total_deposits: Decimal,
    pub total_withdrawals: Decimal,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

// Authentication Gateway extended types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub api_key: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeApiKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetApiKeysRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetApiKeysResponse {
    pub api_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeApiKeyResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    pub valid: bool,
    pub username: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub message: String,
}

// Risk Manager types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSnapshot {
    pub username: String,
    pub timestamp: DateTime<Utc>,
    pub total_exposure: Decimal,
    pub margin_requirement: Decimal,
    pub available_margin: Decimal,
    pub positions: Vec<RiskPosition>,
    pub risk_metrics: RiskMetrics,
}

// Python Risk Manager types (actual response format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRiskSnapshot {
    pub timestamp: String, // Python service returns timestamp as string
    pub username: String,
    pub positions: Vec<PythonRiskPosition>,
    pub cash_balance: Decimal,
    pub total_realized_pnl: Decimal,
    pub total_unrealized_pnl: Decimal,
    pub total_initial_margin: Decimal,
    pub total_maintenance_margin: Decimal,
    pub total_equity: Decimal,
    pub available_initial_margin: Decimal,
    pub available_maintenance_margin: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskPosition {
    pub symbol: String,
    pub position: i64,
    pub notional_value: Decimal,
    pub margin_requirement: Decimal,
    pub unrealized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRiskPosition {
    pub symbol: String,
    pub net_quantity: Decimal,
    pub buy_quantity: Decimal,
    pub sell_quantity: Decimal,
    pub buy_notional: Decimal,
    pub sell_notional: Decimal,
    pub buy_vwap: Decimal,
    pub sell_vwap: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub initial_margin: Decimal,
    pub maintenance_margin: Decimal,
    pub mark_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub var_95: Decimal,
    pub leverage: Decimal,
    pub margin_ratio: Decimal,
    pub risk_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestScenario {
    pub name: String,
    pub price_shocks: HashMap<String, Decimal>, // symbol -> price shock percentage
    pub volatility_multiplier: Option<Decimal>,
}

// Liquidation types (matching Python service)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidation {
    pub symbol: String,
    pub quantity: Decimal,
    pub price: Decimal, // Python service returns price, not side/reason
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationSummary {
    pub username: String,
    pub liquidations: Vec<Liquidation>,
    pub total_initial_margin_pre_liquidations: Decimal,
    pub total_initial_margin_post_liquidations: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    pub risk_snapshot: PythonRiskSnapshot,
    pub liquidation_summary: LiquidationSummary,
}



// Order History types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHistoryFilters {
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub status: Option<String>,
    pub order_type: Option<String>,
    pub date_range: Option<DateRangeParams>,
    pub pagination: Option<PaginationParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalOrder {
    #[serde(rename = "oid")]
    pub order_id: String,
    #[serde(rename = "u")]
    pub username: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "xq")]
    pub executed_quantity: i32,
    #[serde(rename = "xp")]
    pub average_executed_price: Option<String>,
    #[serde(rename = "rq")]
    pub remaining_quantity: i32,
    #[serde(rename = "o")]
    pub state: String,
    #[serde(rename = "d")]
    pub side: String,
    #[serde(rename = "tif")]
    pub time_in_force: String,
    #[serde(rename = "ts")]
    pub insert_epoch_seconds: i64,
    #[serde(rename = "tn")]
    pub insert_epoch_nanos: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub orders: Vec<HistoricalOrder>,
    pub total: u64,
    pub limit: usize,
    pub offset: usize,
}

// Settlement Gateway types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementStatus {
    pub status: String,
    pub last_settlement: Option<DateTime<Utc>>,
    pub next_settlement: Option<DateTime<Utc>>,
    pub active_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub settlement_id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub total_trades: u64,
    pub settlement_price: HashMap<String, Decimal>, // symbol -> settlement price
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementConfig {
    pub settlement_frequency: String, // e.g., "daily", "weekly"
    pub settlement_time: String, // e.g., "16:00:00"
    pub enabled_symbols: Vec<String>,
    pub risk_parameters: HashMap<String, String>,
}

// Candle Server types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
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
