mod authority;
mod cells;
mod parser;

use ckb_std::high_level::load_script;

use crate::error::Error;

pub use authority::check_authority;
use cells::find_meta_in_source;
use ckb_std::ckb_constants::Source;
use standard_udt_types::metadata::Authority;

pub struct MetaContext {
    pub input_config_flags: Option<u8>,
    pub output_config_flags: Option<u8>,
    pub mint_authority: Option<Authority>,
    pub access_authority: Option<Authority>,
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

    let authority_meta = input
        .as_ref()
        .or(cell_dep.as_ref())
        .or(output.as_ref())
        .ok_or(Error::MetaMissing)?;
    Ok(MetaContext {
        input_config_flags: input
            .as_ref()
            .or(cell_dep.as_ref())
            .map(|meta| meta.config_flags),
        output_config_flags: output
            .as_ref()
            .or(cell_dep.as_ref())
            .map(|meta| meta.config_flags),
        mint_authority: authority_meta.mint_authority.clone(),
        access_authority: authority_meta.access_authority.clone(),
    })
}
