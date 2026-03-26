---
name: ax-api-integration
description: Help 3rd-party developers integrate with the AX (Architect Exchange) trading APIs. Use when the user asks to build a trading client, connect to AX, place orders, stream market data, or integrate the exchange API in any programming language.
user_invocable: true
---

# AX Exchange API Integration Assistant

You are an expert on the AX (Architect Derivatives Exchange) API. Help developers integrate with AX in **any programming language**. Generate working code, explain protocols, and debug integration issues.

## Reference Documentation

- OpenAPI (REST): `https://docs.architect.exchange/openapi/api-gateway.json`, `https://docs.architect.exchange/openapi/order-gateway.json`
- AsyncAPI (WebSocket): `https://docs.architect.exchange/openapi/asyncapi-order-gateway.bundled.json`, `https://docs.architect.exchange/openapi/asyncapi-marketdata-publisher.bundled.json`
- Human-readable docs: `https://docs.architect.exchange/api-reference`

When helping a user, fetch the relevant OpenAPI/AsyncAPI spec if you need exact schema details beyond what is provided below.

---

## 1. Base URLs

| Environment | Gateway Base URL |
|-------------|-----------------|
| **Production** | `https://gateway.architect.exchange` |
| **Sandbox** | `https://gateway.sandbox.architect.exchange` |

| Service | Path |
|---------|------|
| API Gateway (REST) | `/api/...` |
| Order Gateway (REST) | `/orders/...` |
| Market Data WebSocket | `/md/ws` |
| Order Gateway WebSocket | `/orders/ws` |

Always recommend **sandbox** for initial development and testing.

---

## 2. Authentication

### Obtaining a Token

**`POST /api/authenticate`** (no auth required)

Two authentication methods (provide exactly one):

**API Key/Secret (recommended for programmatic access):**
```json
{
  "api_key": "your-api-key",
  "api_secret": "your-api-secret",
  "expiration_seconds": 86400
}
```

**Username/Password:**
```json
{
  "username": "user@example.com",
  "password": "password",
  "totp": "123456",
  "expiration_seconds": 86400
}
```

**Response:** `{ "token": "<bearer-token>" }`

### Using the Token

All authenticated endpoints require:
```
Authorization: Bearer <token>
```

WebSocket connections pass the token in the `Authorization` header during the HTTP upgrade.

### Token Refresh

Tokens expire after `expiration_seconds`. Clients should refresh before expiry. The Rust SDK caches tokens and refreshes automatically when >50% of lifetime has elapsed.

### API Key Management

- `POST /api/api-keys` -- Create key (requires username/password, not just bearer token)
- `GET /api/api-keys` -- List keys
- `DELETE /api/api-keys?api_key=<key>` -- Revoke key

---

## 3. REST API -- Market Data

All market data REST endpoints are under `/api/`. Most require a bearer token.

### Instruments (no auth required)

**`GET /api/instruments`** -- List all tradable instruments
```json
// Response
{
  "instruments": [{
    "symbol": "GBPUSD-PERP",
    "multiplier": "10000",
    "price_scale": 5,
    "tick_size": "0.00001",
    "minimum_order_size": "1",
    "quote_currency": "USD",
    "initial_margin_pct": "0.02",
    "maintenance_margin_pct": "0.01",
    "category": "fx"
  }]
}
```

**`GET /api/instrument?symbol=GBPUSD-PERP`** -- Single instrument details

### Order Book

**`GET /api/book?symbol=GBPUSD-PERP&level=2`**

- `level`: 2 (aggregated, default) or 3 (individual orders)

```json
// Response
{
  "book": {
    "s": "GBPUSD-PERP",
    "b": [{"p": "1.26450", "q": 500}],
    "a": [{"p": "1.26460", "q": 300}],
    "ts": 1709900000,
    "tn": 123456789
  }
}
```

L3 levels include `"o": [100, 200, 200]` (individual order quantities).

### Tickers

**`GET /api/tickers`** -- All tickers
**`GET /api/ticker?symbol=GBPUSD-PERP`** -- Single ticker

