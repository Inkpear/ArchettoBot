use thiserror::Error;

#[derive(Error, Debug)]
pub enum NapError {
    #[error("WebSocket error: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error: retcode={retcode}, status={status}")]
    Api { retcode: i64, status: String },

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Timeout waiting for response (echo: {echo})")]
    Timeout { echo: String },

    #[error("No connected client")]
    NoClient,

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Invalid access token")]
    Unauthorized,
}

pub type Result<T> = std::result::Result<T, NapError>;
