use alloy_primitives::FixedBytes;
use beacon_state_proof::state_proof_fetcher::{StateProofFetcher, SyncCommitteeProof, TreeHash};
use bls12_381::{G1Affine, G1Projective};
use cairo_vm::Felt252;
use core::convert::TryInto; // add near the top if not in scope
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::utils::{
        hashing::{get_committee_hash, validator_commitment},
        merkle::poseidon,
    };

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
    // pub validator_pubs: Vec<FixedBytes<48>>,
    // /// Precomputed root of the committee key merkle tree (poseidon)
    // pub precomputed_root: FixedBytes<32>,
}

impl SyncCommitteeData {
    /// Creates a new sync committee update for a given slot
    pub async fn new(
        client: &crate::clients::beacon_chain::BeaconRpcClient,
        slot: u64,
    ) -> Result<SyncCommitteeData, SyncCommitteeError> {
        let state_proof_fetcher = StateProofFetcher::new(client.rpc_url.clone());
        let proof = state_proof_fetcher
            .fetch_next_sync_committee_proof(slot)
            .await?;

        Ok(SyncCommitteeData::from(proof))
    }

    /// Computes the state root by hashing the committee keys root and the aggregate pubkey
    pub fn compute_state_root(&self) -> FixedBytes<32> {
        let mut padded_aggregate = vec![0u8; 64];
        padded_aggregate[..48].copy_from_slice(&self.next_aggregate_sync_committee[..]);
        let aggregate_root: FixedBytes<32> =
            FixedBytes::from_slice(&Sha256::digest(&padded_aggregate));

        let mut leaf_data = [0u8; 64];
        leaf_data[0..32].copy_from_slice(self.committee_keys_root.as_slice());
        leaf_data[32..64].copy_from_slice(aggregate_root.as_slice());
        let leaf = FixedBytes::from_slice(&Sha256::digest(leaf_data));

        crate::utils::merkle::sha256::hash_path(self.next_sync_committee_branch.clone(), leaf, 55)
    }
}

// impl SyncCommitteeData {
//     pub fn new(committee_proof: SyncCommitteeProof) -> Self {
//         let mut validator_pubs: Vec<FixedBytes<48>> = committee_proof
//             .next_sync_committee
//             .pubkeys
//             .iter()
//             .map(|pubkey| FixedBytes::from_slice(pubkey.as_serialized()))
//             .collect();
//         validator_pubs.sort();

//         let precomputed_root = FixedBytes::from([0u8; 32]);

//         // Self::precompute_root(validator_pubs.clone());

//         let committee_keys_root = &committee_proof.next_sync_committee.pubkeys.tree_hash_root();

//         Self {
//             beacon_slot: committee_proof.slot,
//             next_sync_committee_branch: committee_proof
//                 .proof
//                 .into_iter()
//                 .map(|bytes| FixedBytes::from_slice(bytes.as_slice()))
//                 .collect(),
//             next_aggregate_sync_committee: FixedBytes::from_slice(
//                 committee_proof
//                     .next_sync_committee
//                     .aggregate_pubkey
//                     .as_serialized(),
//             ),
//             committee_keys_root: FixedBytes::from_slice(committee_keys_root.as_slice()),
//             validator_pubs,
//             precomputed_root,
//         }
//     }

//     // fn precompute_root(validator_pubs: Vec<FixedBytes<48>>) -> FixedBytes<32> {
//     //     // I first want to hash the validator pubkeys with poseidon hash

//     //     let key_hashes = validator_pubs.iter().map(|pubkey| poseidon_hash_many(pubkey)).collect();

//     // }
// }

