#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

use alloc::vec::Vec;
use ckb_std::env::argv;

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

const REQUIRE_MINT_CHECKED: &[u8] = b"726571756972655f6d696e745f636865636b6564";
const REQUIRE_MINT_NONE: &[u8] = b"726571756972655f6d696e745f6e6f6e65";
const REQUIRE_TRANSFER: &[u8] = b"726571756972655f7472616e73666572";
const REQUIRE_MINT: &[u8] = b"726571756972655f6d696e74";
const REQUIRE_PROTOCOL_BURN: &[u8] = b"726571756972655f70726f746f636f6c5f6275726e";
const REQUIRE_INDEX_0: &[u8] = b"726571756972655f696e6465785f30";
const REQUIRE_INDEX_1: &[u8] = b"726571756972655f696e6465785f31";

pub fn program_entry() -> i8 {
    let args: Vec<_> = argv().iter().collect();
    if args.len() != 4 {
        return 2;
    }

    let op_type = args[0].to_bytes();
    let ext_index = args[1].to_bytes();
    let ext_data = args[2].to_bytes();
    let mint_authority_checked = args[3].to_bytes();

    if ext_data.is_empty() {
        return 0;
    }
    if ext_data == REQUIRE_MINT_CHECKED && mint_authority_checked != b"1" {
        return 3;
    }
    if ext_data == REQUIRE_MINT_NONE && mint_authority_checked != b"2" {
        return 4;
    }
    if ext_data == REQUIRE_TRANSFER && op_type != b"0" {
        return 5;
    }
    if ext_data == REQUIRE_MINT && op_type != b"1" {
        return 6;
    }
    if ext_data == REQUIRE_PROTOCOL_BURN && op_type != b"2" {
        return 7;
    }
    if ext_data == REQUIRE_INDEX_0 && ext_index != b"0" {
        return 8;
    }
    if ext_data == REQUIRE_INDEX_1 && ext_index != b"1" {
        return 9;
    }
    if ext_data != REQUIRE_MINT_CHECKED
        && ext_data != REQUIRE_MINT_NONE
        && ext_data != REQUIRE_TRANSFER
        && ext_data != REQUIRE_MINT
        && ext_data != REQUIRE_PROTOCOL_BURN
        && ext_data != REQUIRE_INDEX_0
        && ext_data != REQUIRE_INDEX_1
    {
        return 10;
    }

    0
}
