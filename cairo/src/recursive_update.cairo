%builtins output pedersen range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin, HashBuiltin
// from cairo.src.verify_stone import verify_cairo_proof
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.builtin_poseidon.poseidon import poseidon_hash_many
from starkware.cairo.common.alloc import alloc
from definitions import UInt384

from cairo.src.utils import pow2alloc128
from sha import SHA256
from debug import print_felt_hex, print_string
from cairo.src.types import EpochUpdate, EpochUpdateOutput, CircuitOutput, CommitteeUpdateData
from cairo.src.verify_epoch import run_epoch_update
from starkware.cairo.stark_verifier.core.stark import StarkProof
from cairo.src.committee_update import run_committee_update
from cairo.src.utils import felt_divmod
from cairo.src.signer import validator_commitment

const BOOTLOADER_PROGRAM_HASH = 0x5AB580B04E3532B6B18F81CFA654A05E29DD8E2352D88DF1E765A84072DB07;
const SYNC_COMMITTEE_PERIOD = 8192;

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

    local epoch_update: EpochUpdate;
    local is_genesis: felt;
    local is_committee_update: felt; // do we add a new committee? 1 if yes, 0 if no
    local program_hash: felt;
    %{ write_epoch_update_inputs() %}

    if (is_genesis == 1) {
        with pow2_array, sha256_ptr {
            let (epoch_update_output) = handle_genesis_case(epoch_update);
        }
        let next_validator_root = 0;
        assert is_committee_update = 0;
        write_circuit_output(epoch_output=epoch_update_output, next_validator_root=next_validator_root, is_committee_transition=0);

        SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

        return ();
    } else {
        print_string('recursive case');

        local expected_proof_output: CircuitOutput;
        %{ load_expected_proof_output() %}

        let (previous_term, _) = felt_divmod(expected_proof_output.beacon_height, SYNC_COMMITTEE_PERIOD);
        let (current_term, _) = felt_divmod(epoch_update.header.slot.low, SYNC_COMMITTEE_PERIOD);

        local is_committee_transition: felt;
        if (previous_term == current_term) {
            is_committee_transition = 0;
        } else {
            is_committee_transition = 1;
        }

        print_string('is_committee_transition');
        print_felt_hex(is_committee_transition);

        with pow2_array, sha256_ptr {
            let (epoch_update_output, next_validator_root) = handle_recursive_case(epoch_update, program_hash, is_committee_transition, expected_proof_output);
        }
        print_string('confirmed epoch');

        if (is_committee_update == 1) {
            print_string('committee update');
            // sanity check: next_validator_root should be 0x0 if we update
            assert next_validator_root = 0x0;

            local committee_update_data: CommitteeUpdateData;
            %{ write_committee_update_data() %}
            with pow2_array, sha256_ptr {
                let (state_root, new_next_validator_root) = run_committee_update(committee_update_data);
            }
            print_string('committee update done');

            // Ensure a valid state root is used to decommit new next_validator_root
            assert epoch_update_output.beacon_state_root.low = state_root.low;
            assert epoch_update_output.beacon_state_root.high = state_root.high;
            write_circuit_output(epoch_output=epoch_update_output, next_validator_root=new_next_validator_root, is_committee_transition=is_committee_transition);

            SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);
            return ();
        } else {
            print_string('no committee update');
            write_circuit_output(epoch_output=epoch_update_output, next_validator_root=next_validator_root, is_committee_transition=is_committee_transition);
            
            SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);
            return ();
        }
    }
}

