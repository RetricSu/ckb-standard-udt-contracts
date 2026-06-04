use std::collections::HashMap;

use ckb_sdk::traits::{CellDepResolver, DefaultCellDepResolver};
use ckb_sdk::types::ScriptId;
use ckb_sdk::rpc::ckb_indexer::{Cell, ScriptType, SearchKey, SearchMode};
use ckb_types::core::{BlockView, ScriptHashType, TransactionView};
use ckb_types::packed::{Byte32, CellDep, OutPoint, Script};
use ckb_types::prelude::*;
use standard_udt_types::metadata::{SudtMeta, XudtMeta};

use crate::config::{ContractRef, ProfileConfig, TokenKind};
use crate::error::TokenCliError;
use crate::rpc::RpcClient;

fn parse_script_hash_type(hash_type: &str) -> ScriptHashType {
    match hash_type {
        "type" => ScriptHashType::Type,
        "data" => ScriptHashType::Data,
        "data1" => ScriptHashType::Data1,
        "data2" => ScriptHashType::Data2,
        _ => ScriptHashType::Data,
    }
}

fn parse_byte32_hex(value: &str, what: &str) -> Result<Byte32, TokenCliError> {
    Byte32::from_slice(
        &hex::decode(value.trim_start_matches("0x")).map_err(|e| TokenCliError::TxBuild {
            message: format!("invalid {}: {}", what, e),
        })?,
    )
    .map_err(|e| TokenCliError::TxBuild {
        message: format!("invalid {} bytes: {}", what, e),
    })
}

pub struct CombinedCellDepResolver {
    genesis: DefaultCellDepResolver,
    custom: HashMap<ScriptId, CellDep>,
}

impl CombinedCellDepResolver {
    fn new(genesis: DefaultCellDepResolver, custom: HashMap<ScriptId, CellDep>) -> Self {
        Self { genesis, custom }
    }
}

impl CellDepResolver for CombinedCellDepResolver {
    fn resolve(&self, script: &Script) -> Option<CellDep> {
        let script_id = ScriptId::from(script);
        self.custom
            .get(&script_id)
            .cloned()
            .or_else(|| self.genesis.resolve(script))
    }
}

pub fn build_profile_cell_dep_resolver(
    profile: &ProfileConfig,
    genesis_block: &BlockView,
) -> Result<CombinedCellDepResolver, TokenCliError> {
    let genesis_resolver = DefaultCellDepResolver::from_genesis(genesis_block).map_err(|e| {
        TokenCliError::TxBuild {
            message: format!("resolve cell deps failed: {}", e),
        }
    })?;

    let mut custom_deps = HashMap::new();
    for (name, contract) in &profile.contracts {
        let code_hash = parse_byte32_hex(&contract.code_hash, &format!("{} code hash", name))?;
        let hash_type = parse_script_hash_type(&contract.hash_type);
        let script_id = ScriptId::new(code_hash.unpack(), hash_type);

        let tx_hash = ckb_types::H256::from_slice(
            &hex::decode(contract.outpoint.tx_hash.trim_start_matches("0x")).map_err(|e| {
                TokenCliError::TxBuild {
                    message: format!("invalid tx_hash for {}: {}", name, e),
                }
            })?,
        )
        .map_err(|e| TokenCliError::TxBuild {
            message: format!("invalid tx_hash bytes for {}: {}", name, e),
        })?;

        let out_point = OutPoint::new_builder()
            .tx_hash(tx_hash.pack())
            .index(contract.outpoint.index)
            .build();
        let cell_dep = CellDep::new_builder().out_point(out_point).build();
        custom_deps.insert(script_id, cell_dep);
    }

    Ok(CombinedCellDepResolver::new(genesis_resolver, custom_deps))
}

async fn collect_cells(
    client: &RpcClient,
    search_key: SearchKey,
    with_data: bool,
) -> Result<Vec<Cell>, TokenCliError> {
    let mut cells = Vec::new();
    let mut cursor = None;
    let mut search_key = search_key;
    search_key.with_data = Some(with_data);

    loop {
        let page = client.get_cells(search_key.clone(), 500, cursor.clone()).await?;
        if page.objects.is_empty() {
            break;
        }
        let page_len = page.objects.len();
        cells.extend(page.objects);
        if page_len < 500 {
            break;
        }
        cursor = Some(page.last_cursor);
    }

    Ok(cells)
}

fn search_key_for_lock(lock_script: &Script) -> SearchKey {
    SearchKey {
        script: lock_script.clone().into(),
        script_type: ScriptType::Lock,
        script_search_mode: Some(SearchMode::Exact),
        filter: None,
        with_data: Some(false),
        group_by_transaction: None,
    }
}

