use crate::{
    protocol::{
        self,
        marketdata_publisher::{MarketdataRequest, SubscriptionLevel},
        ws::Request as WsRequest,
    },
    routing::extract_id,
    types::trading::CandleWidth,
};
use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use log::{error, info, trace};
use protocol::marketdata_publisher::*;
use std::sync::Arc;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use url::Url;
use yawc::{Frame, OpCode, WebSocket};

pub type SendCallback = Box<dyn Fn(&str) + Send + Sync>;
pub type ReceiveCallback = Box<dyn Fn(&str) + Send + Sync>;

// Commands that can be sent to the WebSocket task
enum WsCommand {
    Send(String),
}

pub struct MarketdataWsClient {
    command_sender: Sender<WsCommand>,
    next_request_id: i32,
    pub market_data_receiver: Receiver<MarketdataEvent>,
    #[allow(dead_code)] // Kept for future graceful shutdown implementation
    task_handle: Option<JoinHandle<()>>,
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

        info!("connecting to {}", url);

        // Create WebSocket connection with Authorization header
        let ws = WebSocket::connect(url.to_string().parse()?)
            .with_request(yawc::HttpRequestBuilder::new().header("Authorization", token.as_ref()))
            .await?;

        let (market_data_sender, market_data_receiver) = tokio::sync::mpsc::channel(100);
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(100);

        Ok(Self {
            command_sender,
            next_request_id: 1,
            market_data_receiver,
            task_handle: Some(Self::spawn_ws_task(
                ws,
                command_receiver,
                market_data_sender,
            )),
        })
    }

    // Spawn a task to manage the WebSocket connection
    fn spawn_ws_task(
        mut ws: WebSocket<yawc::MaybeTlsStream<tokio::net::TcpStream>>,
        mut command_receiver: Receiver<WsCommand>,
        market_data_sender: Sender<MarketdataEvent>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle incoming messages from the WebSocket
                    frame_result = ws.next() => {
                        let frame = match frame_result {
                            Some(f) => f,
                            None => {
                                error!("ws stream ended");
                                break;
                            }
                        };

                        let (opcode, _is_fin, payload) = frame.into_parts();
                        match opcode {
                            OpCode::Text => {
                                let id = extract_id(&payload);
                                if id.is_some() {
                                    trace!("decoding marketdata response with id: {:?}", id);
                                } else {
                                    let text = match std::str::from_utf8(&payload) {
                                        Ok(t) => t,
                                        Err(e) => {
                                            error!("invalid UTF-8 in text frame: {}", e);
                                            continue;
                                        }
                                    };
                                    trace!("decoding marketdata message: {text}");
                                    match serde_json::from_str::<Arc<protocol::marketdata_publisher::MarketdataEvent>>(
                                        text,
                                    ) {
                                        Ok(e) => {
                                            trace!("decoded marketdata event: {:?}", e);
                                            if let Err(e) = market_data_sender.send((*e).clone()).await {
                                                error!("failed to send marketdata event: {}", e);
                                                break;
                                            }
                                        }
                                        Err(e_as_event) => {
                                            error!("decoding marketdata message: {e_as_event:?}");
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
                    // Handle commands sent to the WebSocket
                    command = command_receiver.recv() => {
                        match command {
                            Some(WsCommand::Send(payload)) => {
                                if let Err(e) = ws.send(Frame::text(payload)).await {
                                    error!("failed to send ws message: {}", e);
                                    break;
                                }
                            }
                            None => {
                                info!("command channel closed, shutting down ws task");
                                break;
                            }
                        }
                    }
                }
            }
            info!("marketdata ws task exiting");
        })
    }

    // Helper method to send messages via the command channel
    async fn send_message(&self, payload: String) -> Result<()> {
        self.command_sender
            .send(WsCommand::Send(payload))
            .await
            .map_err(|e| anyhow!("failed to send command: {}", e))
    }

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
        trace!("sending subscribe request: {payload}");
        self.send_message(payload).await?;
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
        trace!("sending unsubscribe request: {payload}");
        self.send_message(payload).await?;
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
        trace!("sending candle subscribe request: {payload}");
        self.send_message(payload).await?;
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
        trace!("sending candle unsubscribe request: {payload}");
        self.send_message(payload).await?;
        Ok(())
    }

    pub async fn subscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::SubscribeBboCandles {
                symbol: symbol.as_ref(),
                width,
            },
        };
        self.next_request_id += 1;

        let payload = serde_json::to_string(&req)?;
        trace!("sending bbo candle subscribe request: {payload}");
        self.send_message(payload).await?;
        Ok(())
    }

    pub async fn unsubscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<()> {
        let req = WsRequest {
            request_id: self.next_request_id,
            request: MarketdataRequest::UnsubscribeBboCandles {
                symbol: symbol.as_ref(),
                width,
            },
        };
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        trace!("sending bbo candle unsubscribe request: {payload}");
        self.send_message(payload).await?;
        Ok(())
    }
}
