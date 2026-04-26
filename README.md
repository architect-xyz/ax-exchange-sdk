<div align="center">

# 🏛️ ax-exchange-sdk

**The (un)official Rust SDK for the [ArchitectX](https://architect.exchange) derivatives exchange.**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](https://opensource.org/licenses/AGPL-3.0)
[![Rust Edition](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/)
[![Version](https://img.shields.io/badge/version-13.35.0-green.svg)](Cargo.toml)
[![Docs](https://img.shields.io/badge/docs-docs.architect.exchange-informational)](https://docs.architect.exchange/api-reference)

</div>

---

A full-featured, async-first Rust SDK for the ArchitectX perpetuals exchange. Provides strongly-typed REST and WebSocket clients for market data streaming, order management, and account operations — with automatic token refresh, exponential-backoff reconnection, and subscription replay built in.

---

## ✨ Feature Highlights

| Feature | Details |
|---|---|
| 🔐 **Authentication** | API key/secret, username+password, optional TOTP/2FA |
| 📡 **Market Data WebSocket** | L1/L2/L3 order book streaming, trades, tickers, OHLCV & BBO candles |
| 📋 **Order Gateway WebSocket** | Place, cancel, replace, bulk-cancel, open order queries, real-time events |
| 🔄 **Auto-Reconnect** | Exponential backoff with subscription replay on reconnect |
| 🪙 **Token Management** | Automatic bearer token caching and silent refresh |
| 📊 **REST API** | Full API Gateway and Order Gateway REST clients |
| 📈 **Risk & Positions** | Risk snapshots, margin calculations, PnL, positions |
| 🧾 **Audit Trail** | Fills, transactions, funding rates, equity history |
| 💡 **Type Safety** | Rich domain types for orders, instruments, candles, and more |
| ⚙️ **Optional Schemas** | `utoipa` and `schemars` feature flags for OpenAPI/JSON Schema generation |

---

## 🚀 Quick Start

```toml
[dependencies]
ax-exchange-sdk = { git = "https://github.com/architect-xyz/adx.git" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

```rust
// examples/create_client.rs
use anyhow::Result;
use ax_exchange_sdk::{environment::Environment, ArchitectX};

#[tokio::main]
async fn main() -> Result<()> {
    let client = ArchitectX::new(
        Environment::Sandbox,
        Some("your-api-key"),
        Some("your-api-secret"),
    )?;

    let api = client.api_gateway()?;
    let instruments = api.get_instruments().await?;
    println!("{:?}", instruments);

    Ok(())
}

```

---

## 📡 Streaming Market Data

```rust
// examples/price_streaming.rs
use anyhow::Result;
use ax_exchange_sdk::{
    environment::Environment, protocol::marketdata_publisher::SubscriptionLevel,
    types::ws::ConnectionState, ArchitectX,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("AX_API_KEY")?;
    let api_secret = env::var("AX_API_SECRET")?;
    let environment: Environment = env::var("AX_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .parse()?;

    tracing_subscriber::fmt::init();

    let client = ArchitectX::new(environment, Some(api_key), Some(api_secret))?;

    let api = client.api_gateway()?;
    println!("Fetching instruments...");
    let instruments = api.get_instruments().await?;
    println!("Collected a total of {}", instruments.instruments.len());

    let mut market_ws = client.marketdata_ws().await?;

    for instrument in instruments.instruments {
        market_ws
            .subscribe(instrument.0.symbol.clone(), SubscriptionLevel::Level1)
            .await?;
    }

    let mut watcher = market_ws.state_watcher();

    loop {
        tokio::select! {
            msg = market_ws.market_data_receiver.recv() => {
                match msg {
                    Some(event) => println!("Received market data event: {:?}", event),
                    None => {
                        println!("Market data stream closed.");
                        break;
                    }
                }
            }
            state = watcher.run_till_event() => {
                println!("Connection state changed: {:?}", state);
                match state {
                    ConnectionState::Exited => break,
                    ConnectionState::Disconnected => {
                        println!("Connection lost, waiting for reconnect...");
                    }
                    ConnectionState::Connected => {
                        println!("Reconnected.");
                    }
                }
            }
        }
    }
    Ok(())
}

```


---

## 📋 Order Management

```rust
// examples/order_lifecycle.rs
use anyhow::Result;
use ax_exchange_sdk::{
    environment::Environment,
    protocol::order_gateway::OrderGatewayEvent,
    trading::TimeInForce,
    types::{trading::Side, ws::ConnectionState, PlaceOrder},
    ArchitectX,
};
use rust_decimal::Decimal;
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("AX_API_KEY")?;
    let api_secret = env::var("AX_API_SECRET")?;
    let environment: Environment = env::var("AX_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .parse()?;

    tracing_subscriber::fmt::init();

    let client = ArchitectX::new(environment, Some(api_key), Some(api_secret))?;

    let mut order_ws = client.order_gateway_ws().await?;

    println!("Waiting for connection...");
    order_ws.wait_for_connection().await;
    println!("Connected to order gateway.");

    let open_orders = order_ws.get_open_orders().await?;
    println!("Currently have {} open orders.", open_orders.orders.len());

    let symbol = "XAU-PERP";
    // we now get the market price of XAU such that we can place a resting order at a reasonable price level
    let md_api = client.api_gateway()?;
    let xau_instrument = md_api
        .get_tickers()
        .await?
        .tickers
        .into_iter()
        .find(|t| t.symbol == symbol)
        .expect("XAU-PERP ticker not found");
    println!("Current XAU price: {:?}", xau_instrument.bid_price);

    let side = Side::Buy;
    let quantity = 1;
    let price =
        xau_instrument.bid_price.expect("No Price for symbol!") - Decimal::from_str("10.0")?; // place a resting order $10 below the current price

    let place_order = PlaceOrder {
        symbol: symbol.to_string(),
        side,
        quantity,
        price,
        time_in_force: TimeInForce::GoodTillCanceled, // ensure the order stays open until we cancel it
        post_only: true, // ensure the order rests and doesn't take liquidity
        tag: Some("test_order".to_string()),
        clord_id: None,
    };

    let res = order_ws.place_order(place_order).await?;

    println!("Placed order: {:?}", res);

    // we now cancel the order immediately to trigger a cancel event as well
    let _cancel_res = order_ws.cancel_order(&res.order_id).await?;

    let mut watcher = order_ws.state_watcher();

    println!("Streaming order events (Ctrl-C to exit)...");
    loop {
        tokio::select! {
            msg = order_ws.event_receiver.recv() => {
                match msg {
                    Some(event) => print_event(&event),
                    None => {
                        println!("Order event stream closed.");
                        break;
                    }
                }
            }
            state = watcher.run_till_event() => {
                println!("[connection] state → {state:?}");
                match state {
                    ConnectionState::Exited => break,
                    ConnectionState::Disconnected => {
                        println!("[connection] lost — waiting for reconnect...");
                    }
                    ConnectionState::Connected => {
                        println!("[connection] reconnected.");
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_event(event: &OrderGatewayEvent) {
    match event {
        OrderGatewayEvent::Heartbeat(ts) => {
            println!("[heartbeat] {ts:?}");
        }
        OrderGatewayEvent::OrderAcked(e) => {
            println!(
                "[acked]     order_id={} symbol={} side={:?} qty={} price={}",
                e.order.order_id, e.order.symbol, e.order.side, e.order.quantity, e.order.price
            );
        }
        OrderGatewayEvent::OrderRejected(e) => {
            println!(
                "[rejected]  order_id={} reason={:?}",
                e.order.order_id, e.reject_reason
            );
        }
        OrderGatewayEvent::OrderCanceled(e) => {
            println!("[canceled]  order_id={}", e.order.order_id);
        }
        OrderGatewayEvent::CancelRejected(e) => {
            println!("[cancel_rejected] order_id={}", e.order_id);
        }
        OrderGatewayEvent::OrderPartiallyFilled(e) => {
            println!(
                "[partial]   order_id={} filled={} remaining={}",
                e.order.order_id, e.order.filled_quantity, e.order.remaining_quantity
            );
        }
        OrderGatewayEvent::OrderFilled(e) => {
            println!(
                "[filled]    order_id={} qty={}",
                e.order.order_id, e.order.quantity
            );
        }
        OrderGatewayEvent::OrderReplacedOrAmended(e) => {
            println!(
                "[replaced]  old_order_id={} new_order_id={:?}",
                e.replaced_order.order_id, e.replacement_order_id
            );
        }
        OrderGatewayEvent::OrderExpired(e) => {
            println!("[expired]   order_id={}", e.order.order_id);
        }
        OrderGatewayEvent::OrderDoneForDay(e) => {
            println!("[done_for_day] order_id={}", e.order.order_id);
        }
    }
}

```

---

## 🔐 Authentication

The SDK supports three authentication methods via `ArchitectX`:

```
// API key + secret (recommended for automated systems)
client.authenticate("api-key", "api-secret").await?;

// Username + password (with optional TOTP for 2FA-enabled accounts)
client.login("username", "password", Some("123456")).await?;

// Tokens are cached and refreshed automatically — call refresh_user_token
// to force an early refresh:
client.refresh_user_token(true).await?;
```

### 2FA / TOTP Management

```
let api = client.api_gateway()?;

// Set up 2FA — returns a TOTP provisioning URI for QR code scanning
let setup = api.setup_2fa().await?;
println!("Scan this URI: {}", setup.uri);

// Confirm with a TOTP code
api.confirm_2fa(Confirm2faRequest {
    validate_token: setup.validate_token,
    code: "123456".to_string(),
}).await?;

// Disable 2FA
api.disable_2fa().await?;
```

---

## 📊 REST API Reference

### API Gateway — Public Endpoints

| Method | Description |
|---|---|
| `health()` | Exchange health check |
| `get_instruments()` | All listed instruments |
| `get_instrument(symbol)` | Single instrument details |
| `get_tickers()` | All ticker snapshots |
| `get_book(symbol, level)` | Orderbook snapshot (Level 2 or 3) |
| `get_trades(symbol, limit)` | Recent public trades |
| `get_candles(symbol, start, end, width)` | Historical OHLCV candles |
| `get_bbo_candles(symbol, start, end, width)` | Historical BBO (bid/ask/mid) candles |

### API Gateway — Authenticated Endpoints

| Method | Description |
|---|---|
| `whoami()` | Current user profile, fees, accounts |
| `get_balances()` | Account balances |
| `get_positions()` | Open perpetual positions |
| `get_fills()` | Trade fill history |
| `get_transactions(types)` | Transaction ledger (paginated) |
| `get_risk_snapshot()` | Full margin/PnL risk snapshot |
| `get_funding_rates(symbol, start, end)` | Historical funding rates |
| `get_account_equity_history(start, end, resolution)` | Equity curve history |
| `create_api_key(...)` | Create a new API key |
| `get_api_keys()` | List all API keys |
| `revoke_api_key(key)` | Revoke an API key |
| `change_password(...)` | Change account password |
| `sandbox_deposit(symbol, amount)` | Add sandbox funds |
| `sandbox_withdrawal(symbol, amount)` | Remove sandbox funds |

### Order Gateway — REST

| Method | Description |
|---|---|
| `open_orders()` | List all open orders |
| `order_status(id)` | Get status by order ID or client order ID |
| `place_order(order)` | Place a new limit order |
| `cancel_order(id)` | Cancel a single order |
| `replace_order(req)` | Atomically cancel + replace an order |
| `cancel_all_orders(symbol?)` | Cancel all orders, optionally filtered by symbol |
| `order_fills(id)` | Get all fills for an order |

---

## 🔄 Advanced WebSocket Features

### Cancel-on-Disconnect

Protect against stale orders if your process crashes or disconnects:

```
// All orders placed on this connection are automatically cancelled
// by the exchange if the WebSocket disconnects.
let mut order_ws = client.order_gateway_ws_with_cancel_on_disconnect().await?;
```

### Order Replace (Amend)

Atomically cancel an existing order and place a new one — no risk of double-fill:

```
use ax_exchange_sdk::protocol::order_gateway::ReplaceOrderRequest;
use rust_decimal::Decimal;
use std::str::FromStr;

let new_order_id = order_ws.replace_order(ReplaceOrderRequest {
    order_id: existing_order_id.clone(),
    symbol: "XAU-PERP".to_string(),
    side: Side::Buy,
    quantity: 1,
    price: Decimal::from_str("1950.00")?,
    time_in_force: TimeInForce::GoodTillCanceled,
    post_only: true,
    tag: None,
    clord_id: None,
}).await?;
```

### Candle Streaming (OHLCV + BBO)

Subscribe to live trade candles or BBO (bid/ask/mid) candles at any width:

```
use ax_exchange_sdk::types::trading::CandleWidth;

// Subscribe to 1-minute trade candles
market_ws.subscribe_candles("XAU-PERP", CandleWidth::OneMinute).await?;

// Subscribe to 5-minute BBO (bid/ask/mid-price) candles
market_ws.subscribe_bbo_candles("XAU-PERP", CandleWidth::FiveMinute).await?;

// Unsubscribe
market_ws.unsubscribe_candles("XAU-PERP", CandleWidth::OneMinute).await?;
market_ws.unsubscribe_bbo_candles("XAU-PERP", CandleWidth::FiveMinute).await?;
```

Available candle widths: `1s`, `5s`, `1m`, `5m`, `15m`, `1h`, `1d`.

### L3 Orderbook Streaming (Individual Orders)

Subscribe at Level 3 to see individual order quantities at each price level:

```
use ax_exchange_sdk::protocol::marketdata_publisher::SubscriptionLevel;

// Level 1: top-of-book only
market_ws.subscribe("XAU-PERP", SubscriptionLevel::Level1).await?;

// Level 2: full book with aggregated quantities per level
market_ws.subscribe("XAU-PERP", SubscriptionLevel::Level2).await?;

// Level 3: full book with individual order quantities per level
market_ws.subscribe("XAU-PERP", SubscriptionLevel::Level3).await?;
```

The `Orderbook` type can be built directly from L2 or L3 book update events:

```
use ax_exchange_sdk::Orderbook;
use ax_exchange_sdk::protocol::marketdata_publisher::MarketdataEvent;

match event {
    MarketdataEvent::L2BookUpdate(update) => {
        let book = Orderbook::from(&update);
        // book.bids / book.asks are BTreeMaps keyed by price
    }
    MarketdataEvent::L3BookUpdate(update) => {
        let book = Orderbook::from(&update);
        // book.asks[price].order_quantities contains per-order sizes
    }
    _ => {}
}
```

### Subscription Replay on Reconnect

All active market data subscriptions are automatically replayed after a reconnect — no manual re-subscription needed. The internal `connection_supervisor` replays every tracked subscription at each new connection before resuming the event loop.

### Bulk Cancel

Cancel all orders at once, or narrow to a single symbol:

```
// Cancel everything
order_ws.cancel_all_orders(None).await?;

// Cancel only XAU-PERP orders
order_ws.cancel_all_orders(Some("XAU-PERP")).await?;
```

---

## 💡 Risk & Account Snapshot

```
let api = client.api_gateway()?;

let snapshot = api.get_risk_snapshot().await?;
let risk = &snapshot.risk_snapshot;

println!("Equity:                      {}", risk.equity);
println!("Balance (USD):               {}", risk.balance_usd);
println!("Unrealized PnL:              {}", risk.unrealized_pnl);
println!("Initial margin (total):      {}", risk.initial_margin_required_total);
println!("Maintenance margin avail:    {}", risk.maintenance_margin_available);

for (symbol, sym_risk) in &risk.per_symbol {
    println!("{symbol}: qty={} notional={} liq_price={:?}",
        sym_risk.signed_quantity,
        sym_risk.signed_notional,
        sym_risk.liquidation_price,
    );
}
```

---

## 🗂️ Order State Machine

Orders transition through the following states:

```
PENDING → ACCEPTED → PARTIALLY_FILLED → FILLED
       ↘          ↘                  ↘
        REJECTED    CANCELED           CANCELED
                    EXPIRED            EXPIRED
                    REPLACED           REPLACED
                    DONE_FOR_DAY       DONE_FOR_DAY
```

Helper methods on `OrderState`:
- `is_open()` — `true` for `Accepted` or `PartiallyFilled`
- `is_terminal()` — `true` for `Filled`, `Canceled`, `Rejected`, `Expired`, `Replaced`, `DoneForDay`
- `can_be_canceled()` — `true` if the order can still be canceled
- `can_be_replaced()` — `true` if the order can be amended
- `can_transition_to(next)` — validates a proposed state transition

---

## ⚡ Connection Management

Both WebSocket clients share the same lifecycle API:

```
// Wait for the initial connection before sending orders
order_ws.wait_for_connection().await;

// Obtain an independent state watcher (borrow-safe for use in select!)
let mut watcher = order_ws.state_watcher();

tokio::select! {
    state = watcher.run_till_event() => {
        match state {
            ConnectionState::Connected    => println!("online"),
            ConnectionState::Disconnected => println!("reconnecting..."),
            ConnectionState::Exited       => break,
        }
    }
    // ... other branches
}

// Graceful shutdown
order_ws.shutdown("done").await?;
```

### Custom URL Connection

Both clients support connecting to a custom URL (useful for integration tests or self-hosted deployments):

```
use ax_exchange_sdk::marketdata::MarketdataWsClient;
use url::Url;

let md = MarketdataWsClient::connect_to_url(
    Url::parse("wss://my-custom-host/md/ws")?,
    token_refresh_fn,
).await?;
```

---

## 🏗️ Architecture Overview

```
ArchitectX (client)
├── api_gateway()              → ApiGatewayRestClient  (REST)
├── order_gateway()            → OrderGatewayRestClient (REST)
├── marketdata_ws()            → MarketdataWsClient    (WS)
└── order_gateway_ws[_with_cancel_on_disconnect]()
                               → OrderGatewayWsClient  (WS)

WebSocket internals (ws_utils)
├── connection_supervisor      — reconnect loop + token refresh
├── run_single_connection      — ping/pong, frame dispatch, pending request routing
├── ConnectionStateWatcher     — borrow-safe state change observer
└── PendingRequests (DashMap)  — request/response correlation
```

---

## 🔧 Optional Feature Flags

```toml
[dependencies]
# Enable utoipa OpenAPI schema derivation
ax-exchange-sdk = { ..., features = ["utoipa"] }

# Enable schemars JSON Schema derivation
ax-exchange-sdk = { ..., features = ["schemars"] }

# Enable both
ax-exchange-sdk = { ..., features = ["all"] }
```

---

## 📚 Documentation

- [API Reference](https://docs.architect.exchange/api-reference)
- [OpenAPI Spec](https://docs.architect.exchange/openapi/api-gateway.json)

---

## 🛠️ Development

The SDK is built using a `makefile` for common tasks:

```sh
make fmt      # Format the code
make lint     # Check for lint warnings
make test     # Run tests
make build    # Build the library
make all      # Run all of the above
```

---

## 📄 License

[AGPL-3.0-only](LICENSE)

---

## 🤝 Contributing

Contributions are welcome! Please open an issue or submit a pull request.