use crate::protocol::{self, order_gateway::*};
use crate::types::PlaceOrder;
use anyhow::{Result, anyhow, bail};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, handshake::client::generate_key, http::Request},
};
use url::Url;

pub type SendCallback = Box<dyn Fn(&str) + Send + Sync>;
pub type ReceiveCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Order gateway WebSocket client.
///
/// After initializing a connection with `connect`, drive the connection
/// by calling `next` on loop.
///
/// It's expected that the first non-heartbeat message received should
/// be a login response.
pub struct OrderGatewayWsClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    in_flight_requests: HashMap<i32, OrderGatewayRequestType>,
    account_id: Option<String>,
    on_send: Option<SendCallback>,
    on_receive: Option<ReceiveCallback>,
}

impl OrderGatewayWsClient {
    /// Connect to an order gateway and login with the provided credentials.
    pub async fn connect(base_url: Url, token: impl AsRef<str>) -> Result<Self> {
        Self::connect_inner(base_url, "ws", token, false, None).await
    }

    /// Connect to an order gateway with the WebSocket session scoped to an account.
    pub async fn connect_for_account(
        base_url: Url,
        token: impl AsRef<str>,
        account_id: impl Into<String>,
    ) -> Result<Self> {
        Self::connect_inner(base_url, "ws", token, false, Some(account_id.into())).await
    }

    /// Connect to an order gateway with cancel-on-disconnect enabled.
    ///
    /// When the connection closes, the gateway will cancel all orders
    /// placed on this session.
    pub async fn connect_with_cancel_on_disconnect(
        base_url: Url,
        token: impl AsRef<str>,
    ) -> Result<Self> {
        Self::connect_inner(base_url, "ws", token, true, None).await
    }

    /// Connect to an account-scoped order gateway session with cancel-on-disconnect enabled.
    pub async fn connect_for_account_with_cancel_on_disconnect(
        base_url: Url,
        token: impl AsRef<str>,
        account_id: impl Into<String>,
    ) -> Result<Self> {
        Self::connect_inner(base_url, "ws", token, true, Some(account_id.into())).await
    }

    async fn connect_inner(
        base_url: Url,
        path: &str,
        token: impl AsRef<str>,
        cancel_on_disconnect: bool,
        account_id: Option<String>,
    ) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let mut order_gateway_url = ws_base_url.join(path)?;
        if cancel_on_disconnect || account_id.is_some() {
            let mut query = order_gateway_url.query_pairs_mut();
            if cancel_on_disconnect {
                query.append_pair("cancel_on_disconnect", "true");
            }
            if let Some(account_id) = &account_id {
                query.append_pair("account_id", account_id);
            }
        }

        // connect to order gateway
        info!("connecting to {order_gateway_url}");
        let authority = order_gateway_url.authority();
        let host = authority
            .find('@')
            .map(|idx| authority.split_at(idx + 1).1)
            .unwrap_or_else(|| authority);
        let request = Request::builder()
            .method("GET")
            .uri(order_gateway_url.as_str())
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .header("Authorization", token.as_ref())
            .body(())?;
        let (ws, _) = connect_async(request).await?;

