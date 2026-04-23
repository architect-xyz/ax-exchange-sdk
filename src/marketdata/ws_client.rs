use crate::{
    constants::READ_TIMEOUT,
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
use log::{error, info, trace, warn};
use protocol::marketdata_publisher::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
    time::sleep,
};
use url::Url;
use yawc::{Frame, OpCode, WebSocket};

pub type SendCallback = Box<dyn Fn(&str) + Send + Sync>;
pub type ReceiveCallback = Box<dyn Fn(&str) + Send + Sync>;

// Commands that can be sent to the WebSocket task
enum WsCommand {
    Send(String),
}

// Subscription tracking
#[derive(Debug, Clone)]
enum Subscription {
    Level {
        symbol: String,
        level: SubscriptionLevel,
    },
    Candles {
        symbol: String,
        width: CandleWidth,
    },
    BboCandles {
        symbol: String,
        width: CandleWidth,
    },
}

// Reconnection configuration
struct ReconnectConfig {
    max_retries: Option<usize>,
    initial_backoff: Duration,
    max_backoff: Duration,
    backoff_multiplier: f64,
}

pub struct MarketdataWsClient {
    command_sender: Sender<WsCommand>,
    next_request_id: i32,
    pub market_data_receiver: Receiver<MarketdataEvent>,
    subscriptions: Arc<tokio::sync::RwLock<Vec<Subscription>>>,
    #[allow(dead_code)] // Kept for future graceful shutdown implementation
    task_handle: Option<JoinHandle<()>>,
}

impl ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: None, // Infinite retries
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }

    fn calculate_backoff(&self, attempt: usize) -> Duration {
        let backoff_ms =
            self.initial_backoff.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);
        let backoff_ms = backoff_ms.min(self.max_backoff.as_millis() as f64);
        Duration::from_millis(backoff_ms as u64)
    }
}

impl MarketdataWsClient {
    /// Connect to the marketdata websocket using the standard path derivation.
    /// This joins `md/ws` to the base URL (e.g., `http://example.com` -> `ws://example.com/md/ws`).
    pub async fn connect(base_url: Url, token: impl AsRef<str> + Send + 'static) -> Result<Self> {
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
    pub async fn connect_to_url(
        mut url: Url,
        token: impl AsRef<str> + Send + 'static,
    ) -> Result<Self> {
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

        let (market_data_sender, market_data_receiver) = tokio::sync::mpsc::channel(100);
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel(100);

        let subscriptions = Arc::new(tokio::sync::RwLock::new(Vec::new()));

        Ok(Self {
            command_sender,
            next_request_id: 1,
            market_data_receiver,
            subscriptions: subscriptions.clone(),
            task_handle: Some(Self::spawn_ws_task(
                url,
                token,
                command_receiver,
                market_data_sender,
                subscriptions,
            )),
        })
    }

    // Spawn a task to manage the WebSocket connection with automatic reconnection
    fn spawn_ws_task(
        url: Url,
        token: impl AsRef<str> + Send + 'static,
        mut command_receiver: Receiver<WsCommand>,
        market_data_sender: Sender<MarketdataEvent>,
        subscriptions: Arc<tokio::sync::RwLock<Vec<Subscription>>>,
    ) -> JoinHandle<()> {
        let token = token.as_ref().to_string();

        tokio::spawn(async move {
            let reconnect_config = ReconnectConfig::default();
            let mut attempt = 0;

            loop {
                // Attempt to connect
                info!("attempting to connect to {} (attempt {})", url, attempt + 1);

                let ws_res = WebSocket::connect(url.to_string().parse().unwrap())
                    .with_request(yawc::HttpRequestBuilder::new().header("Authorization", &token))
                    .await;

                let mut ws = match ws_res {
                    Ok(ws) => {
                        info!("successfully connected to {}", url);
                        attempt = 0; // Reset attempt counter on successful connection
                        ws
                    }
                    Err(e) => {
                        error!("failed to connect to {}: {}", url, e);

                        // Check if we should retry
                        if let Some(max) = reconnect_config.max_retries {
                            if attempt >= max {
                                error!("max reconnection attempts reached, giving up");
                                break;
                            }
                        }

                        // Calculate backoff and wait
                        let backoff = reconnect_config.calculate_backoff(attempt);
                        warn!("retrying in {:?}", backoff);
                        sleep(backoff).await;
                        attempt += 1;
                        continue;
                    }
                };

                // Replay all subscriptions after successful connection
                {
                    let subs = subscriptions.read().await;
                    if !subs.is_empty() {
                        info!("replaying {} subscriptions", subs.len());
                    }
                    for sub in subs.iter() {
                        let payload = match Self::subscription_to_request(sub, &mut 1) {
                            Ok(p) => p,
                            Err(e) => {
                                error!("failed to serialize subscription request: {}", e);
                                continue;
                            }
                        };

                        if let Err(e) = ws.send(Frame::text(payload)).await {
                            error!("failed to replay subscription: {}", e);
                            break;
                        }
                    }
                }

                // Main event loop for this connection
                let disconnect_reason = loop {
                    tokio::select! {
                        // Handle incoming messages from the WebSocket
                        frame_result = ws.next() => {
                            let frame = match frame_result {
                                Some(f) => f,
                                None => {
                                    error!("ws stream ended");
                                    break "stream_ended";
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
                                                    break "send_failed";
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
                                        break "send_command_failed";
                                    }
                                }
                                None => {
                                    info!("command channel closed, shutting down ws task");
                                    return; // Exit completely, don't reconnect
                                }
                            }
                        }
                        // Timeout if no message is received
                        _ = sleep(READ_TIMEOUT) => {
                            error!("read timeout after {:?}, connection may be stale", READ_TIMEOUT);
                            break "read_timeout";
                        }
                    }
                };

                warn!(
                    "connection lost (reason: {}), will attempt to reconnect",
                    disconnect_reason
                );

                // Calculate backoff before reconnecting
                let backoff = reconnect_config.calculate_backoff(attempt);
                warn!("reconnecting in {:?}", backoff);
                sleep(backoff).await;
                attempt += 1;
            }

            info!("marketdata ws task exiting");
        })
    }

