use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_script, load_script_hash},
    type_id::check_type_id,
};

use crate::{error::Error, meta_cell::parser::parse_meta};
use standard_udt_types::metadata::XudtMeta;

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

pub struct MetaGroup {
    pub input: Option<XudtMeta>,
    pub output: Option<XudtMeta>,
    pub meta_type_hash: [u8; 32],
}

pub fn load_meta_group() -> Result<MetaGroup, Error> {
    Ok(MetaGroup {
        input: load_group_meta(Source::GroupInput)?,
        output: load_group_meta(Source::GroupOutput)?,
        meta_type_hash: load_script_hash()?,
    })
}

pub fn validate_type_args() -> Result<(), Error> {
    let script = load_script()?;
    if script.args().raw_data().len() != 32 {
        return Err(Error::InvalidArgs);
    }
    Ok(())
}

pub fn validate_create_type_id() -> Result<(), Error> {
    check_type_id(0, 32).map_err(Error::from)
}

fn load_group_meta(source: Source) -> Result<Option<XudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                if found.is_some() {
                    return Err(Error::DuplicateMetaCell);
                }
                if source == Source::GroupOutput {
                    validate_meta_lock(index, source)?;
                }
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(error) => return Err(error.into()),
        }
    }
}

fn validate_meta_lock(index: usize, source: Source) -> Result<(), Error> {
    let lock = load_cell_lock(index, source)?;
    let code_hash: [u8; 32] = lock.code_hash().unpack();
    if is_allowed_always_success_lock_code_hash(&code_hash) {
        Ok(())
    } else {
        Err(Error::InvalidArgs)
    }
}
