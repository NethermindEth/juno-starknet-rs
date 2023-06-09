mod juno_state_reader;

use crate::juno_state_reader::{ptr_to_felt, JunoStateReader};
use std::{
    collections::HashMap,
    ffi::{c_char, c_uchar, c_ulonglong, CStr, CString},
    slice,
};

use blockifier::{
    block_context::BlockContext,
    execution::entry_point::{CallEntryPoint, CallType, ExecutionContext},
    state::cached_state::CachedState,
    transaction::{
        objects::AccountTransactionContext, transaction_execution::Transaction, transactions::ExecutableTransaction,
    }, abi::constants::INITIAL_GAS_COST,
};
use juno_state_reader::contract_class_from_json_str;
use juno_state_reader::felt_to_byte_array;
use starknet_api::{core::{ChainId, ContractAddress, EntryPointSelector}, hash::StarkHash};
use starknet_api::transaction::{Calldata, Transaction as StarknetApiTransaction};
use starknet_api::{
    block::{BlockNumber, BlockTimestamp},
    deprecated_contract_class::EntryPointType,
    hash::StarkFelt,
};

extern "C" {
    fn JunoReportError(reader_handle: usize, err: *const c_char);
    fn JunoAppendResponse(reader_handle: usize, ptr: *const c_uchar);
    fn JunoSetGasConsumed(reader_handle: usize, ptr: *const c_uchar);
}

const N_STEPS_FEE_WEIGHT: f64 = 0.01;

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
        class_hash: None,
        code_address: None,
        caller_address: ContractAddress::default(),
        initial_gas: INITIAL_GAS_COST.into(),
    };

    let mut state = CachedState::new(reader);
    let mut context = ExecutionContext::new(
        build_block_context(chain_id_str, block_number, block_timestamp),
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

#[no_mangle]
pub extern "C" fn cairoVMExecute(
    txn_json: *const c_char,
    class_json: *const c_char,
    reader_handle: usize,
    block_number: c_ulonglong,
    block_timestamp: c_ulonglong,
    chain_id: *const c_char,
) {
    let reader = JunoStateReader::new(reader_handle);
    let chain_id_str = unsafe { CStr::from_ptr(chain_id) }.to_str().unwrap();
    let txn_json_str = unsafe { CStr::from_ptr(txn_json) }.to_str().unwrap();
    let sn_api_txn: Result<StarknetApiTransaction, serde_json::Error> =
        serde_json::from_str(txn_json_str);

    if sn_api_txn.is_err() {
        report_error(reader_handle, sn_api_txn.unwrap_err().to_string().as_str());
        return;
    }

    let contract_class = if !class_json.is_null() {
        let class_json_str = unsafe { CStr::from_ptr(class_json) }.to_str().unwrap();
        let maybe_cc = contract_class_from_json_str(class_json_str);
        if maybe_cc.is_err() {
            report_error(reader_handle, maybe_cc.unwrap_err().to_string().as_str());
            return;
        }
        Some(maybe_cc.unwrap())
    } else {
        None
    };

    let txn = Transaction::from_api(sn_api_txn.unwrap(), contract_class, None);
    if txn.is_err() {
        report_error(reader_handle, txn.unwrap_err().to_string().as_str());
        return;
    }

    let block_context: BlockContext = build_block_context(chain_id_str, block_number, block_timestamp);
    let mut state = CachedState::new(reader);

    let res = match txn.unwrap() {
        Transaction::AccountTransaction(t) => t.execute(&mut state, &block_context),
        Transaction::L1HandlerTransaction(t) => t.execute(&mut state, &block_context),
    };

    match res {
        Err(e) => report_error(reader_handle, e.to_string().as_str()),
        Ok(t) => unsafe {
            JunoSetGasConsumed(
                reader_handle,
                felt_to_byte_array(&t.actual_fee.0.into()).as_ptr(),
            )
        },
    }
}

fn report_error(reader_handle: usize, msg: &str) {
    let err_msg = CString::new(msg).unwrap();
    unsafe {
        JunoReportError(reader_handle, err_msg.as_ptr());
    };
}

fn build_block_context(chain_id_str: &str, block_number: c_ulonglong, block_timestamp: c_ulonglong) -> BlockContext {
    BlockContext {
        chain_id: ChainId(chain_id_str.into()),
        block_number: BlockNumber(block_number),
        block_timestamp: BlockTimestamp(block_timestamp),

        sequencer_address: ContractAddress::default(),
        // https://github.com/starknet-io/starknet-addresses/blob/df19b17d2c83f11c30e65e2373e8a0c65446f17c/bridged_tokens/mainnet.json
        // fee_token_address is the same for all networks
        fee_token_address: ContractAddress::try_from(StarkHash::try_from("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap()).unwrap(),
        gas_price: 1, // fixed gas price, so that we can return "consumed gas" to Go side
        vm_resource_fee_cost: HashMap::from([
            ("n_steps".to_string(), N_STEPS_FEE_WEIGHT),
            ("output".to_string(), 0.0),
            ("pedersen".to_string(), N_STEPS_FEE_WEIGHT * 32.0),
            ("range_check".to_string(), N_STEPS_FEE_WEIGHT * 16.0),
            ("ecdsa".to_string(), N_STEPS_FEE_WEIGHT * 2048.0),
            ("bitwise".to_string(), N_STEPS_FEE_WEIGHT * 64.0),
            ("ec_op".to_string(), N_STEPS_FEE_WEIGHT * 1024.0),
            ("poseidon".to_string(), N_STEPS_FEE_WEIGHT * 32.0),
            ("segment_arena".to_string(), N_STEPS_FEE_WEIGHT * 10.0),
        ]),
        invoke_tx_max_n_steps: 1_000_000,
        validate_max_n_steps: 1_000_000,
    }
}