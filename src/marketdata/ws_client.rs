use crate::protocol::marketdata_publisher::*;
use crate::{
    constants::{PING_INTERVAL, READ_TIMEOUT},
    protocol::{
        marketdata_publisher::{MarketdataRequest, SubscriptionLevel},
        ws::Request as WsRequest,
    },
    routing::extract_id,
    types::{
        trading::CandleWidth,
        ws::{InternalCommand, TokenRefreshFn, WsClientError},
    },
};
use futures::SinkExt;
use futures::StreamExt;
use log::{error, info, trace, warn};
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, UnboundedSender},
        watch, Mutex,
    },
    task::JoinHandle,
    time::{interval, sleep, Instant, MissedTickBehavior},
};
use url::Url;
use yawc::{Frame, MaybeTlsStream, OpCode, Options, WebSocket};

/// Type alias for backwards compatibility.
pub type ClientError = WsClientError;

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

pub struct MarketdataWsClient {
    write_tx: UnboundedSender<InternalCommand>,
    pub connection_state_rx: watch::Receiver<ConnectionState>,
    pub market_data_receiver: mpsc::Receiver<MarketdataEvent>,
    subscriptions: Arc<tokio::sync::RwLock<Vec<Subscription>>>,
    next_request_id: i32,
    shutdown_tx: watch::Sender<bool>,
    supervisor_handle: Arc<Mutex<JoinHandle<()>>>,
    current_connection_state: Arc<Mutex<ConnectionState>>,
}

/// An independent handle for watching connection state changes.
/// Obtain one via [`MarketdataWsClient::state_watcher`]. It holds no
/// reference to the client, so it can be used freely inside `tokio::select!`
/// alongside mutable borrows of the client (e.g. `market_data_receiver.recv()`).
pub use crate::types::ws::ConnectionState;
pub use crate::ws_utils::ConnectionStateWatcher;

impl MarketdataWsClient {
    /// Connect to the marketdata websocket using the standard path derivation.
    /// This joins `md/ws` to the base URL (e.g., `http://example.com` -> `ws://example.com/md/ws`).
    pub async fn connect(
        base_url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        let mut ws_base_url = base_url.clone();
        match base_url.scheme() {
            "http" => ws_base_url
                .set_scheme("ws")
                .map_err(|_| ClientError::InvalidScheme)?,
            "https" => ws_base_url
                .set_scheme("wss")
                .map_err(|_| ClientError::InvalidScheme)?,
            _ => return Err(ClientError::InvalidScheme),
        };
        let md_url = ws_base_url.join("md/ws")?;
        Self::connect_to_url(md_url, token_refresh).await
    }

    /// Connect to a marketdata websocket at a specific URL.
    /// Use this for integration tests or custom deployments where the standard
    /// path derivation doesn't apply.
    pub async fn connect_to_url(
        mut url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        match url.scheme() {
            "http" => url
                .set_scheme("ws")
                .map_err(|_| ClientError::InvalidScheme)?,
            "https" => url
                .set_scheme("wss")
                .map_err(|_| ClientError::InvalidScheme)?,
            "ws" | "wss" => {}
            _ => return Err(ClientError::InvalidScheme),
        };

        info!("connecting to {}", url);

        let (market_data_sender, market_data_receiver) = mpsc::channel(100);
        let (write_tx, write_rx) = mpsc::unbounded_channel::<InternalCommand>();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (connection_state_tx, connection_state_rx) =
            watch::channel::<ConnectionState>(ConnectionState::Disconnected);

        let subscriptions = Arc::new(tokio::sync::RwLock::new(Vec::new()));

        let supervisor_handle = tokio::spawn(connection_supervisor(
            url.to_string(),
            token_refresh,
            write_rx,
            shutdown_rx,
            market_data_sender,
            subscriptions.clone(),
            connection_state_tx,
        ));

        Ok(Self {
            write_tx,
            connection_state_rx,
            market_data_receiver,
            subscriptions,
            next_request_id: 1,
            shutdown_tx,
            supervisor_handle: Arc::new(Mutex::new(supervisor_handle)),
            current_connection_state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        })
    }

    /// Returns an independent [`ConnectionStateWatcher`] that can be used
    /// inside `tokio::select!` alongside mutable borrows of this client.
    pub fn state_watcher(&self) -> ConnectionStateWatcher {
        ConnectionStateWatcher::new(
            self.connection_state_rx.clone(),
            self.current_connection_state.clone(),
        )
    }

