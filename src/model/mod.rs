pub mod market_data;
pub mod order;
pub mod account;
pub mod orderbook;

use serde::Deserialize;

/// Generic Rakuten Wallet API error response
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct RakutenwErrorDetail {
    pub code: i32,
    pub message: String,
}
