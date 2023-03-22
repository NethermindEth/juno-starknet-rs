mod juno_state_reader;

use crate::juno_state_reader::JunoStateReader;

extern {
    fn JunoStateGetStorageAt(readerHandle: usize);
}

#[no_mangle]
pub extern "C" fn cairoVMCall(readerHandle: usize) {
    let reader = JunoStateReader(readerHandle);
    println!("called from Go");
    unsafe { JunoStateGetStorageAt(reader.0); }
}
