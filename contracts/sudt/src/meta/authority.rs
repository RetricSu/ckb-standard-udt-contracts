use standard_udt_script_utils::{
    authority::check_authority as check_runtime_authority, error::ScriptError,
};
use standard_udt_types::metadata::Authority;

use crate::error::Error;

pub(crate) fn require_authority(authority: Option<&Authority>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
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
