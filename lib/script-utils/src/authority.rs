#![allow(deprecated)]

use alloc::vec::Vec;
#[cfg(target_arch = "riscv64")]
use alloc::{ffi::CString, string::String};
#[cfg(target_arch = "riscv64")]
use core::ffi::CStr;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash},
};
#[cfg(target_arch = "riscv64")]
use ckb_std::{
    ckb_types::{core::ScriptHashType, packed::Script},
    dynamic_loading_c_impl::{CKBDLContext, Symbol},
    high_level::spawn_cell,
    syscalls::wait,
};
use standard_udt_types::metadata::{Authority, AuthorityType};

use crate::error::ScriptError;

#[cfg(target_arch = "riscv64")]
type AuthorityFn = unsafe extern "C" fn(*const u8, *const u8, usize) -> i8;

pub fn check_authority(authority: &Authority) -> Result<bool, ScriptError> {
    validate_authority_shape(authority)?;

    match authority.authority_type {
        AuthorityType::InputLock => has_input_lock_hash(&authority.script_hash),
        AuthorityType::InputType => has_input_type_hash(&authority.script_hash),
        AuthorityType::OutputType => has_output_type_hash(&authority.script_hash),
        AuthorityType::DynamicLinking => run_dynamic_linking_authority(authority),
        AuthorityType::Spawn => run_spawn_authority(authority),
    }
}

pub struct AuthorityVerifier {
    checked: Vec<(Authority, bool)>,
    checker: fn(&Authority) -> Result<bool, ScriptError>,
}

impl AuthorityVerifier {
    pub fn new() -> Self {
        Self::with_checker(check_authority)
    }

    pub fn with_checker(checker: fn(&Authority) -> Result<bool, ScriptError>) -> Self {
        Self {
            checked: Vec::new(),
            checker,
        }
    }

