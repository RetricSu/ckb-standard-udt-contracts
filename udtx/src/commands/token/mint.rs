use crate::config::{ProfileConfig, TokenKind, UdtxConfig};
use crate::error::TokenCliError;
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use ckb_sdk::traits::{CellCollector, DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver, DefaultTransactionDependencyProvider, Signer, SignerError};
use ckb_sdk::tx_builder::{CapacityBalancer, CapacityProvider, TransferAction, TxBuilder, UdtTargetReceiver, UdtIssueBuilder, UdtType};
use ckb_sdk::types::ScriptId;
use ckb_sdk::unlock::SecpSighashUnlocker;
use ckb_types::bytes::Bytes;
use ckb_types::packed::WitnessArgs;
use ckb_types::prelude::*;
use std::collections::HashMap;

struct KeyManagerSigner<'a> {
    km: &'a KeyManager,
}

impl<'a> KeyManagerSigner<'a> {
    fn new(km: &'a KeyManager) -> Self {
        Self { km }
    }
}

impl<'a> Signer for KeyManagerSigner<'a> {
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

pub async fn mint_token(
    amount: String,
    token_type: Option<TokenKind>,
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

    let account = key_manager.load_account(owner_name, owner_account, profile)?;

    let kind = token_type.as_ref().unwrap_or(&config.token.kind);

    let contract = profile.contracts.get(match kind {
        TokenKind::Sudt => "sudt",
        TokenKind::Xudt => "xudt",
    }).ok_or_else(|| TokenCliError::Config(
        crate::config::ConfigError::Validation(
            format!("Contract reference for {:?} not found in profile", kind)
        )
    ))?;

    let amount_u128 = amount.parse::<u128>()
        .map_err(|e| TokenCliError::TxBuild { message: format!("invalid amount: {}", e) })?;

    let contract_code_hash = ckb_types::packed::Byte32::from_slice(
        &hex::decode(contract.code_hash.trim_start_matches("0x"))
            .map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash: {}", e) })?
    ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash bytes: {}", e) })?;

    let script_id = ScriptId::new(
        contract_code_hash.unpack(),
        match contract.hash_type.as_str() {
            "type" => ckb_jsonrpc_types::ScriptHashType::Type,
            "data" | "data1" => ckb_jsonrpc_types::ScriptHashType::Data,
            _ => ckb_jsonrpc_types::ScriptHashType::Data,
        },
    );

    let udt_type = match kind {
        TokenKind::Sudt => UdtType::Sudt,
        TokenKind::Xudt => UdtType::Xudt(Bytes::default()),
    };

    let receiver = UdtTargetReceiver::new(
        TransferAction::Create,
        account.lock_script.clone(),
        amount_u128,
    );

    let builder = UdtIssueBuilder {
        udt_type,
        script_id,
        owner: account.lock_script.clone(),
        receivers: vec![receiver],
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

    let cell_dep_resolver = DefaultCellDepResolver::from_genesis(&genesis_block)
        .map_err(|e| TokenCliError::TxBuild { message: format!("resolve cell deps failed: {}", e) })?;

    let header_dep_resolver = DefaultHeaderDepResolver::new(rpc_url);
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(rpc_url, 10);

    let placeholder_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();

    let capacity_provider = CapacityProvider::new_simple(vec![(
        account.lock_script.clone(),
        placeholder_witness,
    )]);

    let balancer = CapacityBalancer::new_with_provider(1000, capacity_provider);

    let signer = Box::new(KeyManagerSigner::new(key_manager));
    let unlocker: Box<dyn ckb_sdk::unlock::ScriptUnlocker> = Box::new(SecpSighashUnlocker::from(signer));
    let mut unlockers = HashMap::new();
    unlockers.insert(ScriptId::new_type(
        ckb_types::H256::from_slice(
            &hex::decode("9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").unwrap()
        ).unwrap()
    ), unlocker);

    let balanced_tx = builder
        .build_balanced_async(
            &mut cell_collector,
            &cell_dep_resolver,
            &header_dep_resolver,
            &tx_dep_provider,
            &balancer,
            &unlockers,
        )
        .await
        .map_err(|e| TokenCliError::TxBuild {
            message: format!("build tx failed: {}", e),
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
        println!("Token Mint Preview");
        println!("==================");
        println!("  Amount: {}", amount_u128);
        println!("  Token Type: {:?}", kind);
        println!("  Owner: {} ({})", owner_name, account.address);
        println!("  Transaction Hash: 0x{}", hex::encode(tx.hash().as_slice()));
        println!("\n[Dry Run] Mint preview complete. No transaction sent.");
        return Ok(());
    }

    let client = RpcClient::new(rpc_url)?;
    let hash = client.send_transaction(tx).await?;
    println!("Token mint submitted.");
    println!("  Amount: {}", amount_u128);
    println!("  Owner: {} ({})", owner_name, account.address);
    println!("  Transaction Hash: 0x{}", hash);

    Ok(())
}
