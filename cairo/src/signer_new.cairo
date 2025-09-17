%builtins output pedersen range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin, HashBuiltin
from starkware.cairo.common.builtin_poseidon.poseidon import poseidon_hash_many

from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from cairo.src.ssz import MerkleTree, MerkleUtils
from cairo.src.merkle import PoseidonMerkleTree
from definitions import UInt384, G1Point
from debug import print_felt_hex, print_string
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from cairo.src.utils import pow2alloc128
from sha import HashUtils, SHA256
from cairo.src.committee_update import decompress_g1
from ec_ops import derive_g1_point_from_x
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

        let (commitments: felt*) = alloc();
        compute_validator_pub_commitments(committee_update_data.validator_pubs, 0, commitments);

        let val_root = PoseidonMerkleTree.compute_root(leafs=commitments, leafs_len=512);
        print_string('val root');
        print_felt_hex(val_root);
    }

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    return ();
}





func compute_validator_pub_commitments{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    range_check96_ptr: felt*,
    poseidon_ptr: PoseidonBuiltin*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*, 
    pow2_array: felt*
}(
    pubs: UInt384*, counter: felt, result: felt*
) {
    alloc_locals;
    
    if (counter == 512) {
        return ();
    }

    // Decompress G1 point and perform sanity checks
    let (flags, x_point) = decompress_g1(pubs[counter]);
    assert flags.compression_bit = 1;
    assert flags.infinity_bit = 0;

    let (point) = derive_g1_point_from_x(curve_id=1, x=x_point, s=flags.sign_bit);

    let (commitment, _) = validator_commitment(point);

    assert result[counter] = commitment;

    return compute_validator_pub_commitments(pubs, counter + 1, result);
}

func validator_commitment{range_check_ptr, poseidon_ptr: PoseidonBuiltin*}(
    point: G1Point
) -> (commitment: felt, pubkey: G1Point) {
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

    print_string('commitment');
    print_felt_hex(commitment);
    return (commitment=commitment, pubkey=point);
}