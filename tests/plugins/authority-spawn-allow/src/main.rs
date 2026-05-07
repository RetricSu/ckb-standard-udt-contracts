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

const ALLOW: &[u8] = b"616c6c6f77";
const REQUIRE_HASH: &[u8] = b"726571756972655f68617368";

pub fn program_entry() -> i8 {
    let args: Vec<_> = argv().iter().collect();
    if args.len() != 2 {
        return 2;
    }
    let authority_hash = args[0].to_bytes();
    let script_args = args[1].to_bytes();
    if script_args == ALLOW {
        return 0;
    }
    if script_args == REQUIRE_HASH && authority_hash.len() == 64 {
        let has_nonzero_nibble = authority_hash.iter().any(|byte| *byte != b'0');
        return if has_nonzero_nibble { 0 } else { 3 };
    }

    4
}
