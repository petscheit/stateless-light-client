//! Epoch Update Processing Implementation
//!
//! This module handles individual epoch updates and their verification on the StarkNet blockchain.
//! It provides functionality to process beacon chain headers, sync committee signatures, and execution
//! payload proofs, generating the necessary data for verification on StarkNet.

use std::fs;

use crate::clients::ClientError;
use crate::fetcher::execution_header_input::ExecutionHeaderError;
use crate::utils::constants;
use crate::utils::hashing::get_committee_hash;
use crate::{
    clients::beacon_chain::BeaconError, fetcher::execution_header_input::ExecutionHeaderProof,
};

use crate::clients::beacon_chain::BeaconRpcClient;
// use crate::utils::{constants, hashing::get_committee_hash};
use alloy_primitives::FixedBytes;
use alloy_rpc_types_beacon::{
    events::light_client_finality::SyncAggregate, header::HeaderResponse,
};
use bls12_381::{G1Affine, G1Projective, G2Affine};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

/// Represents a single epoch update with its inputs and expected outputs
#[derive(Debug, Serialize, Deserialize)]
pub struct RecursiveEpochUpdate {
    /// Input data for the epoch circuit
    pub inputs: RecursiveEpochInputs,
    // Expected outputs after processing.
    pub outputs: RecursiveEpochOutput,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveEpochOutput {
    pub beacon_header_root: FixedBytes<32>,
    pub beacon_state_root: FixedBytes<32>,
    pub beacon_height: u64,
    pub n_signers: u64,
    pub execution_header_root: FixedBytes<32>,
    pub execution_header_height: u64,
    pub current_committee_hash: FixedBytes<32>,
    pub next_committee_hash: FixedBytes<32>,
}

/// Represents the inputs for recursive epoch update processing using native types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveEpochInputs {
    /// The core epoch data
    pub epoch_update: EpochUpdate,
    /// Optional sync committee update data
    pub sync_committee_update: Option<SyncCommitteeData>,
    /// Optional stark proof from previous epoch update
    pub stark_proof: Option<serde_json::Value>,
    /// The output of the previous epoch proof. Required to decommit the output hash of the proof
    pub stark_proof_output: Option<RecursiveEpochOutput>
}

/// Contains all necessary inputs for generating and verifying a single epoch proof (native types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochUpdate {
    /// The beacon chain block header
    pub header: BeaconHeader,
    /// BLS signature point in G2
    pub signature_point: G2Point,
    /// Aggregate public key of all validators
    #[serde(rename = "committee_pub")]
    pub aggregate_pub: G1Point,
    /// Public keys of validators who didn't sign
    pub non_signers: Vec<G1Point>,
    /// Proof of inclusion for the execution payload header
    pub execution_header_proof: ExecutionHeaderProof,
}

/// Represents a beacon chain block header
#[derive(Debug, Clone, Serialize, Deserialize, TreeHash)]
pub struct BeaconHeader {
    /// Slot number of the block
    pub slot: u64,
    /// Index of the block proposer
    pub proposer_index: u64,
    /// Root hash of the parent block
    pub parent_root: FixedBytes<32>,
    /// Root hash of the state
    pub state_root: FixedBytes<32>,
    /// Root hash of the block body
    pub body_root: FixedBytes<32>,
}

/// Represents the public keys of sync committee validators and their aggregate
#[derive(Debug, Clone)]
pub struct SyncCommitteeValidatorPubs {
    /// Individual public keys of all validators in the committee
    pub validator_pubs: Vec<G1Affine>,
    /// Aggregated public key of all validators combined
    pub aggregate_pub: G1Affine,
}

/// Contains sync committee update data for epoch transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCommitteeData {
    /// Beacon chain slot number
    pub beacon_slot: u64,
    /// Merkle branch for next sync committee
    pub next_sync_committee_branch: Vec<FixedBytes<32>>,
    /// Aggregated public key of next sync committee
    pub next_aggregate_sync_committee: FixedBytes<48>,
    /// Root hash of committee keys
    pub committee_keys_root: FixedBytes<32>,
}

