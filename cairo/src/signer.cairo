from starkware.cairo.common.cairo_builtins import ModBuiltin, PoseidonBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.alloc import alloc
from debug import print_felt_hex, print_string, print_felt
from starkware.cairo.common.builtin_poseidon.poseidon import poseidon_hash_many
from definitions import G1Point
from ec_ops import add_ec_points, is_on_curve_g1, sub_ec_points
from sha import HashUtils, SHA256
from cairo.src.types import SignerDataNew
from cairo.src.merkle import PoseidonMerkleTree


func generate_block_signer_pub{
    range_check_ptr,
    pow2_array: felt*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    sha256_ptr: felt*,
}(signer_data: SignerDataNew) -> (agg_pub: G1Point) {
    alloc_locals;

    assert_signers_inclusion(signer_data, 0);


    let (agg_pub) = aggregate_signer_pubs(signer_data, 0);
    return (agg_pub=agg_pub);
}

func assert_signers_inclusion{
    range_check_ptr,
    pow2_array: felt*,
    poseidon_ptr: PoseidonBuiltin*,
    sha256_ptr: felt*,
}(signer_data: SignerDataNew, counter: felt) {
    alloc_locals;

    if (counter == signer_data.n_signers) {
        return ();
    }

    let (commitment) = validator_commitment(signer_data.signers[counter]);
    PoseidonMerkleTree.verify_merkle_path_poseidon(
        path=signer_data.proofs[counter],
        path_len=signer_data.proofs_len,
        leaf=commitment,
        index=signer_data.indexes[counter],
        expected_root=signer_data.validator_root
    );

    return assert_signers_inclusion(signer_data, counter + 1);
}

func aggregate_signer_pubs{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(signer_data: SignerDataNew, counter: felt) -> (res: G1Point) {
    // Base case: last signer
    if (counter == signer_data.n_signers - 1) {
        return (signer_data.signers[counter],);
    }


    let (tail_res) = aggregate_signer_pubs(signer_data, counter + 1);
    return add_ec_points(1, tail_res, signer_data.signers[counter]);
}

// func faster_fast_aggregate_signer_pubs{
//     range_check_ptr,
//     pow2_array: felt*,
//     range_check96_ptr: felt*,
//     add_mod_ptr: ModBuiltin*,
//     mul_mod_ptr: ModBuiltin*,
//     sha256_ptr: felt*,
// }(signer_data: SignerData) -> (committee_hash: Uint256, agg_pub: G1Point, n_non_signers: felt) {
//     alloc_locals;

//     let non_signers = signer_data.non_signers;
//     let n_non_signers = signer_data.n_non_signers;
//     let committee_pub = signer_data.committee_pub;

//     // Call the recursive function to aggregate public keys
//     if (n_non_signers != 0) {
//         let (agg_non_signer_pub) = faster_fast_aggregate_signer_pubs_inner(
//             non_signers, n_non_signers
//         );
//         let (signer_key) = sub_ec_points(1, committee_pub, agg_non_signer_pub);
//         let committee_hash = commit_committee_key(point=committee_pub);
//         return (committee_hash=committee_hash, agg_pub=signer_key, n_non_signers=n_non_signers);
//     } else {
//         let committee_hash = commit_committee_key(point=committee_pub);
//         return (committee_hash=committee_hash, agg_pub=committee_pub, n_non_signers=n_non_signers);
//     }
// }

// // Recursive helper function for fast aggregation of signer public keys
// // In this version we add the non-signers (which is cheaper then subtracting)
// // And then we subtract the result from the committee aggregate key. Saves avg 5k steps in normal epoch proof.
// func faster_fast_aggregate_signer_pubs_inner{
//     range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
// }(non_signers: G1Point*, n_non_signers: felt) -> (res: G1Point) {
//     // Base case: if there are no non-signers, verify agg_key is on the curve and return it
//     if (n_non_signers == 1) {
//         let (on_curve) = is_on_curve_g1(1, non_signers[0]);
//         assert on_curve = 1;
//         return (non_signers[0],);
//     }

//     // Verify that the current non-signer's public key is on the curve
//     let (on_curve) = is_on_curve_g1(1, non_signers[0]);
//     assert on_curve = 1;

//     // Recursively process the remaining non-signer keys
//     let (res) = faster_fast_aggregate_signer_pubs_inner(
//         non_signers + G1Point.SIZE, n_non_signers - 1
//     );
//     // Subtract the current non-signer's public key from the aggregated result
//     return add_ec_points(1, res, non_signers[0]);  // try adding non signers and the subbing result
// }

// This function generates the hash of an aggregate committee key.
// This hash is stored in the cairo1 state, and is used to check if the correct committee was used
func commit_committee_key{range_check_ptr, sha256_ptr: felt*, pow2_array: felt*}(
    point: G1Point
) -> Uint256 {
    alloc_locals;

    let (x_chunks) = HashUtils.chunk_uint384(point.x);
    let (y_chunks) = HashUtils.chunk_uint384(point.y);

    // Concatenate x and y chunks and compute the hash
    memcpy(dst=x_chunks + 12, src=y_chunks, len=12);
    let (committee_point_hash_chunks) = SHA256.hash_bytes(x_chunks, 96);
    let committee_point_hash = HashUtils.chunks_to_uint256(committee_point_hash_chunks);

    return committee_point_hash;
}

func validator_commitment{range_check_ptr, poseidon_ptr: PoseidonBuiltin*}(
    point: G1Point
) -> (commitment: felt) {
    alloc_locals;

    let (chunks: felt*) = alloc();
    assert [chunks] = point.x.d3;
    assert [chunks + 1] = point.x.d2;
    assert [chunks + 2] = point.x.d1;
    assert [chunks + 3] = point.x.d0;
    assert [chunks + 4] = point.y.d3;
    assert [chunks + 5] = point.y.d2;
    assert [chunks + 6] = point.y.d1;
    assert [chunks + 7] = point.y.d0;

    let (commitment) = poseidon_hash_many(8, chunks);

    return (commitment=commitment);
}