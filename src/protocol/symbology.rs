use crate::websocket_rpc;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

websocket_rpc!(InstrumentsRequest, "instruments");

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstrumentsRequest {
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstrumentsResponse {
    pub instruments: Vec<Instrument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Instrument {
    pub symbol: String,
    pub multiplier: Decimal,
    pub tick_size: Decimal,
    pub min_order_quantity: Decimal,
}
