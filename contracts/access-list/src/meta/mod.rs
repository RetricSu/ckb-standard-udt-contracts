mod authority;
mod cells;
mod parser;

use ckb_std::high_level::load_script;

use crate::error::Error;

pub use authority::check_authority;
use cells::find_meta_in_source;
use ckb_std::ckb_constants::Source;
pub use parser::ScriptAttr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParsedXudtMeta {
    pub(super) config_flags: u8,
    pub(super) access_authority: Option<ScriptAttr>,
}

pub struct MetaContext {
    pub output_config_flags: u8,
    pub access_authority: Option<ScriptAttr>,
}

pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error> {
    let script = load_script()?;
    let args = script.args().raw_data();
    if args.len() != 32 {
        return Err(Error::InvalidArgs);
    }

    let mut meta_type_hash = [0u8; 32];
    meta_type_hash.copy_from_slice(&args);
    Ok(meta_type_hash)
}

pub fn load_meta_context(meta_type_hash: &[u8; 32]) -> Result<MetaContext, Error> {
    let input = find_meta_in_source(meta_type_hash, Source::Input)?;
    let output = find_meta_in_source(meta_type_hash, Source::Output)?;
    let cell_dep = find_meta_in_source(meta_type_hash, Source::CellDep)?;

    if cell_dep.is_some() && (input.is_some() || output.is_some()) {
        return Err(Error::MetaNotUnique);
    }

    let authority_meta = input
        .as_ref()
        .or(cell_dep.as_ref())
        .or(output.as_ref())
        .ok_or(Error::MetaMissing)?;
    let output_meta = output
        .as_ref()
        .or(input.as_ref())
        .or(cell_dep.as_ref())
        .unwrap();

    Ok(MetaContext {
        output_config_flags: output_meta.config_flags,
        access_authority: authority_meta.access_authority.clone(),
    })
}
