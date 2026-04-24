//! Reusable WebSocket utilities: connection state watcher.

use crate::types::ws::ConnectionState;
use log::info;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

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
