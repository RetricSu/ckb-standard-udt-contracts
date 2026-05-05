#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

#[cfg(not(any(feature = "library", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "library", test)))]
// By default, the following heap configuration is used:
// * 16KB fixed heap
// * 1.2MB(rounded up to be 16-byte aligned) dynamic heap
// * Minimal memory block in dynamic heap is 64 bytes
// For more details, please refer to ckb-std's default_alloc macro
// and the buddy-alloc alloc implementation.
ckb_std::default_alloc!(16384, 1258306, 64);

#[unsafe(no_mangle)]
pub extern "C" fn eudt_validate(
    _script_hash: *const u8,
    _op_type: u8,
    _ext_index: u8,
    ext_data_ptr: *const u8,
    ext_data_len: usize,
    mint_authority_checked: u8,
) -> i8 {
    let ext_data = unsafe { core::slice::from_raw_parts(ext_data_ptr, ext_data_len) };
    if ext_data == b"require_mint_checked" && mint_authority_checked != 1 {
        return 1;
    }
    0
}

pub fn program_entry() -> i8 {
    1
}
