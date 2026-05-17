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
        let current_meta = meta::find_current_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        validate_transfer(&meta_type_hash, &current_meta.meta)
    } else if output_amount > input_amount {
        match meta::find_current_meta(&meta_type_hash)? {
            Some(current_meta) => validate_mint(&meta_type_hash, &current_meta),
            None => require_initial_mint_output_meta(&meta_type_hash),
        }
    } else {
        match meta::find_meta_in_source(&meta_type_hash, Source::Input)? {
            Some(input_meta) if meta::is_supply_tracked(&input_meta) => {
                validate_protocol_burn(&meta_type_hash, &input_meta)
            }
            Some(input_meta) if input_meta.current_supply != 0 => Err(Error::MetaStateMismatch),
            Some(input_meta) if output_amount == 0 => Ok(()),
            Some(input_meta) => validate_transfer(&meta_type_hash, &input_meta),
            None if output_amount == 0 => Ok(()),
            None => {
                let current_meta = meta::find_meta_in_source(&meta_type_hash, Source::CellDep)?
                    .ok_or(Error::MetaMissing)?;
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

fn validate_mint(meta_type_hash: &[u8; 32], current_meta: &meta::CurrentMeta) -> Result<(), Error> {
    let current_meta_data = &current_meta.meta;
    if meta::is_paused(current_meta_data) {
        return Err(Error::MetaStateMismatch);
    }
    if meta::is_supply_tracked(current_meta_data) {
        if current_meta.source != Source::Input {
            return Err(Error::MetaInputMissing);
        }
    } else {
        meta::require_authority(current_meta_data.mint_authority.as_ref())?;
        if current_meta_data.current_supply != 0 {
            return Err(Error::MetaStateMismatch);
        }
    }
    access::validate_if_enabled(meta_type_hash, current_meta_data, CheckedLocks::Outputs)?;
    extensions::run_extensions(Operation::Mint, &current_meta_data.extensions, Some(true))
}

fn require_initial_mint_output_meta(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    meta::find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;
    Ok(())
}

fn validate_protocol_burn(meta_type_hash: &[u8; 32], input_meta: &XudtMeta) -> Result<(), Error> {
    access::validate_if_enabled(meta_type_hash, input_meta, CheckedLocks::InputsAndOutputs)?;
    extensions::run_extensions(Operation::ProtocolBurn, &input_meta.extensions, None)
}
