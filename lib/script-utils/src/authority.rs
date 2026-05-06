use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash},
};
use standard_udt_types::metadata::{Authority, AuthorityType};

use crate::error::ScriptError;

pub fn check_authority(authority: &Authority) -> Result<bool, ScriptError> {
    match authority.authority_type {
        AuthorityType::InputLock => has_input_lock_hash(&authority.script_hash),
        AuthorityType::InputType => has_input_type_hash(&authority.script_hash),
        AuthorityType::OutputType => has_output_type_hash(&authority.script_hash),
        AuthorityType::DynamicLinking | AuthorityType::Spawn => {
            Err(ScriptError::UnsupportedAuthorityLocation)
        }
    }
}

pub fn has_input_lock_hash(lock_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    scan_lock_hash(*lock_hash, |index| {
        load_cell_lock_hash(index, Source::Input)
    })
}

pub fn has_input_type_hash(type_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    has_type_hash(type_hash, Source::Input)
}

pub fn has_output_type_hash(type_hash: &[u8; 32]) -> Result<bool, ScriptError> {
    has_type_hash(type_hash, Source::Output)
}

fn has_type_hash(type_hash: &[u8; 32], source: Source) -> Result<bool, ScriptError> {
    scan_type_hash(*type_hash, |index| load_cell_type_hash(index, source))
}

fn scan_lock_hash<F>(target: [u8; 32], mut load_hash: F) -> Result<bool, ScriptError>
where
    F: FnMut(usize) -> Result<[u8; 32], SysError>,
{
    let mut index = 0;

    loop {
        match load_hash(index) {
            Ok(candidate) if candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(ScriptError::SyscallUnknown),
        }
    }
}

fn scan_type_hash<F>(target: [u8; 32], mut load_hash: F) -> Result<bool, ScriptError>
where
    F: FnMut(usize) -> Result<Option<[u8; 32]>, SysError>,
{
    let mut index = 0;

    loop {
        match load_hash(index) {
            Ok(Some(candidate)) if candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(ScriptError::SyscallUnknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(authority_type: AuthorityType) -> Authority {
        Authority {
            authority_type,
            script_hash: [7u8; 32],
            script: None,
        }
    }

    #[test]
    fn lock_hash_scanner_reports_present_and_missing() {
        assert_eq!(
            scan_lock_hash([2u8; 32], |index| match index {
                0 => Ok([1u8; 32]),
                1 => Ok([2u8; 32]),
                _ => Err(SysError::IndexOutOfBound),
            }),
            Ok(true)
        );
        assert_eq!(
            scan_lock_hash([3u8; 32], |index| match index {
                0 => Ok([1u8; 32]),
                1 => Ok([2u8; 32]),
                _ => Err(SysError::IndexOutOfBound),
            }),
            Ok(false)
        );
    }

    #[test]
    fn lock_hash_scanner_maps_unexpected_syscall_error() {
        assert_eq!(
            scan_lock_hash([2u8; 32], |_| Err(SysError::LengthNotEnough(32))),
            Err(ScriptError::SyscallUnknown)
        );
    }

    #[test]
    fn type_hash_scanner_reports_present_missing_and_none_cells() {
        assert_eq!(
            scan_type_hash([2u8; 32], |index| match index {
                0 => Ok(None),
                1 => Ok(Some([2u8; 32])),
                _ => Err(SysError::IndexOutOfBound),
            }),
            Ok(true)
        );
        assert_eq!(
            scan_type_hash([3u8; 32], |index| match index {
                0 => Ok(None),
                1 => Ok(Some([2u8; 32])),
                _ => Err(SysError::IndexOutOfBound),
            }),
            Ok(false)
        );
    }

    #[test]
    fn type_hash_scanner_maps_unexpected_syscall_error() {
        assert_eq!(
            scan_type_hash([2u8; 32], |_| Err(SysError::LengthNotEnough(32))),
            Err(ScriptError::SyscallUnknown)
        );
    }

    #[test]
    fn unsupported_authority_locations_do_not_scan_cells() {
        assert_eq!(
            check_authority(&attr(AuthorityType::DynamicLinking)),
            Err(ScriptError::UnsupportedAuthorityLocation)
        );
        assert_eq!(
            check_authority(&attr(AuthorityType::Spawn)),
            Err(ScriptError::UnsupportedAuthorityLocation)
        );
    }
}
