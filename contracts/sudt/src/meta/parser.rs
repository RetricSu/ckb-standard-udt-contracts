use ckb_std::ckb_types::packed::Script;
use standard_udt_types::metadata::{Authority as TypeAuthority, SudtMeta};

use crate::error::Error;

const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub mint_authority: Option<ParsedAuthority>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

pub(crate) fn parse_meta(data: &[u8]) -> Result<ParsedSudtMeta, Error> {
    let meta = SudtMeta::from_slice(data).map_err(Error::from)?;

    Ok(ParsedSudtMeta {
        config_flags: meta.config_flags,
        current_supply: meta.current_supply,
        mint_authority: meta.mint_authority.map(parsed_authority),
    })
}

pub(crate) fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

fn parsed_authority(authority: TypeAuthority) -> ParsedAuthority {
    ParsedAuthority {
        authority_type: authority.authority_type.into(),
        script_hash: authority.script_hash,
        script: authority.script,
    }
}
