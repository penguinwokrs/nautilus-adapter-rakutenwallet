use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::error::RakutenwError;
use crate::model::{
    market_data::{Ticker, Depth, SymbolInfo},
    account::{Asset, EquityData},
};
use crate::rate_limit::TokenBucket;
use std::time::{SystemTime, UNIX_EPOCH};
use pyo3::prelude::*;

type HmacSha256 = Hmac<Sha256>;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct RakutenwRestClient {
    client: Client,
    api_key: String,
    api_secret: String,
    base_url: String,
    rate_limit: TokenBucket,
}

#[pymethods]
impl RakutenwRestClient {
    /// Create a new RakutenwRestClient.
    ///
    /// `rate_limit_per_sec`: API rate limit (requests/sec). Default 5.0 (200ms interval).
    #[new]
    pub fn new(
        api_key: String,
        api_secret: String,
        timeout_ms: u64,
        proxy_url: Option<String>,
        rate_limit_per_sec: Option<f64>,
    ) -> Self {
        let mut builder = Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms));

        if let Some(proxy) = proxy_url {
            if let Ok(p) = reqwest::Proxy::all(proxy) {
                builder = builder.proxy(p);
            }
        }

        let rate = rate_limit_per_sec.unwrap_or(5.0);

        Self {
            client: builder.build().unwrap_or_else(|_| Client::new()),
            api_key,
            api_secret,
            base_url: "https://exchange.rakuten-wallet.co.jp".to_string(),
            rate_limit: TokenBucket::new(rate, rate),
        }
    }

    // ========== Public API (Python) ==========

    pub fn get_symbols_py<'py>(&self, py: Python<'py>, authority: Option<String>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let mut query_owned: Vec<(String, String)> = vec![];
            if let Some(a) = authority { query_owned.push(("authority".to_string(), a)); }
            let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let q = if query.is_empty() { None } else { Some(query.as_slice()) };
            let res: Vec<SymbolInfo> = client.public_get("/api/v1/cfd/symbol", q).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_ticker_py<'py>(&self, py: Python<'py>, symbol_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let query = vec![("symbolId", symbol_id.as_str())];
            let res: Ticker = client.public_get("/api/v1/ticker", Some(&query)).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_orderbook_py<'py>(&self, py: Python<'py>, symbol_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let query = vec![("symbolId", symbol_id.as_str())];
            let res: Depth = client.public_get("/api/v1/orderbook", Some(&query)).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_trades_py<'py>(&self, py: Python<'py>, symbol_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let query = vec![("symbolId", symbol_id.as_str())];
            let res: serde_json::Value = client.public_get("/api/v1/trades", Some(&query)).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id, candlestick_type, date_from, date_to=None))]
    pub fn get_candlestick_py<'py>(
        &self,
        py: Python<'py>,
        symbol_id: String,
        candlestick_type: String,
        date_from: String,
        date_to: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let mut query_owned: Vec<(String, String)> = vec![
                ("symbolId".to_string(), symbol_id),
                ("candlestickType".to_string(), candlestick_type),
                ("dateFrom".to_string(), date_from),
            ];
            if let Some(dt) = date_to { query_owned.push(("dateTo".to_string(), dt)); }
            let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let res: serde_json::Value = client.public_get("/api/v1/candlestick", Some(&query)).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    // ========== Private API (Python) ==========

    pub fn get_assets_py<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let res: Vec<Asset> = client.private_get("/api/v1/asset", None).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn get_equity_data_py<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let res: EquityData = client.private_get("/api/v1/cfd/equitydata", None).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id=None, ids=None, date_from=None, date_to=None, order_behavior=None, order_side=None, order_pattern=None, order_type=None, order_status=None, size=None))]
    pub fn get_orders_py<'py>(
        &self,
        py: Python<'py>,
        symbol_id: Option<String>,
        ids: Option<Vec<String>>,
        date_from: Option<String>,
        date_to: Option<String>,
        order_behavior: Option<String>,
        order_side: Option<String>,
        order_pattern: Option<Vec<String>>,
        order_type: Option<Vec<String>>,
        order_status: Option<Vec<String>>,
        size: Option<i32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let mut query_owned: Vec<(String, String)> = vec![];
            if let Some(s) = symbol_id { query_owned.push(("symbolId".to_string(), s)); }
            if let Some(ids_list) = ids {
                for id in ids_list { query_owned.push(("id[]".to_string(), id)); }
            }
            if let Some(df) = date_from { query_owned.push(("dateFrom".to_string(), df)); }
            if let Some(dt) = date_to { query_owned.push(("dateTo".to_string(), dt)); }
            if let Some(ob) = order_behavior { query_owned.push(("orderBehavior".to_string(), ob)); }
            if let Some(os) = order_side { query_owned.push(("orderSide".to_string(), os)); }
            if let Some(ops) = order_pattern {
                for op in ops { query_owned.push(("orderPattern[]".to_string(), op)); }
            }
            if let Some(ots) = order_type {
                for ot in ots { query_owned.push(("orderType[]".to_string(), ot)); }
            }
            if let Some(oss) = order_status {
                for os in oss { query_owned.push(("orderStatus[]".to_string(), os)); }
            }
            if let Some(sz) = size { query_owned.push(("size".to_string(), sz.to_string())); }

            let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let q = if query.is_empty() { None } else { Some(query.as_slice()) };
            let res: serde_json::Value = client.private_get("/api/v1/cfd/order", q).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    // ========== Order Operations (Python) ==========

    #[pyo3(signature = (symbol_id, order_pattern, order_behavior, order_side, order_type, amount, price=None, leverage=None, close_behavior=None, post_only=None, order_expire=None, position_id=None, ifd_close_limit_price=None, ifd_close_stop_price=None, oco_order_type_1=None, oco_price_1=None, oco_order_type_2=None, oco_price_2=None))]
    pub fn post_order_py<'py>(
        &self,
        py: Python<'py>,
        symbol_id: String,
        order_pattern: String,
        order_behavior: String,
        order_side: String,
        order_type: String,
        amount: String,
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
        let client = self.clone();
        let future = async move {
            let res = client.submit_order(
                &symbol_id, &order_pattern, &order_behavior, &order_side, &order_type, &amount,
                price.as_deref(), leverage.as_deref(), close_behavior.as_deref(), post_only,
                order_expire.as_deref(), position_id.as_deref(),
                ifd_close_limit_price.as_deref(), ifd_close_stop_price.as_deref(),
                oco_order_type_1.as_deref(), oco_price_1.as_deref(),
                oco_order_type_2.as_deref(), oco_price_2.as_deref(),
            ).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id, order_pattern, order_id, order_type, price, amount, ifd_close_limit_price=None, ifd_close_stop_price=None))]
    pub fn put_order_py<'py>(
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
        let client = self.clone();
        let future = async move {
            let res = client.modify_order(
                &symbol_id, &order_pattern, &order_id, &order_type, &price, &amount,
                ifd_close_limit_price.as_deref(), ifd_close_stop_price.as_deref(),
            ).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    pub fn delete_order_py<'py>(&self, py: Python<'py>, symbol_id: String, id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let query = vec![("symbolId", symbol_id.as_str()), ("id", id.as_str())];
            let res: serde_json::Value = client.private_delete("/api/v1/cfd/order", Some(&query)).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    // ========== Trade / Position API (Python) ==========

    #[pyo3(signature = (symbol_id=None, ids=None, date_from=None, date_to=None, order_behavior=None, trade_behavior=None, order_side=None, order_pattern=None, order_type=None, trade_action=None, order_ids=None, position_ids=None, size=None))]
    pub fn get_trades_private_py<'py>(
        &self,
        py: Python<'py>,
        symbol_id: Option<String>,
        ids: Option<Vec<String>>,
        date_from: Option<String>,
        date_to: Option<String>,
        order_behavior: Option<String>,
        trade_behavior: Option<String>,
        order_side: Option<String>,
        order_pattern: Option<Vec<String>>,
        order_type: Option<Vec<String>>,
        trade_action: Option<String>,
        order_ids: Option<Vec<String>>,
        position_ids: Option<Vec<String>>,
        size: Option<i32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let mut query_owned: Vec<(String, String)> = vec![];
            if let Some(s) = symbol_id { query_owned.push(("symbolId".to_string(), s)); }
            if let Some(ids_list) = ids {
                for id in ids_list { query_owned.push(("id[]".to_string(), id)); }
            }
            if let Some(df) = date_from { query_owned.push(("dateFrom".to_string(), df)); }
            if let Some(dt) = date_to { query_owned.push(("dateTo".to_string(), dt)); }
            if let Some(ob) = order_behavior { query_owned.push(("orderBehavior".to_string(), ob)); }
            if let Some(tb) = trade_behavior { query_owned.push(("tradeBehavior".to_string(), tb)); }
            if let Some(os) = order_side { query_owned.push(("orderSide".to_string(), os)); }
            if let Some(ops) = order_pattern {
                for op in ops { query_owned.push(("orderPattern[]".to_string(), op)); }
            }
            if let Some(ots) = order_type {
                for ot in ots { query_owned.push(("orderType[]".to_string(), ot)); }
            }
            if let Some(ta) = trade_action { query_owned.push(("tradeAction".to_string(), ta)); }
            if let Some(oids) = order_ids {
                for oid in oids { query_owned.push(("orderId[]".to_string(), oid)); }
            }
            if let Some(pids) = position_ids {
                for pid in pids { query_owned.push(("positionId[]".to_string(), pid)); }
            }
            if let Some(sz) = size { query_owned.push(("size".to_string(), sz.to_string())); }

            let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let q = if query.is_empty() { None } else { Some(query.as_slice()) };
            let res: serde_json::Value = client.private_get("/api/v1/cfd/trade", q).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }

    #[pyo3(signature = (symbol_id=None, ids=None, date_from=None, date_to=None, position_status=None, order_side=None, size=None))]
    pub fn get_positions_py<'py>(
        &self,
        py: Python<'py>,
        symbol_id: Option<String>,
        ids: Option<Vec<String>>,
        date_from: Option<String>,
        date_to: Option<String>,
        position_status: Option<Vec<String>>,
        order_side: Option<String>,
        size: Option<i32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let future = async move {
            let mut query_owned: Vec<(String, String)> = vec![];
            if let Some(s) = symbol_id { query_owned.push(("symbolId".to_string(), s)); }
            if let Some(ids_list) = ids {
                for id in ids_list { query_owned.push(("id[]".to_string(), id)); }
            }
            if let Some(df) = date_from { query_owned.push(("dateFrom".to_string(), df)); }
            if let Some(dt) = date_to { query_owned.push(("dateTo".to_string(), dt)); }
            if let Some(pss) = position_status {
                for ps in pss { query_owned.push(("positionStatus[]".to_string(), ps)); }
            }
            if let Some(os) = order_side { query_owned.push(("orderSide".to_string(), os)); }
            if let Some(sz) = size { query_owned.push(("size".to_string(), sz.to_string())); }

            let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let q = if query.is_empty() { None } else { Some(query.as_slice()) };
            let res: serde_json::Value = client.private_get("/api/v1/cfd/position", q).await.map_err(PyErr::from)?;
            serde_json::to_string(&res).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        };
        pyo3_async_runtimes::tokio::future_into_py(py, future)
    }
}

