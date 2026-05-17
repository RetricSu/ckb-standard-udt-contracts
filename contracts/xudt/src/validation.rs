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

pub(crate) fn validate_negative_delta(
    meta_type_hash: &[u8; 32],
    output_amount: u128,
) -> Result<(), Error> {
    match meta::find_meta_in_source(meta_type_hash, Source::Input)? {
        Some(input_meta) if meta::is_supply_tracked(&input_meta) => {
            validate_protocol_burn(meta_type_hash, &input_meta)
        }
        Some(_) if output_amount == 0 => Ok(()),
        Some(input_meta) => validate_transfer(meta_type_hash, &input_meta),
        None if output_amount == 0 => Ok(()),
        None => {
            let current_meta = meta::find_meta_in_source(meta_type_hash, Source::CellDep)?
                .ok_or(Error::MetaMissing)?;
            validate_transfer(meta_type_hash, &current_meta)
        }
    }
}

pub(crate) fn validate_transfer(
    meta_type_hash: &[u8; 32],
    current_meta: &XudtMeta,
) -> Result<(), Error> {
    if meta::is_paused(current_meta) {
        return Err(Error::MetaStateMismatch);
    }
    access::validate_if_enabled(meta_type_hash, current_meta, CheckedLocks::InputsAndOutputs)?;
    extensions::run_extensions(Operation::Transfer, &current_meta.extensions, None)
}

pub(crate) fn validate_mint(
    meta_type_hash: &[u8; 32],
    current_meta: &meta::CurrentMeta,
) -> Result<(), Error> {
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
    }
    access::validate_if_enabled(meta_type_hash, current_meta_data, CheckedLocks::Outputs)?;
    extensions::run_extensions(Operation::Mint, &current_meta_data.extensions, Some(true))
}

pub(crate) fn require_initial_mint_output_meta(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    meta::find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;
    Ok(())
}

fn validate_protocol_burn(meta_type_hash: &[u8; 32], input_meta: &XudtMeta) -> Result<(), Error> {
    access::validate_if_enabled(meta_type_hash, input_meta, CheckedLocks::InputsAndOutputs)?;
    extensions::run_extensions(Operation::ProtocolBurn, &input_meta.extensions, None)
}
