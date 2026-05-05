use ckb_testtool::ckb_types::{bytes::Bytes, packed::Script, prelude::*};
use standard_udt_types::metadata::{
    AccessListRange, AccessListShardV1, ScriptAttr, ScriptLocation, SudtMetaV1, XudtMetaV1,
};

pub const MAX_EXTENSIONS: usize = 16;

pub fn input_lock_authority(script_hash: [u8; 32]) -> ScriptAttr {
    ScriptAttr {
        location: ScriptLocation::InputLock,
        script_hash,
        script: None,
    }
}

pub fn build_sudt_meta_v1_bytes(
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
) -> Bytes {
    let metadata = SudtMetaV1 {
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        mint_authority,
        metadata_authority,
    };
    Bytes::from(
        metadata
            .to_bytes()
            .expect("build SudtMetaV1 bytes should not fail"),
    )
}

pub fn build_xudt_meta_v1_bytes(
    flag: u8,
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
    access_authority: Option<ScriptAttr>,
) -> Bytes {
    let metadata = XudtMetaV1 {
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        flag,
        mint_authority,
        metadata_authority,
        access_authority,
        extensions: Vec::new(),
    };
    Bytes::from(
        metadata
            .to_bytes()
            .expect("build XudtMetaV1 bytes should not fail"),
    )
}

pub fn build_xudt_meta_v1_with_extensions_bytes(
    flag: u8,
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
    access_authority: Option<ScriptAttr>,
    extensions: Vec<ScriptAttr>,
) -> Bytes {
    let metadata = XudtMetaV1 {
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        flag,
        mint_authority,
        metadata_authority,
        access_authority,
        extensions,
    };
    Bytes::from(
        metadata
            .to_bytes()
            .expect("build XudtMetaV1 bytes should not fail"),
    )
}

pub fn build_access_list_shard_v1_bytes(
    start: [u8; 32],
    end: [u8; 32],
    entries: Vec<[u8; 32]>,
) -> Bytes {
    let shard = AccessListShardV1 {
        range: AccessListRange { start, end },
        entries,
    };
    Bytes::from(
        shard
            .to_bytes()
            .expect("build AccessListShardV1 bytes should not fail"),
    )
}

pub fn udt_amount_bytes(amount: u128) -> Bytes {
    Bytes::from(amount.to_le_bytes().to_vec())
}

pub fn script_hash(script: &Script) -> [u8; 32] {
    script.calc_script_hash().unpack()
}

fn raw_script_attr(
    location: u8,
    script_hash: [u8; 32],
    script: Option<&Script>,
) -> standard_udt_types::generated::metadata::ScriptAttr {
    let script_opt = standard_udt_types::generated::blockchain::ScriptOpt::new_builder()
        .set(script.map(|value| {
            standard_udt_types::generated::blockchain::Script::from_slice(value.as_slice())
                .expect("script to molecule")
        }))
        .build();

    standard_udt_types::generated::metadata::ScriptAttr::new_builder()
        .location(location.into())
        .script_hash(
            standard_udt_types::generated::blockchain::Byte32::from_slice(&script_hash)
                .expect("script hash to molecule"),
        )
        .script(script_opt)
        .build()
}

pub fn raw_script_attr_opt(
    location: u8,
    script_hash: [u8; 32],
    script: Option<&Script>,
) -> standard_udt_types::generated::metadata::ScriptAttrOpt {
    standard_udt_types::generated::metadata::ScriptAttrOpt::new_builder()
        .set(Some(raw_script_attr(location, script_hash, script)))
        .build()
}

pub fn raw_none_script_attr_opt() -> standard_udt_types::generated::metadata::ScriptAttrOpt {
    standard_udt_types::generated::metadata::ScriptAttrOpt::new_builder()
        .set(None)
        .build()
}

pub fn build_raw_sudt_meta_v1_bytes(
    mint_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    metadata_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
) -> Bytes {
    let metadata = standard_udt_types::generated::metadata::SudtMetaV1::new_builder()
        .decimals(0u8.into())
        .name(standard_udt_types::generated::blockchain::Bytes::default())
        .symbol(standard_udt_types::generated::blockchain::Bytes::default())
        .uri(standard_udt_types::generated::blockchain::Bytes::default())
        .extra_data(standard_udt_types::generated::blockchain::Bytes::default())
        .mint_authority(mint_authority)
        .metadata_authority(metadata_authority)
        .build();
    Bytes::from(metadata.as_slice().to_vec())
}

pub fn build_raw_ordered_extensions(
    count: usize,
) -> standard_udt_types::generated::metadata::ScriptAttrVec {
    let attrs = (0..count)
        .map(|index| {
            let tag = u8::try_from(
                index
                    .checked_add(1)
                    .expect("ordered extension tag index overflow"),
            )
            .expect("ordered extension tag exceeds u8");
            raw_script_attr(0, [tag; 32], None)
        })
        .collect();
    build_raw_extensions(attrs)
}

pub fn build_raw_unordered_extensions() -> standard_udt_types::generated::metadata::ScriptAttrVec {
    build_raw_extensions(vec![
        raw_script_attr(0, [2u8; 32], None),
        raw_script_attr(0, [1u8; 32], None),
    ])
}

pub fn build_raw_duplicate_extensions() -> standard_udt_types::generated::metadata::ScriptAttrVec {
    build_raw_extensions(vec![
        raw_script_attr(0, [1u8; 32], None),
        raw_script_attr(0, [1u8; 32], None),
    ])
}

pub fn build_raw_over_limit_extensions() -> standard_udt_types::generated::metadata::ScriptAttrVec {
    build_raw_ordered_extensions(MAX_EXTENSIONS + 1)
}

pub fn build_raw_extensions(
    attrs: Vec<standard_udt_types::generated::metadata::ScriptAttr>,
) -> standard_udt_types::generated::metadata::ScriptAttrVec {
    attrs
        .into_iter()
        .fold(
            standard_udt_types::generated::metadata::ScriptAttrVec::new_builder(),
            |builder, attr| builder.push(attr),
        )
        .build()
}

pub fn build_raw_xudt_meta_v1_bytes(
    flag: u8,
    mint_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    metadata_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    access_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
) -> Bytes {
    build_raw_xudt_meta_v1_with_extensions_bytes(
        flag,
        mint_authority,
        metadata_authority,
        access_authority,
        standard_udt_types::generated::metadata::ScriptAttrVec::default(),
    )
}

pub fn build_raw_xudt_meta_v1_with_extensions_bytes(
    flag: u8,
    mint_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    metadata_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    access_authority: standard_udt_types::generated::metadata::ScriptAttrOpt,
    extensions: standard_udt_types::generated::metadata::ScriptAttrVec,
) -> Bytes {
    let metadata = standard_udt_types::generated::metadata::XudtMetaV1::new_builder()
        .decimals(0u8.into())
        .name(standard_udt_types::generated::blockchain::Bytes::default())
        .symbol(standard_udt_types::generated::blockchain::Bytes::default())
        .uri(standard_udt_types::generated::blockchain::Bytes::default())
        .extra_data(standard_udt_types::generated::blockchain::Bytes::default())
        .flag(flag.into())
        .mint_authority(mint_authority)
        .metadata_authority(metadata_authority)
        .access_authority(access_authority)
        .extensions(extensions)
        .build();
    Bytes::from(metadata.as_slice().to_vec())
}
