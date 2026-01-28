use crate::protocol::marketdata_publisher::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

pub struct Orderbook {
    pub bids: BTreeMap<Decimal, OrderbookLevel>,
    pub asks: BTreeMap<Decimal, OrderbookLevel>,
}

pub struct OrderbookLevel {
    pub quantity: u64,
    pub order_quantities: Option<Vec<u64>>, // for LEVEL_3
}

impl<Snapshot> From<&BookUpdateData<Snapshot>> for Orderbook {
    fn from(u: &BookUpdateData<Snapshot>) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        for l in &u.bids {
            bids.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: None,
                },
            );
        }
        for l in &u.asks {
            asks.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: None,
                },
            );
        }
        Self { bids, asks }
    }
}

impl From<&L3BookUpdate> for Orderbook {
    fn from(u: &L3BookUpdate) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        for l in &u.bids {
            bids.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: Some(l.order_quantities.clone()),
                },
            );
        }
        for l in &u.asks {
            asks.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: Some(l.order_quantities.clone()),
                },
            );
        }
        Self { bids, asks }
    }
}
