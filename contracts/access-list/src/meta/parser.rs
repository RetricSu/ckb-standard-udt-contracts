use ckb_std::ckb_types::packed::Script;
use standard_udt_types::metadata::{Authority as TypeAuthority, XudtMeta};

use crate::error::Error;

use super::ParsedXudtMeta;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

pub(super) fn parse_meta(data: &[u8]) -> Result<ParsedXudtMeta, Error> {
    let meta = XudtMeta::from_slice(data).map_err(Error::from)?;

    Ok(ParsedXudtMeta {
        config_flags: meta.config_flags,
        access_authority: meta.access_authority.map(parsed_authority),
    })
}

fn parsed_authority(authority: TypeAuthority) -> ParsedAuthority {
    ParsedAuthority {
        authority_type: authority.authority_type.into(),
        script_hash: authority.script_hash,
        script: authority.script,
    }
}