// ========== Internal (Rust-only) ==========

impl RakutenwRestClient {
    /// Generate HMAC-SHA256 signature
    fn generate_signature(&self, text: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(text.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Generate NONCE (millisecond Unix timestamp)
    fn nonce_ms() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string()
    }

    /// Public GET: base_url + endpoint
    pub async fn public_get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, RakutenwError> {
        self.rate_limit.acquire().await;

        let url = format!("{}{}", self.base_url, endpoint);
        let mut builder = self.client.get(&url);
        if let Some(q) = query {
            builder = builder.query(q);
        }

        let response = builder.send().await?;
        let text = response.text().await?;

        self.parse_response::<T>(&text)
    }

    /// Private GET: base_url + endpoint with auth headers
    /// Signature: HMAC-SHA256(NONCE + URI + queryString)
    pub async fn private_get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, RakutenwError> {
        self.rate_limit.acquire().await;

        let nonce = Self::nonce_ms();

        // Build query string for signature
        let query_string = if let Some(q) = query {
            let qs = serde_urlencoded::to_string(q).unwrap_or_default();
            if qs.is_empty() { String::new() } else { format!("?{}", qs) }
        } else {
            String::new()
        };

        // Rakuten Wallet GET/DELETE signature: NONCE + URI + queryString
        let text_to_sign = format!("{}{}{}", nonce, endpoint, query_string);
        let signature = self.generate_signature(&text_to_sign);

        let url = format!("{}{}", self.base_url, endpoint);
        let mut builder = self.client.get(&url)
            .header("API-KEY", &self.api_key)
            .header("NONCE", &nonce)
            .header("SIGNATURE", signature);

        if let Some(q) = query {
            builder = builder.query(q);
        }

        let response = builder.send().await?;
        let text = response.text().await?;
        self.parse_response::<T>(&text)
    }

    /// Private POST: base_url + endpoint with auth headers
    /// Signature: HMAC-SHA256(NONCE + JSON_body)
    pub async fn private_post<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &str,
    ) -> Result<T, RakutenwError> {
        self.private_request_with_body::<T>(Method::POST, endpoint, body).await
    }

