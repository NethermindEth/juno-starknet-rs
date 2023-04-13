mod juno_state_reader;

use starknet_rs::{
    business_logic::{
        state::{state_api::StateReader},
        execution::execution_entry_point::ExecutionEntryPoint
    },
    utils::Address,
};

use crate::juno_state_reader::{JunoStateReader, felt_to_byte_array, ptr_to_felt};
use felt::Felt;
use std:: { ffi::c_uchar , slice };

#[no_mangle]
pub extern "C" fn cairoVMCall(reader_handle: usize, contract_address: *const c_uchar, entry_point_selector: *const c_uchar , calldata: *const*const c_uchar, len_calldata: usize) {
    let mut reader = JunoStateReader::new(reader_handle);
    let contract_addr_felt = ptr_to_felt(contract_address);
    let entry_point_selector_felt = ptr_to_felt(entry_point_selector);
    let calldata_slice = unsafe { slice::from_raw_parts(calldata, len_calldata) };
    let calldata_vec : Vec<Felt> = calldata_slice.iter().map(|ptr| ptr_to_felt(ptr.cast())).collect();

    println!("called from Go");
    println!("contract_addr_felt {:?}", contract_addr_felt);
    println!("entry_point_selector_felt {:?}", entry_point_selector_felt);
    println!("calldata_vec {:?}", calldata_vec);
}
