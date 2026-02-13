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
    #[serde(rename = "floatingPnl")]
    #[serde(default)]
    pub floating_pnl: Option<String>,
    #[serde(rename = "maintenanceRate")]
    #[serde(default)]
    pub maintenance_rate: Option<String>,
    #[serde(rename = "usableAmount")]
    #[serde(default)]
    pub usable_amount: Option<String>,
    #[serde(rename = "withdrawableAmount")]
    #[serde(default)]
    pub withdrawable_amount: Option<String>,
    #[serde(rename = "requiredMarginAmount")]
    #[serde(default)]
    pub required_margin_amount: Option<String>,
    #[serde(rename = "orderMarginAmount")]
    #[serde(default)]
    pub order_margin_amount: Option<String>,
}
