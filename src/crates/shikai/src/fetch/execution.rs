use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{EIP1186AccountProofResponse, Header as ExecutionHeader};
use url::Url;

use crate::error::Error;

pub struct ExecutionFetcher {
    pub base_url: String,
}

impl ExecutionFetcher {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn fetch_header(&self, block_number: u64) -> Result<ExecutionHeader, Error> {
        let rpc_url: Url = self.base_url.parse()?;
        let provider = ProviderBuilder::new().on_http(rpc_url);

        let block = provider
            .get_block_by_number(block_number.into())
            .await
            .map_err(|_| Error::BlockNotFound)?
            .unwrap();

        Ok(block.header)
    }

    pub async fn fetch_account_proof(
        &self,
        address: Address,
        block_number: u64,
    ) -> Result<EIP1186AccountProofResponse, Error> {
        let rpc_url: Url = self.base_url.parse()?;
        let provider = ProviderBuilder::new().on_http(rpc_url);

        let proof = provider
            .get_proof(address, vec![])
            .block_id(block_number.into())
            .await
            .map_err(|_| Error::BlockNotFound)?;

        Ok(proof)
    }
}
