use crate::config::{AccessMode, SupplyMode, TokenKind, UdtxConfig, ProfileConfig};
use crate::error::{tx_build_error, TokenCliError};
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use ckb_sdk::traits::{CellCollector, CellDepResolver, CellQueryOptions, DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver, DefaultTransactionDependencyProvider, Signer, SignerError, ValueRangeOption};
use ckb_sdk::tx_builder::{CapacityBalancer, CapacityProvider, balance_tx_capacity_async, fill_placeholder_witnesses_async};
use ckb_sdk::types::ScriptId;
use ckb_sdk::unlock::SecpSighashUnlocker;
use ckb_types::bytes::{BufMut, Bytes, BytesMut};
use ckb_types::core::{Capacity, TransactionBuilder};
use ckb_types::packed::{CellDep, CellInput, CellOutput, OutPoint, WitnessArgs};
use ckb_types::prelude::*;
use std::collections::HashMap;
use standard_udt_types::metadata::{SudtMeta, XudtMeta, Authority, AuthorityType, CONFIG_SUPPLY_TRACKED, CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST};

struct CombinedCellDepResolver {
    genesis: DefaultCellDepResolver,
    custom: HashMap<ScriptId, CellDep>,
}

impl CombinedCellDepResolver {
    fn new(genesis: DefaultCellDepResolver, custom: HashMap<ScriptId, CellDep>) -> Self {
        Self { genesis, custom }
    }
}

impl CellDepResolver for CombinedCellDepResolver {
    fn resolve(&self, script: &ckb_types::packed::Script) -> Option<CellDep> {
        let script_id = ScriptId::from(script);
        self.custom.get(&script_id).cloned().or_else(|| {
            self.genesis.resolve(script)
        })
    }
}

struct KeyManagerSigner {
    km: KeyManager,
}

impl KeyManagerSigner {
    fn new(km: KeyManager) -> Self {
        Self { km }
    }
}

impl Signer for KeyManagerSigner {
    fn match_id(&self, id: &[u8]) -> bool {
        id.len() == 20
    }

    fn sign(
        &self,
        id: &[u8],
        message: &[u8],
        recoverable: bool,
        _tx: &ckb_types::core::TransactionView,
    ) -> Result<Bytes, SignerError> {
        let sig = self
            .km
            .sign_by_id(id, message)
            .map_err(|e| SignerError::Other(anyhow::anyhow!("{}", e)))?;
        if recoverable {
            Ok(Bytes::from(sig))
        } else {
            Ok(Bytes::from(sig[..64].to_vec()))
        }
    }
}

