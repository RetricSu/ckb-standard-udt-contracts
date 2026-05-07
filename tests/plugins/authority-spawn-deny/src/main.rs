#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

use alloc::vec::Vec;
use ckb_std::env::argv;

#[cfg(not(any(feature = "library", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "library", test)))]
ckb_std::default_alloc!(16384, 1258306, 64);

pub fn program_entry() -> i8 {
    let args: Vec<_> = argv().iter().collect();
    if args.len() != 2 {
        return 2;
    }
    if args[0].to_bytes().len() != 64 {
        return 3;
    }
    1
}
