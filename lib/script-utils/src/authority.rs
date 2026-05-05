use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash},
};
use standard_udt_types::metadata::{ScriptAttr, ScriptLocation};

use crate::error::ScriptError;

pub fn check_authority(attr: &ScriptAttr) -> Result<bool, ScriptError> {
    match attr.location {
        ScriptLocation::InputLock => has_input_lock_hash(&attr.script_hash),
        ScriptLocation::InputType => has_input_type_hash(&attr.script_hash),
        ScriptLocation::OutputType => has_output_type_hash(&attr.script_hash),
        ScriptLocation::DynamicLinking | ScriptLocation::Spawn => {
            Err(ScriptError::UnsupportedAuthorityLocation)
        }
    }
}

pub fn has_input_lock_hash(lock_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    let mut index = 0;

    loop {
        match load_cell_lock_hash(index, Source::Input) {
            Ok(candidate) if &candidate == lock_hash => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(ScriptError::Syscall),
        }
    }
}

pub fn has_input_type_hash(type_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    has_type_hash(type_hash, Source::Input)
}

pub fn has_output_type_hash(type_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    has_type_hash(type_hash, Source::Output)
}

fn has_type_hash(type_hash: &[u8; 32], source: Source) -> Result<bool, ScriptError> {
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(candidate)) if &candidate == type_hash => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(ScriptError::Syscall),
        }
    }
}