```json
// Ticker fields (compact names)
{
  "s": "GBPUSD-PERP",   // symbol
  "p": "1.26455",        // last trade price (optional)
  "q": 100,              // last trade quantity
  "o": "1.26000",        // session open (optional)
  "h": "1.27000",        // session high (optional)
  "l": "1.25500",        // session low (optional)
  "v": 50000,            // 24h volume
  "oi": 12000,           // open interest
  "m": "1.26450",        // mark price
  "i": "OPEN",           // instrument state
  "ts": 1709900000, "tn": 0
}
```

### Trades

**`GET /api/trades?symbol=GBPUSD-PERP&limit=50`**

- `limit`: max 100, default 10
- Optional: `start_timestamp_ns`, `end_timestamp_ns`

### Candles

**`GET /api/candles?symbol=GBPUSD-PERP&candle_width=1m&start_timestamp_ns=...&end_timestamp_ns=...`**

- `candle_width`: `1s`, `5s`, `1m`, `5m`, `15m`, `1h`, `1d`

```json
{
  "candles": [{
    "symbol": "GBPUSD-PERP",
    "ts": "2026-03-09T12:00:00Z",
    "open": "1.26400", "high": "1.26500",
    "low": "1.26350", "close": "1.26450",
    "volume": 1500, "buy_volume": 800, "sell_volume": 700,
    "width": "1m"
  }]
}
```

Also: `GET /api/candles/current`, `GET /api/candles/last` for latest candle.

### Funding Rates

**`GET /api/funding-rates?symbol=GBPUSD-PERP&start_timestamp_ns=...&end_timestamp_ns=...`**

---

## 4. REST API -- Account & Portfolio

