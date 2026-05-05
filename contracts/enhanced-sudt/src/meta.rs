use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_script},
};

use crate::error::Error;

const UDT_AMOUNT_LEN: usize = 16;
const SUDT_META_FIELDS: usize = 9;
const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const SUDT_ALLOWED_CONFIG_MASK: u8 = CONFIG_SUPPLY_TRACKED;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub mint_authority: Option<ScriptAttr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptAttr {
    pub location: u8,
    pub script_hash: [u8; 32],
}

pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error> {
    let script = load_script().map_err(|_| Error::Syscall)?;
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
            Err(_) => return Err(Error::Syscall),
        }
    }
}

pub fn validate_mint(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error> {
    let Some(visible_meta) = find_unique_visible_meta(meta_type_hash)? else {
        return validate_initial_create_mint(meta_type_hash, delta);
    };
    require_authority(visible_meta.mint_authority.as_ref())?;

    if is_supply_tracked(visible_meta.config_flags) {
        let input_meta =
            find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(Error::MetaInputMissing)?;
        let output_meta =
            find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaOutputMissing)?;
        let expected = input_meta
            .current_supply
            .checked_add(delta)
            .ok_or(Error::SupplyOverflow)?;
        if output_meta.current_supply != expected
            || output_meta.config_flags != input_meta.config_flags
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if visible_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn validate_initial_create_mint(meta_type_hash: &[u8; 32], _delta: u128) -> Result<(), Error> {
    if find_meta_in_source(meta_type_hash, Source::Input)?.is_some() {
        return Err(Error::MetaNotUnique);
    }

    let _output_meta =
        find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;

    Ok(())
}

pub fn validate_burn_or_destruction(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error> {
    let Some(input_meta) = find_meta_in_source(meta_type_hash, Source::Input)? else {
        return Ok(());
    };

    require_authority(input_meta.mint_authority.as_ref())?;

    if is_supply_tracked(input_meta.config_flags) {
        let output_meta =
            find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaOutputMissing)?;
        let expected = input_meta
            .current_supply
            .checked_sub(delta)
            .ok_or(Error::SupplyUnderflow)?;
        if output_meta.current_supply != expected
            || output_meta.config_flags != input_meta.config_flags
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if input_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() != UDT_AMOUNT_LEN {
        return Err(Error::AmountEncoding);
    }

    let mut raw = [0u8; UDT_AMOUNT_LEN];
    raw.copy_from_slice(data);
    Ok(u128::from_le_bytes(raw))
}

fn find_unique_visible_meta(meta_type_hash: &[u8; 32]) -> Result<Option<SudtMeta>, Error> {
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

fn find_meta_in_source(
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
                let data = load_cell_data(index, source).map_err(|_| Error::Syscall)?;
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

fn parse_meta(data: &[u8]) -> Result<SudtMeta, Error> {
    let offsets = table_offsets(data, SUDT_META_FIELDS)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    if config_flags & !SUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidMetaData);
    }

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    let mint_authority = parse_script_attr_opt(&data[offsets[7]..offsets[8]])?;

    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidMetaData);
    }

    // TODO: enforce the Meta lock whitelist once canonical lock hashes are available.
    Ok(SudtMeta {
        config_flags,
        current_supply,
        mint_authority,
    })
}

fn require_authority(authority: Option<&ScriptAttr>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
    }
}

fn check_authority(authority: &ScriptAttr) -> Result<bool, Error> {
    match authority.location {
        0 => has_input_lock_hash(&authority.script_hash),
        1 => has_type_hash(&authority.script_hash, Source::Input),
        2 => has_type_hash(&authority.script_hash, Source::Output),
        3 | 4 => Err(Error::UnsupportedAuthorityLocation),
        _ => Err(Error::InvalidMetaData),
    }
}

fn has_input_lock_hash(target: &[u8; 32]) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_lock_hash(index, Source::Input) {
            Ok(candidate) if &candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

fn has_type_hash(target: &[u8; 32], source: Source) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(candidate)) if &candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

fn table_offsets(data: &[u8], fields: usize) -> Result<Vec<usize>, Error> {
    if data.len() < 4 + fields * 4 {
        return Err(Error::InvalidMetaData);
    }

    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut offsets = Vec::with_capacity(fields + 1);
    for index in 0..fields {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

    let expected_header = 4 + fields * 4;
    if offsets[0] != expected_header {
        return Err(Error::InvalidMetaData);
    }
    for index in 1..offsets.len() {
        if offsets[index] < offsets[index - 1] || offsets[index] > total_size {
            return Err(Error::InvalidMetaData);
        }
    }

    Ok(offsets)
}

fn single_byte_field(data: &[u8], start: usize, end: usize) -> Result<u8, Error> {
    if end != start + 1 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }
    Ok(data[start])
}

fn u128_field(data: &[u8], start: usize, end: usize) -> Result<u128, Error> {
    if end != start + 16 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 16];
    raw.copy_from_slice(&data[start..end]);
    Ok(u128::from_le_bytes(raw))
}

fn parse_script_attr_opt(data: &[u8]) -> Result<Option<ScriptAttr>, Error> {
    if data.is_empty() {
        return Ok(None);
    }
    parse_script_attr(data).map(Some)
}

fn parse_script_attr(data: &[u8]) -> Result<ScriptAttr, Error> {
    let offsets = table_offsets(data, 3)?;
    let location = single_byte_field(data, offsets[0], offsets[1])?;
    let script_hash = byte32_field(data, offsets[1], offsets[2])?;
    let script_opt = &data[offsets[2]..offsets[3]];

    match location {
        0..=2 if script_opt.is_empty() => Ok(ScriptAttr {
            location,
            script_hash,
        }),
        3 | 4 if !script_opt.is_empty() => Ok(ScriptAttr {
            location,
            script_hash,
        }),
        0..=4 => Err(Error::InvalidMetaData),
        _ => Err(Error::InvalidMetaData),
    }
}

fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
