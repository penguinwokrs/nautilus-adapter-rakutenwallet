#![allow(non_local_definitions)]

use pyo3::prelude::*;

mod client;
mod error;
mod model;
mod rate_limit;

#[pymodule]
fn _nautilus_rakutenwallet(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<client::rest::RakutenwRestClient>()?;
    m.add_class::<client::data_client::RakutenwDataClient>()?;
    m.add_class::<client::execution_client::RakutenwExecutionClient>()?;

    // Models
    m.add_class::<model::market_data::Ticker>()?;
    m.add_class::<model::market_data::Depth>()?;
    m.add_class::<model::market_data::TradeEntry>()?;
    m.add_class::<model::market_data::SymbolInfo>()?;
    m.add_class::<model::orderbook::OrderBook>()?;
    Ok(())
}