impl From<RecursiveEpochInputs> for RecursiveEpochUpdate {
    fn from(val: RecursiveEpochInputs) -> Self {
        RecursiveEpochUpdate {
            outputs: val.clone().into(),
            inputs: val,
        }
    }
}

impl From<RecursiveEpochInputs> for RecursiveEpochOutput {
    fn from(val: RecursiveEpochInputs) -> Self {
        let execution_header_hash = val.epoch_update.execution_header_proof.execution_payload_header.block_hash();
        RecursiveEpochOutput {
            beacon_header_root: val.epoch_update.header.tree_hash_root(),
            beacon_state_root: val.epoch_update.header.state_root,
            beacon_height: val.epoch_update.header.slot,
            n_signers: 512 - val.epoch_update.non_signers.len() as u64,
            execution_header_root: FixedBytes::from_slice(execution_header_hash.0.as_slice()),
            execution_header_height: val.epoch_update.execution_header_proof.execution_payload_header.block_number(),
            current_committee_hash:  get_committee_hash(val.epoch_update.aggregate_pub.0),
            next_committee_hash: FixedBytes::from([0u8; 32]),
        }
    }
}

impl RecursiveEpochInputs {
    /// Creates a new epoch update for the next epoch based on database state
    ///
    /// # Arguments
    /// * `client` - Reference to the beacon chain client
    /// * `db` - Reference to the database
    ///
    /// # Returns
    /// * `Result<Self, EpochUpdateError>` - New epoch update or error
    pub async fn new(
        client: &BeaconRpcClient,
        db: &crate::db::Database,
    ) -> Result<Self, EpochUpdateError> {

        // Query database for latest epoch entry
        let latest_epoch_update = db.get_latest_epoch_update().await
            .map_err(|e| EpochUpdateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        match latest_epoch_update {
            Some(update) => {
                let slot = (update.epoch_number as u64 + 1)  * constants::SLOTS_PER_EPOCH + constants::SLOTS_PER_EPOCH - 1;
                let epoch_update = EpochUpdate::generate_epoch_proof(client, slot).await?;
                let stark_proof = match update.proof_id {
                    Some(proof_id) => {
                        let proof = db.get_proof(proof_id).await.unwrap().unwrap();
                        serde_json::from_str(&proof.proof).unwrap()
                    }
                    None => panic!("No proof id found for epoch update"),
                };

                Ok(Self {
                    epoch_update,
                    sync_committee_update: None,
                    stark_proof: Some(stark_proof),
                    stark_proof_output: update.outputs,
                })
            }
            None => {
                let slot = constants::GENESIS_EPOCH  * constants::SLOTS_PER_EPOCH + constants::SLOTS_PER_EPOCH - 1;
                let epoch_update = EpochUpdate::generate_epoch_proof(client, slot).await?;
                
                Ok(Self {
                    epoch_update,
                    sync_committee_update: None,
                    stark_proof: None,
                    stark_proof_output: None,
                })
            }
        }
    }

    /// Loads recursive epoch inputs from JSON file
    ///
    /// # Arguments
    /// * `slot` - Slot number to load inputs for
    ///
    /// # Returns
    /// * `Result<Self, EpochUpdateError>` - Loaded inputs or error
    pub fn from_json(slot: u64) -> Result<Self, EpochUpdateError> {
        let path = format!("batches/recursive_epoch/{}/input_{}.json", slot, slot);
        let json = fs::read_to_string(path)?;
        let inputs = serde_json::from_str(&json)?;
        Ok(inputs)
    }
}

// impl Exportable for RecursiveEpochInputs { // Changed from EpochUpdate
//     fn export(&self) -> Result<String, ProofError> {
//         let json = serde_json::to_string_pretty(&self).unwrap();
//         let dir_path = format!("batches/recursive_epoch/{}", self.epoch_update.header.slot); // Assuming header is in epoch_update
//         fs::create_dir_all(dir_path.clone()).map_err(EpochUpdateError::Io)?;
//         let path = format!(
//             "{}/input_{}.json",
//             dir_path, self.epoch_update.header.slot // Assuming header is in epoch_update
//         );
//         fs::write(path.clone(), json).map_err(EpochUpdateError::Io)?;
//         Ok(path)
//     }
// }

impl SyncCommitteeValidatorPubs {
    /// Computes the committee hash used throughout the project
    ///
    /// # Returns
    /// * `FixedBytes<32>` - Hash identifying the committee
    pub fn get_committee_hash(&self) -> FixedBytes<32> {
        get_committee_hash(self.aggregate_pub)
    }
}

impl From<Vec<String>> for SyncCommitteeValidatorPubs {
    /// Converts a vector of hex-encoded public key strings into `SyncCommitteeValidatorPubs`.
    ///
    /// # Arguments
    ///
    /// * `validator_pubs` - A vector of hex-encoded public key strings.
    ///
    /// # Returns
    ///
    /// A new `SyncCommitteeValidatorPubs` instance with parsed public keys.
    fn from(validator_pubs: Vec<String>) -> Self {
        let validator_pubs = validator_pubs
            .iter()
            .map(|s| {
                let mut bytes = [0u8; 48];
                let hex_str = s.trim_start_matches("0x");
                hex::decode_to_slice(hex_str, &mut bytes).unwrap();
                G1Affine::from_compressed(&bytes).unwrap()
            })
            .collect::<Vec<_>>();

        // Aggregate all public keys into a single G1Projective point
        let aggregate_pub = validator_pubs
            .iter()
            .fold(G1Projective::identity(), |acc, pubkey| {
                acc.add_mixed(pubkey)
            });
        Self {
            validator_pubs,
            aggregate_pub: aggregate_pub.into(),
        }
    }
}

impl EpochUpdate {
    /// Generates an epoch proof by fetching and processing beacon chain data
    ///
    /// # Arguments
    /// * `client` - Reference to the beacon chain client
    /// * `slot` - Slot number to generate proof for
    ///
    /// # Returns
    /// * `Result<EpochUpdate, EpochUpdateError>` - Generated inputs or error
    pub(crate) async fn generate_epoch_proof(
        client: &BeaconRpcClient,
        mut slot: u64,
    ) -> Result<EpochUpdate, EpochUpdateError> {
        let mut attempts = 0;

        let header = loop {
            match client.get_header(slot).await {
                Ok(header) => break header,
                Err(BeaconError::EmptySlot(_)) => {
                    attempts += 1;
                    if attempts >= constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS {
                        return Err(EpochUpdateError::Client(
                            BeaconError::EmptySlot(slot).into(),
                        ));
                    }
                    slot += 1;
                    info!(
                        "Empty slot detected! Attempt {}/{}. Fetching slot: {}",
                        attempts,
                        constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS,
                        slot
                    );
                }
                Err(e) => return Err(EpochUpdateError::Client(e.into())), // Propagate other errors immediately
            }
        };

        let sync_agg = client
            .get_sync_aggregate(slot)
            .await
            .map_err(ClientError::Beacon)?;
        let validator_pubs = client
            .get_sync_committee_validator_pubs(slot)
            .await
            .map_err(ClientError::Beacon)?;
        // Process the sync committee data
        let signature_point = Self::extract_signature_point(&sync_agg)?;
        let non_signers = Self::derive_non_signers(&sync_agg, &validator_pubs);

        Ok(EpochUpdate {
            header: header.into(),
            signature_point,
            aggregate_pub: G1Point(validator_pubs.aggregate_pub),
            non_signers: non_signers.iter().map(|p| G1Point(*p)).collect(),
            execution_header_proof: ExecutionHeaderProof::fetch_proof(client, slot).await?,
        })
    }

