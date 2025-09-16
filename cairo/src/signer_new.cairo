%builtins output pedersen range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin, HashBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from cairo.src.ssz import MerkleTree, MerkleUtils
from definitions import UInt384, G1Point
from debug import print_felt_hex, print_string
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from cairo.src.utils import pow2alloc128
from sha import HashUtils, SHA256
from cairo.src.types import CommitteeUpdateData

func main{
    output_ptr: felt*,
    pedersen_ptr: HashBuiltin*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;

    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    let (pow2_array) = pow2alloc128();


    local committee_update_data: CommitteeUpdateData;
    %{ write_committee_update_data() %}

    with pow2_array {
        let val = MerkleUtils.chunks_to_uint256(committee_update_data.committee_keys_root);
    }


    print_felt_hex(val.low);
    print_felt_hex(val.high);
    with sha256_ptr, pow2_array {
        let committee_root = compute_committee_root(committee_update_data.validator_pubs);
    }



    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    return ();
}



func compute_committee_root{range_check_ptr,bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*}(
    committee_keys: UInt384*
) -> Uint256 {
    alloc_locals;

    let (ssz_leafs: Uint256*) = alloc();
    compute_committee_root_inner(committee_keys, 0, ssz_leafs);

    let root = MerkleTree.compute_root(leafs=ssz_leafs, leafs_len=512);
    print_string('committee root');
    print_felt_hex(root.low);
    print_felt_hex(root.high);

    return root;
}

func compute_committee_root_inner{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(
    committee_keys: UInt384*, counter: felt, result: Uint256*
) {
    alloc_locals;
    if (counter == 512) {
        return ();
    }

    let (aggregate_committee_key_chunks) = HashUtils.chunk_uint384(committee_keys[counter]);
    // Pad the key to 64 bytes
    memset(dst=aggregate_committee_key_chunks + 12, value=0, n=4);
    let (aggregate_committee_root) = SHA256.hash_bytes(aggregate_committee_key_chunks, 64);

    let val = MerkleUtils.chunks_to_uint256(aggregate_committee_root);

    assert result[counter] = val;
    
    return compute_committee_root_inner(committee_keys, counter + 1, result);
    
}