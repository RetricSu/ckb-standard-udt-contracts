use crate::{
    error::Error,
    meta_cell::{CONFIG_SUPPLY_TRACKED, SudtMeta, is_supply_tracked},
};

pub fn validate_update(input: &SudtMeta, output: &SudtMeta) -> Result<(), Error> {
    if input.config_flags & CONFIG_SUPPLY_TRACKED != output.config_flags & CONFIG_SUPPLY_TRACKED {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if input.current_supply != output.current_supply {
        return Err(Error::AuthorityMissing);
    }

    Ok(())
}
