use crate::config::{ProfileConfig, TokenKind, UdtxConfig};
use crate::error::TokenCliError;
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use crate::commands::token::resolve::{
    append_missing_cell_deps, build_profile_cell_dep_resolver, resolve_bound_token_context_deps,
    resolve_bound_udt_type_script_for_owner,
};
use ckb_sdk::traits::{DefaultCellCollector, DefaultHeaderDepResolver, DefaultTransactionDependencyProvider, Signer, SignerError};
use ckb_sdk::tx_builder::{CapacityBalancer, CapacityProvider, TransferAction, TxBuilder};
use ckb_sdk::tx_builder::udt::{UdtTargetReceiver, UdtTransferBuilder};
use ckb_sdk::types::ScriptId;
use ckb_sdk::unlock::SecpSighashUnlocker;
use ckb_types::bytes::Bytes;
use ckb_types::core::TransactionView;
use ckb_types::packed::WitnessArgs;
use ckb_types::prelude::*;
use std::collections::HashMap;

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
        _tx: &TransactionView,
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

pub async fn transfer_token(
    to: String,
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

    let account = key_manager.load_account(owner_name, owner_account, profile)?.clone();

    let kind = token_type.as_ref().unwrap_or(&config.token.kind);

    let contract = profile.contracts.get(match kind {
        TokenKind::Sudt => "sudt",
        TokenKind::Xudt => "xudt",
    }).ok_or_else(|| TokenCliError::Config(
        crate::config::ConfigError::Validation(
            format!("Contract reference for {:?} not found in profile", kind)
        )
    ))?;

    let recipient_lock = crate::keys::address_to_lock_script(&to, profile)
        .map_err(|e| TokenCliError::TxBuild { message: format!("invalid recipient address: {}", e) })?;

    let amount_u128 = amount.parse::<u128>()
        .map_err(|e| TokenCliError::TxBuild { message: format!("invalid amount: {}", e) })?;

    let rpc_url = &profile.rpc_url;
    let client = RpcClient::new(rpc_url)?;
    let udt_type_script = resolve_bound_udt_type_script_for_owner(
        &client,
        profile,
        kind,
        &account.lock_script,
        contract,
        Some(&config.token.symbol),
    )
    .await?;
    let mut meta_type_hash = [0u8; 32];
    meta_type_hash.copy_from_slice(udt_type_script.args().raw_data().as_ref());

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
    let cell_dep_resolver = build_profile_cell_dep_resolver(profile, &genesis_block)?;

    let header_dep_resolver = DefaultHeaderDepResolver::new(rpc_url);
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(rpc_url, 10);

    let receiver = UdtTargetReceiver::new(
        TransferAction::Create,
        recipient_lock,
        amount_u128,
    );

    let builder = UdtTransferBuilder {
        type_script: udt_type_script,
        sender: account.lock_script.clone(),
        receivers: vec![receiver],
    };

    let placeholder_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();

    let capacity_provider = CapacityProvider::new_simple(vec![(
        account.lock_script.clone(),
        placeholder_witness,
    )]);

    let balancer = CapacityBalancer::new_with_provider(3000, capacity_provider);

    let signer: Box<dyn Signer> = Box::new(KeyManagerSigner::new(key_manager.clone()));
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

    let extra_deps = resolve_bound_token_context_deps(
        &client,
        profile,
        kind,
        &meta_type_hash,
        matches!(kind, TokenKind::Xudt)
            && config
                .access_control
                .as_ref()
                .map(|ac| ac.enabled)
                .unwrap_or(false),
    )
    .await?;
    let balanced_tx = append_missing_cell_deps(balanced_tx, &extra_deps);

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
        println!("Token Transfer Preview");
        println!("======================");
        println!("  Recipient: {}", to);
        println!("  Amount: {}", amount_u128);
        println!("  Token Type: {:?}", kind);
        println!("  Sender: {} ({})", owner_name, account.address);
        println!("  Transaction Hash: 0x{}", hex::encode(tx.hash().as_slice()));
        println!("\n[Dry Run] Transfer preview complete. No transaction sent.");
        return Ok(());
    }

    let hash = client.send_transaction(tx).await?;
    println!("Token transfer submitted.");
    println!("  Amount: {}", amount_u128);
    println!("  Recipient: {}", to);
    println!("  Transaction Hash: 0x{}", hash);

    Ok(())
}
