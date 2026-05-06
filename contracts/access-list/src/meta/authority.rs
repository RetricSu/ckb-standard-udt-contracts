use standard_udt_script_utils::{
    authority::{ParsedAuthority as RuntimeAuthority, check_authority as check_runtime_authority},
    error::ScriptError,
};

use crate::{error::Error, meta::ParsedAuthority};

pub fn check_authority(authority: &ParsedAuthority) -> Result<bool, Error> {
    check_runtime_authority(&RuntimeAuthority {
        authority_type: authority.authority_type,
        script_hash: authority.script_hash,
        script: authority.script.clone(),
    })
    .map_err(map_script_error)
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
