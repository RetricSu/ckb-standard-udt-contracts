use super::{codec::*, *};

use crate::molecule::prelude::{Builder, Entity};
use alloc::vec;
#[cfg(feature = "no-std")]
use ckb_std::ckb_types::{
    bytes::Bytes,
    core::ScriptHashType,
    packed::{Byte32, Script},
    prelude::*,
};
#[cfg(all(feature = "std", not(feature = "no-std")))]
use ckb_types::{
    bytes::Bytes,
    core::ScriptHashType,
    packed::{Byte32, Script},
    prelude::*,
};

use crate::{error::Error, generated};

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
        .hash_type(ScriptHashType::Data)
        .args(Bytes::from(vec![tag; 4]).pack())
        .build()
}

fn extension(extension_type: ExtensionType, tag: u8) -> Extension {
    Extension {
        extension_type,
        script: build_script(tag),
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
fn authority_enforces_type_shape_and_hash() {
    let script = build_script(0x22);
    let script_hash: [u8; 32] = script.calc_script_hash().unpack();

    let forbidden_script = Authority {
        authority_type: AuthorityType::InputLock,
        script_hash,
        script: Some(script.clone()),
    };
    assert!(matches!(
        forbidden_script.validate(),
        Err(Error::InvalidScriptShape)
    ));

    let missing_script = Authority {
        authority_type: AuthorityType::Spawn,
        script_hash,
        script: None,
    };
    assert!(matches!(
        missing_script.validate(),
        Err(Error::InvalidScriptShape)
    ));

    let wrong_hash = Authority {
        authority_type: AuthorityType::DynamicLinking,
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
        extension(ExtensionType::Spawn, 1),
        extension(ExtensionType::DynamicLinking, 2),
    ];
    assert!(matches!(
        unsorted.to_bytes(),
        Err(Error::ExtensionsNotSorted)
    ));

    let mut duplicated = empty_xudt(0, 0);
    duplicated.extensions = vec![
        extension(ExtensionType::DynamicLinking, 1),
        extension(ExtensionType::DynamicLinking, 1),
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
            generated::metadata::AuthorityOpt::new_builder()
                .set(None)
                .build(),
        )
        .metadata_authority(
            generated::metadata::AuthorityOpt::new_builder()
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

#[test]
fn access_list_rejects_invalid_ranges_and_entries() {
    let invalid_range = AccessListShard {
        range: AccessListRange {
            start: [0x20; 32],
            end: [0x10; 32],
        },
        entries: Vec::new(),
    };
    assert!(matches!(
        invalid_range.to_bytes(),
        Err(Error::AccessListInvalidRange)
    ));

    let misaligned_range = AccessListShard {
        range: AccessListRange {
            start: [1u8; 32],
            end: [0xffu8; 32],
        },
        entries: Vec::new(),
    };
    assert!(matches!(
        misaligned_range.to_bytes(),
        Err(Error::AccessListInvalidRange)
    ));

    let out_of_range = AccessListShard {
        range: AccessListRange {
            start: [0u8; 32],
            end: {
                let mut end = [0xffu8; 32];
                end[0] = 0x0f;
                end
            },
        },
        entries: vec![{
            let mut entry = [0u8; 32];
            entry[0] = 0x10;
            entry
        }],
    };
    assert!(matches!(
        out_of_range.to_bytes(),
        Err(Error::AccessListEntryOutOfRange)
    ));

    let unsorted = AccessListShard {
        range: AccessListRange {
            start: [0u8; 32],
            end: [0xffu8; 32],
        },
        entries: vec![[2u8; 32], [1u8; 32]],
    };
    assert!(matches!(
        unsorted.to_bytes(),
        Err(Error::AccessListEntriesNotSorted)
    ));

    let duplicated = AccessListShard {
        range: AccessListRange {
            start: [0u8; 32],
            end: [0xffu8; 32],
        },
        entries: vec![[1u8; 32], [1u8; 32]],
    };
    assert!(matches!(
        duplicated.to_bytes(),
        Err(Error::AccessListEntriesDuplicated)
    ));
}
