%builtins output pedersen range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin, HashBuiltin
from cairo.src.verify_stone import verify_cairo_proof
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.builtin_poseidon.poseidon import poseidon_hash_many

from cairo.src.utils import pow2alloc128
from sha import SHA256
from debug import print_felt_hex, print_string
from cairo.src.types import EpochUpdate, EpochUpdateOutput, CircuitOutput
from cairo.src.verify_epoch import run_epoch_update
from starkware.cairo.stark_verifier.core.stark import StarkProof

const BOOTLOADER_PROGRAM_HASH = 0x5AB580B04E3532B6B18F81CFA654A05E29DD8E2352D88DF1E765A84072DB07;

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

    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();

    local epoch_update: EpochUpdate;
    local is_genesis: felt;
    local program_hash: felt;
    %{ write_epoch_update_inputs() %}

    with pow2_array, sha256_ptr {
        let (local epoch_update_output) = run_epoch_update(epoch_update); 
    }

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);
    
    if (is_genesis == 1) {
        tempvar committee_hash = Uint256(low=0xe5fec5cd2304cab6086b1eea025ccd74, high=0xf32b83714599ab70193ba4597159560c);
        // ensure we match the genesis committee hash
        assert committee_hash.low = epoch_update_output.current_committee_hash.low;
        assert committee_hash.high = epoch_update_output.current_committee_hash.high;

        let next_committee_hash = Uint256(low=0x0, high=0x0);
        write_circuit_output(epoch_output=epoch_update_output, next_committee_hash=next_committee_hash);
        return ();
    } else {
        local expected_proof_output: CircuitOutput;
        %{ load_expected_proof_output() %}

        // Check that expected matches the committee hash that was used to sign
        assert expected_proof_output.current_committee_hash.low = epoch_update_output.current_committee_hash.low;
        assert expected_proof_output.current_committee_hash.high = epoch_update_output.current_committee_hash.high;

        print_string('after_committee_check');

        tempvar expected_verifier_output = cast(
            new (
                1, 15, program_hash,
                expected_proof_output.beacon_header_root.low,
                expected_proof_output.beacon_header_root.high,
                expected_proof_output.beacon_state_root.low,
                expected_proof_output.beacon_state_root.high,
                expected_proof_output.beacon_height,
                expected_proof_output.n_signers,
                expected_proof_output.execution_header_root.low,
                expected_proof_output.execution_header_root.high,
                expected_proof_output.execution_header_height,
                expected_proof_output.current_committee_hash.low,
                expected_proof_output.current_committee_hash.high,
                expected_proof_output.next_committee_hash.low,
                expected_proof_output.next_committee_hash.high
            ), felt*
        );

        print_string('proof_output');
        print_felt_hex(expected_verifier_output[0]);
        print_felt_hex(expected_verifier_output[1]);
        print_felt_hex(expected_verifier_output[2]);
        print_felt_hex(expected_verifier_output[3]);
        print_felt_hex(expected_verifier_output[4]);
        print_felt_hex(expected_verifier_output[5]);
        print_felt_hex(expected_verifier_output[6]);
        print_felt_hex(expected_verifier_output[7]);
        print_felt_hex(expected_verifier_output[8]);
        print_felt_hex(expected_verifier_output[9]);
        print_felt_hex(expected_verifier_output[10]);
        print_felt_hex(expected_verifier_output[11]);
        print_felt_hex(expected_verifier_output[12]);
        print_felt_hex(expected_verifier_output[13]);
        print_felt_hex(expected_verifier_output[14]);
        print_felt_hex(expected_verifier_output[15]);

        print_string('after_construct_array');

        let (expected_output_hash: felt) = poseidon_hash_many(n=16, elements=expected_verifier_output);
        print_string('expected_out');
        print_felt_hex(expected_output_hash);

        print_string('before_verify_proof');
        %{ write_stark_proof_inputs() %}
        let (proof_program_hash, output_hash) = verify_cairo_proof();
        print_string('program_hash');
        print_felt_hex(proof_program_hash);
        print_string('output_hash');
        print_felt_hex(output_hash);

        // Ensure the proof contains the expected values
        assert output_hash = expected_output_hash;
        
        // A Cairo proof is evaluated within the bootloader in Stone Sharp
        assert proof_program_hash = BOOTLOADER_PROGRAM_HASH;

        write_circuit_output(epoch_output=epoch_update_output, next_committee_hash=expected_proof_output.next_committee_hash);

        return ();
    }
}


func write_circuit_output{
    output_ptr: felt*,
}(epoch_output: EpochUpdateOutput, next_committee_hash: Uint256) {
    assert [output_ptr] = epoch_output.beacon_header_root.low;
    assert [output_ptr + 1] = epoch_output.beacon_header_root.high;
    assert [output_ptr + 2] = epoch_output.beacon_state_root.low;
    assert [output_ptr + 3] = epoch_output.beacon_state_root.high;
    assert [output_ptr + 4] = epoch_output.beacon_height;
    assert [output_ptr + 5] = epoch_output.n_signers;
    assert [output_ptr + 6] = epoch_output.execution_header_root.low;
    assert [output_ptr + 7] = epoch_output.execution_header_root.high;
    assert [output_ptr + 8] = epoch_output.execution_header_height;
    assert [output_ptr + 9] = epoch_output.current_committee_hash.low;
    assert [output_ptr + 10] = epoch_output.current_committee_hash.high;
    assert [output_ptr + 11] = next_committee_hash.low;
    assert [output_ptr + 12] = next_committee_hash.high;

    print_string('output_ptr');
    print_felt_hex(output_ptr[0]);
    print_felt_hex(output_ptr[1]);
    print_felt_hex(output_ptr[2]);
    print_felt_hex(output_ptr[3]);
    print_felt_hex(output_ptr[4]);
    print_felt_hex(output_ptr[5]);
    print_felt_hex(output_ptr[6]);
    print_felt_hex(output_ptr[7]);
    print_felt_hex(output_ptr[8]);
    print_felt_hex(output_ptr[9]);
    print_felt_hex(output_ptr[10]);
    print_felt_hex(output_ptr[11]);
    print_felt_hex(output_ptr[12]);

    let output_ptr = output_ptr + 13;

    return ();

}

