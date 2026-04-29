use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CrawlerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let err = CrawlerError::Parse("bad html".into());
        assert!(err.to_string().contains("bad html"));
    }

    #[test]
    fn json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let crawl_err: CrawlerError = json_err.into();
        assert!(matches!(crawl_err, CrawlerError::Json(_)));
    }

    #[test]
    fn unsupported_platform_error() {
        let err = CrawlerError::UnsupportedPlatform("Unknown".into());
        assert!(err.to_string().contains("Unknown"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CrawlerError>();
    }
}
