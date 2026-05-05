use ckb_std::ckb_constants::Source;

use crate::{
    access,
    error::Error,
    extensions,
    meta::{self, XudtMeta},
};

#[derive(Clone, Copy)]
pub enum Operation {
    Transfer,
    Mint,
    ProtocolBurn,
}

impl Operation {
    pub const fn code(self) -> u8 {
        match self {
            Self::Transfer => 0,
            Self::Mint => 1,
            Self::ProtocolBurn => 2,
        }
    }
}

pub fn run() -> Result<(), Error> {
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(Source::GroupOutput)?;

    if input_amount == output_amount {
        let current_meta =
            meta::find_unique_visible_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        validate_transfer(&meta_type_hash, &current_meta)
    } else if output_amount > input_amount {
        let current_meta =
            meta::find_unique_visible_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        let delta = output_amount
            .checked_sub(input_amount)
            .ok_or(Error::AmountOverflow)?;
        validate_mint(&meta_type_hash, &current_meta, delta)
    } else {
        let delta = input_amount
            .checked_sub(output_amount)
            .ok_or(Error::AmountOverflow)?;
        let Some(input_meta) = meta::find_meta_in_source(&meta_type_hash, Source::Input)? else {
            return Ok(());
        };
        let visible_meta =
            meta::find_unique_visible_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        if visible_meta != input_meta {
            return Err(Error::MetaNotUnique);
        }
        validate_protocol_burn(&meta_type_hash, &input_meta, delta)
    }
}

fn validate_transfer(meta_type_hash: &[u8; 32], current_meta: &XudtMeta) -> Result<(), Error> {
    if current_meta.is_paused() {
        return Err(Error::MetaStateMismatch);
    }
    access::validate_if_enabled(meta_type_hash, current_meta)?;
    extensions::run_extensions(Operation::Transfer, &current_meta.extensions, false)
}

fn validate_mint(
    meta_type_hash: &[u8; 32],
    current_meta: &XudtMeta,
    delta: u128,
) -> Result<(), Error> {
    if current_meta.is_paused() {
        return Err(Error::MetaStateMismatch);
    }
    meta::require_authority(current_meta.mint_authority.as_ref())?;
    validate_supply_delta(meta_type_hash, delta, true)?;
    extensions::run_extensions(Operation::Mint, &current_meta.extensions, true)
}

fn validate_protocol_burn(
    meta_type_hash: &[u8; 32],
    input_meta: &XudtMeta,
    delta: u128,
) -> Result<(), Error> {
    meta::require_authority(input_meta.mint_authority.as_ref())?;
    validate_supply_delta(meta_type_hash, delta, false)?;
    access::validate_if_enabled(meta_type_hash, input_meta)?;
    extensions::run_extensions(Operation::ProtocolBurn, &input_meta.extensions, true)
}

fn validate_supply_delta(meta_type_hash: &[u8; 32], delta: u128, mint: bool) -> Result<(), Error> {
    let input_meta =
        meta::find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(Error::MetaInputMissing)?;
    let output_meta = meta::find_meta_in_source(meta_type_hash, Source::Output)?
        .ok_or(Error::MetaOutputMissing)?;

    if input_meta.is_supply_tracked() {
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
        if output_meta.current_supply != expected
            || output_meta.config_flags != input_meta.config_flags
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if input_meta.current_supply != 0 || output_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}