    async fn send_raw(&self, payload: String) -> Result<(), ClientError> {
        self.write_tx
            .send(InternalCommand::Send(Frame::text(payload)))
            .map_err(|e| ClientError::Transport(Box::new(e)))
    }

    pub async fn subscribe(
        &mut self,
        symbol: impl AsRef<str>,
        level: SubscriptionLevel,
    ) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::Level {
                symbol: symbol_str.clone(),
                level,
            };
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
        self.send_raw(payload).await
    }

    pub async fn unsubscribe(&mut self, symbol: impl AsRef<str>) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
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
        self.send_raw(payload).await
    }

    pub async fn subscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::Candles {
                symbol: symbol_str.clone(),
                width,
            };
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
        self.send_raw(payload).await
    }

    pub async fn unsubscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
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
        self.send_raw(payload).await
    }

    pub async fn subscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
        {
            let mut subs = self.subscriptions.write().await;
            let subscription = Subscription::BboCandles {
                symbol: symbol_str.clone(),
                width,
            };
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
        self.send_raw(payload).await
    }

    pub async fn unsubscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol_str = symbol.as_ref().to_string();
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
        self.send_raw(payload).await
    }

    pub async fn shutdown(&self, reason: &'static str) -> Result<(), ClientError> {
        info!("Shutdown requested: {reason}");
        let _ = self.shutdown_tx.send(true);
        let _ = self.write_tx.send(InternalCommand::Close);
        let supervisor_handle = self.supervisor_handle.lock().await;
        supervisor_handle.abort();
        Ok(())
    }

    pub async fn wait_for_connection(&self) {
        let mut rx = self.connection_state_rx.clone();
        if *rx.borrow_and_update() == ConnectionState::Connected {
            let mut current_state = self.current_connection_state.lock().await;
            *current_state = ConnectionState::Connected;
            return;
        }
        while rx.changed().await.is_ok() {
            if *rx.borrow_and_update() == ConnectionState::Connected {
                let mut current_state = self.current_connection_state.lock().await;
                *current_state = ConnectionState::Connected;
                return;
            }
        }
    }

    pub async fn run_till_event(&self) -> ConnectionState {
        let mut rx = self.connection_state_rx.clone();
        loop {
            if rx.changed().await.is_ok() {
                let state = *rx.borrow_and_update();
                let mut current_state = self.current_connection_state.lock().await;
                if state != *current_state {
                    info!("Connection state changed to: {:?}", state);
                    *current_state = state;
                    return state;
                }
            } else {
                return ConnectionState::Exited;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions: supervisor + single-connection loop
// ---------------------------------------------------------------------------

fn subscription_to_request(
    sub: &Subscription,
    request_id: &mut i32,
) -> Result<String, serde_json::Error> {
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
    serde_json::to_string(&req)
}

async fn connection_supervisor(
    url: String,
    token_refresh: TokenRefreshFn,
    mut cmd_rx: mpsc::UnboundedReceiver<InternalCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    market_data_sender: mpsc::Sender<MarketdataEvent>,
    subscriptions: Arc<tokio::sync::RwLock<Vec<Subscription>>>,
    connection_state_tx: watch::Sender<ConnectionState>,
) {
    let mut attempts: u32 = 0;

    loop {
        info!(
            "Connection supervisor: connecting to {url} (attempt {})",
            attempts + 1
        );

        if *shutdown_rx.borrow() {
            info!("Supervisor sees shutdown signal for {url}");
            break;
        }

        let token = match token_refresh().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to refresh token for {url}: {e}");
                attempts += 1;
                if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                    break;
                }
                let backoff = std::time::Duration::from_secs(3u64.saturating_pow(attempts.min(4)));
                warn!("Retrying token refresh in {backoff:?}");
                sleep(backoff).await;
                continue;
            }
        };

        let ws_res = WebSocket::connect(url.parse().unwrap())
            .with_request(yawc::HttpRequestBuilder::new().header("Authorization", token.as_str()))
            .with_options(Options::default().with_high_compression())
            .await;

        match ws_res {
            Ok(ws) => {
                info!("Connected to {url}");
                attempts = 0;
                connection_state_tx.send(ConnectionState::Connected).ok();

                let result = run_single_connection(
                    ws,
                    &mut cmd_rx,
                    &mut shutdown_rx,
                    &market_data_sender,
                    &subscriptions,
                )
                .await;

                info!("Connection to {url} ended: {result:?}");

                match result {
                    Ok(()) => {
                        info!("Connection exited normally for {url}");
                        connection_state_tx.send(ConnectionState::Exited).ok();
                        break;
                    }
                    Err(e) => {
                        error!("Connection error on {url}: {e}");
                        if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                            break;
                        }
                        connection_state_tx.send(ConnectionState::Reconnecting).ok();
                    }
                }
            }
            Err(e) => {
                attempts += 1;
                error!("Failed to connect to {url}: {e} (attempt {attempts})");
                if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                    break;
                }
                connection_state_tx.send(ConnectionState::Disconnected).ok();
            }
        }

        let backoff = std::time::Duration::from_secs(3u64.saturating_pow(attempts.min(4)));
        warn!("Reconnecting in {backoff:?}");
        sleep(backoff).await;
        attempts = attempts.saturating_add(1);
    }

    info!("Connection supervisor exited for {url}");
}

