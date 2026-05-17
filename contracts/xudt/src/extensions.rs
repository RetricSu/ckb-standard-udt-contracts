#![allow(deprecated)]

#[cfg(target_arch = "riscv64")]
use alloc::{ffi::CString, string::String, vec::Vec};
#[cfg(target_arch = "riscv64")]
use core::ffi::CStr;

#[cfg(target_arch = "riscv64")]
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, prelude::*},
    dynamic_loading_c_impl::{CKBDLContext, Symbol},
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash, spawn_cell},
    syscalls::wait,
};

#[cfg(target_arch = "riscv64")]
use crate::meta;
use crate::{error::Error, validation::Operation};
use standard_udt_types::metadata::Extension;
#[cfg(target_arch = "riscv64")]
use standard_udt_types::metadata::ExtensionType;

#[cfg(target_arch = "riscv64")]
type ExtensionFn = unsafe extern "C" fn(*const u8, u8, u8, *const u8, usize, u8) -> i8;

pub type MintAuthorityContext = Option<bool>;

#[cfg(target_arch = "riscv64")]
const fn mint_authority_context_code(value: MintAuthorityContext) -> u8 {
    match value {
        Some(true) => 1,
        Some(false) => 0,
        None => 2,
    }
}

#[cfg(target_arch = "riscv64")]
pub fn run_extensions(
    operation: Operation,
    extensions: &[Extension],
    mint_authority_context: MintAuthorityContext,
) -> Result<(), Error> {
    for (index, extension) in extensions.iter().enumerate() {
        match meta::extension_kind(extension) {
            ExtensionType::InputLock => {
                require_input_lock_extension(extension)?;
            }
            ExtensionType::InputType => {
                require_type_extension(extension, Source::Input)?;
            }
            ExtensionType::OutputType => {
                require_type_extension(extension, Source::Output)?;
            }
            ExtensionType::DynamicLinking => {
                run_dynamic_linking_extension(operation, index, extension, mint_authority_context)?
            }
            ExtensionType::Spawn => {
                run_spawn_extension(operation, index, extension, mint_authority_context)?
            }
        }
    }
    Ok(())
}

#[cfg(target_arch = "riscv64")]
fn require_input_lock_extension(extension: &Extension) -> Result<(), Error> {
    let script_hash: [u8; 32] = meta::extension_script(extension)
        .calc_script_hash()
        .unpack();
    let mut index = 0;

    loop {
        match load_cell_lock_hash(index, Source::Input) {
            Ok(candidate) if candidate == script_hash => return Ok(()),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Err(Error::ExtensionFailed),
            Err(_) => return Err(Error::ExtensionFailed),
        }
    }
}

#[cfg(target_arch = "riscv64")]
fn require_type_extension(extension: &Extension, source: Source) -> Result<(), Error> {
    let script_hash: [u8; 32] = meta::extension_script(extension)
        .calc_script_hash()
        .unpack();
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(candidate)) if candidate == script_hash => return Ok(()),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Err(Error::ExtensionFailed),
            Err(_) => return Err(Error::ExtensionFailed),
        }
    }
}

#[cfg(not(target_arch = "riscv64"))]
pub fn run_extensions(
    _operation: Operation,
    extensions: &[Extension],
    _mint_authority_context: MintAuthorityContext,
) -> Result<(), Error> {
    if extensions.is_empty() {
        Ok(())
    } else {
        Err(Error::ExtensionFailed)
    }
}

#[cfg(target_arch = "riscv64")]
fn run_dynamic_linking_extension(
    operation: Operation,
    index: usize,
    extension: &Extension,
    mint_authority_context: MintAuthorityContext,
) -> Result<(), Error> {
    let script = meta::extension_script(extension);
    let code_hash = script.code_hash().raw_data();
    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let library = context
        .load_by(&code_hash, script_hash_type(script)?)
        .map_err(|_| Error::ExtensionFailed)?;
    let ext_data = script.args().raw_data();

    let script_hash: [u8; 32] = script.calc_script_hash().unpack();
    let result = unsafe {
        let validate: Symbol<ExtensionFn> =
            library.get(b"udt_validate").ok_or(Error::ExtensionFailed)?;
        validate(
            script_hash.as_ptr(),
            operation.code(),
            index as u8,
            ext_data.as_ptr(),
            ext_data.len(),
            mint_authority_context_code(mint_authority_context),
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::ExtensionFailed)
    }
}

#[cfg(target_arch = "riscv64")]
fn run_spawn_extension(
    operation: Operation,
    index: usize,
    extension: &Extension,
    mint_authority_context: MintAuthorityContext,
) -> Result<(), Error> {
    let script = meta::extension_script(extension);
    let code_hash = script.code_hash().raw_data();
    let op = CString::new(decimal_byte(operation.code())).map_err(|_| Error::InvalidMetaData)?;
    let ext_index = CString::new(decimal_byte(index as u8)).map_err(|_| Error::InvalidMetaData)?;
    let ext_data =
        CString::new(hex_encode(&script.args().raw_data())).map_err(|_| Error::InvalidMetaData)?;
    let checked = CString::new(decimal_byte(mint_authority_context_code(
        mint_authority_context,
    )))
    .map_err(|_| Error::InvalidMetaData)?;
    let args: [&CStr; 4] = [
        op.as_c_str(),
        ext_index.as_c_str(),
        ext_data.as_c_str(),
        checked.as_c_str(),
    ];

    let pid = spawn_cell(&code_hash, script_hash_type(script)?, &args, &[])
        .map_err(|_| Error::ExtensionFailed)?;
    let exit_code = wait(pid).map_err(|_| Error::ExtensionFailed)?;
    if exit_code == 0 {
        Ok(())
    } else {
        Err(Error::ExtensionFailed)
    }
}

#[cfg(target_arch = "riscv64")]
fn script_hash_type(script: &ckb_std::ckb_types::packed::Script) -> Result<ScriptHashType, Error> {
    let value: u8 = script.hash_type().into();
    match value {
        0 => Ok(ScriptHashType::Data),
        1 => Ok(ScriptHashType::Type),
        2 => Ok(ScriptHashType::Data1),
        4 => Ok(ScriptHashType::Data2),
        _ => Err(Error::InvalidMetaData),
    }
}

#[cfg(target_arch = "riscv64")]
fn decimal_byte(value: u8) -> String {
    if value < 10 {
        let mut s = String::new();
        s.push((b'0' + value) as char);
        s
    } else {
        let tens = value / 10;
        let ones = value % 10;
        let mut s = String::new();
        s.push((b'0' + tens) as char);
        s.push((b'0' + ones) as char);
        s
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
