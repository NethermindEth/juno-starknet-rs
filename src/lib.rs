mod juno_state_reader;

use crate::juno_state_reader::{ptr_to_felt, JunoStateReader};
use std::{
    ffi::{c_char, c_uchar, c_ulonglong, CString, CStr},
    slice,
};

use blockifier::{
    block_context::BlockContext,
    execution::entry_point::{CallEntryPoint, CallType, ExecutionContext},
    state::cached_state::CachedState,
    transaction::objects::AccountTransactionContext,
};
use juno_state_reader::felt_to_byte_array;
use starknet_api::core::{EntryPointSelector, ChainId};
use starknet_api::transaction::Calldata;
use starknet_api::{
    block::{BlockNumber, BlockTimestamp},
    deprecated_contract_class::EntryPointType,
    hash::StarkFelt,
};

extern "C" {
    fn JunoReportError(reader_handle: usize, err: *const c_char);
    fn JunoAppendResponse(reader_handle: usize, ptr: *const c_uchar);
}

#[no_mangle]
pub extern "C" fn cairoVMCall(
    contract_address: *const c_uchar,
    entry_point_selector: *const c_uchar,
    calldata: *const *const c_uchar,
    len_calldata: usize,
    reader_handle: usize,
    block_number: c_ulonglong,
    block_timestamp: c_ulonglong,
    chain_id: *const c_char,
) {
    let reader = JunoStateReader::new(reader_handle);
    let contract_addr_felt = ptr_to_felt(contract_address);
    let entry_point_selector_felt = ptr_to_felt(entry_point_selector);
    let calldata_slice = unsafe { slice::from_raw_parts(calldata, len_calldata) };
    let chain_id_str = unsafe { CStr::from_ptr(chain_id) }.to_str().unwrap();

    let mut calldata_vec: Vec<StarkFelt> = vec![];
    for ptr in calldata_slice {
        let data = ptr_to_felt(ptr.cast());
        calldata_vec.push(data);
    }

    let entry_point = CallEntryPoint {
        entry_point_type: EntryPointType::External,
        entry_point_selector: EntryPointSelector(entry_point_selector_felt),
        calldata: Calldata(calldata_vec.into()),
        storage_address: contract_addr_felt.try_into().unwrap(),
        call_type: CallType::Call,
        ..Default::default()
    };

    let mut state = CachedState::new(reader);
    let mut context = ExecutionContext::new(
        BlockContext {
            chain_id: ChainId(chain_id_str.into()),
            block_number: BlockNumber(block_number),
            block_timestamp: BlockTimestamp(block_timestamp),
            ..BlockContext::create_for_testing()
        },
        AccountTransactionContext::default(),
    );
    let call_info = entry_point.execute(&mut state, &mut context);

    match call_info {
        Err(e) => report_error(reader_handle, e.to_string().as_str()),
        Ok(t) => {
            for data in t.execution.retdata.0 {
                unsafe {
                    JunoAppendResponse(reader_handle, felt_to_byte_array(&data).as_ptr());
                };
            }
        }
    }
}

fn report_error(reader_handle: usize, msg: &str) {
    let err_msg = CString::new(msg).unwrap();
    unsafe {
        JunoReportError(reader_handle, err_msg.as_ptr());
    };
}
