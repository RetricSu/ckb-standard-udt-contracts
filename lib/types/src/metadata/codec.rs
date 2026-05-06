use alloc::vec::Vec;

use crate::{error::Error, generated};

use super::config::{
    MAX_METADATA_EXTRA_DATA_BYTES, MAX_METADATA_NAME_BYTES, MAX_METADATA_SYMBOL_BYTES,
    MAX_METADATA_URI_BYTES,
};

pub(crate) fn pack_bytes(data: &[u8]) -> generated::blockchain::Bytes {
    data.iter().copied().collect()
}

pub(crate) fn unpack_bytes(raw: generated::blockchain::Bytes) -> Vec<u8> {
    raw.raw_data().to_vec()
}

pub(crate) fn unpack_limited_bytes(
    raw: generated::blockchain::Bytes,
    max_len: usize,
) -> Result<Vec<u8>, Error> {
    let data = unpack_bytes(raw);
    if data.len() > max_len {
        return Err(Error::MetadataTooLarge);
    }
    Ok(data)
}

pub(crate) fn validate_metadata_sizes(
    name: &[u8],
    symbol: &[u8],
    uri: &[u8],
    extra_data: &[u8],
) -> Result<(), Error> {
    if name.len() > MAX_METADATA_NAME_BYTES
        || symbol.len() > MAX_METADATA_SYMBOL_BYTES
        || uri.len() > MAX_METADATA_URI_BYTES
        || extra_data.len() > MAX_METADATA_EXTRA_DATA_BYTES
    {
        return Err(Error::MetadataTooLarge);
    }
    Ok(())
}

pub(crate) fn pack_u128(value: u128) -> generated::metadata::Uint128 {
    generated::metadata::Uint128::from(value.to_le_bytes())
}

pub(crate) fn unpack_u128(raw: generated::metadata::Uint128) -> u128 {
    u128::from_le_bytes(raw.into())
}