func handle_recursive_case{
    output_ptr: felt*,
    pedersen_ptr: HashBuiltin*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    sha256_ptr: felt*,
    pow2_array: felt*,
}(epoch_update: EpochUpdate, program_hash: felt, is_committee_transition: felt, expected_proof_output: CircuitOutput) -> (EpochUpdateOutput, felt) {
    alloc_locals;

    let (epoch_update_output) = run_epoch_update(epoch_update);
    print_string('epoch update output');

    // Check that expected matches the committee hash that was used to sign

    if (is_committee_transition == 1) {
        // print_string('exp com');
        // print_felt_hex(expected_proof_output.next_validator_root.low);
        // print_felt_hex(expected_proof_output.next_validator_root.high);

        // print_string('epoch update output');
        // print_felt_hex(epoch_update_output.current_committee_hash.low);
        // print_felt_hex(epoch_update_output.current_committee_hash.high);
        assert expected_proof_output.next_validator_root = epoch_update_output.current_validator_root;
    } else {
        // print_string('exp com');
        // print_felt_hex(expected_proof_output.current_committee_hash.low);
        // print_felt_hex(expected_proof_output.current_committee_hash.high);

        // print_string('epoch update output');
        // print_felt_hex(epoch_update_output.current_committee_hash.low);
        // print_felt_hex(epoch_update_output.current_committee_hash.high);
        assert expected_proof_output.current_validator_root = epoch_update_output.current_validator_root;
    }


// 4DB95C9D5280732373889B30F2169657B8C2707B0F67072CCE19C2AF296E26
// 2D9146E7362978E985649A4104E76396D77E1FD69335B6713DC531285AC0976

    // print_string('checked committee hash');

    // print_string('program hash');
    // print_felt_hex(program_hash);

    // print_felt_hex(expected_proof_output.beacon_header_root.low);
    // print_felt_hex(expected_proof_output.beacon_header_root.high);
    // print_felt_hex(expected_proof_output.beacon_state_root.low);
    // print_felt_hex(expected_proof_output.beacon_state_root.high);
    // print_felt_hex(expected_proof_output.beacon_height);
    // print_felt_hex(expected_proof_output.n_signers);
    // print_felt_hex(expected_proof_output.execution_header_root.low);
    // print_felt_hex(expected_proof_output.execution_header_root.high);
    // print_felt_hex(expected_proof_output.execution_header_height);
    // print_felt_hex(expected_proof_output.current_committee_hash.low);
    // print_felt_hex(expected_proof_output.current_committee_hash.high);
    // print_felt_hex(expected_proof_output.next_validator_root.low);
    // print_felt_hex(expected_proof_output.next_validator_root.high);

    // Construct the expected verifier output
    tempvar expected_verifier_output = cast(
        new (
            1, 13, program_hash,
            expected_proof_output.beacon_header_root.low,
            expected_proof_output.beacon_header_root.high,
            expected_proof_output.beacon_state_root.low,
            expected_proof_output.beacon_state_root.high,
            expected_proof_output.beacon_height,
            expected_proof_output.n_signers,
            expected_proof_output.execution_header_root.low,
            expected_proof_output.execution_header_root.high,
            expected_proof_output.execution_header_height,
            expected_proof_output.current_validator_root,
            expected_proof_output.next_validator_root,
        ), felt*
    );

    let (expected_output_hash: felt) = poseidon_hash_many(n=14, elements=expected_verifier_output);
    print_string('expected output hash');
    print_felt_hex(expected_output_hash);



    %{ write_stark_proof_inputs() %}
    let (proof_program_hash, output_hash) = verify_cairo_proof();

    print_string('output hash');
    print_felt_hex(output_hash);

    print_string('proof program hash');
    print_felt_hex(proof_program_hash);

    // Ensure the proof contains the expected values
    assert output_hash = expected_output_hash;    
    assert proof_program_hash = BOOTLOADER_PROGRAM_HASH;

    return (epoch_update_output, expected_proof_output.next_validator_root);
}

func handle_genesis_case{
    output_ptr: felt*,
    pedersen_ptr: HashBuiltin*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    sha256_ptr: felt*,
    pow2_array: felt*,
}(epoch_update: EpochUpdate) -> (epoch_update_output: EpochUpdateOutput) {
    alloc_locals;

    let (epoch_update_output) = run_epoch_update(epoch_update);
    tempvar expected_genesis_validator_root = 0x02d9146e7362978e985649a4104e76396d77e1fd69335b6713dc531285ac0976;
    assert expected_genesis_validator_root = epoch_update_output.current_validator_root;

    return (epoch_update_output=epoch_update_output);
}

func write_circuit_output{
    output_ptr: felt*,
    range_check_ptr,
}(epoch_output: EpochUpdateOutput, next_validator_root: felt, is_committee_transition: felt) {
    assert [output_ptr] = epoch_output.beacon_header_root.low;
    assert [output_ptr + 1] = epoch_output.beacon_header_root.high;
    assert [output_ptr + 2] = epoch_output.beacon_state_root.low;
    assert [output_ptr + 3] = epoch_output.beacon_state_root.high;
    assert [output_ptr + 4] = epoch_output.beacon_height;
    assert [output_ptr + 5] = epoch_output.n_signers;
    assert [output_ptr + 6] = epoch_output.execution_header_root.low;
    assert [output_ptr + 7] = epoch_output.execution_header_root.high;
    assert [output_ptr + 8] = epoch_output.execution_header_height;

    if (is_committee_transition == 1) {
        print_string('committee update');
        assert [output_ptr + 9] = next_validator_root;
        assert [output_ptr + 10] = 0x0;
        tempvar range_check_ptr = range_check_ptr;
    } else {
        print_string('no committee update');
        assert [output_ptr + 9] = epoch_output.current_validator_root;
        assert [output_ptr + 10] = next_validator_root;
        tempvar range_check_ptr = range_check_ptr;
    }

    let output_ptr = output_ptr + 11;
    return ();
}