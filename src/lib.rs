mod juno_state_reader;

use starknet_rs::{
    business_logic::state::{state_api::StateReader}, utils::Address,
};
use crate::juno_state_reader::JunoStateReader;
use felt::Felt;

#[no_mangle]
pub extern "C" fn cairoVMCall(reader_handle: usize) {
    let mut reader = JunoStateReader::new(reader_handle);
    println!("called from Go");

    let addr = Address(Felt::new(44));
    let entry = (Address(Felt::new(44)), [5; 32]);
    println!("res {:?}", reader.get_storage_at(&entry));
    println!("res {:?}", reader.get_nonce_at(&addr));
    println!("res {:?}", reader.get_class_hash_at(&addr));
}
