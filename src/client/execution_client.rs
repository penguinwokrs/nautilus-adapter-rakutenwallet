use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use pyo3::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};
use crate::client::rest::RakutenwRestClient;

/// Rakuten Wallet Execution Client.
///
/// Since Rakuten Wallet does not provide a Private WebSocket,
/// this client uses REST API polling to monitor order status changes.
#[pyclass]
pub struct RakutenwExecutionClient {
    rest_client: RakutenwRestClient,
    order_callback: Arc<std::sync::Mutex<Option<Py<PyAny>>>>,
    /// Tracked orders: order_id -> last known status
    tracked_orders: Arc<RwLock<HashMap<String, String>>>,
    /// client_order_id -> venue_order_id mapping
    client_oid_map: Arc<RwLock<HashMap<String, String>>>,
    shutdown: Arc<AtomicBool>,
    poll_interval_ms: u64,
}

#[pymethods]
impl RakutenwExecutionClient {
    #[new]
    #[pyo3(signature = (api_key, api_secret, timeout_ms, proxy_url=None, rate_limit_per_sec=None, poll_interval_ms=None))]
    pub fn new(
        api_key: String,
        api_secret: String,
        timeout_ms: u64,
        proxy_url: Option<String>,
        rate_limit_per_sec: Option<f64>,
        poll_interval_ms: Option<u64>,
    ) -> Self {
        Self {
            rest_client: RakutenwRestClient::new(api_key, api_secret, timeout_ms, proxy_url, rate_limit_per_sec),
            order_callback: Arc::new(std::sync::Mutex::new(None)),
            tracked_orders: Arc::new(RwLock::new(HashMap::new())),
            client_oid_map: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: poll_interval_ms.unwrap_or(1000),
        }
    }

    pub fn set_order_callback(&self, callback: Py<PyAny>) {
        let mut lock = self.order_callback.lock().unwrap();
        *lock = Some(callback);
    }

    /// Start the order polling loop in a background thread
    pub fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let order_cb_arc = self.order_callback.clone();
        let tracked_orders = self.tracked_orders.clone();
        let shutdown = self.shutdown.clone();
        let poll_interval_ms = self.poll_interval_ms;

        shutdown.store(false, Ordering::SeqCst);