    // Helper to convert a Subscription to a JSON request payload
    fn subscription_to_request(sub: &Subscription, request_id: &mut i32) -> Result<String> {
        let req = match sub {
            Subscription::Level { symbol, level } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::Subscribe {
                    symbol: symbol.as_str(),
                    level: *level,
                },
            },
            Subscription::Candles { symbol, width } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::SubscribeCandles {
                    symbol: symbol.as_str(),
                    width: *width,
                },
            },
            Subscription::BboCandles { symbol, width } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::SubscribeBboCandles {
                    symbol: symbol.as_str(),
                    width: *width,
                },
            },
        };
        *request_id += 1;
        serde_json::to_string(&req).map_err(|e| anyhow!("serialization error: {}", e))
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
        let symbol_str = symbol.as_ref().to_string();

        // Track the subscription
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::Level {
                symbol: symbol_str.clone(),
                level,
            };
            // Avoid duplicate subscriptions
            if !subs.iter().any(|s| matches!(s, Subscription::Level { symbol: s, level: l } if s == &symbol_str && l == &level)) {
                subs.push(subscription);
            }
        }

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
        let symbol_str = symbol.as_ref().to_string();

        // Remove from tracked subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            subs.retain(
                |s| !matches!(s, Subscription::Level { symbol: s, .. } if s == &symbol_str),
            );
        }

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
        let symbol_str = symbol.as_ref().to_string();

        // Track the subscription
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::Candles {
                symbol: symbol_str.clone(),
                width,
            };
            // Avoid duplicate subscriptions
            if !subs.iter().any(|s| matches!(s, Subscription::Candles { symbol: s, width: w } if s == &symbol_str && w == &width)) {
                subs.push(subscription);
            }
        }

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
        let symbol_str = symbol.as_ref().to_string();

        // Remove from tracked subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            subs.retain(|s| !matches!(s, Subscription::Candles { symbol: s, width: w } if s == &symbol_str && w == &width));
        }

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
        let symbol_str = symbol.as_ref().to_string();

        // Track the subscription
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::BboCandles {
                symbol: symbol_str.clone(),
                width,
            };
            // Avoid duplicate subscriptions
            if !subs.iter().any(|s| matches!(s, Subscription::BboCandles { symbol: s, width: w } if s == &symbol_str && w == &width)) {
                subs.push(subscription);
            }
        }

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
        let symbol_str = symbol.as_ref().to_string();

        // Remove from tracked subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            subs.retain(|s| !matches!(s, Subscription::BboCandles { symbol: s, width: w } if s == &symbol_str && w == &width));
        }

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
