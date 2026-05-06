use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_cell_type_hash, load_script},
};

use crate::{
    error::Error,
    meta::parser::{ParsedSudtMeta, parse_meta},
};

const UDT_AMOUNT_LEN: usize = 16;

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

pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error> {
    let script = load_script().map_err(Error::from)?;
    let args = script.args().raw_data();
    if args.len() != 32 {
        return Err(Error::InvalidArgs);
    }

    let mut meta_type_hash = [0u8; 32];
    meta_type_hash.copy_from_slice(&args);
    Ok(meta_type_hash)
}

pub fn collect_group_amount(source: Source) -> Result<u128, Error> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                let amount = decode_amount(&data)?;
                total = total.checked_add(amount).ok_or(Error::AmountOverflow)?;
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(total),
            Err(error) => return Err(error.into()),
        }
    }
}

pub(crate) fn find_unique_visible_meta(
    meta_type_hash: &[u8; 32],
) -> Result<Option<ParsedSudtMeta>, Error> {
    let mut found = None;
    for source in [Source::CellDep, Source::Input] {
        if let Some(meta) = find_meta_in_source(meta_type_hash, source)? {
            if found.is_some() {
                return Err(Error::MetaNotUnique);
            }
            found = Some(meta);
        }
    }
    Ok(found)
}

pub(crate) fn find_meta_in_source(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<Option<ParsedSudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(type_hash)) if &type_hash == meta_type_hash => {
                if found.is_some() {
                    return Err(Error::MetaNotUnique);
                }
                validate_meta_lock(index, source)?;
                let data = load_cell_data(index, source).map_err(Error::from)?;
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(error) => return Err(error.into()),
        }
    }
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() < UDT_AMOUNT_LEN {
        return Err(Error::AmountEncoding);
    }

    let mut raw = [0u8; UDT_AMOUNT_LEN];
    raw.copy_from_slice(&data[..UDT_AMOUNT_LEN]);
    Ok(u128::from_le_bytes(raw))
}

fn validate_meta_lock(index: usize, source: Source) -> Result<(), Error> {
    let lock = load_cell_lock(index, source).map_err(Error::from)?;
    let code_hash: [u8; 32] = lock.code_hash().unpack();
    if is_allowed_always_success_lock_code_hash(&code_hash) {
        Ok(())
    } else {
        Err(Error::MetaLockNotAllowed)
    }
}
