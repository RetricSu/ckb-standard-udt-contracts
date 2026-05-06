use crate::{
    error::Error,
    meta::{check_authority, load_meta_context, load_meta_type_hash_arg},
    mode::AccessMode,
    shards::{collect_group_shards, validate_shards_for_mode},
};

pub fn main() -> Result<(), Error> {
    let meta_type_hash = load_meta_type_hash_arg()?;
    let meta_context = load_meta_context(&meta_type_hash)?;
    let input_shards = collect_group_shards(ckb_std::ckb_constants::Source::GroupInput)?;
    let output_shards = collect_group_shards(ckb_std::ckb_constants::Source::GroupOutput)?;

    let mode = AccessMode::from_flags(meta_context.output_config_flags)?;
    validate_shards_for_mode(mode, &input_shards, &output_shards)?;

    if input_shards == output_shards {
        return Ok(());
    }

    // Disabled mode permits reclaiming stale AccessList cells without requiring
    // a now-irrelevant access authority, but it must not create replacement cells.
    if mode == AccessMode::Disabled && output_shards.is_empty() {
        return Ok(());
    }

    let authority = meta_context
        .access_authority
        .as_ref()
        .ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
    }
}
