use crate::{
    protocol::{
        self,
        marketdata_publisher::{MarketdataRequest, SubscriptionLevel},
        ws::Request as WsRequest,
    },
    types::{trading::CandleWidth, *},
};
use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

pub type SendCallback = Box<dyn Fn(&str) + Send + Sync>;
pub type ReceiveCallback = Box<dyn Fn(&str) + Send + Sync>;

pub struct MarketdataWsClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    pub orderbooks: HashMap<String, Orderbook>,
    on_send: Option<SendCallback>,
    on_receive: Option<ReceiveCallback>,
}

impl MarketdataWsClient {
    /// Connect to the marketdata websocket using the standard path derivation.
    /// This joins `md/ws` to the base URL (e.g., `http://example.com` -> `ws://example.com/md/ws`).
    pub async fn connect(base_url: Url, token: impl AsRef<str>) -> Result<Self> {
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let md_url = ws_base_url.join("md/ws")?;
        Self::connect_to_url(md_url, token).await
    }

    /// Connect to a marketdata websocket at a specific URL.
    /// Use this for integration tests or custom deployments where the standard
    /// path derivation doesn't apply.
    pub async fn connect_to_url(mut url: Url, token: impl AsRef<str>) -> Result<Self> {
        // Convert http(s) to ws(s) if needed
        let res = match url.scheme() {
            "http" => url.set_scheme("ws"),
            "https" => url.set_scheme("wss"),
            "ws" | "wss" => Ok(()),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;

        let url_str = url.to_string();
        let mut request = url_str.clone().into_client_request()?;
        request
            .headers_mut()
            .insert("Authorization", token.as_ref().parse()?);

        info!("connecting to {url_str}");
        let (ws, _) = connect_async(request).await?;

        Ok(Self {
            ws,
            next_request_id: 1,
            orderbooks: HashMap::new(),
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

    /// Set a callback to be called when receiving text frames from the WebSocket.
    /// The callback receives the raw JSON payload as a string.
    pub fn on_receive<F>(&mut self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_receive = Some(Box::new(callback));
    }

    pub async fn next(
        &mut self,
    ) -> Result<Option<Arc<protocol::marketdata_publisher::MarketdataEvent>>> {
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
                trace!("decoding marketdata message: {text}");
                match serde_json::from_str::<protocol::ws::Response<Box<serde_json::value::RawValue>>>(
                    &text,
                ) {
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
                                error!(
                                    "decoding marketdata message as response: {e_as_response:?}"
                                );
                                return Ok(None);
                            }
                        }
                    }
                }
            }
            Message::Ping(..) => {
                trace!("ws ping received");
            }
            Message::Binary(..) | Message::Frame(..) | Message::Pong(..) | Message::Close(..) => {}
        }
        Ok(None)
    }

    fn handle_event(&mut self, e: &protocol::marketdata_publisher::MarketdataEvent) -> Result<()> {
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
            MarketdataEvent::Trade(_t) => {
                // TODO
            }
            MarketdataEvent::Candle(_c) => {
                // TODO
            }
        }
        Ok(())
    }

    // CR alee: also send an unsubscribe (only subscribe one level per symbol
    // at a time); maybe that's just the behavior of the publisher anyways
    pub async fn subscribe(
        &mut self,
        symbol: impl AsRef<str>,
        level: SubscriptionLevel,
    ) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::Subscribe {
                symbol: symbol.as_ref(),
                level,
            },
        };
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending subscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, symbol: impl AsRef<str>) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::Unsubscribe {
                symbol: symbol.as_ref(),
            },
        };
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending unsubscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }

    pub async fn subscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::SubscribeCandles {
                symbol: symbol.as_ref(),
                width,
            },
        };
        self.next_request_id += 1;

        let payload = serde_json::to_string(&req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending candle subscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }

    pub async fn unsubscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::UnsubscribeCandles {
                symbol: symbol.as_ref(),
                width,
            },
        };
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        if let Some(ref callback) = self.on_send {
            callback(&payload);
        }
        trace!("sending candle unsubscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }
}
