use alloc::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "no-std"))]
use ckb_std::ckb_types::{packed::Script, prelude::*};
#[cfg(feature = "std")]
use ckb_types::{packed::Script, prelude::*};
use molecule::prelude::{Builder, Entity};

use crate::error::Error;
use crate::generated;

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
pub const MAX_ACCESSLIST_ENTRIES: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptLocation {
    InputLock,
    InputType,
    OutputType,
    DynamicLinking,
    Spawn,
}

impl TryFrom<u8> for ScriptLocation {
    type Error = Error;

    fn try_from(location: u8) -> Result<Self, Self::Error> {
        match location {
            0 => Ok(Self::InputLock),
            1 => Ok(Self::InputType),
            2 => Ok(Self::OutputType),
            3 => Ok(Self::DynamicLinking),
            4 => Ok(Self::Spawn),
            _ => Err(Error::InvalidScriptLocation),
        }
    }
}

impl From<ScriptLocation> for u8 {
    fn from(location: ScriptLocation) -> Self {
        match location {
            ScriptLocation::InputLock => 0,
            ScriptLocation::InputType => 1,
            ScriptLocation::OutputType => 2,
            ScriptLocation::DynamicLinking => 3,
            ScriptLocation::Spawn => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptAttr {
    pub location: ScriptLocation,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub extra_data: Vec<u8>,
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
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
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
    pub access_authority: Option<ScriptAttr>,
    pub extensions: Vec<ScriptAttr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessListRange {
    pub start: [u8; 32],
    pub end: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessListShard {
    pub range: AccessListRange,
    pub entries: Vec<[u8; 32]>,
}

pub fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

pub fn validate_sudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    validate_config_flags(config_flags, SUDT_ALLOWED_CONFIG_MASK)?;
    validate_supply(config_flags, current_supply)
}

pub fn validate_xudt_config(config_flags: u8, current_supply: u128) -> Result<(), Error> {
    validate_config_flags(config_flags, XUDT_ALLOWED_CONFIG_MASK)?;
    if config_flags & CONFIG_ACCESS_WHITELIST != 0 && config_flags & CONFIG_ACCESS_ENABLED == 0 {
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

impl ScriptAttr {
    pub fn validate(&self) -> Result<(), Error> {
        match self.location {
            ScriptLocation::InputLock | ScriptLocation::InputType | ScriptLocation::OutputType => {
                if self.script.is_some() {
                    return Err(Error::InvalidScriptShape);
                }
            }
            ScriptLocation::DynamicLinking | ScriptLocation::Spawn => {
                let script = self.script.as_ref().ok_or(Error::InvalidScriptShape)?;
                let script_hash: [u8; 32] = script.calc_script_hash().unpack();
                if script_hash != self.script_hash {
                    return Err(Error::InvalidScriptHash);
                }
            }
        }
        Ok(())
    }
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
            .mint_authority(pack_script_attr_opt(&self.mint_authority)?)
            .metadata_authority(pack_script_attr_opt(&self.metadata_authority)?)
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
            mint_authority: unpack_script_attr_opt(raw.mint_authority())?,
            metadata_authority: unpack_script_attr_opt(raw.metadata_authority())?,
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
            .mint_authority(pack_script_attr_opt(&self.mint_authority)?)
            .metadata_authority(pack_script_attr_opt(&self.metadata_authority)?)
            .access_authority(pack_script_attr_opt(&self.access_authority)?)
            .extensions(pack_script_attr_vec(&self.extensions)?)
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
            mint_authority: unpack_script_attr_opt(raw.mint_authority())?,
            metadata_authority: unpack_script_attr_opt(raw.metadata_authority())?,
            access_authority: unpack_script_attr_opt(raw.access_authority())?,
            extensions: unpack_script_attr_vec(raw.extensions())?,
        };
        validate_xudt_config(meta.config_flags, meta.current_supply)?;
        Ok(meta)
    }
}

impl AccessListShard {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        Self::try_from(data)
    }

    pub fn to_molecule(&self) -> Result<generated::metadata::AccessListShard, Error> {
        if self.entries.len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }

        let mut entries = generated::blockchain::Byte32Vec::new_builder();
        for entry in &self.entries {
            entries = entries.push(generated::blockchain::Byte32::from_slice(entry)?);
        }

        Ok(generated::metadata::AccessListShard::new_builder()
            .range(
                generated::metadata::AccessListRange::new_builder()
                    .start(generated::blockchain::Byte32::from_slice(
                        &self.range.start,
                    )?)
                    .end(generated::blockchain::Byte32::from_slice(&self.range.end)?)
                    .build(),
            )
            .entries(entries.build())
            .build())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_molecule()?.as_slice().to_vec())
    }
}

impl TryFrom<&[u8]> for AccessListShard {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let raw = generated::metadata::AccessListShard::from_slice(data)?;
        if raw.entries().len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }

