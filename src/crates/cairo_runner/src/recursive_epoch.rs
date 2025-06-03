use std::collections::HashMap;

use crate::{
    hint_processor::CustomHintProcessor,
    types::{Bytes32, Felt, G1CircuitPoint, G2CircuitPoint, UInt384, Uint256, Uint256Bits32},
};
use cairo_vm::{
    hint_processor::builtin_hint_processor::{
        builtin_hint_processor_definition::HintProcessorData,
        hint_utils::{
            get_ptr_from_var_name, get_relocatable_from_var_name,
        },
    },
    types::{exec_scope::ExecutionScopes, relocatable::Relocatable},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
    Felt252,
};
use garaga_zero::types::CairoType;
use serde::Deserialize;

use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RecursiveEpochUpdate {
    pub inputs: RecursiveEpochUpdateInputs,
    pub outputs: RecursiveEpochUpdateOutputs,
}

#[derive(Debug, Deserialize)]
pub struct RecursiveEpochUpdateOutputs {
    pub beacon_header_root: Uint256,
    pub beacon_state_root: Uint256,
    pub beacon_height: Felt,
    pub n_signers: Felt,
    pub execution_header_root: Uint256,
    pub execution_header_height: Felt,
    pub current_committee_hash: Uint256,
    pub next_committee_hash: Uint256,
}

#[derive(Debug, Deserialize)]
pub struct RecursiveEpochUpdateInputs {
    pub epoch_update: EpochUpdate,
    pub sync_committee_update: Option<SyncCommitteeData>,
    pub stark_proof: Option<Value>, // this is the stark proof of the previous epoch update
}

#[derive(Debug, Deserialize)]
pub struct EpochUpdate {
    pub header: BeaconHeader,
    pub signature_point: G2CircuitPoint,
    pub aggregate_pub: G1CircuitPoint,
    pub non_signers: Vec<G1CircuitPoint>,
    pub execution_header_proof: ExecutionHeaderProof,
}

#[derive(Debug, Deserialize)]
pub struct BeaconHeader {
    pub slot: Uint256,
    pub proposer_index: Uint256,
    pub parent_root: Uint256,
    pub state_root: Uint256,
    pub body_root: Uint256,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionHeaderProof {
    pub root: Uint256,
    pub path: Vec<Uint256Bits32>,
    pub leaf: Uint256,
    pub index: Felt,
    pub execution_payload_header: Vec<Bytes32>,
}

#[derive(Debug, Deserialize)]
pub struct SyncCommitteeData {
    pub beacon_slot: Felt,
    pub next_sync_committee_branch: Vec<Uint256Bits32>,
    pub next_aggregate_sync_committee: UInt384,
    pub committee_keys_root: Uint256Bits32,
}

pub const HINT_WRITE_EPOCH_UPDATE_INPUTS: &str = r#"write_epoch_update_inputs()"#;
pub const HINT_WRITE_STARK_PROOF_INPUTS: &str = r#"write_stark_proof_inputs()"#;
pub const HINT_WRITE_COMMITTEE_UPDATE_INPUTS: &str = r#"write_committee_update_inputs()"#;

impl CustomHintProcessor {
    pub fn write_epoch_update_inputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let epoch_update = &self.recursive_epoch_update.inputs.epoch_update;
        let epoch_update_ptr = get_relocatable_from_var_name(
            "epoch_update",
            vm,
            &hint_data.ids_data,
            &hint_data.ap_tracking,
        )?;
        write_epoch_update(epoch_update_ptr, epoch_update, vm)?;

        Ok(())
    }

    pub fn write_stark_proof_inputs(
        &self,
        _vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        _hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(stark_proof) = &self.recursive_epoch_update.inputs.stark_proof {
            let proof_string = serde_json::json!({
                "proof": stark_proof
            })
            .to_string();
            exec_scopes.insert_value("program_input", proof_string);
        } else {
            panic!("Stark proof not found");
        }

        Ok(())
    }

