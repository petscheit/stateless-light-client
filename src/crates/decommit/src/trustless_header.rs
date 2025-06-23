use std::path::PathBuf;

use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
use alloy_rpc_types::Header as ExecutionHeader;
use serde::{Deserialize, Serialize};
use starknet_core::types::Felt;
use swiftness_air::layout::dynamic::Layout;
pub use swiftness_proof_parser::*;
pub use swiftness_stark::{self, types::StarkProof};

use crate::{
    error::Error,
    retrieve::HeaderReader,
    transform::TransformTo,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecursiveEpochOutput {
    pub beacon_header_root: FixedBytes<32>,
    pub beacon_state_root: FixedBytes<32>,
    pub beacon_height: u64,
    pub n_signers: u64,
    pub execution_header_root: FixedBytes<32>,
    pub execution_header_height: u64,
    pub current_committee_hash: FixedBytes<32>,
    pub next_committee_hash: FixedBytes<32>,
}

fn felts_to_bytes32(low: &Felt, high: &Felt) -> FixedBytes<32> {
    let mut bytes = [0u8; 32];
    let low_bytes = low.to_bytes_be();
    let high_bytes = high.to_bytes_be();
    bytes[16..32].copy_from_slice(&low_bytes[16..32]);
    bytes[0..16].copy_from_slice(&high_bytes[16..32]);
    FixedBytes(bytes)
}

fn deserialize_output(proof_output: Vec<Felt>) -> Result<(RecursiveEpochOutput, Felt), Error> {
    let program_hash = proof_output[2];
    let epoch_output = RecursiveEpochOutput {
        beacon_header_root: felts_to_bytes32(&proof_output[3], &proof_output[4]),
        beacon_state_root: felts_to_bytes32(&proof_output[5], &proof_output[6]),
        beacon_height: proof_output[7].try_into().unwrap(),
        n_signers: proof_output[8].try_into().unwrap(),
        execution_header_root: felts_to_bytes32(&proof_output[9], &proof_output[10]),
        execution_header_height: proof_output[11].try_into().unwrap(),
        current_committee_hash: felts_to_bytes32(&proof_output[12], &proof_output[13]),
        next_committee_hash: felts_to_bytes32(&proof_output[14], &proof_output[15]),
    };
    Ok((epoch_output, program_hash))
}

pub struct VerifiedHeader {
    header: ExecutionHeader,
    program_hash: Felt,
}

impl VerifiedHeader {
    pub async fn from_proof(proof_path: PathBuf) -> Result<Self, Error> {
        let header_reader = HeaderReader::new("https://sepolia.drpc.org".to_string());
        let proof_content = std::fs::read_to_string(proof_path)?;
        let stark_proof: StarkProof = parse(&proof_content)
            .map_err(|e| Error::ProofParsingError(e.to_string()))?
            .transform_to();
        let security_bits = stark_proof.config.security_bits();
        let (_bootloader_hash, program_output) = stark_proof
            .verify::<Layout>(security_bits)
            .map_err(|e| Error::ProofVerificationError(e.to_string()))?;

        println!("Stark Proof Verified");

        let (epoch_output, program_hash) = deserialize_output(program_output)?;

        let header = header_reader
            .fetch_header(epoch_output.execution_header_height)
            .await?;
        println!("Fetched Header from RPC");

        assert_eq!(
            header.hash_slow(),
            epoch_output.execution_header_root        
        );

        println!("Header Hash matches Proof");

        Ok(Self {
            header,
            program_hash,
        })
    }
}

pub trait TrustlessHeader {
    fn parent_hash(&self) -> B256;
    fn ommers_hash(&self) -> B256;
    fn beneficiary(&self) -> Address;
    fn state_root(&self) -> B256;
    fn transactions_root(&self) -> B256;
    fn receipts_root(&self) -> B256;
    fn logs_bloom(&self) -> &[u8];
    fn difficulty(&self) -> U256;
    fn number(&self) -> u64;
    fn gas_limit(&self) -> u64;
    fn gas_used(&self) -> u64;
    fn timestamp(&self) -> u64;
    fn extra_data(&self) -> &Bytes;
    fn mix_hash(&self) -> B256;
    fn nonce(&self) -> u64;
    fn base_fee_per_gas(&self) -> Option<u64>;
    fn withdrawals_root(&self) -> Option<B256>;
    fn blob_gas_used(&self) -> Option<u64>;
    fn excess_blob_gas(&self) -> Option<u64>;
    fn parent_beacon_block_root(&self) -> Option<B256>;
}

impl TrustlessHeader for VerifiedHeader {
    fn parent_hash(&self) -> B256 {
        self.header.parent_hash
    }

    fn ommers_hash(&self) -> B256 {
        self.header.ommers_hash
    }

    fn beneficiary(&self) -> Address {
        self.header.beneficiary
    }

    fn state_root(&self) -> B256 {
        self.header.state_root
    }

    fn transactions_root(&self) -> B256 {
        self.header.transactions_root
    }

    fn receipts_root(&self) -> B256 {
        self.header.receipts_root
    }

    fn logs_bloom(&self) -> &[u8] {
        self.header.logs_bloom.as_slice()
    }

    fn difficulty(&self) -> U256 {
        self.header.difficulty
    }

    fn number(&self) -> u64 {
        self.header.number
    }

    fn gas_limit(&self) -> u64 {
        self.header.gas_limit
    }

    fn gas_used(&self) -> u64 {
        self.header.gas_used
    }

    fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    fn extra_data(&self) -> &Bytes {
        &self.header.extra_data
    }

    fn mix_hash(&self) -> B256 {
        self.header.mix_hash
    }

    fn nonce(&self) -> u64 {
        u64::from_be_bytes(self.header.nonce.0)
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.header.base_fee_per_gas
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.header.withdrawals_root
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.header.blob_gas_used
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.header.excess_blob_gas
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.header.parent_beacon_block_root
    }
} 