impl From<SyncCommitteeProof> for SyncCommitteeData {
    fn from(committee_proof: SyncCommitteeProof) -> Self {
        let committee_keys_root = &committee_proof.next_sync_committee.pubkeys.tree_hash_root();

        Self {
            beacon_slot: committee_proof.slot,
            next_sync_committee_branch: committee_proof
                .proof
                .into_iter()
                .map(|bytes| FixedBytes::from_slice(bytes.as_slice()))
                .collect(),
            next_aggregate_sync_committee: FixedBytes::from_slice(
                committee_proof
                    .next_sync_committee
                    .aggregate_pubkey
                    .as_serialized(),
            ),
            committee_keys_root: FixedBytes::from_slice(committee_keys_root.as_slice()),
        }
    }
}

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

/// Possible errors that can occur during sync committee operations
#[derive(Debug, Error)]
pub enum SyncCommitteeError {
    /// Error communicating with beacon node
    #[error("Beacon error: {0}")]
    Beacon(#[from] crate::clients::beacon_chain::BeaconError),
    /// Error processing beacon state proof
    #[error("Beacon state proof error")]
    BeaconStateProof(beacon_state_proof::error::Error),
}

impl From<beacon_state_proof::error::Error> for SyncCommitteeError {
    fn from(error: beacon_state_proof::error::Error) -> Self {
        SyncCommitteeError::BeaconStateProof(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeUpdateData {
    pub slot: u64,
    pub next_sync_committee_branch: Vec<FixedBytes<32>>,
    pub next_aggregate_sync_committee: FixedBytes<48>,
    pub validator_pubs: Vec<FixedBytes<48>>,
    pub committee_keys_root: FixedBytes<32>,
    pub expected_validator_root: FixedBytes<32>,
}

impl CommitteeUpdateData {
    pub async fn new(
        client: &crate::clients::beacon_chain::BeaconRpcClient,
        slot: u64,
    ) -> Result<CommitteeUpdateData, SyncCommitteeError> {
        println!("Fetching next sync committee proof for slot: {:?}", slot);
        let state_proof_fetcher = StateProofFetcher::new(client.rpc_url.clone());
        let proof = state_proof_fetcher
            .fetch_next_sync_committee_proof(slot)
            .await?;
        // println!("sync_committee_data: {:#?}", proof);

        let validator_pubs = proof
            .next_sync_committee
            .pubkeys
            .iter()
            .map(|pubkey| {
                let affine = G1Affine::from_compressed(&pubkey.serialize()).unwrap();
                FixedBytes::from_slice(affine.to_compressed().as_slice())
            })
            .collect::<Vec<FixedBytes<48>>>();

        let validator_root = Self::compute_validator_pub_merkle_tree(validator_pubs.clone());

        let data = CommitteeUpdateData {
            slot: proof.slot,
            next_sync_committee_branch: proof.proof.clone(),
            next_aggregate_sync_committee: FixedBytes::from_slice(
                proof.next_sync_committee.aggregate_pubkey.as_serialized(),
            ),
            committee_keys_root: FixedBytes::from_slice(
                proof
                    .next_sync_committee
                    .pubkeys
                    .tree_hash_root()
                    .as_slice(),
            ),
            validator_pubs,
            expected_validator_root: validator_root,
        };

        // println!("data: {:#?}", data);
        Ok(data)
    }

    pub fn compute_validator_pub_merkle_tree(
        validator_pubs: Vec<FixedBytes<48>>,
    ) -> FixedBytes<32> {
        let validator_pubs = validator_pubs.clone();
        let validator_points = validator_pubs
            .iter()
            .map(|pubkey| {
                let bytes: [u8; 48] = pubkey.as_slice().try_into().expect("length 48");
                G1Affine::from_compressed(&bytes).unwrap()
            })
            .collect::<Vec<G1Affine>>();
        let validator_commitments = validator_points
            .iter()
            .map(|point| validator_commitment(*point))
            .map(|commitment| Felt252::from_bytes_be_slice(commitment.as_slice()))
            .collect::<Vec<Felt252>>();
        // println!("validator_commitments: {:?}", validator_commitments);

        let root = poseidon::compute_root(validator_commitments);
        FixedBytes::from_slice(&root.to_bytes_be())
    }
}
