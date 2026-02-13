use std::collections::BTreeMap;
use pyo3::prelude::*;
use crate::model::market_data::Depth;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct OrderBook {
    #[pyo3(get)]
    pub symbol_id: String,
    pub asks: BTreeMap<String, String>,
    pub bids: BTreeMap<String, String>,
    #[pyo3(get)]
    pub timestamp: String,
}

#[pymethods]
impl OrderBook {
    #[new]
    pub fn new(symbol_id: String) -> Self {
        Self {
            symbol_id,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            timestamp: String::new(),
        }
    }

    pub fn apply_snapshot(&mut self, depth: Depth) {
        self.asks.clear();
        for entry in &depth.asks {
            self.asks.insert(entry.price.clone(), entry.amount.clone());
        }
        self.bids.clear();
        for entry in &depth.bids {
            self.bids.insert(entry.price.clone(), entry.amount.clone());
        }
        self.timestamp = depth.timestamp.clone();
    }

    pub fn get_asks(&self) -> Vec<Vec<String>> {
        self.asks.iter().map(|(p, a)| vec![p.clone(), a.clone()]).collect()
    }

    pub fn get_bids(&self) -> Vec<Vec<String>> {
        self.bids.iter().rev().map(|(p, a)| vec![p.clone(), a.clone()]).collect()
    }

    pub fn get_top_n(&self, n: usize) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
        let top_asks: Vec<Vec<String>> = self.asks.iter()
            .take(n)
            .map(|(p, a)| vec![p.clone(), a.clone()])
            .collect();

        let top_bids: Vec<Vec<String>> = self.bids.iter()
            .rev()
            .take(n)
            .map(|(p, a)| vec![p.clone(), a.clone()])
            .collect();

        (top_asks, top_bids)
    }
}
