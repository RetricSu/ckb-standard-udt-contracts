use crate::error::Error;

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
pub const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
pub const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
pub const CONFIG_PAUSED: u8 = 0b0000_1000;
pub const SUDT_ALLOWED_CONFIG_MASK: u8 = CONFIG_SUPPLY_TRACKED;
pub const XUDT_ALLOWED_CONFIG_MASK: u8 =
    CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED;

pub const MAX_EXTENSIONS: usize = 16;
pub const MAX_METADATA_NAME_BYTES: usize = 1024;
pub const MAX_METADATA_SYMBOL_BYTES: usize = 128;
pub const MAX_METADATA_URI_BYTES: usize = 2048;
pub const MAX_METADATA_EXTRA_DATA_BYTES: usize = 16 * 1024;
pub const MAX_ACCESSLIST_ENTRIES: usize = 4096;

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

pub fn validate_sudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    validate_config_flags(config_flags, SUDT_ALLOWED_CONFIG_MASK)?;
    validate_supply(config_flags, current_supply)
}

pub fn validate_xudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    validate_config_flags(config_flags, XUDT_ALLOWED_CONFIG_MASK)?;
    if whitelist_mode(config_flags) && !access_enabled(config_flags) {
        return Err(Error::InvalidConfigFlags);
    }
    validate_supply(config_flags, current_supply)
}

fn validate_config_flags(config_flags: u8, allowed_mask: u8) -> Result<(), Error> {
    if config_flags & !allowed_mask != 0 {
        return Err(Error::InvalidConfigFlags);
    }
    Ok(())
}

fn validate_supply(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }
    Ok(())
}