        Ok(Self {
            range: AccessListRange {
                start: raw.range().start().into(),
                end: raw.range().end().into(),
            },
            entries: raw.entries().into_iter().map(Into::into).collect(),
        })
    }
}

fn unpack_script_attr_opt(
    opt: generated::metadata::ScriptAttrOpt,
) -> Result<Option<ScriptAttr>, Error> {
    opt.to_opt().map(unpack_script_attr).transpose()
}

fn unpack_script_attr(attr: generated::metadata::ScriptAttr) -> Result<ScriptAttr, Error> {
    let parsed = ScriptAttr {
        location: ScriptLocation::try_from(u8::from(attr.location()))?,
        script_hash: attr.script_hash().into(),
        script: attr
            .script()
            .to_opt()
            .map(|script| Script::from_slice(script.as_slice()))
            .transpose()?,
    };
    parsed.validate()?;
    Ok(parsed)
}

fn pack_script_attr_opt(
    attr: &Option<ScriptAttr>,
) -> Result<generated::metadata::ScriptAttrOpt, Error> {
    Ok(generated::metadata::ScriptAttrOpt::new_builder()
        .set(match attr {
            Some(attr) => Some(pack_script_attr(attr)?),
            None => None,
        })
        .build())
}

fn pack_script_attr(attr: &ScriptAttr) -> Result<generated::metadata::ScriptAttr, Error> {
    attr.validate()?;
    let script_opt = generated::blockchain::ScriptOpt::new_builder()
        .set(
            attr.script
                .as_ref()
                .map(|script| generated::blockchain::Script::from_slice(script.as_slice()))
                .transpose()?,
        )
        .build();

    Ok(generated::metadata::ScriptAttr::new_builder()
        .location(u8::from(attr.location).into())
        .script_hash(generated::blockchain::Byte32::from_slice(
            &attr.script_hash,
        )?)
        .script(script_opt)
        .build())
}

fn unpack_script_attr_vec(
    raw: generated::metadata::ScriptAttrVec,
) -> Result<Vec<ScriptAttr>, Error> {
    if raw.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let extensions = raw
        .into_iter()
        .map(unpack_script_attr)
        .collect::<Result<Vec<_>, Error>>()?;
    validate_extensions(&extensions)?;
    Ok(extensions)
}

fn pack_script_attr_vec(
    extensions: &[ScriptAttr],
) -> Result<generated::metadata::ScriptAttrVec, Error> {
    validate_extensions(extensions)?;
    let mut builder = generated::metadata::ScriptAttrVec::new_builder();
    for attr in extensions {
        builder = builder.push(pack_script_attr(attr)?);
    }
    Ok(builder.build())
}

fn validate_extensions(extensions: &[ScriptAttr]) -> Result<(), Error> {
    if extensions.len() > MAX_EXTENSIONS {
        return Err(Error::ExtensionsTooMany);
    }

    let mut prev_key: Option<(u8, [u8; 32])> = None;
    for extension in extensions {
        extension.validate()?;
        let key = (u8::from(extension.location), extension.script_hash);
        if let Some(prev) = prev_key {
            if key == prev {
                return Err(Error::ExtensionsDuplicated);
            }
            if key < prev {
                return Err(Error::ExtensionsNotSorted);
            }
        }
        prev_key = Some(key);
    }
    Ok(())
}

fn pack_bytes(data: &[u8]) -> generated::blockchain::Bytes {
    data.iter().copied().collect()
}

fn unpack_bytes(raw: generated::blockchain::Bytes) -> Vec<u8> {
    raw.raw_data().to_vec()
}

fn unpack_limited_bytes(
    raw: generated::blockchain::Bytes,
    max_len: usize,
) -> Result<Vec<u8>, Error> {
    let data = unpack_bytes(raw);
    if data.len() > max_len {
        return Err(Error::MetadataTooLarge);
    }
    Ok(data)
}

