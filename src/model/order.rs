use serde::{Deserialize, Serialize};

/// Order from GET /api/v1/cfd/order
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Order {
    pub id: String,
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[serde(rename = "orderBehavior")]
    pub order_behavior: String, // OPEN, CLOSE
    #[serde(rename = "orderSide")]
    pub order_side: String, // BUY, SELL
    #[serde(rename = "orderPattern")]
    pub order_pattern: String, // NORMAL, OCO, IFD, IFD_OCO
    #[serde(rename = "orderType")]
    pub order_type: String, // MARKET, LIMIT, STOP
    #[serde(rename = "orderStatus")]
    pub order_status: String, // WORKING_ORDER, PARTIAL_FILL
    pub amount: String,
    #[serde(rename = "remainingAmount")]
    #[serde(default)]
    pub remaining_amount: Option<String>,
    #[serde(rename = "executedAmount")]
    #[serde(default)]
    pub executed_amount: Option<String>,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub leverage: Option<String>,
    #[serde(rename = "losscutPrice")]
    #[serde(default)]
    pub losscut_price: Option<String>,
    #[serde(rename = "orderTimestamp")]
    #[serde(default)]
    pub order_timestamp: Option<String>,
    #[serde(rename = "postOnly")]
    #[serde(default)]
    pub post_only: Option<bool>,
    #[serde(rename = "closeBehavior")]
    #[serde(default)]
    pub close_behavior: Option<String>, // CROSS, FIFO
    #[serde(rename = "ifdId")]
    #[serde(default)]
    pub ifd_id: Option<String>,
    #[serde(rename = "ocoId")]
    #[serde(default)]
    pub oco_id: Option<String>,
}

/// Trade (execution) from GET /api/v1/cfd/trade
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TradeRecord {
    pub id: String,
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[serde(rename = "orderBehavior")]
    pub order_behavior: String,
    #[serde(rename = "tradeBehavior")]
    #[serde(default)]
    pub trade_behavior: Option<String>,
    #[serde(rename = "orderSide")]
    pub order_side: String,
    #[serde(rename = "orderPattern")]
    #[serde(default)]
    pub order_pattern: Option<String>,
    #[serde(rename = "orderType")]
    #[serde(default)]
    pub order_type: Option<String>,
    #[serde(rename = "tradeAction")]
    #[serde(default)]
    pub trade_action: Option<String>, // MAKER, TAKER
    pub price: String,
    pub amount: String,
    #[serde(rename = "orderId")]
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(rename = "positionId")]
    #[serde(default)]
    pub position_id: Option<String>,
    #[serde(rename = "pnl")]
    #[serde(default)]
    pub pnl: Option<String>,
    pub fee: Option<String>,
    #[serde(rename = "tradeTimestamp")]
    #[serde(default)]
    pub trade_timestamp: Option<String>,
}

/// Position from GET /api/v1/cfd/position
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Position {
    pub id: String,
    #[serde(rename = "symbolId")]
    pub symbol_id: String,
    #[serde(rename = "positionStatus")]
    pub position_status: String, // OPEN, PARTIALLY_CLOSED
    #[serde(rename = "orderSide")]
    pub order_side: String,
    pub amount: String,
    #[serde(rename = "orderedAmount")]
    #[serde(default)]
    pub ordered_amount: Option<String>,
    pub price: String,
    #[serde(default)]
    pub pnl: Option<String>,
    #[serde(rename = "floatingFee")]
    #[serde(default)]
    pub floating_fee: Option<String>,
    #[serde(default)]
    pub leverage: Option<String>,
    #[serde(rename = "requiredMargin")]
    #[serde(default)]
    pub required_margin: Option<String>,
    #[serde(rename = "openTimestamp")]
    #[serde(default)]
    pub open_timestamp: Option<String>,
}
