use crate::config::NetworkType;
use crate::error::{rpc_error, TokenCliError};
use ckb_sdk::rpc::ckb_indexer::{Order, ScriptType, SearchKey, SearchMode};
use ckb_sdk::rpc::CkbRpcClient;
use ckb_types::packed::Script;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub struct RpcClient {
    inner: CkbRpcClient,
    url: String,
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub chain: String,
    pub median_time: u64,
    pub epoch: String,
    pub difficulty: String,
    pub is_initial_block_download: bool,
    pub alerts: Vec<String>,
}

impl RpcClient {
    pub fn new(rpc_url: &str) -> Result<Self, TokenCliError> {
        let client = CkbRpcClient::new(rpc_url);
        Ok(Self {
            inner: client,
            url: rpc_url.to_string(),
        })
    }

    pub async fn get_chain_info(&self) -> Result<ChainInfo, TokenCliError> {
        let info = self
            .with_retry(|| async {
                self.inner
                    .get_blockchain_info()
                    .map_err(|e| rpc_error(format!("get_blockchain_info failed: {}", e)))
            })
            .await?;

        Ok(ChainInfo {
            chain: info.chain,
            median_time: info.median_time.into(),
            epoch: info.epoch.to_string(),
            difficulty: info.difficulty.to_string(),
            is_initial_block_download: info.is_initial_block_download,
            alerts: info.alerts.into_iter().map(|a| a.message).collect(),
        })
    }

    pub async fn get_balance(&self, address: &str) -> Result<u64, TokenCliError> {
        let addr = ckb_sdk::Address::from_str(address)
            .map_err(|e| rpc_error(format!("Invalid address '{}': {}", address, e)))?;
        let lock_script: Script = addr.payload().into();

        let search_key = SearchKey {
            script: lock_script.into(),
            script_type: ScriptType::Lock,
            script_search_mode: Some(SearchMode::Exact),
            filter: None,
            with_data: Some(false),
            group_by_transaction: None,
        };

        let mut balance: u64 = 0;
        let mut cursor = None;

        loop {
            let page = self
                .with_retry(|| async {
                    self.inner
                        .get_cells(
                            search_key.clone(),
                            Order::Asc,
                            1000.into(),
                            cursor.clone(),
                        )
                        .map_err(|e| rpc_error(format!("get_cells failed: {}", e)))
                })
                .await?;

            for cell in &page.objects {
                let cap: u64 = cell.output.capacity.into();
                balance = balance.checked_add(cap).ok_or_else(|| {
                    rpc_error("Balance overflow: total capacity exceeds u64")
                })?;
            }

            if page.objects.len() < 1000 {
                break;
            }
            cursor = Some(page.last_cursor);
        }

        Ok(balance)
    }

    pub async fn get_tip_block_number(&self) -> Result<u64, TokenCliError> {
        let num = self
            .with_retry(|| async {
                self.inner
                    .get_tip_block_number()
                    .map_err(|e| rpc_error(format!("get_tip_block_number failed: {}", e)))
            })
            .await?;
        Ok(num.into())
    }

    pub async fn validate_network(&self, expected: NetworkType) -> Result<(), TokenCliError> {
        let info = self.get_chain_info().await?;

        let detected = match info.chain.as_str() {
            "ckb" => NetworkType::Mainnet,
            "ckb_testnet" | "pudge" => NetworkType::Testnet,
            "ckb_dev" => NetworkType::Devnet,
            other => {
                return Err(rpc_error(format!(
                    "Unknown chain '{}'. Expected one of: ckb, ckb_testnet, ckb_dev.",
                    other
                )))
            }
        };

        if detected != expected {
            return Err(rpc_error(format!(
                "Network mismatch: profile expects {:?}, but RPC node reports {:?} (chain='{}').\n  → Check your profile's network_type and RPC URL.",
                expected, detected, info.chain
            )));
        }

        Ok(())
    }

    async fn with_retry<F, Fut, T>(&self, f: F) -> Result<T, TokenCliError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, TokenCliError>>,
    {
        let delays = [Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];

        for (attempt, delay) in delays.iter().enumerate() {
            match timeout(Duration::from_secs(30), f()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(err)) => {
                    let is_last = attempt == delays.len() - 1;
                    if is_last {
                        return Err(rpc_error(format!(
                            "RPC request failed after {} retries: {}",
                            delays.len(),
                            err
                        )));
                    }
                    tracing::warn!(
                        attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error = %err,
                        "RPC attempt failed, retrying..."
                    );
                    tokio::time::sleep(*delay).await;
                }
                Err(_) => {
                    let is_last = attempt == delays.len() - 1;
                    if is_last {
                        return Err(rpc_error(format!(
                            "RPC request timed out after {} retries (30s timeout each)",
                            delays.len()
                        )));
                    }
                    tracing::warn!(
                        attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        "RPC request timed out, retrying..."
                    );
                    tokio::time::sleep(*delay).await;
                }
            }
        }

        Err(rpc_error("RPC request failed after all retries"))
    }
}

impl From<ckb_sdk::rpc::RpcError> for TokenCliError {
    fn from(err: ckb_sdk::rpc::RpcError) -> Self {
        rpc_error(format!("CKB RPC error: {}", err))
    }
}