    /// Extracts and validates the BLS signature point from the sync aggregate
    ///
    /// # Arguments
    /// * `sync_agg` - Sync aggregate containing the signature
    ///
    /// # Returns
    /// * `Result<G2Point, EpochUpdateError>` - Validated signature point or error
    fn extract_signature_point(sync_agg: &SyncAggregate) -> Result<G2Point, EpochUpdateError> {
        let mut bytes = [0u8; 96];
        bytes.copy_from_slice(&sync_agg.sync_committee_signature.0);
        match G2Affine::from_compressed(&bytes).into() {
            Some(point) => Ok(G2Point(point)),
            None => Err(EpochUpdateError::InvalidBLSPoint),
        }
    }

    /// Identifies validators who didn't sign the sync committee message
    ///
    /// # Arguments
    /// * `sync_aggregate` - Sync aggregate containing participation bits
    /// * `validator_pubs` - Public keys of all validators
    ///
    /// # Returns
    /// * `Vec<G1Affine>` - Public keys of non-signing validators
    fn derive_non_signers(
        sync_aggregate: &SyncAggregate,
        validator_pubs: &SyncCommitteeValidatorPubs,
    ) -> Vec<G1Affine> {
        let bits = Self::convert_bits_to_bool_array(&sync_aggregate.sync_committee_bits);
        validator_pubs
            .validator_pubs
            .iter()
            .enumerate()
            .filter_map(|(i, pubkey)| if !bits[i] { Some(*pubkey) } else { None })
            .collect()
    }

