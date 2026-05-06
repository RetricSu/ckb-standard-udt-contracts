use crate::error::Error;
use standard_udt_types::metadata::{access_enabled, whitelist_mode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessMode {
    Disabled,
    Blacklist,
    Whitelist,
}

impl AccessMode {
    pub fn from_flags(config_flags: u8) -> Result<Self, Error> {
        let enabled = access_enabled(config_flags);
        let whitelist = whitelist_mode(config_flags);
        match (enabled, whitelist) {
            (false, false) => Ok(Self::Disabled),
            (true, false) => Ok(Self::Blacklist),
            (true, true) => Ok(Self::Whitelist),
            (false, true) => Err(Error::InvalidMetaData),
        }
    }
}
