use crate::metadata_builders::build_xudt_meta_bytes;
use ckb_testtool::{
    ckb_hash::new_blake2b,
    ckb_types::{bytes::Bytes, packed::CellInput, prelude::*},
};
use standard_udt_types::metadata::{Authority, Extension};

pub fn calculate_type_id(input: &CellInput, output_index: u64) -> [u8; 32] {
    let mut type_id = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(input.as_slice());
    hasher.update(&output_index.to_le_bytes());
    hasher.finalize(&mut type_id);
    type_id
}

pub fn xudt_meta_data(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    extensions: Vec<Extension>,
) -> Bytes {
    xudt_meta_data_with_authorities(
        config_flags,
        current_supply,
        mint_authority,
        None,
        None,
        extensions,
    )
}

pub fn xudt_meta_data_with_authorities(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    metadata_authority: Option<Authority>,
    access_authority: Option<Authority>,
    extensions: Vec<Extension>,
) -> Bytes {
    build_xudt_meta_bytes(
        config_flags,
        current_supply,
        mint_authority,
        metadata_authority,
        access_authority,
        extensions,
    )
}