fn validate_metadata_sizes(
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

fn pack_u128(value: u128) -> generated::metadata::Uint128 {
    generated::metadata::Uint128::from(value.to_le_bytes())
}

fn unpack_u128(raw: generated::metadata::Uint128) -> u128 {
    u128::from_le_bytes(raw.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::vec;
    #[cfg(all(not(feature = "std"), feature = "no-std"))]
    use ckb_std::ckb_types::{bytes::Bytes, core::ScriptHashType, packed::Byte32};
    #[cfg(feature = "std")]
    use ckb_types::{bytes::Bytes, core::ScriptHashType, packed::Byte32};

    fn empty_sudt(config_flags: u8, current_supply: u128) -> SudtMeta {
        SudtMeta {
            config_flags,
            current_supply,
            decimals: 8,
            name: Vec::new(),
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data: Vec::new(),
            mint_authority: None,
            metadata_authority: None,
        }
    }

    fn empty_xudt(config_flags: u8, current_supply: u128) -> XudtMeta {
        XudtMeta {
            config_flags,
            current_supply,
            decimals: 8,
            name: Vec::new(),
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data: Vec::new(),
            mint_authority: None,
            metadata_authority: None,
            access_authority: None,
            extensions: Vec::new(),
        }
    }

    fn append_empty_table_field(raw: &[u8]) -> Vec<u8> {
        let old_total = u32::from_le_bytes(raw[0..4].try_into().expect("total size")) as usize;
        let field_count =
            (u32::from_le_bytes(raw[4..8].try_into().expect("first offset")) as usize / 4) - 1;
        let new_total = old_total + 4;

        let mut extended = Vec::with_capacity(new_total);
        extended.extend_from_slice(&(new_total as u32).to_le_bytes());
        for index in 0..field_count {
            let start = 4 + index * 4;
            let offset = u32::from_le_bytes(raw[start..start + 4].try_into().expect("offset"));
            extended.extend_from_slice(&(offset + 4).to_le_bytes());
        }
        extended.extend_from_slice(&(new_total as u32).to_le_bytes());
        extended.extend_from_slice(&raw[4 + field_count * 4..]);
        extended
    }

    fn build_script(tag: u8) -> Script {
        Script::new_builder()
            .code_hash(Byte32::from_slice(&[tag; 32]).expect("byte32"))
            .hash_type(ScriptHashType::Data.into())
            .args(Bytes::from(vec![tag; 4]).pack())
            .build()
    }

    fn sorted_attr(location: ScriptLocation, tag: u8) -> ScriptAttr {
        ScriptAttr {
            location,
            script_hash: [tag; 32],
            script: None,
        }
    }

    #[test]
    fn sudt_rejects_xudt_config_bits() {
        let meta = empty_sudt(CONFIG_ACCESS_ENABLED, 0);

        assert!(matches!(meta.to_bytes(), Err(Error::InvalidConfigFlags)));
        assert!(matches!(
            validate_sudt_config(CONFIG_ACCESS_ENABLED, 0),
            Err(Error::InvalidConfigFlags)
        ));
    }

    #[test]
    fn sudt_rejects_paused_config_bit() {
        let meta = empty_sudt(CONFIG_PAUSED, 0);

        assert!(matches!(meta.to_bytes(), Err(Error::InvalidConfigFlags)));
        assert!(matches!(
            validate_sudt_config(CONFIG_PAUSED, 0),
            Err(Error::InvalidConfigFlags)
        ));
    }

    #[test]
    fn untracked_requires_zero_supply() {
        let meta = empty_sudt(0, 1);

        assert!(!is_supply_tracked(meta.config_flags));
        assert!(matches!(meta.to_bytes(), Err(Error::InvalidSupply)));
    }

    #[test]
    fn xudt_rejects_access_mode_when_access_disabled() {
        let meta = empty_xudt(CONFIG_ACCESS_WHITELIST, 0);

        assert!(matches!(meta.to_bytes(), Err(Error::InvalidConfigFlags)));
        assert!(matches!(
            validate_xudt_config(CONFIG_ACCESS_WHITELIST, 0),
            Err(Error::InvalidConfigFlags)
        ));
    }

    #[test]
    fn script_attr_enforces_location_shape_and_hash() {
        let script = build_script(0x22);
        let script_hash: [u8; 32] = script.calc_script_hash().unpack();

        let forbidden_script = ScriptAttr {
            location: ScriptLocation::InputLock,
            script_hash,
            script: Some(script.clone()),
        };
        assert!(matches!(
            forbidden_script.validate(),
            Err(Error::InvalidScriptShape)
        ));

        let missing_script = ScriptAttr {
            location: ScriptLocation::Spawn,
            script_hash,
            script: None,
        };
        assert!(matches!(
            missing_script.validate(),
            Err(Error::InvalidScriptShape)
        ));

        let wrong_hash = ScriptAttr {
            location: ScriptLocation::DynamicLinking,
            script_hash: [0u8; 32],
            script: Some(script),
        };
        assert!(matches!(
            wrong_hash.validate(),
            Err(Error::InvalidScriptHash)
        ));
    }

    #[test]
    fn xudt_rejects_unsorted_and_duplicate_extensions() {
        let mut unsorted = empty_xudt(0, 0);
        unsorted.extensions = vec![
            sorted_attr(ScriptLocation::InputType, 2),
            sorted_attr(ScriptLocation::InputLock, 1),
        ];
        assert!(matches!(
            unsorted.to_bytes(),
            Err(Error::ExtensionsNotSorted)
        ));

        let mut duplicated = empty_xudt(0, 0);
        duplicated.extensions = vec![
            sorted_attr(ScriptLocation::InputLock, 1),
            sorted_attr(ScriptLocation::InputLock, 1),
        ];
        assert!(matches!(
            duplicated.to_bytes(),
            Err(Error::ExtensionsDuplicated)
        ));
    }

    #[test]
    fn metadata_round_trips_and_uses_strict_decoding() {
        let mut sudt = empty_sudt(CONFIG_SUPPLY_TRACKED, 42);
        sudt.name = b"Example".to_vec();
        sudt.symbol = b"EX".to_vec();

        let encoded = sudt.to_bytes().expect("encode sudt");
        assert_eq!(SudtMeta::from_slice(&encoded).expect("decode sudt"), sudt);
        assert!(matches!(
            SudtMeta::from_slice(&append_empty_table_field(&encoded)),
            Err(Error::Molecule)
        ));

        let xudt = empty_xudt(CONFIG_ACCESS_ENABLED, 0);
        let encoded = xudt.to_bytes().expect("encode xudt");
        assert_eq!(XudtMeta::from_slice(&encoded).expect("decode xudt"), xudt);
        assert!(matches!(
            XudtMeta::from_slice(&append_empty_table_field(&encoded)),
            Err(Error::Molecule)
        ));

        let shard = AccessListShard {
            range: AccessListRange {
                start: [0u8; 32],
                end: [0xffu8; 32],
            },
            entries: vec![[1u8; 32]],
        };
        let encoded = shard.to_bytes().expect("encode shard");
        assert_eq!(
            AccessListShard::from_slice(&encoded).expect("decode shard"),
            shard
        );
        assert!(matches!(
            AccessListShard::from_slice(&append_empty_table_field(&encoded)),
            Err(Error::Molecule)
        ));
    }

    #[test]
    fn metadata_rejects_byte_fields_over_limit() {
        let mut sudt = empty_sudt(0, 0);
        sudt.name = vec![0; MAX_METADATA_NAME_BYTES + 1];

        assert!(matches!(sudt.to_bytes(), Err(Error::MetadataTooLarge)));

        let raw = generated::metadata::SudtMeta::new_builder()
            .config_flags(0u8.into())
            .current_supply(pack_u128(0))
            .decimals(8u8.into())
            .name(pack_bytes(&vec![0; MAX_METADATA_NAME_BYTES + 1]))
            .symbol(generated::blockchain::Bytes::default())
            .uri(generated::blockchain::Bytes::default())
            .extra_data(generated::blockchain::Bytes::default())
            .mint_authority(
                generated::metadata::ScriptAttrOpt::new_builder()
                    .set(None)
                    .build(),
            )
            .metadata_authority(
                generated::metadata::ScriptAttrOpt::new_builder()
                    .set(None)
                    .build(),
            )
            .build();

        assert!(matches!(
            SudtMeta::from_slice(raw.as_slice()),
            Err(Error::MetadataTooLarge)
        ));
    }

    #[test]
    fn access_list_rejects_too_many_entries() {
        let shard = AccessListShard {
            range: AccessListRange {
                start: [0u8; 32],
                end: [0xffu8; 32],
            },
            entries: vec![[0u8; 32]; MAX_ACCESSLIST_ENTRIES + 1],
        };

        assert!(matches!(shard.to_bytes(), Err(Error::AccessListTooLarge)));
    }
}