    /// Converts a byte array of participation bits into a boolean array
    ///
    /// # Arguments
    /// * `bits` - Byte array of participation bits
    ///
    /// # Returns
    /// * `Vec<bool>` - Array where true indicates a validator signed
    fn convert_bits_to_bool_array(bits: &[u8]) -> Vec<bool> {
        bits.iter()
            .flat_map(|byte| (0..8).map(move |i| (byte & (1 << i)) != 0))
            .collect()
    }
}

impl From<HeaderResponse> for BeaconHeader {
    fn from(header: HeaderResponse) -> Self {
        Self {
            slot: header.data.header.message.slot,
            proposer_index: header.data.header.message.proposer_index,
            parent_root: header.data.header.message.parent_root,
            state_root: header.data.header.message.state_root,
            body_root: header.data.header.message.body_root,
        }
    }
}

/// Point on the G1 curve used for public keys
#[derive(Debug, Clone)]
pub struct G1Point(pub G1Affine);

/// Point on the G2 curve used for signatures
#[derive(Debug, Clone)]
pub struct G2Point(pub G2Affine);

impl Serialize for G1Point {
    /// Serializes a G1 point to its uncompressed form
    ///
    /// Outputs x and y coordinates as hex strings
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let uncompressed = self.0.to_uncompressed();
        let mut x_bytes = [0u8; 48];
        let mut y_bytes = [0u8; 48];

        x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);

        serde_json::json!({
            "x": format!("0x{}", hex::encode(x_bytes)),
            "y": format!("0x{}", hex::encode(y_bytes))
        })
        .serialize(serializer)
    }
}

