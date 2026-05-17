use ckb_std::ckb_constants::Source;

use crate::{
    access::{self, CheckedLocks},
    error::Error,
    extensions, meta,
};
use standard_udt_types::metadata::XudtMeta;

#[derive(Clone, Copy)]
pub enum Operation {
    Transfer,
    Mint,
    ProtocolBurn,
}

impl Operation {
    #[cfg(target_arch = "riscv64")]
    pub const fn code(self) -> u8 {
        match self {
            Self::Transfer => 0,
            Self::Mint => 1,
            Self::ProtocolBurn => 2,
        }
    }
}

pub fn main() -> Result<(), Error> {
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(Source::GroupOutput)?;

    if input_amount == output_amount {
        let current_meta =
            meta::find_unique_visible_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        validate_transfer(&meta_type_hash, &current_meta)
    } else if output_amount > input_amount {
        let delta = output_amount
            .checked_sub(input_amount)
            .ok_or(Error::AmountOverflow)?;
        match meta::find_unique_visible_meta(&meta_type_hash)? {
            Some(current_meta) => validate_mint(&meta_type_hash, &current_meta, delta),
            None => require_initial_mint_output_meta(&meta_type_hash),
        }
    } else {
        let delta = input_amount
            .checked_sub(output_amount)
            .ok_or(Error::AmountOverflow)?;
        match meta::find_meta_in_source(&meta_type_hash, Source::Input)? {
            Some(input_meta) => validate_protocol_burn(&meta_type_hash, &input_meta, delta),
            None if output_amount == 0 => Ok(()),
            None => {
                let current_meta =
                    meta::find_unique_visible_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
                validate_transfer(&meta_type_hash, &current_meta)
            }
        }
    }
}

fn validate_transfer(meta_type_hash: &[u8; 32], current_meta: &XudtMeta) -> Result<(), Error> {
    if meta::is_paused(current_meta) {
        return Err(Error::MetaStateMismatch);
    }
    access::validate_if_enabled(meta_type_hash, current_meta, CheckedLocks::InputsAndOutputs)?;
    extensions::run_extensions(Operation::Transfer, &current_meta.extensions, None)
}

fn validate_mint(
    meta_type_hash: &[u8; 32],
    current_meta: &XudtMeta,
    delta: u128,
) -> Result<(), Error> {
    if meta::is_paused(current_meta) {
        return Err(Error::MetaStateMismatch);
    }
    meta::require_authority(current_meta.mint_authority.as_ref())?;
    if meta::is_supply_tracked(current_meta) {
        validate_supply_delta(meta_type_hash, delta, true)?;
    } else if current_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }
    access::validate_if_enabled(meta_type_hash, current_meta, CheckedLocks::Outputs)?;
    extensions::run_extensions(Operation::Mint, &current_meta.extensions, Some(true))
}

fn require_initial_mint_output_meta(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    meta::find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;
    Ok(())
}

fn validate_protocol_burn(
    meta_type_hash: &[u8; 32],
    input_meta: &XudtMeta,
    delta: u128,
) -> Result<(), Error> {
    meta::require_authority(input_meta.mint_authority.as_ref())?;
    if meta::is_supply_tracked(input_meta) {
        validate_supply_delta(meta_type_hash, delta, false)?;
    } else if input_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }
    access::validate_if_enabled(meta_type_hash, input_meta, CheckedLocks::InputsAndOutputs)?;
    extensions::run_extensions(Operation::ProtocolBurn, &input_meta.extensions, None)
}

fn validate_supply_delta(meta_type_hash: &[u8; 32], delta: u128, mint: bool) -> Result<(), Error> {
    let input_meta =
        meta::find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(Error::MetaInputMissing)?;
    let output_meta = meta::find_meta_in_source(meta_type_hash, Source::Output)?
        .ok_or(Error::MetaOutputMissing)?;

    if meta::is_supply_tracked(&input_meta) {
        let expected = if mint {
            input_meta
                .current_supply
                .checked_add(delta)
                .ok_or(Error::SupplyOverflow)?
        } else {
            input_meta
                .current_supply
                .checked_sub(delta)
                .ok_or(Error::SupplyUnderflow)?
        };
        if output_meta.current_supply != expected || supply_mode_changed(&input_meta, &output_meta)
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if input_meta.current_supply != 0 || output_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn supply_mode_changed(input_meta: &XudtMeta, output_meta: &XudtMeta) -> bool {
    meta::is_supply_tracked(input_meta) != meta::is_supply_tracked(output_meta)
}
