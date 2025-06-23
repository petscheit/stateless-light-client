use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::Header as ExecutionHeader;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Block not found")]
    BlockNotFound,
}

pub struct HeaderReader {
    eth_rpc: String,
}

impl HeaderReader {
    pub fn new(eth_rpc: String) -> Self {
        Self { eth_rpc }
    }

    pub async fn fetch_header(&self, block_number: u64) -> Result<ExecutionHeader, Error> {
        let rpc_url: Url = self.eth_rpc.parse()?;
        let provider = ProviderBuilder::new().on_http(rpc_url);

        let block = provider
            .get_block_by_number(block_number.into())
            .await
            .map_err(|e| Error::BlockNotFound)?
            .unwrap();


        Ok(block.header)
    }
}