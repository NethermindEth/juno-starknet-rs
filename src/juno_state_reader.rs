use std:: { ffi::{c_uchar, c_char, CStr}, slice };

use starknet_rs::{
    business_logic::state::{state_api::StateReader, state_cache::StorageEntry},
    core::errors::state_errors::StateError,
    services::api::contract_classes::{
        compiled_class::CompiledClass, deprecated_contract_class::ContractClass,
    },
    utils::{Address, ClassHash, CompiledClassHash},
};
use cairo_vm::felt::Felt252;
use cairo_lang_starknet::contract_class::ContractClass as SierraContractClass

extern {
    fn JunoFree(ptr: *const c_uchar);

    fn JunoStateGetStorageAt(reader_handle: usize, contract_address: *const c_uchar, storage_location: *const c_uchar) -> *const c_uchar;
    fn JunoStateGetNonceAt(reader_handle: usize, contract_address: *const c_uchar) -> *const c_uchar;
    fn JunoStateGetClassHashAt(reader_handle: usize, contract_address: *const c_uchar) -> *const c_uchar;
    fn JunoStateGetClass(reader_handle: usize, class_hash: *const c_uchar) -> *const c_char;
}

pub struct JunoStateReader{
    pub handle: usize, // uintptr_t equivalent
}

impl JunoStateReader {
    pub fn new(
        handle: usize
    ) -> Self {
        Self {
            handle: handle,
        }
    }
}

impl StateReader for JunoStateReader {
    fn get_contract_class(&mut self, class_hash: &ClassHash) -> Result<ContractClass, StateError> {
        let ptr = unsafe { JunoStateGetClass(self.handle, class_hash.as_ptr()) };

        if ptr.is_null() {
            Err(StateError::MissingClassHash())
        } else {
            let json_cstr = unsafe { CStr::from_ptr(ptr) };
            let json_str = json_cstr.to_str().or_else(|_err| Err(StateError::MissingClassHash()))?;

            ContractClass::try_from(json_str).or_else(|_err| Err(StateError::MissingClassHash()))
        }
    }

    fn get_class_hash_at(&mut self, contract_address: &Address) -> Result<ClassHash, StateError> {
        let addr = felt_to_byte_array(&contract_address.0);
        let ptr = unsafe { JunoStateGetClassHashAt(self.handle, addr.as_ptr()) };
        if ptr.is_null() {
            Err(StateError::NoneClassHash(contract_address.clone()))
        } else {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };

            Ok(felt_to_byte_array(&felt_val))
        }
    }

    fn get_nonce_at(&mut self, contract_address: &Address) -> Result<Felt252, StateError> {
        let addr = felt_to_byte_array(&contract_address.0);
        let ptr = unsafe { JunoStateGetNonceAt(self.handle, addr.as_ptr()) };
        if ptr.is_null() {
            Err(StateError::NoneContractState(contract_address.clone()))
        } else {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };

            Ok(felt_val)
        }
    }

    fn get_storage_at(&mut self, storage_entry: &StorageEntry) -> Result<Felt252, StateError> {
        let addr = felt_to_byte_array(&storage_entry.0.0);
        let ptr = unsafe { JunoStateGetStorageAt(self.handle, addr.as_ptr(), storage_entry.1.as_ptr()) };
        if ptr.is_null() {
            Err(StateError::NoneStorage(storage_entry.clone()))
        } else {
            let felt_val = ptr_to_felt(ptr);
            unsafe { JunoFree(ptr) };

            Ok(felt_val)
        }
    }

    fn count_actual_storage_changes(&mut self) -> (usize, usize) {
        unimplemented!("todo")
    }

    fn get_compiled_class(
        &mut self,
        compiled_class_hash: &CompiledClassHash,
    ) -> Result<CompiledClass, StateError> {
        let class: ContractClass = self.get_contract_class(compiled_class_hash)?;
        Ok(CompiledClass::Deprecated(Box::new(class)))
    }

    /// Return the class hash of the given casm contract class
    fn get_compiled_class_hash(
        &mut self,
        class_hash: &ClassHash,
    ) -> Result<CompiledClassHash, StateError> {
        unimplemented!("todo")
    }
}

impl Clone for JunoStateReader {
    fn clone(&self) -> Self {
        JunoStateReader::new(self.handle)
    }
}

impl Default for JunoStateReader {
    fn default() -> Self {
        JunoStateReader::new(usize::MAX)
    }
}

pub fn felt_to_byte_array(felt: &Felt252) -> [u8; 32] {
    let felt_bytes = felt.to_bytes_be();
    let mut zeros = [0; 32];
    let mut start_idx =  zeros.len() - felt_bytes.len();

    for val in felt_bytes {
        zeros[start_idx] = val;
        start_idx += 1;
    }
    zeros
}


pub fn ptr_to_felt(bytes: *const c_uchar) -> Felt252 {
     let slice = unsafe { slice::from_raw_parts(bytes, 32) };
    Felt252::from_bytes_be(slice)
}