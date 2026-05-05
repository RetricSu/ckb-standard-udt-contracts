#![allow(deprecated)]

use alloc::{ffi::CString, string::String, vec::Vec};
use core::ffi::CStr;

use ckb_std::{
    ckb_types::core::ScriptHashType,
    dynamic_loading::{CKBDLContext, Symbol},
    high_level::spawn_cell,
    syscalls::wait,
};

use crate::{error::Error, meta::ScriptAttr, run::Operation};

type ExtensionFn = unsafe extern "C" fn(*const u8, u8, u8, *const u8, usize, u8) -> i8;

pub fn run_extensions(
    operation: Operation,
    extensions: &[ScriptAttr],
    mint_authority_checked: bool,
) -> Result<(), Error> {
    for (index, extension) in extensions.iter().enumerate() {
        match extension.location {
            3 => {
                run_dynamic_linking_extension(operation, index, extension, mint_authority_checked)?
            }
            4 => run_spawn_extension(operation, index, extension, mint_authority_checked)?,
            _ => return Err(Error::InvalidMetaData),
        }
    }
    Ok(())
}

fn run_dynamic_linking_extension(
    operation: Operation,
    index: usize,
    extension: &ScriptAttr,
    mint_authority_checked: bool,
) -> Result<(), Error> {
    let script = extension.script.as_ref().ok_or(Error::InvalidMetaData)?;
    let code_hash = script.code_hash().raw_data();
    let mut context = unsafe { CKBDLContext::<[u8; 128 * 1024]>::new() };
    let library = context
        .load(&code_hash)
        .map_err(|_| Error::ExtensionFailed)?;
    let ext_data = script.args().raw_data();

    let result = unsafe {
        let validate: Symbol<ExtensionFn> = library
            .get(b"eudt_validate")
            .ok_or(Error::ExtensionFailed)?;
        validate(
            extension.script_hash.as_ptr(),
            operation.code(),
            index as u8,
            ext_data.as_ptr(),
            ext_data.len(),
            u8::from(mint_authority_checked),
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::ExtensionFailed)
    }
}

fn run_spawn_extension(
    operation: Operation,
    index: usize,
    extension: &ScriptAttr,
    mint_authority_checked: bool,
) -> Result<(), Error> {
    let script = extension.script.as_ref().ok_or(Error::InvalidMetaData)?;
    let code_hash = script.code_hash().raw_data();
    let op = CString::new(decimal_byte(operation.code())).map_err(|_| Error::InvalidMetaData)?;
    let ext_index = CString::new(decimal_byte(index as u8)).map_err(|_| Error::InvalidMetaData)?;
    let ext_data =
        CString::new(hex_encode(&script.args().raw_data())).map_err(|_| Error::InvalidMetaData)?;
    let checked = CString::new(if mint_authority_checked { "1" } else { "0" })
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

fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(data.len() * 2);
    for byte in data {
        out.push(HEX[(byte >> 4) as usize]);
        out.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(out).unwrap_or_else(|_| String::new())
}