    /// Private PUT: base_url + endpoint with auth headers
    /// Signature: HMAC-SHA256(NONCE + JSON_body)
    pub async fn private_put<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &str,
    ) -> Result<T, RakutenwError> {
        self.private_request_with_body::<T>(Method::PUT, endpoint, body).await
    }

    /// Private DELETE: base_url + endpoint with auth headers
    /// Signature: HMAC-SHA256(NONCE + URI + queryString)
    pub async fn private_delete<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, RakutenwError> {
        self.rate_limit.acquire().await;

        let nonce = Self::nonce_ms();

        let query_string = if let Some(q) = query {
            let qs = serde_urlencoded::to_string(q).unwrap_or_default();
            if qs.is_empty() { String::new() } else { format!("?{}", qs) }
        } else {
            String::new()
        };

        let text_to_sign = format!("{}{}{}", nonce, endpoint, query_string);
        let signature = self.generate_signature(&text_to_sign);

        let url = format!("{}{}", self.base_url, endpoint);
        let mut builder = self.client.delete(&url)
            .header("API-KEY", &self.api_key)
            .header("NONCE", &nonce)
            .header("SIGNATURE", signature);

        if let Some(q) = query {
            builder = builder.query(q);
        }

        let response = builder.send().await?;
        let text = response.text().await?;
        self.parse_response::<T>(&text)
    }

    async fn private_request_with_body<T: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        body: &str,
    ) -> Result<T, RakutenwError> {
        self.rate_limit.acquire().await;

        let nonce = Self::nonce_ms();

        // POST/PUT signature: NONCE + JSON_body
        let text_to_sign = format!("{}{}", nonce, body);
        let signature = self.generate_signature(&text_to_sign);

        let url = format!("{}{}", self.base_url, endpoint);
        let mut builder = self.client.request(method, &url)
            .header("API-KEY", &self.api_key)
            .header("NONCE", &nonce)
            .header("SIGNATURE", signature)
            .header("Content-Type", "application/json");

        if !body.is_empty() {
            builder = builder.body(body.to_string());
        }

        let response = builder.send().await?;
        let text = response.text().await?;
        self.parse_response::<T>(&text)
    }

    /// Parse Rakuten Wallet API response.
    /// Success: direct JSON data (array or object)
    /// Error: {"code": 10001, "message": "..."}
    fn parse_response<T: DeserializeOwned>(&self, text: &str) -> Result<T, RakutenwError> {
        // Check if it's an error response
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(code) = val.get("code").and_then(|c| c.as_i64()) {
                let message = val.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();

                // Authentication errors
                if code >= 20001 && code <= 20010 {
                    return Err(RakutenwError::AuthError(format!("code={}: {}", code, message)));
                }

                return Err(RakutenwError::ExchangeError {
                    code: code as i32,
                    message,
                });
            }
        }

        // Try to deserialize as the expected type
        serde_json::from_str::<T>(text).map_err(|e| {
            RakutenwError::Unknown(format!(
                "Parse error: {}. Body: {}",
                e,
                &text[..text.len().min(500)]
            ))
        })
    }

    // ========== Internal Rust methods for execution_client ==========

    /// Submit a new order.
    ///
    /// `order_pattern`: "NORMAL", "OCO", "IFD", "IFD_OCO"
    /// For NORMAL/IFD/IFD_OCO, orderData is used.
    /// For OCO, ocoOrderData is used.
    pub async fn submit_order(
        &self,
        symbol_id: &str,
        order_pattern: &str,
        order_behavior: &str,
        order_side: &str,
        order_type: &str,
        amount: &str,
        price: Option<&str>,
        leverage: Option<&str>,
        close_behavior: Option<&str>,
        post_only: Option<bool>,
        order_expire: Option<&str>,
        position_id: Option<&str>,
        ifd_close_limit_price: Option<&str>,
        ifd_close_stop_price: Option<&str>,
        // OCO-specific fields
        oco_order_type_1: Option<&str>,
        oco_price_1: Option<&str>,
        oco_order_type_2: Option<&str>,
        oco_price_2: Option<&str>,
    ) -> Result<serde_json::Value, RakutenwError> {
        let mut body = serde_json::json!({
            "symbolId": symbol_id,
            "orderPattern": order_pattern,
        });

        if order_pattern == "OCO" {
            // OCO pattern uses ocoOrderData with two legs
            let mut oco_data = serde_json::json!({
                "orderBehavior": order_behavior,
                "orderSide": order_side,
                "amount": amount,
            });
            if let Some(pid) = position_id { oco_data["positionId"] = serde_json::json!(pid); }
            if let Some(l) = leverage { oco_data["leverage"] = serde_json::json!(l); }
            if let Some(cb) = close_behavior { oco_data["closeBehavior"] = serde_json::json!(cb); }
            if let Some(exp) = order_expire { oco_data["orderExpire"] = serde_json::json!(exp); }
            if let Some(ot1) = oco_order_type_1 { oco_data["orderType1"] = serde_json::json!(ot1); }
            if let Some(p1) = oco_price_1 { oco_data["price1"] = serde_json::json!(p1); }
            if let Some(ot2) = oco_order_type_2 { oco_data["orderType2"] = serde_json::json!(ot2); }
            if let Some(p2) = oco_price_2 { oco_data["price2"] = serde_json::json!(p2); }
            body["ocoOrderData"] = oco_data;
        } else {
            // NORMAL / IFD / IFD_OCO use orderData
            let mut order_data = serde_json::json!({
                "orderBehavior": order_behavior,
                "orderSide": order_side,
                "orderType": order_type,
                "amount": amount,
            });
            if let Some(p) = price { order_data["price"] = serde_json::json!(p); }
            if let Some(l) = leverage { order_data["leverage"] = serde_json::json!(l); }
            if let Some(cb) = close_behavior { order_data["closeBehavior"] = serde_json::json!(cb); }
            if let Some(po) = post_only { order_data["postOnly"] = serde_json::json!(po); }
            if let Some(exp) = order_expire { order_data["orderExpire"] = serde_json::json!(exp); }
            if let Some(pid) = position_id { order_data["positionId"] = serde_json::json!(pid); }
            if let Some(lp) = ifd_close_limit_price { order_data["ifdCloseLimitPrice"] = serde_json::json!(lp); }
            if let Some(sp) = ifd_close_stop_price { order_data["ifdCloseStopPrice"] = serde_json::json!(sp); }
            body["orderData"] = order_data;
        }

        let body_str = body.to_string();
        self.private_post("/api/v1/cfd/order", &body_str).await
    }

    /// Modify an existing order.
    ///
    /// All of `symbol_id`, `order_pattern`, `order_type`, `price`, `amount` are required by the API.
    pub async fn modify_order(
        &self,
        symbol_id: &str,
        order_pattern: &str,
        order_id: &str,
        order_type: &str,
        price: &str,
        amount: &str,
        ifd_close_limit_price: Option<&str>,
        ifd_close_stop_price: Option<&str>,
    ) -> Result<serde_json::Value, RakutenwError> {
        let mut body = serde_json::json!({
            "symbolId": symbol_id,
            "orderPattern": order_pattern,
            "orderId": order_id,
            "orderType": order_type,
            "price": price,
            "amount": amount,
        });
        if let Some(lp) = ifd_close_limit_price { body["ifdCloseLimitPrice"] = serde_json::json!(lp); }
        if let Some(sp) = ifd_close_stop_price { body["ifdCloseStopPrice"] = serde_json::json!(sp); }

        let body_str = body.to_string();
        self.private_put("/api/v1/cfd/order", &body_str).await
    }

    pub async fn cancel_order(&self, symbol_id: &str, id: &str) -> Result<serde_json::Value, RakutenwError> {
        let query = vec![("symbolId", symbol_id), ("id", id)];
        self.private_delete("/api/v1/cfd/order", Some(&query)).await
    }

    pub async fn get_orders(
        &self,
        symbol_id: Option<&str>,
        order_status: Option<&[&str]>,
    ) -> Result<serde_json::Value, RakutenwError> {
        let mut query_owned: Vec<(String, String)> = vec![];
        if let Some(s) = symbol_id { query_owned.push(("symbolId".to_string(), s.to_string())); }
        if let Some(statuses) = order_status {
            for st in statuses { query_owned.push(("orderStatus[]".to_string(), st.to_string())); }
        }
        let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let q = if query.is_empty() { None } else { Some(query.as_slice()) };
        self.private_get("/api/v1/cfd/order", q).await
    }

    pub async fn get_trades_for_order(&self, order_id: &str) -> Result<serde_json::Value, RakutenwError> {
        let query = vec![("orderId[]", order_id)];
        self.private_get("/api/v1/cfd/trade", Some(&query)).await
    }

    pub async fn get_positions(&self, symbol_id: Option<&str>) -> Result<serde_json::Value, RakutenwError> {
        let mut query_owned: Vec<(String, String)> = vec![];
        if let Some(s) = symbol_id { query_owned.push(("symbolId".to_string(), s.to_string())); }
        query_owned.push(("positionStatus[]".to_string(), "OPEN".to_string()));
        query_owned.push(("positionStatus[]".to_string(), "PARTIALLY_CLOSED".to_string()));
        let query: Vec<(&str, &str)> = query_owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        self.private_get("/api/v1/cfd/position", Some(&query)).await
    }

    pub async fn get_assets(&self) -> Result<Vec<Asset>, RakutenwError> {
        self.private_get("/api/v1/asset", None).await
    }

    pub async fn get_equity_data(&self) -> Result<EquityData, RakutenwError> {
        self.private_get("/api/v1/cfd/equitydata", None).await
    }
}
