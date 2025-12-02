use crate::protocol::{self, order_gateway::*};
use crate::types::PlaceOrder;
use crate::OrderId;
use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::generate_key, http::Request, Message},
    MaybeTlsStream, WebSocketStream,
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
    on_send: Option<SendCallback>,
    on_receive: Option<ReceiveCallback>,
}

impl OrderGatewayWsClient {
    /// Connect to an order gateway and login with the provided credentials.
    pub async fn connect(base_url: Url, token: impl AsRef<str>) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let order_gateway_url = ws_base_url.join("orders/ws")?;

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
                            match serde_json::from_str::<OrderGatewayEvent>(&text) {
                                Ok(e) => {
                                    self.handle_event(&e);
                                    return Ok(OrderGatewayMessage::Event(e));
                                }
                                Err(e_as_event) => {
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
        let parsed = if let Some(req_type) = self.in_flight_requests.remove(&res.request_id) {
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
                OrderGatewayRequestType::GetOpenOrders => {
                    try_parse!(
                        res,
                        GetOpenOrdersResponse,
                        OrderGatewayResponse::GetOpenOrdersResponse
                    )
                }
            }
        } else {
            warn!("response to unknown request: {}", res.request_id);
            return Ok(None);
        };
        Ok(Some(protocol::ws::Response {
            request_id: res.request_id,
            response: parsed,
            error: res.error,
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
        self.ws.send(Message::Text(payload.into())).await?;
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
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::PlaceOrder);
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
        self.ws.send(Message::Text(payload.into())).await?;
        self.in_flight_requests
            .insert(request_id, OrderGatewayRequestType::CancelOrder);
        Ok(request_id)
    }
}
