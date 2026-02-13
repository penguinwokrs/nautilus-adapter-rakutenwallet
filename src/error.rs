use thiserror::Error;
use pyo3::prelude::*;

#[derive(Error, Debug)]
pub enum RakutenwError {
    #[error("API Request Error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("WebSocket Error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Parse Error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Authentication Error: {0}")]
    AuthError(String),

    #[error("Exchange Error: code={code}, {message}")]
    ExchangeError {
        code: i32,
        message: String,
    },

    #[error("Unknown Error: {0}")]
    Unknown(String),
}

impl From<RakutenwError> for PyErr {
    fn from(err: RakutenwError) -> Self {
        match err {
            RakutenwError::AuthError(e) => {
                pyo3::exceptions::PyPermissionError::new_err(e)
            }
            RakutenwError::ExchangeError { code, message } => {
                pyo3::exceptions::PyRuntimeError::new_err(
                    format!("Rakuten Wallet Error (code={}): {}", code, message),
                )
            }
            _ => pyo3::exceptions::PyRuntimeError::new_err(err.to_string()),
        }
    }
}
