use crate::{protocol, types::*};
use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use rust_decimal::Decimal;
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;
use auth_gateway::{AuthGatewayClient, AuthGatewayConfig};

#[derive(Clone)]
pub struct ArchitectX {
    base_url: Url,
    auth_client: Arc<AuthGatewayClient>,
    username: Option<String>,
    password: Option<String>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl ArchitectX {
    // CR alee: default empty construction, use builder pattern
    pub fn new(base_url: Url, username: Option<&str>, password: Option<&str>) -> Self {
        // Create auth-gateway client configuration
        let auth_config = AuthGatewayConfig {
            base_url: base_url.join("auth/").unwrap_or(base_url.clone()).to_string(),
            admin_secret_key: std::env::var("DECODE_TOKEN_SECRET_KEY")
                .expect("DECODE_TOKEN_SECRET_KEY environment variable must be set"),
            timeout_seconds: 10,
            max_retries: 3,
            pool_max_idle_per_host: 10,
        };
        
        let auth_client = AuthGatewayClient::new(auth_config)
            .expect("Failed to create auth-gateway client");
        
        Self {
            base_url,
            auth_client: Arc::new(auth_client),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
            user_token: Arc::new(ArcSwapOption::const_empty()),
        }
    }

    pub async fn refresh_user_token(&self, force: bool) -> Result<ArcStr> {
        let now = Utc::now();
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, expires_at) = &**stored;
            if !force && *expires_at > now {
                return Ok(token.clone());
            }
        }
        let username =
            self.username.as_ref().ok_or_else(|| anyhow!("no username provided"))?;
        let password =
            self.password.as_ref().ok_or_else(|| anyhow!("no password provided"))?;
        let token = self.get_user_token(username, password, 3600).await?;
        let token = ArcStr::from(token);
        self.user_token.store(Some(Arc::new((
            token.clone(),
            now + chrono::Duration::seconds(
                3300, /* one hour, less 5 minutes buffer */
            ),
        ))));
        Ok(token)
    }

    pub async fn get_user_token(
        &self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        expiration_seconds: i32,
    ) -> Result<String> {
        self.auth_client.get_user_token(username.as_ref(), password.as_ref(), expiration_seconds as u64).await
            .map_err(|e| anyhow!("Failed to get user token: {}", e))
    }

    pub async fn get_instrument(&self, symbol: impl AsRef<str>) -> Result<Instrument> {
        let token = self.refresh_user_token(false).await?;
        let instruments = self.auth_client.get_instruments(&token).await
            .map_err(|e| anyhow!("Failed to get instruments: {}", e))?;
        
        let symbol_str = symbol.as_ref();
        let auth_instrument = instruments.into_iter()
            .find(|i| i.symbol == symbol_str)
            .ok_or_else(|| anyhow!("Instrument not found: {}", symbol_str))?;
        
        Ok(Instrument {
            symbol: auth_instrument.symbol,
            tick_size: auth_instrument.tick_size,
            base_currency: auth_instrument.base_currency,
            multiplier: 1, // Default multiplier since auth-gateway doesn't provide this
            minimum_trade_quantity: auth_instrument.minimum_trade_quantity as i32,
            description: auth_instrument.description,
            product_id: auth_instrument.product_id,
            state: auth_instrument.state,
            price_scale: auth_instrument.price_scale.to_string().parse::<i32>().unwrap_or(1),
        })
    }

    pub async fn marketdata_client(&self) -> Result<MarketdataClient> {
        let username =
            self.username.as_ref().ok_or_else(|| anyhow!("no username provided"))?;
        let token = self.refresh_user_token(false).await?;
        MarketdataClient::connect(self.base_url.clone(), username, token).await
    }

    pub async fn order_gateway_client(&self) -> Result<OrderGatewayClient> {
        let username =
            self.username.as_ref().ok_or_else(|| anyhow!("no username provided"))?;
        let token = self.refresh_user_token(false).await?;
        OrderGatewayClient::connect(self.base_url.clone(), username, token).await
    }
}

pub struct Orderbook {
    pub bids: BTreeMap<Decimal, OrderbookLevel>,
    pub asks: BTreeMap<Decimal, OrderbookLevel>,
}

pub struct OrderbookLevel {
    pub quantity: i32,
    pub order_quantities: Option<Vec<i32>>, // for LEVEL_3
}

impl From<&protocol::marketdata_publisher::L2BookUpdate> for Orderbook {
    fn from(u: &protocol::marketdata_publisher::L2BookUpdate) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        for l in &u.bids {
            bids.insert(
                l.price,
                OrderbookLevel { quantity: l.quantity, order_quantities: None },
            );
        }
        for l in &u.asks {
            asks.insert(
                l.price,
                OrderbookLevel { quantity: l.quantity, order_quantities: None },
            );
        }
        Self { bids, asks }
    }
}