fn search_key_for_type(type_script: &Script) -> SearchKey {
    SearchKey {
        script: type_script.clone().into(),
        script_type: ScriptType::Type,
        script_search_mode: Some(SearchMode::Exact),
        filter: None,
        with_data: Some(false),
        group_by_transaction: None,
    }
}

fn search_key_for_type_code_hash(code_hash: Byte32, hash_type: ScriptHashType) -> SearchKey {
    let script = Script::new_builder()
        .code_hash(code_hash)
        .hash_type(hash_type)
        .args(ckb_types::bytes::Bytes::new().pack())
        .build();
    SearchKey {
        script: script.into(),
        script_type: ScriptType::Type,
        script_search_mode: Some(SearchMode::Prefix),
        filter: None,
        with_data: Some(false),
        group_by_transaction: None,
    }
}

async fn resolve_metadata_symbol_for_meta_hash(
    client: &RpcClient,
    profile: &ProfileConfig,
    kind: &TokenKind,
    meta_type_hash: &[u8; 32],
) -> Result<Option<String>, TokenCliError> {
    let meta_contract_name = match kind {
        TokenKind::Sudt => "sudt-meta",
        TokenKind::Xudt => "xudt-meta",
    };
    let Some(meta_contract) = profile.contracts.get(meta_contract_name) else {
        return Ok(None);
    };

    let meta_code_hash = parse_byte32_hex(&meta_contract.code_hash, "meta contract code hash")?;
    let meta_hash_type = parse_script_hash_type(&meta_contract.hash_type);
    let cells = collect_cells(
        client,
        search_key_for_type_code_hash(meta_code_hash, meta_hash_type),
        true,
    )
    .await?;

    let mut matched = Vec::new();
    for cell in cells {
        let Some(type_script_json) = cell.output.type_.clone() else {
            continue;
        };
        let type_script: Script = type_script_json.into();
        let script_hash: [u8; 32] = type_script.calc_script_hash().unpack();
        if script_hash == *meta_type_hash {
            matched.push(cell);
        }
    }

    if matched.is_empty() {
        return Ok(None);
    }

    matched.sort_by_key(|cell| cell.block_number.value());
    let latest = matched.last();
    let Some(data) = latest.and_then(|c| c.output_data.as_ref()) else {
        return Ok(None);
    };

    let parse_symbol = |raw: &[u8]| {
        String::from_utf8_lossy(raw)
            .trim_end_matches('\0')
            .to_string()
    };

    let symbol = match kind {
        TokenKind::Sudt => SudtMeta::from_slice(data.as_bytes().as_ref())
            .ok()
            .map(|meta| parse_symbol(&meta.symbol)),
        TokenKind::Xudt => XudtMeta::from_slice(data.as_bytes().as_ref())
            .ok()
            .map(|meta| parse_symbol(&meta.symbol)),
    };

    Ok(symbol)
}

pub async fn resolve_bound_udt_type_script_for_owner(
    client: &RpcClient,
    profile: &ProfileConfig,
    kind: &TokenKind,
    owner_lock_script: &Script,
    contract: &ContractRef,
    symbol_hint: Option<&str>,
) -> Result<Script, TokenCliError> {
    let contract_code_hash = parse_byte32_hex(&contract.code_hash, "token contract code hash")?;
    let contract_hash_type = parse_script_hash_type(&contract.hash_type);

    let owner_cells = collect_cells(client, search_key_for_lock(owner_lock_script), false).await?;
    let mut candidates: HashMap<[u8; 32], u64> = HashMap::new();

    for cell in owner_cells {
        let Some(type_script_json) = cell.output.type_.clone() else {
            continue;
        };
        let type_script: Script = type_script_json.into();
        if type_script.code_hash() != contract_code_hash {
            continue;
        }
        if type_script.hash_type() != contract_hash_type.into() {
            continue;
        }

        let args = type_script.args().raw_data();
        if args.len() != 32 {
            continue;
        }

        let mut meta_type_hash = [0u8; 32];
        meta_type_hash.copy_from_slice(args.as_ref());
        let block_number = cell.block_number.value();
        candidates
            .entry(meta_type_hash)
            .and_modify(|h| *h = (*h).max(block_number))
            .or_insert(block_number);
    }

    if candidates.is_empty() {
        return Err(TokenCliError::TxBuild {
            message: "sender cell not found for selected token type".into(),
        });
    }

    let mut candidate_vec: Vec<([u8; 32], u64)> = candidates.into_iter().collect();

    if let Some(symbol) = symbol_hint {
        let mut filtered = Vec::new();
        for (hash, height) in &candidate_vec {
            if resolve_metadata_symbol_for_meta_hash(client, profile, kind, hash)
                .await?
                .as_deref()
                == Some(symbol)
            {
                filtered.push((*hash, *height));
            }
        }

        if filtered.is_empty() {
            return Err(TokenCliError::TxBuild {
                message: format!("no token cells found matching the symbol '{}'", symbol),
            });
        }

        candidate_vec = filtered;
    }

    candidate_vec.sort_by_key(|(_, height)| *height);
    let selected_meta_hash = candidate_vec
        .last()
        .map(|(hash, _)| *hash)
        .ok_or_else(|| TokenCliError::TxBuild {
            message: "failed to resolve meta type hash for token".into(),
        })?;

    Ok(Script::new_builder()
        .code_hash(contract_code_hash)
        .hash_type(contract_hash_type)
        .args(ckb_types::bytes::Bytes::from(selected_meta_hash.to_vec()).pack())
        .build())
}

