use crate::error::Error;

pub const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
pub const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessMode {
    Disabled,
    Blacklist,
    Whitelist,
}

impl AccessMode {
    pub fn from_flags(config_flags: u8) -> Result<Self, Error> {
        let enabled = config_flags & CONFIG_ACCESS_ENABLED != 0;
        let whitelist = config_flags & CONFIG_ACCESS_WHITELIST != 0;
        match (enabled, whitelist) {
            (false, false) => Ok(Self::Disabled),
            (true, false) => Ok(Self::Blacklist),
            (true, true) => Ok(Self::Whitelist),
            (false, true) => Err(Error::InvalidMetaData),
        }
    }
}