impl From<&protocol::marketdata_publisher::L3BookUpdate> for Orderbook {
    fn from(u: &protocol::marketdata_publisher::L3BookUpdate) -> Self {
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

pub struct MarketdataClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    pub orderbooks: HashMap<String, Orderbook>,
}

impl MarketdataClient {
    pub async fn connect(
        base_url: Url,
        username: impl AsRef<str>,
        token: impl AsRef<str>,
    ) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let md_url = ws_base_url.join("md/ws")?.to_string();

        // connect to market data publisher
        info!("connecting to {md_url}");
        let (mut ws, _) = connect_async(md_url).await?;

        // send login request
        let req = json!({
            "request_id": 1,
            "type": "login",
            "username": username.as_ref().to_string(),
            "token": token.as_ref().to_string(),
        });
        let payload = serde_json::to_string(&req)?;
        info!("sending login request: {payload}");
        ws.send(Message::Text(payload.into())).await?;

        Ok(Self { ws, next_request_id: 1, orderbooks: HashMap::new() })
    }

    pub async fn next(
        &mut self,
    ) -> Result<Option<Arc<protocol::marketdata_publisher::MarketdataEvent>>> {
        let msg = self.ws.next().await.ok_or_else(|| anyhow!("ws stream ended"))??;
        match msg {
            Message::Text(text) => {
                trace!("decoding marketdata message: {text}");
                match serde_json::from_str::<
                    protocol::ws::Response<Box<serde_json::value::RawValue>>,
                >(&text)
                {
                    Ok(_r) => {
                        // TODO: do something
                    }
                    Err(e_as_response) => {
                        match serde_json::from_str::<
                            Arc<protocol::marketdata_publisher::MarketdataEvent>,
                        >(&text)
                        {
                            Ok(e) => {
                                self.handle_event(&e)?;
                                return Ok(Some(e));
                            }
                            Err(e_as_event) => {
                                error!("decoding marketdata message as event: {e_as_event:?}");
                                error!("decoding marketdata message as response: {e_as_response:?}");
                                return Ok(None);
                            }
                        }
                    }
                }
            }
            Message::Ping(..) => {
                trace!("ws ping received");
            }
            Message::Binary(..)
            | Message::Frame(..)
            | Message::Pong(..)
            | Message::Close(..) => {}
        }
        Ok(None)
    }

    fn handle_event(
        &mut self,
        e: &protocol::marketdata_publisher::MarketdataEvent,
    ) -> Result<()> {
        use protocol::marketdata_publisher::*;
        trace!("marketdata event: {e:?}");
        match e {
            MarketdataEvent::Heartbeat(t) => {
                debug!("heartbeat: {:?}", t.as_datetime());
            }
            MarketdataEvent::Ticker(_t) => {
                // TODO
            }
            MarketdataEvent::L1BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
            MarketdataEvent::L2BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
            MarketdataEvent::L3BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
        }
        Ok(())
    }

