use alloc::vec::Vec;

use crate::molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

use super::{
    authority::{pack_authority_opt, unpack_authority_opt, Authority},
    codec::{pack_bytes, pack_u128, unpack_limited_bytes, unpack_u128, validate_metadata_sizes},
    config::{
        validate_sudt_config, validate_xudt_config, MAX_METADATA_EXTRA_DATA_BYTES,
        MAX_METADATA_NAME_BYTES, MAX_METADATA_SYMBOL_BYTES, MAX_METADATA_URI_BYTES,
    },
    extension::{pack_extension_vec, unpack_extension_vec, validate_extensions, Extension},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub extra_data: Vec<u8>,
    pub mint_authority: Option<Authority>,
    pub metadata_authority: Option<Authority>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub extra_data: Vec<u8>,
    pub mint_authority: Option<Authority>,
    pub metadata_authority: Option<Authority>,
    pub access_authority: Option<Authority>,
    pub extensions: Vec<Extension>,
}

impl SudtMeta {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        Self::try_from(data)
    }

    pub fn to_molecule(&self) -> Result<generated::metadata::SudtMeta, Error> {
        validate_sudt_config(self.config_flags, self.current_supply)?;
        validate_metadata_sizes(&self.name, &self.symbol, &self.uri, &self.extra_data)?;
        Ok(generated::metadata::SudtMeta::new_builder()
            .config_flags(self.config_flags.into())
            .current_supply(pack_u128(self.current_supply))
            .decimals(self.decimals.into())
            .name(pack_bytes(&self.name))
            .symbol(pack_bytes(&self.symbol))
            .uri(pack_bytes(&self.uri))
            .extra_data(pack_bytes(&self.extra_data))
            .mint_authority(pack_authority_opt(&self.mint_authority)?)
            .metadata_authority(pack_authority_opt(&self.metadata_authority)?)
            .build())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_molecule()?.as_slice().to_vec())
    }
}

impl TryFrom<&[u8]> for SudtMeta {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let raw = generated::metadata::SudtMeta::from_slice(data)?;
        let meta = Self {
            config_flags: raw.config_flags().into(),
            current_supply: unpack_u128(raw.current_supply()),
            decimals: raw.decimals().into(),
            name: unpack_limited_bytes(raw.name(), MAX_METADATA_NAME_BYTES)?,
            symbol: unpack_limited_bytes(raw.symbol(), MAX_METADATA_SYMBOL_BYTES)?,
            uri: unpack_limited_bytes(raw.uri(), MAX_METADATA_URI_BYTES)?,
            extra_data: unpack_limited_bytes(raw.extra_data(), MAX_METADATA_EXTRA_DATA_BYTES)?,
            mint_authority: unpack_authority_opt(raw.mint_authority())?,
            metadata_authority: unpack_authority_opt(raw.metadata_authority())?,
        };
        validate_sudt_config(meta.config_flags, meta.current_supply)?;
        Ok(meta)
    }
}

impl XudtMeta {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        Self::try_from(data)
    }

    pub fn to_molecule(&self) -> Result<generated::metadata::XudtMeta, Error> {
        validate_xudt_config(self.config_flags, self.current_supply)?;
        validate_metadata_sizes(&self.name, &self.symbol, &self.uri, &self.extra_data)?;
        validate_extensions(&self.extensions)?;
        Ok(generated::metadata::XudtMeta::new_builder()
            .config_flags(self.config_flags.into())
            .current_supply(pack_u128(self.current_supply))
            .decimals(self.decimals.into())
            .name(pack_bytes(&self.name))
            .symbol(pack_bytes(&self.symbol))
            .uri(pack_bytes(&self.uri))
            .extra_data(pack_bytes(&self.extra_data))
            .mint_authority(pack_authority_opt(&self.mint_authority)?)
            .metadata_authority(pack_authority_opt(&self.metadata_authority)?)
            .access_authority(pack_authority_opt(&self.access_authority)?)
            .extensions(pack_extension_vec(&self.extensions)?)
            .build())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_molecule()?.as_slice().to_vec())
    }
}

impl TryFrom<&[u8]> for XudtMeta {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let raw = generated::metadata::XudtMeta::from_slice(data)?;
        let meta = Self {
            config_flags: raw.config_flags().into(),
            current_supply: unpack_u128(raw.current_supply()),
            decimals: raw.decimals().into(),
            name: unpack_limited_bytes(raw.name(), MAX_METADATA_NAME_BYTES)?,
            symbol: unpack_limited_bytes(raw.symbol(), MAX_METADATA_SYMBOL_BYTES)?,
            uri: unpack_limited_bytes(raw.uri(), MAX_METADATA_URI_BYTES)?,
            extra_data: unpack_limited_bytes(raw.extra_data(), MAX_METADATA_EXTRA_DATA_BYTES)?,
            mint_authority: unpack_authority_opt(raw.mint_authority())?,
            metadata_authority: unpack_authority_opt(raw.metadata_authority())?,
            access_authority: unpack_authority_opt(raw.access_authority())?,
            extensions: unpack_extension_vec(raw.extensions())?,
        };
        validate_xudt_config(meta.config_flags, meta.current_supply)?;
        Ok(meta)
    }
}
