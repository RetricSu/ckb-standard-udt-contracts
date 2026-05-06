use ckb_testtool::ckb_types::prelude::Entity as TesttoolEntity;
use ckb_testtool::ckb_types::{bytes::Bytes, packed::Script, prelude::*};
use ckb_types::{packed::Script as MetadataScript, prelude::Entity as CkbEntity};
use standard_udt_types::generated::{blockchain, metadata};
use standard_udt_types::metadata::{
    Authority, AuthorityType, Extension, ExtensionType, SudtMeta, XudtMeta,
};
use standard_udt_types::molecule::prelude::Builder;

pub struct DeployedScript {
    pub out_point: ckb_testtool::ckb_types::packed::OutPoint,
    pub script: Script,
    pub script_hash: [u8; 32],
}

pub fn input_lock_authority(script_hash: [u8; 32]) -> Authority {
    Authority {
        authority_type: AuthorityType::InputLock,
        script_hash,
        script: None,
    }
}

pub fn dynamic_linking_authority(deployed: &DeployedScript) -> Authority {
    Authority {
        authority_type: AuthorityType::DynamicLinking,
        script_hash: deployed.script_hash,
        script: Some(metadata_script(deployed)),
    }
}

pub fn spawn_authority(deployed: &DeployedScript) -> Authority {
    Authority {
        authority_type: AuthorityType::Spawn,
        script_hash: deployed.script_hash,
        script: Some(metadata_script(deployed)),
    }
}

pub fn dynamic_linking_extension(deployed: &DeployedScript) -> Extension {
    Extension {
        extension_type: ExtensionType::DynamicLinking,
        script: metadata_script(deployed),
    }
}

pub fn spawn_extension(deployed: &DeployedScript) -> Extension {
    Extension {
        extension_type: ExtensionType::Spawn,
        script: metadata_script(deployed),
    }
}

fn metadata_script(deployed: &DeployedScript) -> MetadataScript {
    MetadataScript::from_slice(deployed.script.as_slice()).expect("convert script")
}

pub fn build_sudt_meta_bytes(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    metadata_authority: Option<Authority>,
) -> Bytes {
    let metadata = SudtMeta {
        config_flags,
        current_supply,
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        mint_authority,
        metadata_authority,
    };
    Bytes::from(metadata.to_bytes().expect("build SudtMeta bytes"))
}

pub fn build_xudt_meta_bytes(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    metadata_authority: Option<Authority>,
    access_authority: Option<Authority>,
    extensions: Vec<Extension>,
) -> Bytes {
    let metadata = XudtMeta {
        config_flags,
        current_supply,
        decimals: 0,
        name: Vec::new(),
        symbol: Vec::new(),
        uri: Vec::new(),
        extra_data: Vec::new(),
        mint_authority,
        metadata_authority,
        access_authority,
        extensions,
    };
    Bytes::from(metadata.to_bytes().expect("build XudtMeta bytes"))
}

pub fn build_access_list_shard_bytes(
    start: [u8; 32],
    end: [u8; 32],
    entries: Vec<[u8; 32]>,
) -> Bytes {
    let entries = entries
        .iter()
        .map(|entry| blockchain::Byte32::from_slice(entry).expect("build Byte32"))
        .collect::<Vec<_>>();
    let shard = metadata::AccessListShard::new_builder()
        .range(
            metadata::AccessListRange::new_builder()
                .start(blockchain::Byte32::from_slice(&start).expect("build start Byte32"))
                .end(blockchain::Byte32::from_slice(&end).expect("build end Byte32"))
                .build(),
        )
        .entries(blockchain::Byte32Vec::new_builder().set(entries).build())
        .build();
    Bytes::from(shard.as_slice().to_vec())
}

pub fn udt_amount_bytes(amount: u128) -> Bytes {
    Bytes::from(amount.to_le_bytes().to_vec())
}

pub fn script_hash(script: &Script) -> [u8; 32] {
    script.calc_script_hash().unpack()
}