    // CR alee: also send an unsubscribe (only subscribe one level per symbol
    // at a time); maybe that's just the behavior of the publisher anyways
    pub async fn subscribe(
        &mut self,
        symbol: impl AsRef<str>,
        level: &str, // LEVEL_1, LEVEL_2, LEVEL_3
    ) -> Result<()> {
        let req_id = self.next_request_id;
        let req = json!({
            "request_id": req_id,
            "type": "subscribe",
            "symbol": symbol.as_ref().to_string(),
            "level": level,
        });
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        trace!("sending subscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, symbol: impl AsRef<str>) -> Result<()> {
        let req_id = self.next_request_id;
        let req = json!({
            "request_id": req_id,
            "type": "unsubscribe",
            "symbol": symbol.as_ref().to_string(),
        });
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        trace!("sending unsubscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }
}

pub struct OrderGatewayClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    pub logged_in: bool,
    pending_requests: HashMap<i32, protocol::order_gateway::OrderGatewayRequest>,
    pub open_orders: HashMap<String, Order>,
}

impl OrderGatewayClient {
    pub async fn connect(
        base_url: Url,
        username: impl AsRef<str>,
        token: impl AsRef<str>,
    ) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let order_gateway_url = ws_base_url.join("orders/ws")?.to_string();

        // connect to order gateway
        info!("connecting to {order_gateway_url}");
        let (mut ws, _) = connect_async(order_gateway_url).await?;

        // send login request
        let req = json!({
            "rid": 1,
            "t": "a",
            "u": username.as_ref().to_string(),
            "k": token.as_ref().to_string(),
        });
        let payload = serde_json::to_string(&req)?;
        trace!("sending login request: {payload}");
        ws.send(Message::Text(payload.into())).await?;

        Ok(Self {
            ws,
            next_request_id: 2,
            logged_in: false,
            pending_requests: HashMap::new(),
            open_orders: HashMap::new(),
        })
    }

    pub async fn next(
        &mut self,
    ) -> Result<Option<protocol::order_gateway::OrderGatewayEvent>> {
        let msg = self.ws.next().await.ok_or_else(|| anyhow!("ws stream ended"))??;
        match msg {
            Message::Text(text) => {
                trace!("decoding order gateway message: {text}");
                match serde_json::from_str::<
                    protocol::ws::Response<Box<serde_json::value::RawValue>>,
                >(&text)
                {
                    Ok(r) => {
                        if let Err(e_res) = self.handle_response(r) {
                            error!("handling response: {e_res:?}");
                        }
                    }
                    Err(e_as_response) => {
                        match serde_json::from_str::<
                            protocol::order_gateway::OrderGatewayEvent,
                        >(&text)
                        {
                            Ok(e) => {
                                self.handle_event(&e)?;
                                return Ok(Some(e));
                            }
                            Err(e_as_event) => {
                                error!("decoding order gateway message as event: {e_as_event:?}");
                                error!("decoding order gateway message as response: {e_as_response:?}");
                                return Ok(None);
                            }
                        }
                    }
                }
            }
            Message::Ping(..) => {
                trace!("ws ping received");
            }
            Message::Binary(..)
            | Message::Frame(..)
            | Message::Pong(..)
            | Message::Close(..) => {}
        }
        Ok(None)
    }

    fn handle_response(
        &mut self,
        res: protocol::ws::Response<Box<serde_json::value::RawValue>>,
    ) -> Result<()> {
        if res.request_id == 1 {
            self.handle_login_response(res)?;
        } else if self.pending_requests.remove(&res.request_id).is_some() {
            // TODO
        } else {
            warn!("response to unknown request: {}", res.request_id);
        }
        Ok(())
    }

    fn handle_login_response(
        &mut self,
        res: protocol::ws::Response<Box<serde_json::value::RawValue>>,
    ) -> Result<()> {
        if res.error.is_some() {
            bail!("login failed: {:?}", res.error.unwrap());
        }
        if let Some(res) = res.response {
            let res: protocol::order_gateway::LoginResponse =
                serde_json::from_str(res.get())?;
            for order in res.open_orders {
                let order = match Order::try_from(order) {
                    Ok(o) => o,
                    Err(e) => {
                        error!("invalid order: {e:?}");
                        continue;
                    }
                };
                self.open_orders.insert(order.order_id.clone(), order);
            }
        } else {
            bail!("login failed: expected non-empty response");
        }
        self.logged_in = true;
        Ok(())
    }

    fn handle_event(
        &mut self,
        e: &protocol::order_gateway::OrderGatewayEvent,
    ) -> Result<()> {
        use protocol::order_gateway::*;
        trace!("order gateway event: {e:?}");
        match e {
            OrderGatewayEvent::Heartbeat(t) => {
                debug!("heartbeat: {:?}", t.as_datetime());
            }
            OrderGatewayEvent::CancelRejected(_r) => {
                // CR alee: do something
            }
            OrderGatewayEvent::OrderRejected(OrderRejected { order, .. }) => {
                self.open_orders.remove(&order.order_id);
            }
            OrderGatewayEvent::OrderAcked(OrderAcked { order, .. })
            | OrderGatewayEvent::OrderReplacedOrAmended(OrderReplacedOrAmended {
                order,
                ..
            })
            | OrderGatewayEvent::OrderPartiallyFilled(OrderPartiallyFilled {
                order,
                ..
            }) => {
                let o: Order = order.clone().try_into()?;
                self.open_orders.insert(o.order_id.clone(), o);
            }
            OrderGatewayEvent::OrderCanceled(OrderCanceled { order, .. })
            | OrderGatewayEvent::OrderDoneForDay(OrderDoneForDay { order, .. })
            | OrderGatewayEvent::OrderExpired(OrderExpired { order, .. })
            | OrderGatewayEvent::OrderFilled(OrderFilled { order, .. }) => {
                if self.open_orders.remove(&order.order_id).is_none() {
                    warn!("order not found in open orders: {order:?}");
                }
            }
        }
        Ok(())
    }

    pub async fn place_order(&mut self, place_order: PlaceOrder) -> Result<()> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req =
            protocol::order_gateway::OrderGatewayRequest::PlaceOrder(place_order.into());
        let wrapped_req = protocol::ws::Request { request_id, request: req.clone() };
        let payload = serde_json::to_string(&wrapped_req)?;
        trace!("sending place order request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.pending_requests.insert(request_id, req);
        Ok(())
    }

    pub async fn cancel_order(&mut self, order_id: impl AsRef<str>) -> Result<()> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::CancelOrder(
            protocol::order_gateway::CancelOrderRequest {
                order_id: order_id.as_ref().to_string(),
            },
        );
        let wrapped_req = protocol::ws::Request { request_id, request: req.clone() };
        let payload = serde_json::to_string(&wrapped_req)?;
        trace!("sending cancel order request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.pending_requests.insert(request_id, req);
        Ok(())
    }

    /// Cancel all open orders
    pub async fn cancel_all_orders(&mut self) -> Result<()> {
        let order_ids = self.open_orders.keys().cloned().collect::<Vec<_>>();
        for order_id in order_ids {
            self.cancel_order(order_id).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login_response() -> Result<()> {
        let res = r#"{"rid":1,"res":{"li":"market-maker-01","o":[]}}"#;
        let _res: protocol::ws::Response<Box<serde_json::value::RawValue>> =
            serde_json::from_str(res)?;
        Ok(())
    }
}
