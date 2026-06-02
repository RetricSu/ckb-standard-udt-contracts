use crate::config::{ConfigError, ProfileConfig};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("No deployment artifacts found at {0}")]
    NoArtifacts(PathBuf),
    #[error("Contract '{0}' not found in deployment artifacts")]
    ContractNotFound(String),
}

#[derive(Debug, Deserialize)]
struct ScriptsJson {
    #[serde(default)]
    devnet: HashMap<String, ScriptInfo>,
    #[serde(default)]
    testnet: HashMap<String, ScriptInfo>,
    #[serde(default)]
    mainnet: HashMap<String, ScriptInfo>,
}

#[derive(Debug, Deserialize)]
struct ScriptInfo {
    #[serde(rename = "codeHash")]
    code_hash: String,
    #[serde(rename = "hashType")]
    _hash_type: String,
    #[serde(default, rename = "cellDeps")]
    cell_deps: Vec<CellDepWrapper>,
}

#[derive(Debug, Deserialize)]
struct CellDepWrapper {
    #[serde(rename = "cellDep")]
    cell_dep: CellDep,
}

#[derive(Debug, Deserialize)]
struct CellDep {
    #[serde(rename = "outPoint")]
    out_point: OutPoint,
}

#[derive(Debug, Deserialize)]
struct OutPoint {
    #[serde(rename = "txHash")]
    tx_hash: String,
    index: u32,
}

#[derive(Debug, Deserialize)]
struct MigrationJson {
    #[serde(default)]
    cell_recipes: Vec<CellRecipe>,
}

#[derive(Debug, Deserialize)]
struct CellRecipe {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "tx_hash")]
    _tx_hash: String,
    #[serde(rename = "index")]
    _index: u32,
    #[serde(rename = "data_hash")]
    data_hash: String,
    #[serde(rename = "type_id")]
    _type_id: Option<String>,
}

/// Sync profile contract references from offckb deployment artifacts.
///
/// Reads `deployment/scripts.json` and per-contract migration JSONs to
/// extract the real on-chain outpoints and data hashes, then writes
/// them back into the selected profile YAML.
pub fn sync_profile_from_deployment(
    _project_root: &Path,
    profile: &mut ProfileConfig,
    artifacts_dir: &Path,
) -> Result<Vec<String>, SyncError> {
    let scripts_json_path = artifacts_dir.join("scripts.json");
    if !scripts_json_path.exists() {
        return Err(SyncError::NoArtifacts(scripts_json_path));
    }

    let scripts_content = std::fs::read_to_string(&scripts_json_path)?;
    let scripts: ScriptsJson = serde_json::from_str(&scripts_content)?;

    let network_key = match profile.network_type {
        crate::config::NetworkType::Devnet => &scripts.devnet,
        crate::config::NetworkType::Testnet => &scripts.testnet,
        crate::config::NetworkType::Mainnet => &scripts.mainnet,
    };

    if network_key.is_empty() {
        return Err(SyncError::NoArtifacts(scripts_json_path));
    }

    let mut updated = Vec::new();

    for (contract_name, contract_ref) in profile.contracts.iter_mut() {
        // offckb uses kebab-case names (e.g. "access-list") while the
        // profile uses snake_case (e.g. "access_list").
        let search_name = contract_name.replace('_', "-");
        let script_info = match network_key.get(&search_name) {
            Some(info) => info,
            None => continue,
        };

        // Get outpoint from the first cell dep
        let outpoint = script_info
            .cell_deps
            .first()
            .map(|dep| &dep.cell_dep.out_point)
            .ok_or_else(|| SyncError::ContractNotFound(contract_name.clone()))?;

        // Try to read the latest migration JSON to get the real data_hash.
        // This is needed because offckb deploy with --type-id produces
        // scripts.json where codeHash is the type_id, but our profile
        // needs the data_hash (with hash_type data1/data2).
        let data_hash = find_data_hash_from_migration(artifacts_dir, &profile.name, &search_name)?;

        let new_code_hash = data_hash.unwrap_or_else(|| script_info.code_hash.clone());

        contract_ref.code_hash = new_code_hash;
        contract_ref.outpoint.tx_hash = outpoint.tx_hash.clone();
        contract_ref.outpoint.index = outpoint.index;

        // Update hash_type from deployment when it is a data reference.
        // If the contract was deployed with --type-id, scripts.json reports
        // "type" (referencing the type_id script), but our profile needs
        // "data1" or "data2" (referencing the actual binary). In that case
        // we keep the existing profile value and warn.
        match script_info._hash_type.as_str() {
            "data1" | "data2" => {
                contract_ref.hash_type = script_info._hash_type.clone();
            }
            "type" => {
                // type-id deployment: keep profile hash_type, use data_hash from migration
            }
            other => {
                // Unknown hash type; fall back to deployment value
                contract_ref.hash_type = other.to_string();
            }
        }

        updated.push(format!(
            "{}: code_hash={}..{}, hash_type={}, outpoint={}:{}",
            contract_name,
            &contract_ref.code_hash[..10],
            &contract_ref.code_hash[contract_ref.code_hash.len().saturating_sub(6)..],
            contract_ref.hash_type,
            &contract_ref.outpoint.tx_hash[..10],
            contract_ref.outpoint.index
        ));
    }

    if updated.is_empty() {
        return Err(SyncError::NoArtifacts(scripts_json_path));
    }

    Ok(updated)
}

/// Walk the migration directory for a contract and return the data_hash
/// from the most recent migration JSON.
fn find_data_hash_from_migration(
    artifacts_dir: &Path,
    network: &str,
    contract_name: &str,
) -> Result<Option<String>, SyncError> {
    let migrations_dir = artifacts_dir
        .join(network)
        .join(contract_name)
        .join("migrations");

    if !migrations_dir.exists() {
        return Ok(None);
    }

    let mut entries: Vec<_> = std::fs::read_dir(&migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return Ok(None);
    }

    // Sort by file name (which is a timestamp like 2024-01-01-120000.json)
    entries.sort_by_key(|a| a.file_name());

    let latest = entries.last().unwrap();
    let content = std::fs::read_to_string(latest.path())?;
    let migration: MigrationJson = serde_json::from_str(&content)?;

    Ok(migration.cell_recipes.first().map(|r| r.data_hash.clone()))
}