pub async fn create_token(
    token_type: TokenKind,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<u8>,
    supply: Option<String>,
    owner: Option<String>,
    dry_run: bool,
    config: &UdtxConfig,
    profile: &ProfileConfig,
    key_manager: &mut KeyManager,
) -> Result<(), TokenCliError> {
    let owner_name = owner.as_deref().unwrap_or("owner");

    let owner_account = config.accounts.get(owner_name)
        .ok_or_else(|| TokenCliError::AuthMissing {
            role: format!("owner account '{}' not found in config", owner_name),
        })?;

    let account = key_manager.load_account(owner_name, owner_account, profile)?.clone();

    let kind = token_type;

    let contract = profile.contracts.get(match kind {
        TokenKind::Sudt => "sudt",
        TokenKind::Xudt => "xudt",
    }).ok_or_else(|| TokenCliError::Config(
        crate::config::ConfigError::Validation(
            format!("Contract reference for {:?} not found in profile", kind)
        )
    ))?;

    let meta_contract_name = match kind {
        TokenKind::Sudt => "sudt-meta",
        TokenKind::Xudt => "xudt-meta",
    };
    let meta_contract = profile.contracts.get(meta_contract_name)
        .ok_or_else(|| TokenCliError::Config(
            crate::config::ConfigError::Validation(
                format!("Contract reference for '{}' not found in profile", meta_contract_name)
            )
        ))?;

    let amount_u128 = supply.as_deref().unwrap_or("0").parse::<u128>()
        .map_err(|e| TokenCliError::TxBuild { message: format!("invalid supply amount: {}", e) })?;

    let token_name = name.as_deref().unwrap_or(&config.token.symbol).to_string();
    let token_symbol = symbol.as_deref().unwrap_or(&config.token.symbol).to_string();
    let token_decimals = decimals.unwrap_or(config.token.decimals);

    let contract_code_hash = ckb_types::packed::Byte32::from_slice(
        &hex::decode(contract.code_hash.trim_start_matches("0x"))
            .map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash: {}", e) })?
    ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash bytes: {}", e) })?;

    let hash_type = match contract.hash_type.as_str() {
        "type" => ckb_types::core::ScriptHashType::Type,
        "data" => ckb_types::core::ScriptHashType::Data,
        "data1" => ckb_types::core::ScriptHashType::Data1,
        "data2" => ckb_types::core::ScriptHashType::Data2,
        _ => ckb_types::core::ScriptHashType::Data,
    };

    let meta_code_hash = ckb_types::packed::Byte32::from_slice(
        &hex::decode(meta_contract.code_hash.trim_start_matches("0x"))
            .map_err(|e| TokenCliError::TxBuild { message: format!("invalid meta code hash: {}", e) })?
    ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid meta code hash bytes: {}", e) })?;

    let meta_hash_type = match meta_contract.hash_type.as_str() {
        "type" => ckb_types::core::ScriptHashType::Type,
        "data" => ckb_types::core::ScriptHashType::Data,
        "data1" => ckb_types::core::ScriptHashType::Data1,
        "data2" => ckb_types::core::ScriptHashType::Data2,
        _ => ckb_types::core::ScriptHashType::Data,
    };

    let rpc_url = &profile.rpc_url;
    let mut cell_collector = DefaultCellCollector::new(rpc_url);
    cell_collector.check_ckb_chain().map_err(|e| TokenCliError::TxBuild {
        message: format!("cell collector check failed: {}", e),
    })?;

    let genesis_block = cell_collector.ckb_client
        .get_block_by_number(0u64.into())
        .await
        .map_err(|e| TokenCliError::Rpc { message: format!("get genesis block failed: {}", e) })?
        .ok_or_else(|| TokenCliError::Rpc { message: "genesis block not found".into() })?;

    let genesis_block: ckb_types::core::BlockView = genesis_block.into();
    let genesis_resolver = DefaultCellDepResolver::from_genesis(&genesis_block)
        .map_err(|e| TokenCliError::TxBuild { message: format!("resolve cell deps failed: {}", e) })?;

    let mut custom_deps = HashMap::new();
    for (name, contract) in &profile.contracts {
        let code_hash = ckb_types::packed::Byte32::from_slice(
            &hex::decode(contract.code_hash.trim_start_matches("0x"))
                .map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash for {}: {}", name, e) })?
        ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash bytes for {}: {}", name, e) })?;
        let hash_type = match contract.hash_type.as_str() {
            "type" => ckb_types::core::ScriptHashType::Type,
            "data" => ckb_types::core::ScriptHashType::Data,
            "data1" => ckb_types::core::ScriptHashType::Data1,
            "data2" => ckb_types::core::ScriptHashType::Data2,
            _ => ckb_types::core::ScriptHashType::Data,
        };
        let script_id = ScriptId::new(code_hash.unpack(), hash_type);
        let tx_hash = ckb_types::H256::from_slice(
            &hex::decode(contract.outpoint.tx_hash.trim_start_matches("0x"))
                .map_err(|e| TokenCliError::TxBuild { message: format!("invalid tx_hash for {}: {}", name, e) })?
        ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid tx_hash bytes for {}: {}", name, e) })?;
        let outpoint = OutPoint::new_builder()
            .tx_hash(tx_hash.pack())
            .index(contract.outpoint.index)
            .build();
        let cell_dep = CellDep::new_builder()
            .out_point(outpoint)
            .build();
        custom_deps.insert(script_id, cell_dep);
    }

    // always_success cell dep is required for metadata cell lock;
    // it is added automatically from profile.contracts if present.

    let cell_dep_resolver = CombinedCellDepResolver::new(genesis_resolver, custom_deps);

    let header_dep_resolver = DefaultHeaderDepResolver::new(rpc_url);
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(rpc_url, 10);

    // Collect owner CKB cell as input
    let owner_query = {
        let mut query = CellQueryOptions::new_lock(account.lock_script.clone());
        query.secondary_script_len_range = Some(ValueRangeOption::new_exact(0));
        query.data_len_range = Some(ValueRangeOption::new_exact(0));
        query
    };

    let (owner_cells, _) = cell_collector
        .collect_live_cells_async(&owner_query, true)
        .await
        .map_err(|e| TokenCliError::TxBuild {
            message: format!("collect owner cells failed: {}", e),
        })?;

    if owner_cells.is_empty() {
        return Err(TokenCliError::TxBuild {
            message: "owner cell not found (need a live CKB cell with no type script and empty data)".into(),
        });
    }

    let inputs = vec![CellInput::new(owner_cells[0].out_point.clone(), 0)];

    // Calculate type_id for metadata type script
    // type_id = blake2b(first_input.as_slice() || output_index.to_le_bytes())
    let first_input = inputs[0].clone();
    let mut meta_args_hasher = ckb_hash::new_blake2b();
    meta_args_hasher.update(first_input.as_slice());
    meta_args_hasher.update(&0u64.to_le_bytes());
    let mut meta_args = [0u8; 32];
    meta_args_hasher.finalize(&mut meta_args);

    let meta_type_script = ckb_types::packed::Script::new_builder()
        .code_hash(meta_code_hash)
        .hash_type(meta_hash_type)
        .args(Bytes::from(meta_args.to_vec()).pack())
        .build();

    let meta_type_hash: [u8; 32] = meta_type_script.calc_script_hash().unpack();

    // Build sUDT/xUDT type script with meta_type_hash as args
    let udt_type_script = ckb_types::packed::Script::new_builder()
        .code_hash(contract_code_hash)
        .hash_type(hash_type)
        .args(Bytes::from(meta_type_hash.to_vec()).pack())
        .build();

    let owner_lock_hash: [u8; 32] = account.lock_script.calc_script_hash().unpack();
    let owner_lock_authority = Authority {
        authority_type: AuthorityType::InputLock,
        script_hash: owner_lock_hash,
        script: None,
    };

    // Build metadata output cell
    let mut config_flags = if config.token.supply_policy.mode == SupplyMode::Tracked {
        CONFIG_SUPPLY_TRACKED
    } else {
        0
    };

    if let Some(ref ac) = config.access_control {
        if ac.enabled && matches!(kind, TokenKind::Xudt) {
            config_flags |= CONFIG_ACCESS_ENABLED;
            if matches!(ac.mode, AccessMode::Whitelist) {
                config_flags |= CONFIG_ACCESS_WHITELIST;
            }
        }
    }

    let current_supply = if config.token.supply_policy.mode == SupplyMode::Tracked {
        amount_u128
    } else {
        0
    };

    let meta_data_bytes = match kind {
        TokenKind::Sudt => {
            let meta = SudtMeta {
                config_flags,
                current_supply,
                decimals: token_decimals,
                name: token_name.as_bytes().to_vec(),
                symbol: token_symbol.as_bytes().to_vec(),
                uri: Vec::new(),
                extra_data: Vec::new(),
                mint_authority: Some(owner_lock_authority.clone()),
                metadata_authority: Some(owner_lock_authority.clone()),
            };
            Bytes::from(meta.to_bytes().map_err(|e| TokenCliError::TxBuild {
                message: format!("build SudtMeta bytes failed: {:?}", e),
            })?)
        }
        TokenKind::Xudt => {
            let access_authority = config.access_control
                .as_ref()
                .filter(|ac| ac.enabled)
                .map(|_| owner_lock_authority.clone());
            let meta = XudtMeta {
                config_flags,
                current_supply,
                decimals: token_decimals,
                name: token_name.as_bytes().to_vec(),
                symbol: token_symbol.as_bytes().to_vec(),
                uri: Vec::new(),
                extra_data: Vec::new(),
                mint_authority: Some(owner_lock_authority.clone()),
                metadata_authority: Some(owner_lock_authority.clone()),
                access_authority,
                extensions: Vec::new(),
            };
            Bytes::from(meta.to_bytes().map_err(|e| TokenCliError::TxBuild {
                message: format!("build XudtMeta bytes failed: {:?}", e),
            })?)
        }
    };

    // sudt-meta contract requires the metadata cell lock to be always_success (Data2)
    let always_success_contract = profile.contracts.get("always_success")
        .ok_or_else(|| TokenCliError::Config(
            crate::config::ConfigError::Validation(
                "Contract reference for 'always_success' not found in profile (required for metadata cell lock)".into()
            )
        ))?;
    let always_success_code_hash = ckb_types::packed::Byte32::from_slice(
        &hex::decode(always_success_contract.code_hash.trim_start_matches("0x"))
            .map_err(|e| TokenCliError::TxBuild { message: format!("invalid always_success code hash: {}", e) })?
    ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid always_success code hash bytes: {}", e) })?;
    let always_success_hash_type = match always_success_contract.hash_type.as_str() {
        "type" => ckb_types::core::ScriptHashType::Type,
        "data" => ckb_types::core::ScriptHashType::Data,
        "data1" => ckb_types::core::ScriptHashType::Data1,
        "data2" => ckb_types::core::ScriptHashType::Data2,
        _ => ckb_types::core::ScriptHashType::Data,
    };
    let always_success_lock = ckb_types::packed::Script::new_builder()
        .code_hash(always_success_code_hash)
        .hash_type(always_success_hash_type)
        .args(Bytes::new().pack())
        .build();

    let meta_output = CellOutput::new_builder()
        .lock(always_success_lock.clone())
        .type_(Some(meta_type_script.clone()).pack())
        .build();
    let meta_occupied = meta_output
        .occupied_capacity(Capacity::bytes(meta_data_bytes.len()).unwrap())
        .unwrap()
        .as_u64();
    let meta_output = meta_output.as_builder().capacity(meta_occupied).build();

    // Build UDT output cell
    let mut udt_data = BytesMut::with_capacity(16);
    udt_data.put(&amount_u128.to_le_bytes()[..]);
    let udt_data_bytes = udt_data.freeze();

    let udt_output = CellOutput::new_builder()
        .lock(account.lock_script.clone())
        .type_(Some(udt_type_script.clone()).pack())
        .build();
    let udt_occupied = udt_output
        .occupied_capacity(Capacity::bytes(udt_data_bytes.len()).unwrap())
        .unwrap()
        .as_u64();
    let udt_output = udt_output.as_builder().capacity(udt_occupied).build();

    // Resolve cell deps
    let owner_cell_dep = cell_dep_resolver
        .resolve(&account.lock_script)
        .ok_or_else(|| TokenCliError::TxBuild {
            message: "resolve owner cell dep failed".into(),
        })?;
    let udt_cell_dep = cell_dep_resolver
        .resolve(&udt_type_script)
        .ok_or_else(|| TokenCliError::TxBuild {
            message: format!("resolve {} cell dep failed", match kind { TokenKind::Sudt => "sudt", TokenKind::Xudt => "xudt" }),
        })?;
    let meta_cell_dep = cell_dep_resolver
        .resolve(&meta_type_script)
        .ok_or_else(|| TokenCliError::TxBuild {
            message: format!("resolve {} cell dep failed", meta_contract_name),
        })?;
    let always_success_cell_dep = cell_dep_resolver
        .resolve(&always_success_lock)
        .ok_or_else(|| TokenCliError::TxBuild {
            message: "resolve always_success cell dep failed".into(),
        })?;

    #[allow(clippy::mutable_key_type)]
    let mut cell_deps = HashMap::new();
    cell_deps.insert(owner_cell_dep.clone(), ());
    cell_deps.insert(udt_cell_dep.clone(), ());
    cell_deps.insert(meta_cell_dep.clone(), ());
    cell_deps.insert(always_success_cell_dep.clone(), ());
    let cell_deps_vec: Vec<CellDep> = cell_deps.into_iter().map(|(dep, _)| dep).collect();

    let base_tx = TransactionBuilder::default()
        .set_cell_deps(cell_deps_vec)
        .set_inputs(inputs)
        .set_outputs(vec![meta_output, udt_output])
        .set_outputs_data(vec![meta_data_bytes.pack(), udt_data_bytes.pack()])
        .build();

    let placeholder_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();

    let capacity_provider = CapacityProvider::new_simple(vec![(
        account.lock_script.clone(),
        placeholder_witness,
    )]);

    let balancer = CapacityBalancer::new_with_provider(1000, capacity_provider);

    let signer: Box<dyn Signer> = Box::new(KeyManagerSigner::new(key_manager.clone()));
    let unlocker: Box<dyn ckb_sdk::unlock::ScriptUnlocker> = Box::new(SecpSighashUnlocker::from(signer));
    let mut unlockers = HashMap::new();
    unlockers.insert(ScriptId::new_type(
        ckb_types::H256::from_slice(
            &hex::decode("9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").unwrap()
        ).unwrap()
    ), unlocker);

    let tx_filled = fill_placeholder_witnesses_async(base_tx, &tx_dep_provider, &unlockers)
        .await
        .map_err(|e| TokenCliError::TxBuild {
            message: format!("fill placeholder witnesses failed: {}", e),
        })?
        .0;

    let balanced_tx = balance_tx_capacity_async(
        &tx_filled,
        &balancer,
        &mut cell_collector,
        &tx_dep_provider,
        &cell_dep_resolver,
        &header_dep_resolver,
    )
    .await
    .map_err(|e| TokenCliError::TxBuild {
        message: format!("balance tx capacity failed: {}", e),
    })?;

    let (tx, _not_unlocked) = ckb_sdk::tx_builder::unlock_tx_async(
        balanced_tx,
        &tx_dep_provider,
        &unlockers,
    )
    .await
    .map_err(|e| TokenCliError::TxBuild {
        message: format!("unlock tx failed: {}", e),
    })?;

    if dry_run {
        println!("Token Issue Preview");
        println!("===================");
        println!("  Token Type: {:?}", kind);
        println!("  Name: {}", token_name);
        println!("  Symbol: {}", token_symbol);
        println!("  Decimals: {}", token_decimals);
        println!("  Initial Supply: {}", amount_u128);
        println!("  Owner: {} ({})", owner_name, account.address);
        println!("  Metadata Type Hash: 0x{}", hex::encode(meta_type_hash));
        println!("  Transaction Hash: 0x{}", hex::encode(tx.hash().as_slice()));
        println!("\n[Dry Run] Issue preview complete. No transaction sent.");
        return Ok(());
    }

    let client = RpcClient::new(rpc_url)?;
    let hash = client.send_transaction(tx).await?;
    println!("Token issued successfully.");
    println!("  Token Type: {:?}", kind);
    println!("  Name: {}", token_name);
    println!("  Symbol: {}", token_symbol);
    println!("  Decimals: {}", token_decimals);
    println!("  Initial Supply: {}", amount_u128);
    println!("  Owner: {} ({})", owner_name, account.address);
    println!("  Metadata Type Hash: 0x{}", hex::encode(meta_type_hash));
    println!("  Transaction Hash: 0x{}", hash);

    Ok(())
}