pub async fn resolve_bound_token_context_deps(
    client: &RpcClient,
    profile: &ProfileConfig,
    kind: &TokenKind,
    meta_type_hash: &[u8; 32],
    require_access_list: bool,
) -> Result<Vec<CellDep>, TokenCliError> {
    let meta_contract_name = match kind {
        TokenKind::Sudt => "sudt-meta",
        TokenKind::Xudt => "xudt-meta",
    };
    let meta_contract = profile.contracts.get(meta_contract_name).ok_or_else(|| {
        TokenCliError::Config(crate::config::ConfigError::Validation(format!(
            "Contract reference for '{}' not found in profile",
            meta_contract_name
        )))
    })?;

    let meta_code_hash = parse_byte32_hex(&meta_contract.code_hash, "meta contract code hash")?;
    let meta_hash_type = parse_script_hash_type(&meta_contract.hash_type);
    let mut deps = Vec::new();

    let meta_cells = collect_cells(
        client,
        search_key_for_type_code_hash(meta_code_hash, meta_hash_type),
        false,
    )
    .await?;

    let mut matched_meta_cells = Vec::new();
    for cell in meta_cells {
        let Some(type_script_json) = cell.output.type_.clone() else {
            continue;
        };
        let type_script: Script = type_script_json.into();
        let script_hash: [u8; 32] = type_script.calc_script_hash().unpack();
        if script_hash == *meta_type_hash {
            matched_meta_cells.push(cell);
        }
    }

    if matched_meta_cells.is_empty() {
        return Err(TokenCliError::TxBuild {
            message: "metadata cell not found for selected token".into(),
        });
    }

    matched_meta_cells.sort_by_key(|cell| cell.block_number.value());
    let latest_meta_cell = matched_meta_cells.last().ok_or_else(|| TokenCliError::TxBuild {
        message: "metadata cell lookup failed".into(),
    })?;
    deps.push(
        CellDep::new_builder()
            .out_point(latest_meta_cell.out_point.clone())
            .build(),
    );

    if matches!(kind, TokenKind::Xudt) {
        if let Some(access_contract) = profile.contracts.get("access_list") {
            let access_hash_type = parse_script_hash_type(&access_contract.hash_type);
            if access_hash_type != ScriptHashType::Data2 {
                return Err(TokenCliError::TxBuild {
                    message: format!(
                        "access_list hash_type must be data2, got '{}'",
                        access_contract.hash_type
                    ),
                });
            }

            let access_code_hash =
                parse_byte32_hex(&access_contract.code_hash, "access_list code hash")?;
            let access_script = Script::new_builder()
                .code_hash(access_code_hash)
                .hash_type(access_hash_type)
                .args(ckb_types::bytes::Bytes::from(meta_type_hash.to_vec()).pack())
                .build();

            let access_cells = collect_cells(client, search_key_for_type(&access_script), false).await?;
            if require_access_list && access_cells.is_empty() {
                return Err(TokenCliError::TxBuild {
                    message: "access list shard cells not found for selected token".into(),
                });
            }

            for cell in access_cells {
                deps.push(
                    CellDep::new_builder()
                        .out_point(cell.out_point)
                        .build(),
                );
            }
        } else if require_access_list {
            return Err(TokenCliError::Config(
                crate::config::ConfigError::Validation(
                    "Contract reference for 'access_list' not found in profile".into(),
                ),
            ));
        }
    }

    Ok(deps)
}

pub fn append_missing_cell_deps(tx: TransactionView, extra_deps: &[CellDep]) -> TransactionView {
    let mut cell_deps: Vec<CellDep> = tx.cell_deps().into_iter().collect();
    for dep in extra_deps {
        let exists = cell_deps
            .iter()
            .any(|existing| existing.out_point() == dep.out_point());
        if !exists {
            cell_deps.push(dep.clone());
        }
    }
    tx.as_advanced_builder().set_cell_deps(cell_deps).build()
}