        Ok(Self {
            ws,
            next_request_id: 1,
            in_flight_requests: HashMap::new(),
            account_id,
            on_send: None,
            on_receive: None,
        })
    }

    fn request_account(&self, account_id: Option<&str>) -> Option<String> {
        account_id
            .map(str::to_string)
            .or_else(|| self.account_id.clone())
    }

    /// Set a callback to be called when sending messages to the WebSocket.
    /// The callback receives the raw JSON payload as a string.
    pub fn on_send<F>(&mut self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_send = Some(Box::new(callback));
    }

    /// Set a callback to be called when receiving messages from the WebSocket.
    /// The callback receives the raw JSON payload as a string.
    pub fn on_receive<F>(&mut self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_receive = Some(Box::new(callback));
    }

    pub async fn next(&mut self) -> Result<OrderGatewayMessage> {
        loop {
            let msg = self
                .ws
                .next()
                .await
                .ok_or_else(|| anyhow!("ws stream ended"))??;
            match msg {
                Message::Text(text) => {
                    if let Some(ref callback) = self.on_receive {
                        callback(&text);
                    }
                    trace!("decoding order gateway message: {text}");
                    // Parse as Event first: events require a "t" tag field,
                    // so Response messages won't accidentally match as events,
                    // but the reverse is not true.
                    match serde_json::from_str::<OrderGatewayEvent>(&text) {
                        Ok(e) => {
                            self.handle_event(&e);
                            return Ok(OrderGatewayMessage::Event(e));
                        }
                        Err(e_as_event) => {
                            match serde_json::from_str::<
                                protocol::ws::Response<Box<serde_json::value::RawValue>>,
                            >(&text)
                            {
                                Ok(r) => match self.handle_response(r) {
                                    Ok(Some(res)) => return Ok(OrderGatewayMessage::Response(res)),
                                    Ok(None) => continue,
                                    Err(e_res) => {
                                        error!("handling response: {e_res:?}");
                                    }
                                },
                                Err(e_as_response) => {
                                    error!(
                                        "decoding order gateway message as event: {e_as_event:?}"
                                    );
                                    error!(
                                        "decoding order gateway message as response: {e_as_response:?}"
                                    );
                                    continue;
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
        }
    }

    fn handle_response(
        &mut self,
        res: protocol::ws::Response<Box<serde_json::value::RawValue>>,
    ) -> Result<Option<protocol::ws::Response<OrderGatewayResponse>>> {
        macro_rules! try_parse {
            ($res:expr, $type:ty, $v:path) => {
                $res.response
                    .map(|r| serde_json::from_str::<$type>(r.get()))
                    .transpose()?
                    .map(|r| $v(r))
            };
        }
        let Some(request_id) = res.request_id else {
            if let Some(err) = res.error {
                warn!("received error with unknown request_id: {}", err);
            }
            return Ok(None);
        };
        let parsed = if let Some(req_type) = self.in_flight_requests.remove(&request_id) {
            match req_type {
                OrderGatewayRequestType::PlaceOrder => {
                    try_parse!(
                        res,
                        PlaceOrderResponse,
                        OrderGatewayResponse::PlaceOrderResponse
                    )
                }
                OrderGatewayRequestType::CancelOrder => {
                    try_parse!(
                        res,
                        CancelOrderResponse,
                        OrderGatewayResponse::CancelOrderResponse
                    )
                }
                OrderGatewayRequestType::GetOrderStatus => {
                    try_parse!(
                        res,
                        GetOrderStatusResponse,
                        OrderGatewayResponse::GetOrderStatusResponse
                    )
                }
                OrderGatewayRequestType::ReplaceOrder => {
                    try_parse!(
                        res,
                        ReplaceOrderResponse,
                        OrderGatewayResponse::ReplaceOrderResponse
                    )
                }
                OrderGatewayRequestType::CancelAllOrders => {
                    try_parse!(
                        res,
                        CancelAllOrdersResponse,
                        OrderGatewayResponse::CancelAllOrdersResponse
                    )
                }
                OrderGatewayRequestType::GetOpenOrders => {
                    try_parse!(
                        res,
                        GetOpenOrdersResponse,
                        OrderGatewayResponse::GetOpenOrdersResponse
                    )
                }
                OrderGatewayRequestType::GetEstimatedFundingRate => {
                    try_parse!(
                        res,
                        crate::protocol::api_gateway::GetEstimatedFundingRateResponse,
                        OrderGatewayResponse::GetEstimatedFundingRateResponse
                    )
                }
            }
        } else {
            warn!("response to unknown request: {}", request_id);
            return Ok(None);
        };
        Ok(Some(protocol::ws::Response {
            request_id: Some(request_id),
            response: parsed,
            error: res.error,
            data: None,
        }))
    }

    fn handle_event(&mut self, e: &protocol::order_gateway::OrderGatewayEvent) {
        trace!("order gateway event: {e:?}");
        if let OrderGatewayEvent::Heartbeat(t) = e {
            debug!("heartbeat: {:?}", t.as_datetime());
        }
    }

    pub async fn get_open_orders(&mut self) -> Result<()> {
        self.get_open_orders_inner(None).await
    }

    pub async fn get_open_orders_for_account(&mut self, account_id: impl AsRef<str>) -> Result<()> {
        self.get_open_orders_inner(Some(account_id.as_ref())).await
    }

    async fn get_open_orders_inner(&mut self, account_id: Option<&str>) -> Result<()> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::GetOpenOrders(
            protocol::order_gateway::GetOpenOrdersRequest {
                account_id: self.request_account(account_id),
            },
        );
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending get open orders request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::GetOpenOrders);
        Ok(())
    }

    pub async fn get_estimated_funding_rate(&mut self, symbol: &str) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::GetEstimatedFundingRate(
            protocol::api_gateway::GetEstimatedFundingRateRequest {
                symbol: symbol.to_string(),
            },
        );
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending get estimated funding rate request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::GetEstimatedFundingRate);
        Ok(request_id)
    }

    pub async fn place_order(&mut self, mut place_order: PlaceOrder) -> Result<i32> {
        place_order.account_id = self.request_account(place_order.account_id.as_deref());
        self.place_order_inner(place_order).await
    }

    pub async fn place_order_for_account(
        &mut self,
        mut place_order: PlaceOrder,
        account_id: impl Into<String>,
    ) -> Result<i32> {
        place_order.account_id = Some(account_id.into());
        self.place_order_inner(place_order).await
    }

    async fn place_order_inner(&mut self, place_order: PlaceOrder) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::PlaceOrder(place_order.into());
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending place order request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::PlaceOrder);
        Ok(request_id)
    }

    pub async fn cancel_all_orders(&mut self, symbol: Option<&str>) -> Result<i32> {
        self.cancel_all_orders_inner(symbol, None).await
    }

    pub async fn cancel_all_orders_for_account(
        &mut self,
        symbol: Option<&str>,
        account_id: impl AsRef<str>,
    ) -> Result<i32> {
        self.cancel_all_orders_inner(symbol, Some(account_id.as_ref()))
            .await
    }

    async fn cancel_all_orders_inner(
        &mut self,
        symbol: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::CancelAllOrders(
            protocol::order_gateway::CancelAllOrdersRequest {
                symbol: symbol.map(|s| s.to_string()),
                account_id: self.request_account(account_id),
            },
        );
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending cancel all orders request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::CancelAllOrders);
        Ok(request_id)
    }

    /// Cancel an existing order identified by either `OrderId` or
    /// `ClientOrderId`.
    pub async fn cancel_order(
        &mut self,
        order: impl Into<protocol::order_gateway::OrderReference>,
    ) -> Result<i32> {
        self.cancel_order_inner(order.into(), None).await
    }

    pub async fn cancel_order_for_account(
        &mut self,
        order: impl Into<protocol::order_gateway::OrderReference>,
        account_id: impl AsRef<str>,
    ) -> Result<i32> {
        self.cancel_order_inner(order.into(), Some(account_id.as_ref()))
            .await
    }

    async fn cancel_order_inner(
        &mut self,
        order: protocol::order_gateway::OrderReference,
        account_id: Option<&str>,
    ) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::CancelOrder(
            protocol::order_gateway::CancelOrderRequest {
                order,
                account_id: self.request_account(account_id),
            },
        );
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending cancel order request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::CancelOrder);
        Ok(request_id)
    }

    pub async fn replace_order(
        &mut self,
        mut req: protocol::order_gateway::ReplaceOrderRequest,
    ) -> Result<i32> {
        req.account_id = self.request_account(req.account_id.as_deref());
        self.replace_order_inner(req).await
    }

    pub async fn replace_order_for_account(
        &mut self,
        mut req: protocol::order_gateway::ReplaceOrderRequest,
        account_id: impl Into<String>,
    ) -> Result<i32> {
        req.account_id = Some(account_id.into());
        self.replace_order_inner(req).await
    }

    async fn replace_order_inner(
        &mut self,
        req: protocol::order_gateway::ReplaceOrderRequest,
    ) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::ReplaceOrder(req);
        let wrapped_req = protocol::ws::Request {
            request_id,
            request: req,
        };
        let payload = serde_json::to_string(&wrapped_req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending replace order request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::ReplaceOrder);
        Ok(request_id)
    }
}
