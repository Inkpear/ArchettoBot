use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Render error: {0}")]
    Render(String),
}