        let future = async move {
            std::thread::Builder::new()
                .name("rakutenw-order-poll".to_string())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build tokio runtime for order polling");

                    rt.block_on(Self::poll_loop(
                        rest_client, order_cb_arc, tracked_orders, shutdown, poll_interval_ms,
                    ));
                })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to spawn order polling thread: {}", e)
                ))?;

            Ok("Connected")
        };

        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    // ========== Order Operations (Python) ==========

    #[pyo3(signature = (symbol_id, order_pattern, order_behavior, order_side, order_type, amount, client_order_id, price=None, leverage=None, close_behavior=None, post_only=None, order_expire=None, position_id=None, ifd_close_limit_price=None, ifd_close_stop_price=None, oco_order_type_1=None, oco_price_1=None, oco_order_type_2=None, oco_price_2=None))]
    pub fn submit_order<'py>(
        &self,
        py: Python<'py>,
        symbol_id: String,
        order_pattern: String,
        order_behavior: String,
        order_side: String,
        order_type: String,
        amount: String,
        client_order_id: String,
        price: Option<String>,
        leverage: Option<String>,
        close_behavior: Option<String>,
        post_only: Option<bool>,
        order_expire: Option<String>,
        position_id: Option<String>,
        ifd_close_limit_price: Option<String>,
        ifd_close_stop_price: Option<String>,
        oco_order_type_1: Option<String>,
        oco_price_1: Option<String>,
        oco_order_type_2: Option<String>,
        oco_price_2: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let client_oid_map = self.client_oid_map.clone();
        let tracked_orders = self.tracked_orders.clone();

        let future = async move {
            let res = rest_client
                .submit_order(
                    &symbol_id, &order_pattern, &order_behavior, &order_side, &order_type, &amount,
                    price.as_deref(), leverage.as_deref(), close_behavior.as_deref(), post_only,
                    order_expire.as_deref(), position_id.as_deref(),
                    ifd_close_limit_price.as_deref(), ifd_close_stop_price.as_deref(),
                    oco_order_type_1.as_deref(), oco_price_1.as_deref(),
                    oco_order_type_2.as_deref(), oco_price_2.as_deref(),
                )
                .await
                .map_err(PyErr::from)?;

            // Extract order ID from response
            let order_id = res.get("id")
                .or_else(|| res.get("orderId"))
                .and_then(|v| v.as_str().or_else(|| v.as_i64().map(|_| "")).or(Some("")))
                .map(|s| {
                    if s.is_empty() {
                        res.get("id").or_else(|| res.get("orderId"))
                            .and_then(|v| v.as_i64())
                            .map(|n| n.to_string())
                            .unwrap_or_default()
                    } else {
                        s.to_string()
                    }
                })
                .unwrap_or_default();

            if !order_id.is_empty() {
                let mut map = client_oid_map.write().await;
                map.insert(client_order_id, order_id.clone());

                let mut orders = tracked_orders.write().await;
                orders.insert(order_id.clone(), "WORKING_ORDER".to_string());
            }

            let result = serde_json::json!({"order_id": order_id});
            serde_json::to_string(&result)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn cancel_order<'py>(&self, py: Python<'py>, symbol_id: String, order_id: String) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let future = async move {
            let res = rest_client.cancel_order(&symbol_id, &order_id).await.map_err(PyErr::from)?;
            serde_json::to_string(&res)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id, order_pattern, order_id, order_type, price, amount, ifd_close_limit_price=None, ifd_close_stop_price=None))]
    pub fn modify_order<'py>(
        &self,
        py: Python<'py>,
        symbol_id: String,
        order_pattern: String,
        order_id: String,
        order_type: String,
        price: String,
        amount: String,
        ifd_close_limit_price: Option<String>,
        ifd_close_stop_price: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let future = async move {
            let res = rest_client.modify_order(
                &symbol_id, &order_pattern, &order_id, &order_type, &price, &amount,
                ifd_close_limit_price.as_deref(), ifd_close_stop_price.as_deref(),
            ).await.map_err(PyErr::from)?;
            serde_json::to_string(&res)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id=None, order_status=None))]
    pub fn get_orders<'py>(
        &self,
        py: Python<'py>,
        symbol_id: Option<String>,
        order_status: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let future = async move {
            let sid_ref = symbol_id.as_deref();
            let statuses: Vec<String> = order_status.unwrap_or_default();
            let status_refs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();
            let status_opt = if status_refs.is_empty() { None } else { Some(status_refs.as_slice()) };

            let res = rest_client.get_orders(sid_ref, status_opt).await.map_err(PyErr::from)?;
            serde_json::to_string(&res)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_trades_for_order<'py>(&self, py: Python<'py>, order_id: String) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let future = async move {
            let res = rest_client.get_trades_for_order(&order_id).await.map_err(PyErr::from)?;
            serde_json::to_string(&res)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_positions<'py>(&self, py: Python<'py>, symbol_id: Option<String>) -> PyResult<Bound<'py, PyAny>> {
        let rest_client = self.rest_client.clone();
        let future = async move {
            let sid_ref = symbol_id.as_deref();
            let res = rest_client.get_positions(sid_ref).await.map_err(PyErr::from)?;
            serde_json::to_string(&res)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_assets_py<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.rest_client.get_assets_py(py)
    }

    pub fn get_equity_data_py<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.rest_client.get_equity_data_py(py)
    }

    /// Stop the polling loop gracefully
    pub fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let shutdown = self.shutdown.clone();
        let future = async move {
            shutdown.store(true, Ordering::SeqCst);
            Ok("Disconnected")
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    /// Track an order for polling
    pub fn track_order<'py>(&self, py: Python<'py>, order_id: String) -> PyResult<Bound<'py, PyAny>> {
        let tracked_orders = self.tracked_orders.clone();
        let future = async move {
            let mut orders = tracked_orders.write().await;
            orders.insert(order_id, "WORKING_ORDER".to_string());
            Ok("Tracked")
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    /// Untrack an order (stop polling for it)
    pub fn untrack_order<'py>(&self, py: Python<'py>, order_id: String) -> PyResult<Bound<'py, PyAny>> {
        let tracked_orders = self.tracked_orders.clone();
        let future = async move {
            let mut orders = tracked_orders.write().await;
            orders.remove(&order_id);
            Ok("Untracked")
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }
}