impl Serialize for G2Point {
    /// Serializes a G2 point to its uncompressed form
    ///
    /// Outputs x0, x1, y0, y1 coordinates as hex strings
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let uncompressed = self.0.to_uncompressed();
        let mut x0_bytes = [0u8; 48];
        let mut x1_bytes = [0u8; 48];
        let mut y0_bytes = [0u8; 48];
        let mut y1_bytes = [0u8; 48];
        x0_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);
        x1_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y0_bytes.copy_from_slice(&uncompressed.as_ref()[144..192]);
        y1_bytes.copy_from_slice(&uncompressed.as_ref()[96..144]);
        serde_json::json!({
            "x0": format!("0x{}", hex::encode(x0_bytes)),
            "x1": format!("0x{}", hex::encode(x1_bytes)),
            "y0": format!("0x{}", hex::encode(y0_bytes)),
            "y1": format!("0x{}", hex::encode(y1_bytes))
        })
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for G1Point {
    /// Deserializes a G1 point from its uncompressed form
    ///
    /// Expects x and y coordinates as hex strings
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into a Value first
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        // Extract x and y coordinates
        let x_str = value["x"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x coordinate"))?;
        let y_str = value["y"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y coordinate"))?;

        // Safely remove "0x" prefix if it exists
        let x_hex = x_str.strip_prefix("0x").unwrap_or(x_str);
        let y_hex = y_str.strip_prefix("0x").unwrap_or(y_str);

        let x_bytes = hex::decode(x_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x hex: {}", e)))?;
        let y_bytes = hex::decode(y_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y hex: {}", e)))?;

        // Combine into uncompressed format
        let mut uncompressed = [0u8; 96];
        uncompressed[0..48].copy_from_slice(&x_bytes);
        uncompressed[48..96].copy_from_slice(&y_bytes);

        // Convert to G1Affine point
        let point = G1Affine::from_uncompressed(&uncompressed).unwrap();

        Ok(G1Point(point))
    }
}

impl<'de> Deserialize<'de> for G2Point {
    /// Deserializes a G2 point from its uncompressed form
    ///
    /// Expects x0, x1, y0, y1 coordinates as hex strings
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into a Value first
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        // Extract coordinates
        let x0_str = value["x0"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x0 coordinate"))?;
        let x1_str = value["x1"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x1 coordinate"))?;
        let y0_str = value["y0"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y0 coordinate"))?;
        let y1_str = value["y1"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y1 coordinate"))?;

        // Safely remove "0x" prefix if it exists
        let x0_hex = x0_str.strip_prefix("0x").unwrap_or(x0_str);
        let x1_hex = x1_str.strip_prefix("0x").unwrap_or(x1_str);
        let y0_hex = y0_str.strip_prefix("0x").unwrap_or(y0_str);
        let y1_hex = y1_str.strip_prefix("0x").unwrap_or(y1_str);

        // Decode hex strings to bytes
        let x0_bytes = hex::decode(x0_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x0 hex: {}", e)))?;
        let x1_bytes = hex::decode(x1_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x1 hex: {}", e)))?;
        let y0_bytes = hex::decode(y0_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y0 hex: {}", e)))?;
        let y1_bytes = hex::decode(y1_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y1 hex: {}", e)))?;

        // Combine into uncompressed format
        let mut uncompressed = [0u8; 192];
        uncompressed[0..48].copy_from_slice(&x1_bytes);
        uncompressed[48..96].copy_from_slice(&x0_bytes);
        uncompressed[96..144].copy_from_slice(&y1_bytes);
        uncompressed[144..192].copy_from_slice(&y0_bytes);

        // Convert to G2Affine point
        let point = G2Affine::from_uncompressed(&uncompressed).unwrap();

        Ok(G2Point(point))
    }
}

/// Possible errors that can occur during epoch update operations
#[derive(Debug, Error)]
pub enum EpochUpdateError {
    /// Error during Cairo program execution
    // #[error("Cairo run error: {0}")]
    // Cairo(#[from] CairoError),
    /// Database operation error
    // #[error("Database error: {0}")]
    // Database(#[from] DatabaseError),
    /// File system operation error
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization/deserialization error
    #[error("Deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// Client communication error
    #[error("Beacon error: {0}")]
    Client(#[from] ClientError),
    /// Error processing execution header
    #[error("Execution header error: {0}")]
    ExecutionHeader(#[from] ExecutionHeaderError),
    /// Invalid BLS cryptographic point
    #[error("Invalid BLS point")]
    InvalidBLSPoint,
}

