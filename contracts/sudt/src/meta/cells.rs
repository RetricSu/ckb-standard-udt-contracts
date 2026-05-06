use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};

use crate::{error::Error, meta::parser::parse_meta};
use standard_udt_script_utils::{amount, error::ScriptError};
use standard_udt_types::metadata::SudtMeta;

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
    amount::collect_group_amount(source).map_err(map_amount_error)
}

pub(crate) fn find_unique_visible_meta(
    meta_type_hash: &[u8; 32],
) -> Result<Option<SudtMeta>, Error> {
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
) -> Result<Option<SudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(type_hash)) if &type_hash == meta_type_hash => {
                if found.is_some() {
                    return Err(Error::MetaNotUnique);
                }
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

fn map_amount_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding => Error::AmountEncoding,
        ScriptError::AmountOverflow => Error::AmountOverflow,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
