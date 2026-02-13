use serde::{Deserialize, Serialize};
use pyo3::prelude::*;

/// Ticker from GET /api/v1/ticker or WebSocket TICKER channel
#[pyclass(from_py_object)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Ticker {
    #[pyo3(get)]
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[pyo3(get)]
    #[serde(rename = "bestAsk")]
    pub best_ask: String,
    #[pyo3(get)]
    #[serde(rename = "bestBid")]
    pub best_bid: String,
    #[pyo3(get)]
    #[serde(default)]
    pub open: String,
    #[pyo3(get)]
    #[serde(default)]
    pub high: String,
    #[pyo3(get)]
    #[serde(default)]
    pub low: String,
    #[pyo3(get)]
    #[serde(default)]
    pub last: String,
    #[pyo3(get)]
    #[serde(default)]
    pub volume: String,
    #[pyo3(get)]
    #[serde(default)]
    pub timestamp: String,
}

#[pymethods]
impl Ticker {
    #[new]
    pub fn new(
        symbol_id: String,
        best_ask: String,
        best_bid: String,
        open: String,
        high: String,
        low: String,
        last: String,
        volume: String,
        timestamp: String,
    ) -> Self {
        Self { symbol_id, best_ask, best_bid, open, high, low, last, volume, timestamp }
    }
}

/// Depth entry from orderbook
#[pyclass(from_py_object)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DepthEntry {
    #[pyo3(get)]
    pub price: String,
    #[pyo3(get)]
    pub amount: String,
    #[pyo3(get)]
    #[serde(rename = "assetAmount")]
    #[serde(default)]
    pub asset_amount: Option<String>,
}

/// Orderbook depth from GET /api/v1/orderbook or WebSocket ORDERBOOK channel
#[pyclass(from_py_object)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Depth {
    #[pyo3(get)]
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[pyo3(get)]
    pub asks: Vec<DepthEntry>,
    #[pyo3(get)]
    pub bids: Vec<DepthEntry>,
    #[pyo3(get)]
    #[serde(rename = "bestAsk")]
    #[serde(default)]
    pub best_ask: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "bestBid")]
    #[serde(default)]
    pub best_bid: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "midPrice")]
    #[serde(default)]
    pub mid_price: Option<String>,
    #[pyo3(get)]
    #[serde(default)]
    pub spread: Option<String>,
    #[pyo3(get)]
    #[serde(default)]
    pub timestamp: String,
}

#[pymethods]
impl Depth {
    #[new]
    pub fn new(symbol_id: String, asks: Vec<DepthEntry>, bids: Vec<DepthEntry>, timestamp: String) -> Self {
        Self { symbol_id, asks, bids, best_ask: None, best_bid: None, mid_price: None, spread: None, timestamp }
    }
}

/// Single trade entry from WebSocket TRADES channel
#[pyclass(from_py_object)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TradeEntry {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    #[serde(rename = "orderSide")]
    pub order_side: String,
    #[pyo3(get)]
    pub price: String,
    #[pyo3(get)]
    pub amount: String,
    #[pyo3(get)]
    #[serde(rename = "assetAmount")]
    #[serde(default)]
    pub asset_amount: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "tradedAt")]
    pub traded_at: String,
}

#[pymethods]
impl TradeEntry {
    #[new]
    pub fn new(id: String, order_side: String, price: String, amount: String, traded_at: String) -> Self {
        Self { id, order_side, price, amount, asset_amount: None, traded_at }
    }
}

/// Trades message from WebSocket or REST
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TradesMessage {
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    pub trades: Vec<TradeEntry>,
    #[serde(default)]
    pub timestamp: String,
}

/// Symbol info from GET /api/v1/cfd/symbol
#[pyclass(from_py_object)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SymbolInfo {
    #[pyo3(get)]
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[pyo3(get)]
    #[serde(rename = "symbolName")]
    #[serde(default)]
    pub symbol_name: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "tickSize")]
    #[serde(default)]
    pub tick_size: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "tradeUnit")]
    #[serde(default)]
    pub trade_unit: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "minOrderAmount")]
    #[serde(default)]
    pub min_order_amount: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "maxOrderAmount")]
    #[serde(default)]
    pub max_order_amount: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "minLeverage")]
    #[serde(default)]
    pub min_leverage: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "maxLeverage")]
    #[serde(default)]
    pub max_leverage: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "makerFeeRate")]
    #[serde(default)]
    pub maker_fee_rate: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "takerFeeRate")]
    #[serde(default)]
    pub taker_fee_rate: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "baseCurrency")]
    #[serde(default)]
    pub base_currency: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "quoteCurrency")]
    #[serde(default)]
    pub quote_currency: Option<String>,
    #[pyo3(get)]
    #[serde(rename = "baseScale")]
    #[serde(default)]
    pub base_scale: Option<i32>,
    #[pyo3(get)]
    #[serde(rename = "quoteScale")]
    #[serde(default)]
    pub quote_scale: Option<i32>,
    #[pyo3(get)]
    #[serde(rename = "closeOnly")]
    #[serde(default)]
    pub close_only: Option<bool>,
    #[pyo3(get)]
    #[serde(rename = "viewOnly")]
    #[serde(default)]
    pub view_only: Option<bool>,
    #[pyo3(get)]
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[pymethods]
impl SymbolInfo {
    #[new]
    pub fn new(symbol_id: String) -> Self {
        Self {
            symbol_id,
            symbol_name: None,
            tick_size: None,
            trade_unit: None,
            min_order_amount: None,
            max_order_amount: None,
            min_leverage: None,
            max_leverage: None,
            maker_fee_rate: None,
            taker_fee_rate: None,
            base_currency: None,
            quote_currency: None,
            base_scale: None,
            quote_scale: None,
            close_only: None,
            view_only: None,
            enabled: None,
        }
    }
}

/// Candlestick data from GET /api/v1/candlestick
#[derive(Deserialize, Serialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Candlestick {
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    #[serde(rename = "openTime")]
    pub open_time: String,
}
