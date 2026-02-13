use pyo3::prelude::*;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use serde_json::Value;
use std::collections::HashSet;
use tokio::time::{sleep, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn, error};

use crate::model::market_data::{Ticker, Depth, TradesMessage};
use crate::model::orderbook::OrderBook;
use crate::rate_limit::TokenBucket;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct RakutenwDataClient {
    data_callback: Arc<std::sync::Mutex<Option<Py<PyAny>>>>,
    /// (channel, symbol_id) - channel is ORDERBOOK, TICKER, TRADES
    subscriptions: Arc<std::sync::Mutex<HashSet<(String, String)>>>,
    outgoing: Arc<std::sync::Mutex<Vec<String>>>,
    books: Arc<std::sync::Mutex<std::collections::HashMap<String, OrderBook>>>,
    shutdown: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    ws_rate_limit: TokenBucket,
}

#[pymethods]
impl RakutenwDataClient {
    #[new]
    pub fn new(ws_rate_limit_per_sec: Option<f64>) -> Self {
        let ws_rate = ws_rate_limit_per_sec.unwrap_or(0.5);
        Self {
            data_callback: Arc::new(std::sync::Mutex::new(None)),
            subscriptions: Arc::new(std::sync::Mutex::new(HashSet::new())),
            outgoing: Arc::new(std::sync::Mutex::new(Vec::new())),
            books: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
            connected: Arc::new(AtomicBool::new(false)),
            ws_rate_limit: TokenBucket::new(1.0, ws_rate),
        }
    }

    pub fn set_data_callback(&self, callback: Py<PyAny>) {
        let mut lock = self.data_callback.lock().unwrap();
        *lock = Some(callback);
    }

    pub fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data_cb_arc = self.data_callback.clone();
        let subs_arc = self.subscriptions.clone();
        let outgoing_arc = self.outgoing.clone();
        let books_arc = self.books.clone();
        let shutdown = self.shutdown.clone();
        let connected = self.connected.clone();
        let ws_rate_limit = self.ws_rate_limit.clone();

        shutdown.store(false, Ordering::SeqCst);
        connected.store(false, Ordering::SeqCst);

        let future = async move {
            std::thread::Builder::new()
                .name("rakutenw-ws-public".to_string())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build tokio runtime for WS");

                    rt.block_on(Self::ws_loop(
                        subs_arc, outgoing_arc, data_cb_arc, books_arc, shutdown, connected, ws_rate_limit,
                    ));
                })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to spawn WS thread: {}", e)
                ))?;

            Ok("Connected")
        };

        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    /// Subscribe to a channel for a symbol_id.
    /// channel: "ORDERBOOK", "TICKER", "TRADES"
    pub fn subscribe<'py>(&self, py: Python<'py>, channel: String, symbol_id: String) -> PyResult<Bound<'py, PyAny>> {
        let subs_arc = self.subscriptions.clone();
        let outgoing_arc = self.outgoing.clone();
        let connected = self.connected.clone();

        let future = async move {
            {
                let mut subs = subs_arc.lock().unwrap();
                subs.insert((channel.clone(), symbol_id.clone()));
            }

            if connected.load(Ordering::SeqCst) {
                let msg = Self::build_subscribe_msg(&channel, &symbol_id);
                let mut queue = outgoing_arc.lock().unwrap();
                queue.push(msg);
            }

            Ok("Subscribe command stored")
        };

        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    /// Unsubscribe from a channel for a symbol_id.
    pub fn unsubscribe<'py>(&self, py: Python<'py>, channel: String, symbol_id: String) -> PyResult<Bound<'py, PyAny>> {
        let subs_arc = self.subscriptions.clone();
        let outgoing_arc = self.outgoing.clone();
        let connected = self.connected.clone();

        let future = async move {
            {
                let mut subs = subs_arc.lock().unwrap();
                subs.remove(&(channel.clone(), symbol_id.clone()));
            }

            if connected.load(Ordering::SeqCst) {
                let msg = Self::build_unsubscribe_msg(&channel, &symbol_id);
                let mut queue = outgoing_arc.lock().unwrap();
                queue.push(msg);
            }

            Ok("Unsubscribe command stored")
        };

        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let shutdown = self.shutdown.clone();
        let future = async move {
            shutdown.store(true, Ordering::SeqCst);
            Ok("Disconnected")
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }
}