async fn run_single_connection(
    mut ws: WebSocket<MaybeTlsStream<TcpStream>>,
    cmd_rx: &mut mpsc::UnboundedReceiver<InternalCommand>,
    shutdown_rx: &mut watch::Receiver<bool>,
    market_data_sender: &mpsc::Sender<MarketdataEvent>,
    subscriptions: &Arc<tokio::sync::RwLock<Vec<Subscription>>>,
) -> Result<(), ClientError> {
    // Replay subscriptions on (re)connect
    {
        let subs = subscriptions.read().await;
        let mut id = 1;
        for sub in subs.iter() {
            match subscription_to_request(sub, &mut id) {
                Ok(payload) => {
                    if let Err(e) = ws.send(Frame::text(payload)).await {
                        error!("Failed to replay subscription: {e}");
                        return Err(ClientError::WebsocketError(e));
                    }
                }
                Err(e) => error!("Failed to serialize subscription: {e}"),
            }
        }
    }

    let mut ping_interval = interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let read_deadline = sleep(READ_TIMEOUT);
    tokio::pin!(read_deadline);

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if let Err(e) = ws.send(Frame::ping(Vec::new())).await {
                    warn!("Failed to send ping: {e}");
                    return Err(ClientError::WebsocketError(e));
                }
                trace!("Ping sent");
            }

            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown requested.");
                    let _ = ws.close().await;
                    return Ok(());
                }
            }

            maybe_cmd = cmd_rx.recv() => {
                match maybe_cmd {
                    Some(InternalCommand::Send(frame)) => {
                        ws.send(frame).await?;
                    }
                    Some(InternalCommand::Close) => {
                        info!("Close command received");
                        let _ = ws.close().await;
                        return Ok(());
                    }
                    None => {
                        info!("Command channel closed.");
                        let _ = ws.close().await;
                        return Ok(());
                    }
                }
            }

            msg = ws.next() => {
                read_deadline.as_mut().reset(Instant::now() + READ_TIMEOUT);
                match msg {
                    None => {
                        warn!("WebSocket stream ended.");
                        return Err(ClientError::Io(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket stream ended",
                        )));
                    }
                    Some(frame) => {
                        let (opcode, _is_fin, payload) = frame.into_parts();
                        match opcode {
                            OpCode::Text => {
                                let id = extract_id(&payload);
                                if id.is_some() {
                                    trace!("received response with id: {id:?}");
                                } else {
                                    match serde_json::from_slice::<Arc<MarketdataEvent>>(&payload) {
                                        Ok(e) => {
                                            trace!("decoded marketdata event: {e:?}");
                                            if let Err(e) = market_data_sender.send((*e).clone()).await {
                                                error!("failed to forward marketdata event: {e}");
                                                return Err(ClientError::Transport(Box::new(e)));
                                            }
                                        }
                                        Err(e) => {
                                            error!("failed to decode marketdata message: {e:?}");
                                        }
                                    }
                                }
                            }
                            OpCode::Pong => {
                                trace!("Received pong");
                            }
                            OpCode::Close => {
                                info!("Received close frame from server");
                                return Err(ClientError::WebsocketError(
                                    yawc::WebSocketError::ConnectionClosed,
                                ));
                            }
                            OpCode::Ping => {
                                trace!("Received ping");
                            }
                            _ => {
                                warn!("Unsupported frame opcode: {opcode:?}");
                            }
                        }
                    }
                }
            }

            _ = &mut read_deadline => {
                warn!("WebSocket read timeout after {READ_TIMEOUT:?}");
                return Err(ClientError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "WebSocket read timeout",
                )));
            }
        }
    }
}
