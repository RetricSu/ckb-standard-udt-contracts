use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash},
};

use crate::error::ScriptError;

pub struct VisibleMeta {
    pub source: Source,
    pub index: usize,
    pub data: Vec<u8>,
}

pub fn find_unique_meta_by_type_hash(
    meta_type_hash: &[u8; 32],
) -> Result<VisibleMeta, ScriptError> {
    let mut found = None;

    for source in [Source::CellDep, Source::Input] {
        if let Some(meta) = find_meta_in_source(meta_type_hash, source)? {
            if found.is_some() {
                return Err(ScriptError::MetaNotUnique);
            }
            found = Some(meta);
        }
    }

    found.ok_or(ScriptError::MetaMissing)
}

pub fn find_input_meta_by_type_hash(meta_type_hash: &[u8; 32]) -> Result<VisibleMeta, ScriptError> {
    find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(ScriptError::MetaInputMissing)
}

pub fn find_output_meta_by_type_hash(
    meta_type_hash: &[u8; 32],
) -> Result<VisibleMeta, ScriptError> {
    find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(ScriptError::MetaOutputMissing)
}

fn find_meta_in_source(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<Option<VisibleMeta>, ScriptError> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(type_hash)) if &type_hash == meta_type_hash => {
                if found.is_some() {
                    return Err(ScriptError::MetaNotUnique);
                }
                let data = load_cell_data(index, source).map_err(|_| ScriptError::Syscall)?;
                found = Some(VisibleMeta {
                    source,
                    index,
                    data,
                });
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(_) => return Err(ScriptError::Syscall),
        }
    }
}
