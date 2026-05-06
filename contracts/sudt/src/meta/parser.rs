use standard_udt_types::metadata::{SudtMeta, is_supply_tracked as types_is_supply_tracked};

use crate::error::Error;

pub(crate) fn parse_meta(data: &[u8]) -> Result<SudtMeta, Error> {
    SudtMeta::from_slice(data).map_err(Error::from)
}

pub(crate) fn is_supply_tracked(config_flags: u8) -> bool {
    types_is_supply_tracked(config_flags)
}
