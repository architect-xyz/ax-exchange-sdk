//! Reusable WebSocket utilities: connection state watcher, supervisor, single-connection loop.

use crate::{
    constants::{PING_INTERVAL, READ_TIMEOUT},
    routing::extract_id,
    types::ws::{ConnectionState, InternalCommand, TokenRefreshFn, WsClientError},
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use log::{error, info, trace, warn};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, watch, Mutex},
    time::{interval, sleep, Instant, MissedTickBehavior},
};
use yawc::{Frame, MaybeTlsStream, OpCode, Options, WebSocket};

// ---------------------------------------------------------------------------
// Pending requests map
// ---------------------------------------------------------------------------

/// Shared map of in-flight request IDs → oneshot response senders.
/// Pass an `Arc::new(DashMap::new())` from each client; responses whose
/// `rid` field matches an entry are routed directly to the waiting caller.
pub type PendingRequests = Arc<DashMap<i32, oneshot::Sender<Vec<u8>>>>;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// A subscription that knows how to serialize itself into a WS request payload.
///
/// Implement this on your subscription enum/struct so that the generic
/// supervisor can replay subscriptions on reconnect.
pub trait WsSubscription: Send + Sync + 'static {
    fn to_request(&self, request_id: &mut i32) -> Result<String, serde_json::Error>;
}

// ---------------------------------------------------------------------------
// ConnectionStateWatcher
// ---------------------------------------------------------------------------

/// An independent handle for watching connection state changes.
///
/// Obtain one via a client's `state_watcher()` method. It holds no reference
/// to the client itself, so it can be freely used inside `tokio::select!`
/// alongside mutable borrows of the client.
pub struct ConnectionStateWatcher {
    pub rx: watch::Receiver<ConnectionState>,
    pub current: Arc<Mutex<ConnectionState>>,
}

impl ConnectionStateWatcher {
    pub fn new(rx: watch::Receiver<ConnectionState>, current: Arc<Mutex<ConnectionState>>) -> Self {
        Self { rx, current }
    }

    /// Resolves the next time the connection state changes to a new value.
    pub async fn run_till_event(&mut self) -> ConnectionState {
        loop {
            if self.rx.changed().await.is_ok() {
                let state = *self.rx.borrow_and_update();
                let mut cur = self.current.lock().await;
                if state != *cur {
                    info!("Connection state changed to: {:?}", state);
                    *cur = state;
                    return state;
                }
            } else {
                return ConnectionState::Exited;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic supervisor + single-connection loop
// ---------------------------------------------------------------------------

/// Supervises a WebSocket connection, reconnecting on failure.
///
/// - `E` — the event type deserialized from incoming text frames.
/// - `S` — the subscription type; must implement [`WsSubscription`] so
///   subscriptions can be replayed after each reconnect.
#[allow(clippy::too_many_arguments)]
pub async fn connection_supervisor<E, S>(
    url: String,
    token_refresh: TokenRefreshFn,
    mut cmd_rx: mpsc::UnboundedReceiver<InternalCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    event_sender: mpsc::Sender<E>,
    subscriptions: Arc<tokio::sync::RwLock<Vec<S>>>,
    connection_state_tx: watch::Sender<ConnectionState>,
    pending_requests: PendingRequests,
) where
    E: DeserializeOwned + Clone + Send + Sync + 'static,
    S: WsSubscription,
{
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
                    &event_sender,
                    &subscriptions,
                    &pending_requests,
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
                        connection_state_tx.send(ConnectionState::Disconnected).ok();
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to {url}: {e} (attempt {attempts})");
                if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                    break;
                }
                connection_state_tx.send(ConnectionState::Disconnected).ok();
            }
        }

        attempts += 1;
        let sleep_time = 2u64.pow(attempts);
        let backoff = std::time::Duration::from_secs(sleep_time);
        warn!("Reconnecting in {backoff:?}");
        sleep(backoff).await;
    }

    info!("Connection supervisor exited for {url}");
}

/// Drives a single WebSocket connection until it closes or an error occurs.
///
/// Replays all current `subscriptions` on entry (i.e. after a reconnect),
/// then processes the ping timer, shutdown signal, outbound commands, and
/// inbound frames in a `tokio::select!` loop.
pub async fn run_single_connection<E, S>(
    mut ws: WebSocket<MaybeTlsStream<TcpStream>>,
    cmd_rx: &mut mpsc::UnboundedReceiver<InternalCommand>,
    shutdown_rx: &mut watch::Receiver<bool>,
    event_sender: &mpsc::Sender<E>,
    subscriptions: &Arc<tokio::sync::RwLock<Vec<S>>>,
    pending_requests: &PendingRequests,
) -> Result<(), WsClientError>
where
    E: DeserializeOwned + Clone + Send + Sync + 'static,
    S: WsSubscription,
{
    // Replay subscriptions on (re)connect
    {
        let subs = subscriptions.read().await;
        let mut id = 1;
        for sub in subs.iter() {
            match sub.to_request(&mut id) {
                Ok(payload) => {
                    if let Err(e) = ws.send(Frame::text(payload)).await {
                        error!("Failed to replay subscription: {e}");
                        return Err(WsClientError::WebsocketError(e));
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
                    return Err(WsClientError::WebsocketError(e));
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
                        return Err(WsClientError::Io(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket stream ended",
                        )));
                    }
                    Some(frame) => {
                        let (opcode, _is_fin, payload) = frame.into_parts();
                        match opcode {
                            OpCode::Text => {
                                let id = extract_id(&payload);
                                if let Some(id) = id {
                                    trace!("received response with id: {id}");
                                    let id = id as i32;
                                    if let Some((_, tx)) = pending_requests.remove(&id) {
                                        let _ = tx.send(payload.to_vec());
                                    } else {
                                        warn!("response for unknown request id: {id}");
                                        warn!("payload: {:?}", String::from_utf8_lossy(&payload));
                                    }
                                } else {
                                    match serde_json::from_slice::<E>(&payload) {
                                        Ok(event) => {
                                            trace!("decoded event");
                                            if let Err(e) = event_sender.send(event).await {
                                                error!("failed to forward event: {e}");
                                                return Err(WsClientError::Transport(Box::new(e)));
                                            }
                                        }
                                        Err(e) => {
                                            error!("failed to decode message: {e:?}");
                                        }
                                    }
                                }
                            }
                            OpCode::Pong => {
                                trace!("Received pong");
                            }
                            OpCode::Close => {
                                info!("Received close frame from server");
                                return Err(WsClientError::WebsocketError(
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
                return Err(WsClientError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "WebSocket read timeout",
                )));
            }
        }
    }
}