impl RakutenwExecutionClient {
    /// Polling loop to check order status changes
    async fn poll_loop(
        rest_client: RakutenwRestClient,
        order_cb_arc: Arc<std::sync::Mutex<Option<Py<PyAny>>>>,
        tracked_orders: Arc<RwLock<HashMap<String, String>>>,
        shutdown: Arc<AtomicBool>,
        poll_interval_ms: u64,
    ) {
        info!("RakutenW: Order polling loop started (interval={}ms)", poll_interval_ms);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("RakutenW: Order polling loop stopped");
                return;
            }

            // Get list of tracked order IDs
            let order_ids: Vec<String> = {
                let orders = tracked_orders.read().await;
                orders.keys().cloned().collect()
            };

            if !order_ids.is_empty() {
                // Query active orders
                match rest_client.get_orders(None, Some(&["WORKING_ORDER", "PARTIAL_FILL"])).await {
                    Ok(val) => {
                        let orders_list = if val.is_array() {
                            val.as_array().cloned().unwrap_or_default()
                        } else {
                            vec![]
                        };

                        let active_ids: std::collections::HashSet<String> = orders_list.iter()
                            .filter_map(|o| o.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                            .collect();

                        let mut to_remove = Vec::new();

                        for order_id in &order_ids {
                            if !active_ids.contains(order_id) {
                                // Order is no longer active - it was filled, canceled, or expired
                                // Notify Python side
                                let event_data = serde_json::json!({
                                    "orderId": order_id,
                                    "status": "COMPLETED",
                                });

                                Python::try_attach(|py| {
                                    let lock = order_cb_arc.lock().unwrap();
                                    if let Some(cb) = lock.as_ref() {
                                        let _ = cb.call1(py, ("OrderCompleted", event_data.to_string())).ok();
                                    }
                                });

                                to_remove.push(order_id.clone());
                            } else {
                                // Check for partial fills
                                if let Some(order_data) = orders_list.iter().find(|o| {
                                    o.get("id").and_then(|v| v.as_str()) == Some(order_id)
                                }) {
                                    let current_status = order_data.get("orderStatus")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("WORKING_ORDER")
                                        .to_string();

                                    let last_status = {
                                        let orders = tracked_orders.read().await;
                                        orders.get(order_id).cloned().unwrap_or_default()
                                    };

                                    if current_status != last_status {
                                        // Status changed
                                        Python::try_attach(|py| {
                                            let lock = order_cb_arc.lock().unwrap();
                                            if let Some(cb) = lock.as_ref() {
                                                let _ = cb.call1(py, ("OrderUpdate", order_data.to_string())).ok();
                                            }
                                        });

                                        let mut orders = tracked_orders.write().await;
                                        orders.insert(order_id.clone(), current_status);
                                    }
                                }
                            }
                        }

                        // Remove completed orders from tracking
                        if !to_remove.is_empty() {
                            let mut orders = tracked_orders.write().await;
                            for id in to_remove {
                                orders.remove(&id);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("RakutenW: Failed to poll orders: {}", e);
                    }
                }
            }

            sleep(Duration::from_millis(poll_interval_ms)).await;
        }
    }
}
