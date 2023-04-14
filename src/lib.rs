mod juno_state_reader;

use crate::juno_state_reader::{JunoStateReader, ptr_to_felt};
use std:: { ffi::{c_uchar, c_char, CString} , slice, collections::HashMap };

use juno_state_reader::felt_to_byte_array;
use starknet_rs::{
    business_logic::{
        execution::{
            execution_entry_point::ExecutionEntryPoint,
            objects::{
                CallInfo, TransactionExecutionContext,
            },
        },
        fact_state::state::ExecutionResourcesManager,
        state::cached_state::CachedState,
        transaction::error::TransactionError,
    },
    definitions::general_config::StarknetGeneralConfig,
    services::api::contract_class::EntryPointType,
    utils::Address
};
use felt::Felt252;

extern {
    fn JunoReportError(reader_handle: usize, err: *const c_char);
    fn JunoAppendResponse(reader_handle: usize, ptr: *const c_uchar);
}


#[no_mangle]
pub extern "C" fn cairoVMCall(reader_handle: usize, contract_address: *const c_uchar, entry_point_selector: *const c_uchar , calldata: *const*const c_uchar, len_calldata: usize) {
    let reader = JunoStateReader::new(reader_handle);
    let contract_addr_felt = ptr_to_felt(contract_address);
    let entry_point_selector_felt = ptr_to_felt(entry_point_selector);
    let calldata_slice = unsafe { slice::from_raw_parts(calldata, len_calldata) };
    let mut calldata_vec : Vec<Felt252> = vec![];

    for ptr in calldata_slice {
        let data = ptr_to_felt(ptr.cast());
        calldata_vec.push(data);
    }

    let call_info = execute_entry_point_raw(
        reader,
        Address(contract_addr_felt),
        entry_point_selector_felt,
        calldata_vec,
        Address::default());

    match call_info {
        Err(e) => report_error(reader_handle, e.to_string().as_str()),
        Ok(t) => {
            for data in t.retdata {
                unsafe { JunoAppendResponse(reader_handle, felt_to_byte_array(&data).as_ptr()); };
            }
        }
    }
}


fn execute_entry_point_raw(
    state_reader: JunoStateReader,
    contract_address: Address,
    entry_point_selector: Felt252,
    calldata: Vec<Felt252>,
    caller_address: Address,
) -> Result<CallInfo, TransactionError> {
    let call = ExecutionEntryPoint::new(
        contract_address,
        calldata,
        entry_point_selector,
        caller_address,
        EntryPointType::External,
        None,
        None,
    );

    let mut state = CachedState::new(state_reader, Some(HashMap::new()));
    let mut resources_manager = ExecutionResourcesManager::default();
    let config = StarknetGeneralConfig::default();
    let tx_execution_context = TransactionExecutionContext::default();

    call.execute(
        &mut state,
        &config,
        &mut resources_manager,
        &tx_execution_context,
    )
}

fn report_error(reader_handle: usize, msg: &str) {
    let err_msg = CString::new(msg).unwrap();
    unsafe { JunoReportError(reader_handle,err_msg.as_ptr()); };
}
