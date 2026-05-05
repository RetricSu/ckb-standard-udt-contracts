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

/// Finds one metadata cell visible to type scripts through cell deps or inputs.
///
/// This helper intentionally scans only `Source::CellDep` and `Source::Input`.
/// Those are the stable locations used by normal metadata read/update flows:
/// immutable reads load metadata from deps, while update transactions can expose
/// the previous metadata cell in inputs. Outputs are excluded so a newly created
/// or replacement metadata output does not make an otherwise valid update look
/// non-unique. Call `find_input_meta_by_type_hash` or
/// `find_output_meta_by_type_hash` when a contract needs location-specific
/// state comparisons.
pub fn find_unique_meta_by_type_hash(
    meta_type_hash: &[u8; 32],
) -> Result<VisibleMeta, ScriptError> {
    find_unique_meta_in_sources(meta_type_hash, &[Source::CellDep, Source::Input])?
        .ok_or(ScriptError::MetaMissing)
}

pub fn find_cell_dep_meta_by_type_hash(
    meta_type_hash: &[u8; 32],
) -> Result<VisibleMeta, ScriptError> {
    find_meta_in_source(meta_type_hash, Source::CellDep)?.ok_or(ScriptError::MetaMissing)
}

pub fn find_input_meta_by_type_hash(meta_type_hash: &[u8; 32]) -> Result<VisibleMeta, ScriptError> {
    find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(ScriptError::MetaInputMissing)
}

pub fn find_output_meta_by_type_hash(
    meta_type_hash: &[u8; 32],
) -> Result<VisibleMeta, ScriptError> {
    find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(ScriptError::MetaOutputMissing)
}

fn find_unique_meta_in_sources(
    meta_type_hash: &[u8; 32],
    sources: &[Source],
) -> Result<Option<VisibleMeta>, ScriptError> {
    let mut found = None;

    for source in sources {
        if let Some(meta) = find_meta_in_source(meta_type_hash, *source)? {
            if found.is_some() {
                return Err(ScriptError::MetaNotUnique);
            }
            found = Some(meta);
        }
    }

    Ok(found)
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
