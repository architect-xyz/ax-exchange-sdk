use crate::{protocol, types::*};
use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;

#[derive(Clone)]
pub struct ArchitectX {
    base_url: Url,
    rest_client: reqwest::Client,
    username: Option<String>,
    password: Option<String>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl ArchitectX {
    // CR alee: default empty construction, use builder pattern
    pub fn new(base_url: Url, username: Option<&str>, password: Option<&str>) -> Self {
        Self {
            base_url,
            rest_client: reqwest::Client::new(),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
            user_token: Arc::new(ArcSwapOption::const_empty()),
        }
    }

    async fn request<B: Into<reqwest::Body>, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
    ) -> Result<R> {
        let url = self.base_url.join(path)?;
        let mut req = self.rest_client.request(method, url.clone());
        if let Some(body) = body {
            req = req.body(body);
        }
        let res = req.send().await?;
        if res.status().is_success() {
            let body = res.text().await?;
            let t: Result<R> = serde_json::from_str(&body).map_err(|e| anyhow!(e));
            if t.is_err() {
                trace!("could not parse response text: {}", body);
            }
            t
        } else {
            let status_code = res.status().as_u16();
            let status_reason = res.status().canonical_reason().unwrap_or("unknown");
            let err_body = res.text().await?;
            trace!("response error body: {}", err_body);
            bail!("request {url} failed: {status_code} {status_reason}");
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
        let req = protocol::auth_gateway::GetUserTokenRequest {
            username: username.as_ref().to_string(),
            password: password.as_ref().to_string(),
            expiration_seconds,
        };
        let req = serde_json::to_string(&req)?;
        let res: protocol::auth_gateway::GetUserTokenResponse =
            self.request(Method::POST, "auth/get_user_token", Some(req)).await?;
        Ok(res.token)
    }

    pub async fn order_gateway_client(&self) -> Result<OrderGatewayClient> {
        let username =
            self.username.as_ref().ok_or_else(|| anyhow!("no username provided"))?;
        let token = self.refresh_user_token(false).await?;
        OrderGatewayClient::connect(self.base_url.clone(), username, token).await
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
        let order_gateway_url = ws_base_url.join("orders_new/ws/orders")?.to_string();

        // connect to order gateway
        info!("connecting to {}", order_gateway_url);
        let (mut ws, _) = connect_async(order_gateway_url).await?;

        // send login request
        let req = json!({
            "rid": 1,
            "t": "a",
            "u": username.as_ref().to_string(),
            "k": token.as_ref().to_string(),
        });
        let payload = serde_json::to_string(&req)?;
        trace!("sending login request: {}", payload);
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
        } else if let Some(_) = self.pending_requests.remove(&res.request_id) {
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
        trace!("order gateway event: {:?}", e);
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
                    warn!("order not found in open orders: {:?}", order);
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
        trace!("sending place order request: {}", payload);
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
        trace!("sending cancel order request: {}", payload);
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
