use starknet_rs::{
    business_logic::state::{state_api::StateReader, state_cache::StorageEntry},
    core::errors::state_errors::StateError,
    services::api::contract_class::ContractClass,
    utils::{Address, ClassHash},
};
use felt::Felt;

pub struct JunoStateReader(pub usize); // uintptr_t equivalent


impl StateReader for JunoStateReader {
    fn get_contract_class(&mut self, class_hash: &ClassHash) -> Result<ContractClass, StateError> {
        todo!()
    }

    fn get_class_hash_at(&mut self, contract_address: &Address) -> Result<&ClassHash, StateError> {
        todo!()
    }

    fn get_nonce_at(&mut self, contract_address: &Address) -> Result<&Felt, StateError> {
        todo!()
    }

    fn get_storage_at(&mut self, storage_entry: &StorageEntry) -> Result<&Felt, StateError> {
        todo!()
    }

    fn count_actual_storage_changes(&mut self) -> (usize, usize) {
        todo!()
    }
}