    pub fn require(&mut self, authority: Option<&Authority>) -> Result<(), ScriptError> {
        let authority = authority.ok_or(ScriptError::AuthorityMissing)?;
        match self.check(authority) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ScriptError::AuthorityFailed),
            Err(error) => Err(error),
        }
    }

    pub fn require_with_fallback(
        &mut self,
        authority: Option<&Authority>,
        fallback: Option<&Authority>,
    ) -> Result<(), ScriptError> {
        if let Some(authority) = authority {
            match self.check(authority) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(ScriptError::AuthorityFailed) => {
                    if fallback.is_none() {
                        return Err(ScriptError::AuthorityFailed);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        self.require(fallback)
    }

    pub fn require_any(&mut self, authorities: &[Option<&Authority>]) -> Result<(), ScriptError> {
        let mut has_authority = false;
        for authority in authorities.iter().filter_map(|authority| *authority) {
            has_authority = true;
            match self.check(authority) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(ScriptError::AuthorityFailed) => {}
                Err(error) => return Err(error),
            }
        }

        if has_authority {
            Err(ScriptError::AuthorityFailed)
        } else {
            Err(ScriptError::AuthorityMissing)
        }
    }

    pub fn check(&mut self, authority: &Authority) -> Result<bool, ScriptError> {
        if let Some((_, result)) = self
            .checked
            .iter()
            .find(|(checked_authority, _)| checked_authority == authority)
        {
            return Ok(*result);
        }

        let result = (self.checker)(authority)?;
        self.checked.push((authority.clone(), result));
        Ok(result)
    }
}

fn validate_authority_shape(authority: &Authority) -> Result<(), ScriptError> {
    match authority.authority_type {
        AuthorityType::InputLock | AuthorityType::InputType | AuthorityType::OutputType
            if authority.script.is_none() =>
        {
            Ok(())
        }
        AuthorityType::DynamicLinking | AuthorityType::Spawn => {
            let script = authority
                .script
                .as_ref()
                .ok_or(ScriptError::InvalidAuthority)?;
            let script_hash: [u8; 32] = script.calc_script_hash().unpack();
            if script_hash == authority.script_hash {
                Ok(())
            } else {
                Err(ScriptError::InvalidAuthority)
            }
        }
        AuthorityType::InputLock | AuthorityType::InputType | AuthorityType::OutputType => {
            Err(ScriptError::InvalidAuthority)
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

#[cfg(target_arch = "riscv64")]
fn run_dynamic_linking_authority(authority: &Authority) -> Result<bool, ScriptError> {
    let script = authority
        .script
        .as_ref()
        .ok_or(ScriptError::InvalidAuthority)?;
    let code_hash = script.code_hash().raw_data();
    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let library = context
        .load_by(&code_hash, script_hash_type(script)?)
        .map_err(|_| ScriptError::AuthorityFailed)?;
    let args = script.args().raw_data();

    let result = unsafe {
        let authorize: Symbol<AuthorityFn> = library
            .get(b"udt_authorize")
            .ok_or(ScriptError::AuthorityFailed)?;
        authorize(authority.script_hash.as_ptr(), args.as_ptr(), args.len())
    };

    Ok(result == 0)
}

#[cfg(not(target_arch = "riscv64"))]
fn run_dynamic_linking_authority(_authority: &Authority) -> Result<bool, ScriptError> {
    Err(ScriptError::AuthorityFailed)
}

#[cfg(target_arch = "riscv64")]
fn run_spawn_authority(authority: &Authority) -> Result<bool, ScriptError> {
    let script = authority
        .script
        .as_ref()
        .ok_or(ScriptError::InvalidAuthority)?;
    let code_hash = script.code_hash().raw_data();
    let authority_hash = CString::new(hex_encode(&authority.script_hash))
        .map_err(|_| ScriptError::InvalidAuthority)?;
    let script_args = CString::new(hex_encode(&script.args().raw_data()))
        .map_err(|_| ScriptError::InvalidAuthority)?;
    let args: [&CStr; 2] = [authority_hash.as_c_str(), script_args.as_c_str()];

    let pid = spawn_cell(&code_hash, script_hash_type(script)?, &args, &[])
        .map_err(|_| ScriptError::AuthorityFailed)?;
    let exit_code = wait(pid).map_err(|_| ScriptError::AuthorityFailed)?;

    Ok(exit_code == 0)
}

#[cfg(not(target_arch = "riscv64"))]
fn run_spawn_authority(_authority: &Authority) -> Result<bool, ScriptError> {
    Err(ScriptError::AuthorityFailed)
}

#[cfg(target_arch = "riscv64")]
fn script_hash_type(script: &Script) -> Result<ScriptHashType, ScriptError> {
    let value: u8 = script.hash_type().into();
    match value {
        0 => Ok(ScriptHashType::Data),
        1 => Ok(ScriptHashType::Type),
        2 => Ok(ScriptHashType::Data1),
        4 => Ok(ScriptHashType::Data2),
        _ => Err(ScriptError::InvalidAuthority),
    }
}

#[cfg(target_arch = "riscv64")]
fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(data.len() * 2);
    for byte in data {
        out.push(HEX[(byte >> 4) as usize]);
        out.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(out).unwrap_or_else(|_| String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ckb_std::ckb_types::{
        bytes::Bytes,
        core::ScriptHashType,
        packed::{Byte32, Script},
    };
    use standard_udt_types::metadata::AuthorityType;

    fn dummy_script() -> Script {
        Script::new_builder()
            .code_hash(Byte32::from_slice(&[9u8; 32]).expect("byte32"))
            .hash_type(ScriptHashType::Data)
            .args(Bytes::from(&b"args"[..]).pack())
            .build()
    }

    fn attr(authority_type: AuthorityType) -> Authority {
        Authority {
            authority_type,
            script_hash: [7u8; 32],
            script: None,
        }
    }

    fn valid_input_lock_authority(script_hash: [u8; 32]) -> Authority {
        Authority {
            authority_type: AuthorityType::InputLock,
            script_hash,
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
            Err(ScriptError::InvalidAuthority)
        );
        assert_eq!(
            check_authority(&attr(AuthorityType::Spawn)),
            Err(ScriptError::InvalidAuthority)
        );
    }

    #[test]
    fn authority_shape_rejects_scripts_for_hash_scan_modes() {
        let script = dummy_script();
        let authority = Authority {
            authority_type: AuthorityType::InputLock,
            script_hash: [0u8; 32],
            script: Some(script),
        };

        assert_eq!(
            check_authority(&authority),
            Err(ScriptError::InvalidAuthority)
        );
    }

    #[test]
    fn authority_shape_requires_script_for_executable_modes() {
        let authority = Authority {
            authority_type: AuthorityType::DynamicLinking,
            script_hash: [0u8; 32],
            script: None,
        };

        assert_eq!(
            check_authority(&authority),
            Err(ScriptError::InvalidAuthority)
        );
    }

    #[test]
    fn authority_shape_rejects_mismatched_script_hash() {
        let script = dummy_script();
        let authority = Authority {
            authority_type: AuthorityType::Spawn,
            script_hash: [0u8; 32],
            script: Some(script),
        };

        assert_eq!(
            check_authority(&authority),
            Err(ScriptError::InvalidAuthority)
        );
    }

    #[test]
    fn authority_shape_accepts_matching_executable_script() {
        let script = dummy_script();
        let script_hash: [u8; 32] = script.calc_script_hash().unpack();
        let authority = Authority {
            authority_type: AuthorityType::DynamicLinking,
            script_hash,
            script: Some(script),
        };

        assert_eq!(validate_authority_shape(&authority), Ok(()));
    }

    #[test]
    fn authority_verifier_reports_missing_authority() {
        let mut verifier = AuthorityVerifier::with_checker(|_| Ok(true));

        assert_eq!(verifier.require(None), Err(ScriptError::AuthorityMissing));
    }

    #[test]
    fn authority_verifier_falls_back_to_mint_authority() {
        let access = valid_input_lock_authority([1u8; 32]);
        let mint = valid_input_lock_authority([2u8; 32]);
        let mut verifier =
            AuthorityVerifier::with_checker(|authority| Ok(authority.script_hash == [2u8; 32]));

        assert_eq!(
            verifier.require_with_fallback(Some(&access), Some(&mint)),
            Ok(())
        );
    }

    #[test]
    fn authority_verifier_caches_repeated_checks() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);
        CALLS.store(0, Ordering::SeqCst);

        let authority = valid_input_lock_authority([3u8; 32]);
        let mut verifier = AuthorityVerifier::with_checker(|_| {
            CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        });

        assert_eq!(verifier.require(Some(&authority)), Ok(()));
        assert_eq!(verifier.require(Some(&authority)), Ok(()));
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    }
}
