use crate::{error::Error, fetch::api::ApiClient, verified::{beacon_header::VerifiedBeaconHeader, execution_header::VerifiedHeader}};

pub mod error;
pub mod fetch;
pub mod proof_output;
pub mod stone;
pub mod verified;

pub struct Shikai {
    pub api_client: ApiClient,
    pub execution_rpc: String,
    pub beacon_rpc: String,
}

impl Shikai {
    pub fn new(execution_rpc: String, beacon_rpc: String) -> Self {
        let api_client = ApiClient::new("http://127.0.0.1:3030".to_string());
        Self {
            api_client,
            execution_rpc,
            beacon_rpc,
        }
    }

    pub async fn fetch_execution_header(&self, block_number: u64) -> Result<VerifiedHeader, Error> {
        let verified_header =
            VerifiedHeader::new(block_number, &self.api_client, &self.execution_rpc).await?;
        Ok(verified_header)
    }

    pub async fn fetch_beacon_header(&self, block_number: u64) -> Result<VerifiedBeaconHeader, Error> {
        let verified_header =
            VerifiedBeaconHeader::new(block_number, &self.api_client, &self.beacon_rpc).await?;
        Ok(verified_header)
    }
}