impl RakutenwDataClient {
    fn build_subscribe_msg(channel: &str, symbol_id: &str) -> String {
        serde_json::json!({
            "symbolId": symbol_id,
            "type": "subscribe",
            "data": channel,
        }).to_string()
    }

    fn build_unsubscribe_msg(channel: &str, symbol_id: &str) -> String {
        serde_json::json!({
            "symbolId": symbol_id,
            "type": "unsubscribe",
            "data": channel,
        }).to_string()
    }

    async fn ws_loop(
        subs_arc: Arc<std::sync::Mutex<HashSet<(String, String)>>>,
        outgoing_arc: Arc<std::sync::Mutex<Vec<String>>>,
        data_cb_arc: Arc<std::sync::Mutex<Option<Py<PyAny>>>>,
        books_arc: Arc<std::sync::Mutex<std::collections::HashMap<String, OrderBook>>>,
        shutdown: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        ws_rate_limit: TokenBucket,
    ) {
        let mut backoff_sec = 1u64;
        let max_backoff = 64u64;

        // Rakuten Wallet WS max connection: 2 hours
        let max_connection_duration = Duration::from_secs(7000); // ~116 minutes, reconnect before 2h

        loop {
            if shutdown.load(Ordering::SeqCst) { return; }

            let ws_url = "wss://exchange.rakuten-wallet.co.jp/ws";

            match connect_async(ws_url).await {
                Ok((mut ws, _)) => {
                    info!("RakutenW: Connected to Public WebSocket");
                    backoff_sec = 1;
                    connected.store(true, Ordering::SeqCst);
                    let connection_start = std::time::Instant::now();
                    let mut last_activity = std::time::Instant::now();

                    // Collect all messages to send
                    let mut to_send: Vec<String> = Vec::new();

                    // Stored subscriptions
                    {
                        let subs: Vec<_> = {
                            let lock = subs_arc.lock().unwrap();
                            lock.iter().cloned().collect()
                        };
                        for (channel, symbol_id) in &subs {
                            to_send.push(Self::build_subscribe_msg(channel, symbol_id));
                        }
                    }

                    // Queued outgoing messages
                    {
                        let mut queue = outgoing_arc.lock().unwrap();
                        to_send.extend(queue.drain(..));
                    }

                    to_send.sort();
                    to_send.dedup();

                    for msg in to_send {
                        ws_rate_limit.acquire().await;
                        if let Err(e) = ws.send(Message::Text(msg.into())).await {
                            error!("RakutenW: Failed to send subscribe: {}", e);
                        }
                    }

                    // Main message loop
                    loop {
                        if shutdown.load(Ordering::SeqCst) {
                            let _ = ws.send(Message::Close(None)).await;
                            connected.store(false, Ordering::SeqCst);
                            return;
                        }

                        // Reconnect before 2 hour limit
                        if connection_start.elapsed() >= max_connection_duration {
                            info!("RakutenW: Approaching 2h limit, reconnecting...");
                            let _ = ws.send(Message::Close(None)).await;
                            break;
                        }

                        // Send ping every 5 minutes to prevent 10 minute idle timeout
                        if last_activity.elapsed() >= Duration::from_secs(300) {
                            if let Err(e) = ws.send(Message::Ping(vec![].into())).await {
                                error!("RakutenW: Ping failed: {}", e);
                                break;
                            }
                            last_activity = std::time::Instant::now();
                        }

                        // Use timeout to check for pending outgoing messages periodically
                        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
                            Ok(Some(Ok(Message::Text(txt)))) => {
                                last_activity = std::time::Instant::now();

                                // Process queued outgoing
                                {
                                    let mut queue = outgoing_arc.lock().unwrap();
                                    for msg in queue.drain(..) {
                                        if let Err(e) = ws.send(Message::Text(msg.into())).await {
                                            error!("RakutenW: Failed to send msg: {}", e);
                                        }
                                    }
                                }

                                let txt_str: &str = txt.as_ref();
                                if let Ok(val) = serde_json::from_str::<Value>(txt_str) {
                                    Self::dispatch_message(val, &data_cb_arc, &books_arc);
                                }
                            }
                            Ok(Some(Ok(Message::Ping(data)))) => {
                                last_activity = std::time::Instant::now();
                                let _ = ws.send(Message::Pong(data)).await;
                            }
                            Ok(Some(Ok(Message::Close(_)))) => {
                                warn!("RakutenW: Public WS closed by server");
                                break;
                            }
                            Ok(Some(Err(e))) => {
                                error!("RakutenW: Public WS error: {}", e);
                                break;
                            }
                            Ok(None) => {
                                warn!("RakutenW: Public WS stream ended");
                                break;
                            }
                            Err(_) => {
                                // Timeout - check outgoing queue
                                let mut queue = outgoing_arc.lock().unwrap();
                                for msg in queue.drain(..) {
                                    if let Err(e) = ws.send(Message::Text(msg.into())).await {
                                        error!("RakutenW: Failed to send msg: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    connected.store(false, Ordering::SeqCst);
                }
                Err(e) => {
                    error!("RakutenW: Public WS connection failed: {}. Retrying in {}s...", e, backoff_sec);
                }
            }

            if shutdown.load(Ordering::SeqCst) { return; }
            // Add jitter (±25%) to prevent thundering herd on reconnects
            let jitter_factor = 0.75 + (rand::random::<f64>() * 0.5); // 0.75 to 1.25
            let jittered = (backoff_sec as f64 * jitter_factor) as u64;
            sleep(Duration::from_secs(jittered.max(1))).await;
            backoff_sec = (backoff_sec * 2).min(max_backoff);
        }
    }

    fn dispatch_message(
        val: Value,
        data_cb_arc: &Arc<std::sync::Mutex<Option<Py<PyAny>>>>,
        books_arc: &Arc<std::sync::Mutex<std::collections::HashMap<String, OrderBook>>>,
    ) {
        // Determine channel from the data structure
        // Rakuten WS messages contain symbolId and the data type can be inferred

        // TICKER: has bestAsk, bestBid, open, high, low, last, volume
        if val.get("bestAsk").is_some() && val.get("open").is_some() && val.get("trades").is_none() && val.get("asks").is_none() {
            if let Ok(ticker) = serde_json::from_value::<Ticker>(val) {
                Python::try_attach(|py| {
                    let lock = data_cb_arc.lock().unwrap();
                    if let Some(cb) = lock.as_ref() {
                        let py_obj = Py::new(py, ticker).expect("Failed to create Python object");
                        let _ = cb.call1(py, ("TICKER", py_obj)).ok();
                    }
                });
            }
            return;
        }

        // ORDERBOOK: has asks, bids arrays
        if val.get("asks").is_some() && val.get("bids").is_some() {
            if let Ok(depth) = serde_json::from_value::<Depth>(val) {
                let symbol_id = depth.symbol_id.clone();
                let book_clone = {
                    let mut books = books_arc.lock().unwrap();
                    let book = books.entry(symbol_id.clone())
                        .or_insert_with(|| OrderBook::new(symbol_id.clone()));
                    book.apply_snapshot(depth);
                    book.clone()
                };

                Python::try_attach(|py| {
                    let lock = data_cb_arc.lock().unwrap();
                    if let Some(cb) = lock.as_ref() {
                        let py_obj = Py::new(py, book_clone).expect("Failed to create Python object");
                        let _ = cb.call1(py, ("ORDERBOOK", py_obj)).ok();
                    }
                });
            }
            return;
        }

        // TRADES: has trades array
        if val.get("trades").is_some() {
            if let Ok(trades_msg) = serde_json::from_value::<TradesMessage>(val) {
                for trade in trades_msg.trades {
                    let trade_with_symbol = trade.clone();
                    // Ensure we have the symbol_id available
                    Python::try_attach(|py| {
                        let lock = data_cb_arc.lock().unwrap();
                        if let Some(cb) = lock.as_ref() {
                            let py_obj = Py::new(py, trade_with_symbol.clone()).expect("Failed to create Python object");
                            // Pass symbol_id as additional context
                            let _ = cb.call1(py, ("TRADES", py_obj, trades_msg.symbol_id.clone())).ok();
                        }
                    });
                }
            }
        }
    }
}
