use standard_udt_types::metadata::SudtMeta;

use crate::error::Error;

const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;

pub(crate) fn parse_meta(data: &[u8]) -> Result<SudtMeta, Error> {
    SudtMeta::from_slice(data).map_err(Error::from)
}

pub(crate) fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}
