use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::Script,
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};
use standard_udt_script_utils::{
    amount, authority::check_authority as check_runtime_authority, error::ScriptError,
};
use standard_udt_types::metadata::{
    Authority, Extension, ExtensionType, XudtMeta, access_enabled as types_access_enabled,
    is_supply_tracked as types_is_supply_tracked, paused as types_paused,
    whitelist_mode as types_whitelist_mode,
};

use crate::error::Error;

pub fn is_supply_tracked(meta: &XudtMeta) -> bool {
    types_is_supply_tracked(meta.config_flags)
}

pub fn is_access_enabled(meta: &XudtMeta) -> bool {
    types_access_enabled(meta.config_flags)
}

pub fn is_whitelist(meta: &XudtMeta) -> bool {
    types_whitelist_mode(meta.config_flags)
}

pub fn is_paused(meta: &XudtMeta) -> bool {
    types_paused(meta.config_flags)
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
    amount::collect_group_amount(source).map_err(map_amount_error)
}

pub fn find_unique_visible_meta(meta_type_hash: &[u8; 32]) -> Result<Option<XudtMeta>, Error> {
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

pub fn find_meta_in_source(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<Option<XudtMeta>, Error> {
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

pub fn require_authority(authority: Option<&Authority>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
    }
}

fn parse_meta(data: &[u8]) -> Result<XudtMeta, Error> {
    XudtMeta::from_slice(data).map_err(Error::from)
}

fn map_amount_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding => Error::AmountEncoding,
        ScriptError::AmountOverflow => Error::AmountOverflow,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}

fn check_authority(authority: &Authority) -> Result<bool, Error> {
    check_runtime_authority(authority).map_err(map_script_error)
}

fn map_script_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AuthorityFailed => Error::AuthorityFailed,
        ScriptError::UnsupportedAuthorityLocation => Error::UnsupportedAuthorityLocation,
        ScriptError::InvalidAuthority => Error::InvalidMetaData,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}

pub fn extension_script(extension: &Extension) -> &Script {
    &extension.script
}

pub fn extension_kind(extension: &Extension) -> ExtensionType {
    extension.extension_type
}
