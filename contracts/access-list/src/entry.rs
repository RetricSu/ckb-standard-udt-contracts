use crate::{
    error::Error,
    meta::{check_authority, load_meta_context, load_meta_type_hash_arg},
    mode::AccessMode,
    shards::{collect_group_shards, validate_shards_for_modes},
};
use ckb_std::{
    ckb_constants::Source, ckb_types::prelude::*, error::SysError, high_level::load_cell_lock,
};

const ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST: [[u8; 32]; 1] = [[
    0x3b, 0x52, 0x1c, 0xc4, 0xb5, 0x52, 0xf1, 0x09, 0xd0, 0x92, 0xd8, 0xcc, 0x46, 0x8a, 0x80, 0x48,
    0xac, 0xb5, 0x3c, 0x59, 0x52, 0xdb, 0xe7, 0x69, 0xd2, 0xb2, 0xf9, 0xcf, 0x6e, 0x47, 0xf7, 0xf1,
]];

#[cfg(debug_assertions)]
const TESTTOOL_ALWAYS_SUCCESS_LOCK_CODE_HASH: [u8; 32] = [
    0xe6, 0x83, 0xb0, 0x41, 0x39, 0x34, 0x47, 0x68, 0x34, 0x84, 0x99, 0xc2, 0x3e, 0xb1, 0x32, 0x6d,
    0x5a, 0x52, 0xd6, 0xdb, 0x00, 0x6c, 0x0d, 0x2f, 0xec, 0xe0, 0x0a, 0x83, 0x1f, 0x36, 0x60, 0xd7,
];

fn is_allowed_always_success_lock_code_hash(code_hash: &[u8; 32]) -> bool {
    if ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST.contains(code_hash) {
        return true;
    }

    #[cfg(debug_assertions)]
    {
        code_hash == &TESTTOOL_ALWAYS_SUCCESS_LOCK_CODE_HASH
    }

    #[cfg(not(debug_assertions))]
    {
        false
    }
}

pub fn main() -> Result<(), Error> {
    let meta_type_hash = load_meta_type_hash_arg()?;
    let meta_context = load_meta_context(&meta_type_hash)?;
    let input_shards = collect_group_shards(ckb_std::ckb_constants::Source::GroupInput)?;
    let output_shards = collect_group_shards(ckb_std::ckb_constants::Source::GroupOutput)?;
    validate_group_output_locks()?;

    let input_mode = match meta_context.input_config_flags {
        Some(flags) => AccessMode::from_flags(flags)?,
        None => AccessMode::Disabled,
    };
    let output_mode = match meta_context.output_config_flags {
        Some(flags) => AccessMode::from_flags(flags)?,
        None => AccessMode::Disabled,
    };
    validate_shards_for_modes(input_mode, output_mode, &input_shards, &output_shards)?;

    if input_shards == output_shards {
        return Ok(());
    }

    let authority = meta_context
        .access_authority
        .as_ref()
        .ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
    }
}

fn validate_group_output_locks() -> Result<(), Error> {
    let mut index = 0;
    loop {
        match load_cell_lock(index, Source::GroupOutput) {
            Ok(lock) => {
                let code_hash: [u8; 32] = lock.code_hash().unpack();
                if !is_allowed_always_success_lock_code_hash(&code_hash) {
                    return Err(Error::InvalidArgs);
                }
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}
