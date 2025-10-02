use crate::{protocol, types::*};
use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

pub struct MarketdataWsClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    pub orderbooks: HashMap<String, Orderbook>,
}

impl MarketdataWsClient {
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

        Ok(Self {
            ws,
            next_request_id: 1,
            orderbooks: HashMap::new(),
        })
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