    pub fn write_committee_update_inputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(sync_committee_update) =
            &self.recursive_epoch_update.inputs.sync_committee_update
        {
            let slot_ptr = get_relocatable_from_var_name(
                "slot",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            sync_committee_update.beacon_slot.to_memory(vm, slot_ptr)?;

            let aggregate_committee_key_ptr = get_relocatable_from_var_name(
                "aggregate_committee_key",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            sync_committee_update
                .next_aggregate_sync_committee
                .to_memory(vm, aggregate_committee_key_ptr)?;

            let committee_keys_root_ptr = get_ptr_from_var_name(
                "committee_keys_root",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            sync_committee_update
                .committee_keys_root
                .to_memory(vm, committee_keys_root_ptr)?;

            let path_ptr =
                get_ptr_from_var_name("path", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;

            for (i, branch) in sync_committee_update
                .next_sync_committee_branch
                .iter()
                .enumerate()
            {
                let branch_segment = vm.add_memory_segment();
                branch.to_memory(vm, branch_segment)?;
                vm.insert_value((path_ptr + i)?, branch_segment)?;
            }

            let path_len_ptr = get_relocatable_from_var_name(
                "path_len",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            let path_len = Felt252::from(sync_committee_update.next_sync_committee_branch.len());
            vm.insert_value(path_len_ptr, path_len)?;

            Ok(())
        } else {
            panic!("Committee input not found");
        }
    }
}

pub fn write_epoch_update(
    epoch_update_ptr: Relocatable,
    circuit_inputs: &EpochUpdate,
    vm: &mut VirtualMachine,
) -> Result<Relocatable, HintError> {
    let mut current_ptr = epoch_update_ptr;

    // Write signature point
    current_ptr = circuit_inputs.signature_point.to_memory(vm, current_ptr)?;

    // Write header fields
    current_ptr = write_header_fields(vm, current_ptr, &circuit_inputs.header)?;

    // Write signer data (aggregate pub key and non-signers)
    current_ptr = write_signer_data(vm, current_ptr, circuit_inputs)?;

    // Write execution header proof
    current_ptr =
        write_execution_header_proof(vm, current_ptr, &circuit_inputs.execution_header_proof)?;

    Ok(current_ptr)
}

fn write_header_fields(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    header: &BeaconHeader,
) -> Result<Relocatable, HintError> {
    ptr = header.slot.to_memory(vm, ptr)?;
    ptr = header.proposer_index.to_memory(vm, ptr)?;
    ptr = header.parent_root.to_memory(vm, ptr)?;
    ptr = header.state_root.to_memory(vm, ptr)?;
    ptr = header.body_root.to_memory(vm, ptr)?;
    Ok(ptr)
}

fn write_signer_data(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    circuit_inputs: &EpochUpdate,
) -> Result<Relocatable, HintError> {
    // Write aggregate public key
    ptr = circuit_inputs.aggregate_pub.to_memory(vm, ptr)?;

    // Create segment for non-signers and store its pointer
    let non_signers_segment = vm.add_memory_segment();
    vm.insert_value(ptr, non_signers_segment)?;

    // Write all non-signers to the segment
    let mut segment_ptr = non_signers_segment;
    for non_signer in &circuit_inputs.non_signers {
        segment_ptr = non_signer.to_memory(vm, segment_ptr)?;
    }

    // Store the length of non-signers
    vm.insert_value((ptr + 1)?, Felt252::from(circuit_inputs.non_signers.len()))?;

    Ok((ptr + 2)?)
}

fn write_execution_header_proof(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    proof: &ExecutionHeaderProof,
) -> Result<Relocatable, HintError> {
    // Write root
    ptr = proof.root.to_memory(vm, ptr)?;

    // Create and write path segment
    let path_segment = vm.add_memory_segment();
    vm.insert_value(ptr, path_segment)?;
    ptr = (ptr + 1)?;

    // Write each path element
    let mut path_ptr = path_segment;
    for path_element in &proof.path {
        let element_segment = vm.add_memory_segment();
        vm.insert_value(path_ptr, element_segment)?;
        path_ptr = (path_ptr + 1)?;

        path_element.to_memory(vm, element_segment)?;
    }

    // Write leaf and index
    ptr = proof.leaf.to_memory(vm, ptr)?;
    ptr = proof.index.to_memory(vm, ptr)?;

    // Create and write payload fields segment
    let payload_fields_segment = vm.add_memory_segment();
    vm.insert_value(ptr, payload_fields_segment)?;

    // Write each payload field
    let mut payload_fields_ptr = payload_fields_segment;
    for field in &proof.execution_payload_header {
        payload_fields_ptr = field.to_memory(vm, payload_fields_ptr)?;
    }

    Ok((ptr + 1)?)
}
