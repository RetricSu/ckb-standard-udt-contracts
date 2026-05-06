use crate::error::Error;

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
const CONFIG_PAUSED: u8 = 0b0000_1000;

const XUDT_ALLOWED_CONFIG_MASK: u8 =
    CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED;

pub(crate) fn validate_config(config_flags: u8) -> Result<(), Error> {
    if config_flags & !XUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidMetaData);
    }
    if config_flags & CONFIG_ACCESS_WHITELIST != 0 && config_flags & CONFIG_ACCESS_ENABLED == 0 {
        return Err(Error::InvalidMetaData);
    }
    Ok(())
}

pub fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

pub fn access_enabled(config_flags: u8) -> bool {
    config_flags & CONFIG_ACCESS_ENABLED != 0
}

pub fn whitelist_mode(config_flags: u8) -> bool {
    config_flags & CONFIG_ACCESS_WHITELIST != 0
}

pub fn paused(config_flags: u8) -> bool {
    config_flags & CONFIG_PAUSED != 0
}
