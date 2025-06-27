use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Proof verification error: {0}")]
    ProofVerificationError(String),
    #[error("Proof parsing error: {0}")]
    ProofParsingError(String),
    #[error("Proof not found")]
    ProofNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Block not found")]
    BlockNotFound,
}
