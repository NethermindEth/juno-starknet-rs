use starknet_rs::{
    business_logic::state::{state_api::StateReader, state_cache::StorageEntry},
    core::errors::state_errors::StateError,
    services::api::contract_class::ContractClass,
    utils::{Address, ClassHash},
};
use felt::Felt;
use std:: { collections::HashMap, ffi::c_uchar, slice };

extern {
    fn JunoFree(ptr: *const c_uchar);

    fn JunoStateGetStorageAt(reader_handle: usize, contract_address: *const c_uchar, storage_location: *const c_uchar) -> *const c_uchar;
    fn JunoStateGetNonceAt(reader_handle: usize, contract_address: *const c_uchar) -> *const c_uchar;
    fn JunoStateGetClassHashAt(reader_handle: usize, contract_address: *const c_uchar) -> *const c_uchar;
}

pub struct JunoStateReader{
    pub handle: usize, // uintptr_t equivalent

    address_to_storage: HashMap<StorageEntry, Felt>,
    address_to_nonce: HashMap<Address, Felt>,
    address_to_class_hash: HashMap<Address, ClassHash>,
}

impl JunoStateReader {
    pub fn new(
        handle: usize
    ) -> Self {
        Self {
            handle: handle,
            address_to_storage:  HashMap::new(),
            address_to_nonce:  HashMap::new(),
            address_to_class_hash: HashMap::new(),
        }
    }
}

impl StateReader for JunoStateReader {
    fn get_contract_class(&mut self, _class_hash: &ClassHash) -> Result<ContractClass, StateError> {
        todo!()
    }

    fn get_class_hash_at(&mut self, contract_address: &Address) -> Result<&ClassHash, StateError> {
        self.address_to_class_hash.remove(contract_address);


        let addr = felt_to_byte_array(&contract_address.0);
        let ptr = unsafe { JunoStateGetClassHashAt(self.handle, addr.as_ptr()) };
        if !ptr.is_null() {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };
            self.address_to_class_hash.insert(contract_address.clone(), felt_to_byte_array(&felt_val));
        }

        let class_hash = self
            .address_to_class_hash
            .get(contract_address)
            .ok_or_else(|| StateError::NoneClassHash(contract_address.clone()));
        class_hash
    }

    fn get_nonce_at(&mut self, contract_address: &Address) -> Result<&Felt, StateError> {
        self.address_to_nonce.remove(contract_address);


        let addr = felt_to_byte_array(&contract_address.0);
        let ptr = unsafe { JunoStateGetNonceAt(self.handle, addr.as_ptr()) };
        if !ptr.is_null() {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };
            self.address_to_nonce.insert(contract_address.clone(), felt_val);
        }

        let nonce = self
            .address_to_nonce
            .get(contract_address)
            .ok_or_else(|| StateError::NoneContractState(contract_address.clone()));
        nonce
    }

    fn get_storage_at(&mut self, storage_entry: &StorageEntry) -> Result<&Felt, StateError> {
        self.address_to_storage.remove(storage_entry);

        let addr = felt_to_byte_array(&storage_entry.0.0);
        let ptr = unsafe { JunoStateGetStorageAt(self.handle, addr.as_ptr(), storage_entry.1.as_ptr()) };
        if !ptr.is_null() {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };
            self.address_to_storage.insert(storage_entry.clone(), felt_val);
        }

        let storage = self
        .address_to_storage
        .get(storage_entry)
        .ok_or_else(|| StateError::NoneStorage(storage_entry.clone()));

        storage
    }

    fn count_actual_storage_changes(&mut self) -> (usize, usize) {
        todo!()
    }
}


fn felt_to_byte_array(felt: &Felt) -> [u8; 32] {
    let felt_bytes = felt.to_bytes_be();
    let mut zeros = [0; 32];
    let mut start_idx =  zeros.len() - felt_bytes.len();

    for val in felt_bytes {
        zeros[start_idx] = val;
        start_idx += 1;
    }

    zeros
}


fn ptr_to_felt(bytes: *const c_uchar) -> Felt {
    let slice = unsafe { slice::from_raw_parts(bytes, 32) };
    Felt::from_bytes_be(slice)
}