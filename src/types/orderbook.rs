use crate::protocol::marketdata_publisher::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

pub struct Orderbook {
    pub bids: BTreeMap<Decimal, OrderbookLevel>,
    pub asks: BTreeMap<Decimal, OrderbookLevel>,
}

pub struct OrderbookLevel {
    pub quantity: i32,
    pub order_quantities: Option<Vec<i32>>, // for LEVEL_3
}

impl From<&L2BookUpdate> for Orderbook {
    fn from(u: &L2BookUpdate) -> Self {
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
