use crate::protocol::{self, order_gateway::*};
use crate::types::PlaceOrder;
use crate::OrderId;
use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use tokio::net::TcpStream;
use url::Url;
use yawc::{Frame, MaybeTlsStream, OpCode, WebSocket};

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
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    in_flight_requests: HashMap<i32, OrderGatewayRequestType>,
    on_send: Option<SendCallback>,
    on_receive: Option<ReceiveCallback>,
}

impl OrderGatewayWsClient {
    /// Connect to an order gateway and login with the provided credentials.
    pub async fn connect(base_url: Url, token: impl AsRef<str>) -> Result<Self> {
        Self::connect_inner(base_url, token, None).await
    }

    /// Connect to an order gateway with cancel-on-disconnect enabled.
    ///
    /// When the connection closes, the gateway will cancel all orders
    /// placed on this session.
    pub async fn connect_with_cancel_on_disconnect(
        base_url: Url,
        token: impl AsRef<str>,
    ) -> Result<Self> {
        Self::connect_inner(base_url, token, Some("cancel_on_disconnect=true")).await
    }

    async fn connect_inner(
        base_url: Url,
        token: impl AsRef<str>,
        query: Option<&str>,
    ) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let mut order_gateway_url = ws_base_url.join("orders/ws")?;
        if let Some(q) = query {
            order_gateway_url.set_query(Some(q));
        }

        // Add token as query parameter since yawc doesn't support custom headers directly
        // TODO: Check if the server supports authorization via query params,
        // otherwise may need to use reqwest feature or send auth after connection
        order_gateway_url
            .query_pairs_mut()
            .append_pair("token", token.as_ref());

        // Create WebSocket connection with Authorization header
        let ws = WebSocket::connect(order_gateway_url.to_string().parse()?)
            .with_request(yawc::HttpRequestBuilder::new().header("Authorization", token.as_ref()))
            .await?;
        // connect to order gateway
        info!("connecting to {order_gateway_url}");

        Ok(Self {
            ws,
            next_request_id: 1,
            in_flight_requests: HashMap::new(),
            on_send: None,
            on_receive: None,
        })
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
            let frame = self
                .ws
                .next()
                .await
                .ok_or_else(|| anyhow!("ws stream ended"))?;

            let (opcode, _is_fin, payload) = frame.into_parts();

            match opcode {
                OpCode::Text => {
                    let text = std::str::from_utf8(&payload)
                        .map_err(|e| anyhow!("invalid UTF-8 in text frame: {}", e))?;

                    if let Some(ref callback) = self.on_receive {
                        callback(text);
                    }
                    trace!("decoding order gateway message: {text}");
                    // Parse as Event first: events require a "t" tag field,
                    // so Response messages won't accidentally match as events,
                    // but the reverse is not true.
                    match serde_json::from_str::<OrderGatewayEvent>(text) {
                        Ok(e) => {
                            self.handle_event(&e);
                            return Ok(OrderGatewayMessage::Event(e));
                        }
                        Err(e_as_event) => {
                            match serde_json::from_str::<
                                protocol::ws::Response<Box<serde_json::value::RawValue>>,
                            >(text)
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
                                    error!("decoding order gateway message as response: {e_as_response:?}");
                                    continue;
                                }
                            }
                        }
                    }
                }
                OpCode::Ping => {
                    trace!("ws ping received");
                }
                OpCode::Binary | OpCode::Pong | OpCode::Close => {}
                _ => {}
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
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::GetOpenOrders(
            protocol::order_gateway::GetOpenOrdersRequest {},
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
        self.ws.send(Frame::text(payload)).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::GetOpenOrders);
        Ok(())
    }

    pub async fn place_order(&mut self, place_order: PlaceOrder) -> Result<i32> {
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
        self.ws.send(Frame::text(payload)).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::PlaceOrder);
        Ok(request_id)
    }

    pub async fn cancel_all_orders(&mut self, symbol: Option<&str>) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::CancelAllOrders(
            protocol::order_gateway::CancelAllOrdersRequest {
                symbol: symbol.map(|s| s.to_string()),
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
        self.ws.send(Frame::text(payload)).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::CancelAllOrders);
        Ok(request_id)
    }

    pub async fn cancel_order(&mut self, order_id: &OrderId) -> Result<i32> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = protocol::order_gateway::OrderGatewayRequest::CancelOrder(
            protocol::order_gateway::CancelOrderRequest {
                order_id: order_id.clone(),
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
        self.ws.send(Frame::text(payload)).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::CancelOrder);
        Ok(request_id)
    }

    pub async fn replace_order(
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
        self.ws.send(Frame::text(payload)).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::ReplaceOrder);
        Ok(request_id)
    }
}
