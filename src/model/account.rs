use serde::{Deserialize, Serialize};

/// Asset balance from GET /api/v1/asset
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Asset {
    #[serde(rename = "currencyId")]
    #[serde(default)]
    pub currency_id: Option<String>,
    #[serde(rename = "currencyName")]
    #[serde(default)]
    pub currency_name: Option<String>,
    #[serde(rename = "onhandAmount")]
    pub onhand_amount: String,
}

/// Equity data (margin info) from GET /api/v1/cfd/equitydata
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EquityData {
    #[serde(rename = "floatingProfit")]
    #[serde(default)]
    pub floating_profit: Option<String>,
    #[serde(rename = "floatingPositionFee")]
    #[serde(default)]
    pub floating_position_fee: Option<String>,
    #[serde(rename = "remainingFloatingPositionFee")]
    #[serde(default)]
    pub remaining_floating_position_fee: Option<String>,
    #[serde(rename = "floatingTradeFee")]
    #[serde(default)]
    pub floating_trade_fee: Option<String>,
    #[serde(rename = "floatingProfitAll")]
    #[serde(default)]
    pub floating_profit_all: Option<String>,
    #[serde(rename = "usedMargin")]
    #[serde(default)]
    pub used_margin: Option<String>,
    #[serde(rename = "necessaryMargin")]
    #[serde(default)]
    pub necessary_margin: Option<String>,
    #[serde(rename = "balance")]
    #[serde(default)]
    pub balance: Option<String>,
    #[serde(rename = "equity")]
    #[serde(default)]
    pub equity: Option<String>,
    #[serde(rename = "marginMaintenancePercent")]
    #[serde(default)]
    pub margin_maintenance_percent: Option<String>,
    #[serde(rename = "usableAmount")]
    #[serde(default)]
    pub usable_amount: Option<String>,
    #[serde(rename = "withdrawableAmount")]
    #[serde(default)]
    pub withdrawable_amount: Option<String>,
    #[serde(rename = "withdrawalAmountReserved")]
    #[serde(default)]
    pub withdrawal_amount_reserved: Option<String>,
}
