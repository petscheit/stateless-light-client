use crate::retrieve;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Proof verification error: {0}")]
    ProofVerificationError(String),
    #[error("Proof parsing error: {0}")]
    ProofParsingError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Header retrieve error: {0}")]
    Retrieve(#[from] retrieve::Error),
}