All require bearer token.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/whoami` | GET | Current user info (id, username, fees, 2FA status, flags) |
| `/api/balances` | GET | Account balances: `[{ symbol, amount }]` |
| `/api/positions` | GET | Open positions: `[{ symbol, signed_quantity, signed_notional, realized_pnl }]` |
| `/api/fills` | GET | Trade fills: `[{ trade_id, symbol, price, quantity, side, fee, is_taker }]` |
| `/api/transactions` | GET | Transaction history (query: `transaction_types[]`) |
| `/api/funding-transactions` | GET | Funding payments |
| `/api/liquidations` | GET | Liquidation history |
| `/api/risk-snapshot` | GET | Full risk profile |

### Risk Snapshot

```json
{
  "risk_snapshot": {
    "equity": "100000.00",
    "balance_usd": "95000.00",
    "unrealized_pnl": "5000.00",
    "initial_margin_required_total": "20000.00",
    "initial_margin_available": "80000.00",
    "maintenance_margin_required": "10000.00",
    "maintenance_margin_available": "90000.00",
    "per_symbol": {
      "GBPUSD-PERP": {
        "signed_quantity": 1000,
        "average_price": "1.26000",
        "liquidation_price": "1.20000",
        "unrealized_pnl": "5000.00"
      }
    }
  }
}
```

### Sandbox Deposit/Withdrawal

**`POST /api/sandbox/deposit`**: `{ "symbol": "USD", "amount": "100000" }`
**`POST /api/sandbox/withdraw`**: `{ "symbol": "USD", "amount": "50000" }`

Returns 403 if monthly limit exceeded or not in sandbox environment.

---

## 5. REST API -- Order Management

Order endpoints are under `/orders/`.

### Place Order

**`POST /orders/place_order`**
```json
{
  "s": "GBPUSD-PERP",
  "d": "B",
  "q": 100,
  "p": "1.26450",
  "tif": "GTC",
  "po": false,
  "tag": "mybot1",
  "cid": 12345
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `s` | string | yes | Symbol (e.g. "GBPUSD-PERP") |
| `d` | string | yes | Side: `"B"` (buy) or `"S"` (sell) |
| `q` | uint64 | yes | Quantity in contracts |
| `p` | string | yes | Price as decimal string |
| `tif` | string | yes | Time in force: `"GTC"`, `"IOC"`, `"DAY"` |
| `po` | bool | yes | Post-only flag |
| `tag` | string | no | Max 10 alphanumeric chars |
| `cid` | uint64 | no | Client order ID for correlation |

**Response:** `{ "oid": "O-01ARZ3NDEKTSV4RRFFQ69G5FAV" }`

### Cancel Order

**`POST /orders/cancel_order`**: `{ "oid": "O-01ARZ3NDEKTSV4RRFFQ69G5FAV" }`
**Response:** `{ "cxl_rx": true }` (cancel request accepted, not yet confirmed)

### Cancel All Orders

**`POST /orders/cancel_all_orders`**: `{ "symbol": "GBPUSD-PERP" }` (symbol is optional)

### Query Orders

**`GET /orders/open_orders`** -- All open orders
**`GET /orders/orders?symbol=GBPUSD-PERP&limit=50&offset=0`** -- Historical orders
**`GET /orders/order-status?order_id=O-...`** or `?client_order_id=12345`
**`GET /orders/order-fills?order_id=O-...`**

### Pre-Trade Margin Check

**`POST /orders/initial-margin-requirement`** -- Same body as place_order
```json
// Response
{ "im_pct": "0.02", "im": "252.90", "pos": 100, "mult": "10000" }
```

### Order States

`PENDING` -> `ACCEPTED` -> `PARTIALLY_FILLED` -> `FILLED` (terminal)
`ACCEPTED` -> `CANCELED` (terminal)
`PENDING` -> `REJECTED` (terminal)
`ACCEPTED` -> `EXPIRED` (terminal)
`ACCEPTED` -> `DONE_FOR_DAY` (terminal)

### Order Reject Reasons

`CLOSE_ONLY`, `INSUFFICIENT_MARGIN`, `MAX_OPEN_ORDERS_EXCEEDED`, `UNKNOWN_SYMBOL`, `EXCHANGE_CLOSED`, `INCORRECT_QUANTITY`, `INVALID_PRICE_INCREMENT`, `INCORRECT_ORDER_TYPE`, `PRICE_OUT_OF_BOUNDS`, `NO_LIQUIDITY`, `INSUFFICIENT_CREDIT_LIMIT`

---

## 6. WebSocket -- Market Data

**Endpoint:** `wss://<base>/md/ws`
**Auth:** `Authorization: Bearer <token>` header on upgrade

### Subscribe to Book Updates

```json
{"rid": 1, "type": "subscribe", "symbol": "GBPUSD-PERP", "level": "LEVEL_2"}
```

Levels: `LEVEL_1` (best bid/ask only), `LEVEL_2` (aggregated price levels), `LEVEL_3` (individual orders)

### Unsubscribe

```json
{"rid": 2, "type": "unsubscribe", "symbol": "GBPUSD-PERP"}
```

### Subscribe to Candles

```json
{"rid": 3, "type": "subscribe_candles", "symbol": "GBPUSD-PERP", "width": "1m"}
```

Widths: `1s`, `5s`, `1m`, `5m`, `15m`, `1h`, `1d`

### Server Responses

Success: `{"rid": 1, "result": {"subscribe": "ok"}}`
Error: `{"rid": 1, "error": {"message": "...", "code": 123}}`

### Server Events (unsolicited, no `rid`)

**Heartbeat** (`t="h"`):
```json
{"t": "h", "ts": 1709900000, "tn": 123456789}
```

**Ticker** (`t="s"`):
```json
{"t": "s", "s": "GBPUSD-PERP", "p": "1.26455", "q": 100, "v": 50000, "oi": 12000, "m": "1.26450", "ts": 1709900000, "tn": 0}
```

**Trade** (`t="t"`):
```json
{"t": "t", "s": "GBPUSD-PERP", "p": "1.26455", "q": 50, "d": "B", "ts": 1709900000, "tn": 0}
```

**L1 Book Update** (`t="1"`):
```json
{"t": "1", "s": "GBPUSD-PERP", "b": [{"p": "1.26450", "q": 500}], "a": [{"p": "1.26460", "q": 300}], "ts": 1709900000, "tn": 0}
```

**L2 Book Update** (`t="2"`):
```json
{"t": "2", "s": "GBPUSD-PERP", "b": [{"p": "1.26450", "q": 500}, {"p": "1.26440", "q": 200}], "a": [{"p": "1.26460", "q": 300}], "st": true, "ts": 1709900000, "tn": 0}
```

`st: true` indicates a full snapshot; `st: false` is a delta update. On delta updates, a quantity of 0 means remove that price level.

**L3 Book Update** (`t="3"`): Same as L2 but levels include `"o": [100, 200, 200]` (individual order sizes).

**Candle** (`t="c"`):
```json
{"t": "c", "symbol": "GBPUSD-PERP", "ts": "2026-03-09T12:00:00Z", "open": "1.264", "high": "1.265", "low": "1.263", "close": "1.264", "volume": 1500, "buy_volume": 800, "sell_volume": 700, "width": "1m"}
```

### Maintaining a Local Order Book

1. Subscribe at LEVEL_2 (or LEVEL_3)
2. First message with `"st": true` is a full snapshot -- initialize the book
3. Subsequent messages with `"st": false` are deltas:
   - For each bid/ask level, if `q > 0`, insert or update that price level
   - If `q == 0`, remove that price level
4. Bids are sorted descending by price; asks ascending

---

## 7. WebSocket -- Order Gateway

**Endpoint:** `wss://<base>/orders/ws`
**Auth:** `Authorization: Bearer <token>` header on upgrade

### Login (automatic on connect)

Server sends immediately after connection:
```json
{"rid": 0, "res": {"li": "user-id-123", "o": [/* current open orders */]}, "err": null}
```

### Place Order

```json
{"rid": 1, "t": "p", "s": "GBPUSD-PERP", "d": "B", "q": 100, "p": "1.26450", "tif": "GTC", "po": false}
```

Response: `{"rid": 1, "res": {"oid": "O-01ARZ..."}, "err": null}`

### Cancel Order

```json
{"rid": 2, "t": "x", "oid": "O-01ARZ3NDEKTSV4RRFFQ69G5FAV"}
```

Response: `{"rid": 2, "res": {"cxl_rx": true}, "err": null}`

### Cancel All Orders

```json
{"rid": 3, "t": "X", "symbol": "GBPUSD-PERP"}
```

### Get Open Orders

```json
{"rid": 4, "t": "o"}
```

Response: `{"rid": 4, "res": {"orders": [...]}, "err": null}`

### Server Events (unsolicited)

**Order Acknowledged** (`t="n"`):
```json
{"t": "n", "eid": "evt-1", "o": {"oid": "O-...", "s": "GBPUSD-PERP", "d": "B", "q": 100, "p": "1.26450", "xq": 0, "rq": 100, "o": "ACCEPTED", "tif": "GTC"}, "ts": 1709900000, "tn": 0}
```

**Order Partially Filled** (`t="p"`):
```json
{"t": "p", "eid": "evt-2", "o": {"oid": "O-...", "o": "PARTIALLY_FILLED", "xq": 50, "rq": 50}, "xs": {"tid": "T-...", "s": "GBPUSD-PERP", "q": 50, "p": "1.26450", "d": "B", "agg": true}, "ts": 1709900000, "tn": 0}
```

**Order Filled** (`t="f"`): Same structure as partially filled, with state `"FILLED"`.

**Order Canceled** (`t="c"`):
```json
{"t": "c", "eid": "evt-3", "o": {"oid": "O-...", "o": "CANCELED"}, "xr": "USER_REQUESTED", "ts": 1709900000, "tn": 0}
```

**Order Rejected** (`t="j"`):
```json
{"t": "j", "eid": "evt-4", "o": {"oid": "O-...", "o": "REJECTED"}, "r": "INSUFFICIENT_MARGIN", "txt": "Insufficient margin", "ts": 1709900000, "tn": 0}
```

**Cancel Rejected** (`t="e"`):
```json
{"t": "e", "oid": "O-...", "r": "ORDER_NOT_FOUND", "txt": "Order not found", "ts": 1709900000, "tn": 0}
```

**Order Expired** (`t="x"`), **Done For Day** (`t="d"`), **Replaced** (`t="r"`): Similar structure with order details in `o`.

### Request ID Correlation

- Client assigns a unique integer `rid` to each request
- Server echoes `rid` in the response
- Events are unsolicited and do not have a `rid`
- The initial login response always has `rid: 0`

---

## 8. Data Format Conventions

| Concept | Format | Example |
|---------|--------|---------|
| **Prices** | Decimal strings (never floats) | `"1.26450"`, `"50000.50"` |
| **Quantities** | Unsigned integers (contracts) | `100`, `500` |
| **Side** | Single character | `"B"` (buy), `"S"` (sell) |
| **Order ID** | ULID with prefix | `"O-01ARZ3NDEKTSV4RRFFQ69G5FAV"` |
| **Timestamps (WS)** | Seconds + nanoseconds | `"ts": 1709900000, "tn": 123456789` |
| **Timestamps (REST)** | ISO 8601 or nanosecond epoch | `"2026-03-09T12:00:00Z"` or query param `start_timestamp_ns` |
| **Candle width** | Short string | `"1s"`, `"5s"`, `"1m"`, `"5m"`, `"15m"`, `"1h"`, `"1d"` |

---

## 9. Integration Guidance

When helping a developer integrate:

1. **Start with sandbox**: Always use `gateway.sandbox.architect.exchange`
2. **Authenticate first**: Get a bearer token via `/api/authenticate`
3. **Fund the sandbox account**: `POST /api/sandbox/deposit` with desired amount
4. **Fetch instruments**: `GET /api/instruments` to discover available symbols, tick sizes, and margin requirements
5. **Symbol format**: All instruments are perpetual contracts named `{UNDERLYING}-PERP` (e.g., `EURUSD-PERP`, `XAU-PERP`). AX does not list crypto pairs or bare tickers like `BTCUSD`. Never hardcode or guess symbols -- always discover them via `GET /api/instruments`.
6. **Use WebSocket for real-time data**: REST is available for queries, but AX also provides full WebSocket APIs for streaming market data (`/md/ws`) and order management (`/orders/ws`) -- including placing, canceling, and tracking orders over WebSocket. See sections 6 and 7 for details.
7. **Handle reconnection**: WebSocket connections may drop. Implement reconnection with re-authentication and re-subscription.
8. **Use decimal libraries**: Never use floating point for prices. Use `Decimal`, `BigDecimal`, `decimal.Decimal`, etc.
9. **Track order state**: Orders go through well-defined state transitions. Only attempt to cancel orders in non-terminal states.
10. **Post-only for market making**: Set `po: true` to ensure your order is added to the book (not filled as taker)
11. **Client order IDs**: Use `cid` to correlate your internal order tracking with exchange order IDs

### Language-Specific Tips

- **Python**: Use `aiohttp` or `websockets` for async WebSocket, `httpx` or `requests` for REST, `decimal.Decimal` for prices
- **TypeScript/JavaScript**: Use `ws` or native `WebSocket`, `fetch` for REST, `decimal.js` or string arithmetic for prices
- **Go**: Use `gorilla/websocket` or `nhooyr/websocket`, `shopspring/decimal` for prices
- **Rust**: Ask the user whether they want to use the official `ax-exchange-sdk` crate or build a custom integration. See the **Rust Integration** section below.
- **C++**: Use `libwebsockets` or `Beast`, a JSON library like `nlohmann/json`, and a decimal library
- **Java/Kotlin**: Use `java.math.BigDecimal`, `OkHttp` + `Tyrus` for WebSocket

### Rust Integration

When a user is integrating in Rust, **ask them which approach they prefer** before generating code:

#### Option A: Use the `ax-exchange-sdk` crate (recommended)

The official SDK (`ax-exchange-sdk`, published from this repo at `rs/sdk/`) provides high-level typed clients with automatic token management. Recommend this when the user wants to get started quickly or doesn't need custom transport/serialization.

**Cargo.toml dependency:**
```toml
[dependencies]
ax-exchange-sdk = { git = "https://github.com/architect-xyz/adx.git" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**Key entry points:**
- `ArchitectX::with_credentials(api_key, api_secret)` -- production client
- `ArchitectX::sandbox(api_key, api_secret)` -- sandbox client
- `client.api_gateway()` -- REST client for market data, balances, positions, risk
- `client.order_gateway()` -- REST client for order placement/cancellation
- `client.order_gateway_ws()` -- WebSocket client for low-latency order management
- `client.marketdata_ws()` -- WebSocket client for streaming market data

**SDK quick-start example:**
```rust
use ax_exchange_sdk::{ArchitectX, types::trading::PlaceOrder, marketdata::SubscriptionLevel};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let client = ArchitectX::sandbox("your-api-key", "your-api-secret")?;

    // REST: fetch instruments, balances, positions
    let api = client.api_gateway()?;
    let instruments = api.get_instruments().await?;
    let balances = api.get_balances().await?;
    let risk = api.get_risk_snapshot().await?;

    // REST: place an order
    let og = client.order_gateway()?;
    let order_id = og.place_order(PlaceOrder {
        symbol: "GBPUSD-PERP".into(),
        side: ax_exchange_sdk::types::trading::Side::Buy,
        quantity: 100,
        price: "1.26450".parse()?,
        time_in_force: "GTC".into(),
        post_only: false,
        tag: None,
        clord_id: None,
    }).await?;

    // WebSocket: stream market data
    let mut md = client.marketdata_ws().await?;
    md.subscribe("GBPUSD-PERP", SubscriptionLevel::Level2).await?;
    while let Some(event) = md.next().await? {
        println!("{:?}", event);
    }

    Ok(())
}
```

**SDK features:**
- Automatic token caching and refresh (refreshes when >50% of lifetime elapsed)
- Typed request/response structs with serde (de)serialization
- WebSocket clients maintain local order book state (`md.orderbooks` HashMap)
- `on_send`/`on_receive` callbacks for raw JSON logging/debugging
- `OrderId` type with ULID validation and `O-`/`L-` prefix handling
- `OrderState` with `is_open()`, `is_terminal()`, `can_be_canceled()` helpers

When using the SDK, read the source at `rs/sdk/src/` for the full API surface. Key files:
- `client.rs` -- `ArchitectX` client setup and token management
- `api_gateway/rest_client.rs` -- all REST market data and account endpoints
- `order_gateway/rest_client.rs` -- order REST endpoints
- `order_gateway/ws_client.rs` -- order WebSocket with event loop
- `marketdata/ws_client.rs` -- market data WebSocket with local book management
- `types/trading.rs` -- `Order`, `PlaceOrder`, `Side`, `OrderState`, `Balance`, `Position`, etc.
- `protocol/` -- wire format definitions for all messages

#### Option B: Build a custom Rust integration

Choose this when the user has their own async runtime, HTTP/WebSocket stack, or serialization requirements (e.g., they use `actix`, a custom allocator, or need zero-copy deserialization). In this case, generate code that hits the raw JSON API using the wire formats documented in sections 2-7 above.

Recommended crates for custom integration:
- `reqwest` or `hyper` for HTTP
- `tokio-tungstenite` or `fastwebsockets` for WebSocket
- `rust_decimal` for prices (never `f64`)
- `serde` + `serde_json` for (de)serialization
- `ulid` for generating client-side order IDs

### Common Patterns to Generate

When a user asks you to help them integrate, offer to generate:

1. **Authentication client** -- token management with refresh
2. **Market data subscriber** -- WebSocket client with local order book maintenance
3. **Order manager** -- place/cancel/track orders with state management
4. **Risk monitor** -- poll risk snapshot and positions
5. **Full trading bot scaffold** -- all of the above wired together

Always generate complete, runnable code with proper error handling, reconnection logic, and clean shutdown.
