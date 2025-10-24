use alloy_primitives::FixedBytes;
use bls12_381::G1Affine;
use cairo_vm::Felt252;
use sha2::{Digest, Sha256};
use starknet_crypto::poseidon_hash_many;

pub fn get_committee_hash(point: G1Affine) -> FixedBytes<32> {
    let mut hasher = Sha256::new();
    let uncompressed = point.to_uncompressed();
    hasher.update(uncompressed.as_ref());
    FixedBytes::from_slice(&hasher.finalize())
}

pub fn validator_commitment(pubkey: G1Affine) -> FixedBytes<32> {
    let uncompressed = pubkey.to_uncompressed();
    let bytes = uncompressed.as_ref();

    // Split into 12-byte chunks and convert each to Felt
    let mut felts = Vec::new();
    for chunk in bytes.chunks(12) {
        let mut padded = [0u8; 32];
        let chunk_len = chunk.len();
        padded[32 - chunk_len..].copy_from_slice(chunk);
        felts.push(Felt252::from_bytes_be_slice(&padded));
    }

    let commitment = poseidon_hash_many(&felts);

    FixedBytes::from_slice(&commitment.to_bytes_be())
}

mod test {
    

    

    

    #[test]
    fn test_validator_commitment() {
        let point_1 = G1Point::deserialize(serde_json::json!({
            "x": "0x0c9fefe233d0d657349b7efcdc368f5aaead27071d224af780874751e7d241f6b88f7650fbb4133043b24bbebc12aa48",
            "y": "0x090e44d00b3be51c48930e4633678dd226222a2854affffa15d878cbf62af49de2f7954c455ffe859aebc5157303cd33"
        })).unwrap();
        let commitment = validator_commitment(point_1.0);
        assert_eq!(
            commitment,
            FixedBytes::from_slice(
                &hex::decode("04fae68b767518d649045de18a1254e78a384b5d14492cf2236f205e7c1612c7")
                    .unwrap()
            )
        );
    }

    #[test]
    fn test_validator_commitment_2() {
        let point_2 = G1Point::deserialize(serde_json::json!({
            "x": "0x08158d759eafd2205c770f166829fd61e8f17b2c13f440777eaf45f4d88a6e2028bc507680ff435882d5fb462f813735",
            "y": "0x037e44b677e72dbc6a9bdc8868ace739b4985c6a5f8fedd6a8fe8f24e608584c4ee66897ac677b1cb28dcf98e157244c"
        })).unwrap();
        let commitment = validator_commitment(point_2.0);
        assert_eq!(
            commitment,
            FixedBytes::from_slice(
                &hex::decode("065e055373e33abaf70d0bbcdfb9d844ca01cc17ac0029dd94c3cce4eeb1569e")
                    .unwrap()
            )
        );
    }
}

// Steps:

// When new committee is decommited:
// 1. Decommit each key from committee root in beacon state
// 1.a. Optional: ensure keys are on curve, then we can skip this during aggregation
// 2. Hash each key to create the leaf (ideally only poseidon) -> DONE
// 3. Build merkle tree with poseidon

// To compute signer:
// 1. for each pub key in input, we have a merkle path
// 2. hash each key to create the leaf (ideally only poseidon)
// 3. run merkle inclusion proof
// 4. add all signers together to verify signature

// Open Questions:
// 1. Should we hash the entire pubKey or compressed one?
// 2. how do we hash the pubKey, need to serialize it! Poseidon is annoying to work with

// -> G1 Key: (Uint384, Uint384) -> so 8 felt252. this is manageable
