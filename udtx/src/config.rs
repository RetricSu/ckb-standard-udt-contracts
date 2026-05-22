use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdtxConfig {
    pub version: u32,
    pub project: ProjectConfig,
    pub network: NetworkConfig,
    pub accounts: HashMap<String, AccountConfig>,
    pub contracts: ContractsConfig,
    pub token: TokenConfig,
    #[serde(default)]
    pub access_control: Option<AccessControlConfig>,
    #[serde(default)]
    pub scenario: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub profile: String,
    #[serde(default)]
    pub rpc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountConfig {
    PrivateKeyEnv { private_key_env: String },
    Address { address: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractsConfig {
    pub source: ContractSourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum ContractSourceConfig {
    #[serde(rename = "deployed-artifacts")]
    DeployedArtifacts { scripts_json: String },
    #[serde(rename = "built")]
    Built { build_dir: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    pub kind: TokenKind,
    pub symbol: String,
    pub decimals: u8,
    #[serde(default)]
    pub supply_policy: SupplyPolicyConfig,
    #[serde(default)]
    pub authorities: AuthoritiesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Sudt,
    Xudt,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupplyPolicyConfig {
    pub mode: SupplyMode,
    #[serde(default)]
    pub fixed_after_issue: Option<FixedAfterIssueConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SupplyMode {
    #[default]
    Tracked,
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedAfterIssueConfig {
    pub enabled: bool,
    #[serde(default)]
    pub target_amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthoritiesConfig {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub access: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub enabled: bool,
    pub mode: AccessMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessMode {
    Blacklist,
    Whitelist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub action: String,
    #[serde(flatten)]
    pub params: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    pub rpc_url: String,
    pub network_type: NetworkType,
    #[serde(default)]
    pub system_scripts: HashMap<String, ScriptRef>,
    #[serde(default)]
    pub contracts: HashMap<String, ContractRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Devnet,
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRef {
    pub code_hash: String,
    pub hash_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRef {
    pub code_hash: String,
    pub hash_type: String,
    pub outpoint: OutpointRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutpointRef {
    pub tx_hash: String,
    pub index: u32,
}

impl UdtxConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::Validation(format!(
                "Unsupported config version: {}. Expected 1.",
                self.version
            )));
        }

        if self.project.name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "Project name cannot be empty".into(),
            ));
        }

        if self.network.profile.trim().is_empty() {
            return Err(ConfigError::Validation(
                "Network profile cannot be empty".into(),
            ));
        }

        if self.accounts.is_empty() {
            return Err(ConfigError::Validation(
                "At least one account must be configured".into(),
            ));
        }

        let auth = &self.token.authorities;
        for (role, account_ref) in [
            ("mint", auth.mint.as_ref()),
            ("metadata", auth.metadata.as_ref()),
            ("access", auth.access.as_ref()),
        ] {
            if let Some(ref_name) = account_ref {
                if !self.accounts.contains_key(ref_name) && ref_name != &"owner_lock" {
                    return Err(ConfigError::Validation(format!(
                        "Authority '{}' references unknown account: {}",
                        role, ref_name
                    )));
                }
            }
        }

        if self.token.symbol.is_empty() || self.token.symbol.len() > 128 {
            return Err(ConfigError::Validation(
                "Token symbol must be 1-128 characters".into(),
            ));
        }



        if let Some(ref ac) = self.access_control {
            match self.token.kind {
                TokenKind::Sudt if ac.enabled => {
                    return Err(ConfigError::Validation(
                        "Access control is not supported for sUDT".into(),
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl ProfileConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "Profile name cannot be empty".into(),
            ));
        }

        if self.rpc_url.trim().is_empty() {
            return Err(ConfigError::Validation(
                "RPC URL cannot be empty".into(),
            ));
        }

        if !self.rpc_url.starts_with("http://") && !self.rpc_url.starts_with("https://") {
            return Err(ConfigError::Validation(
                "RPC URL must start with http:// or https://".into(),
            ));
        }

        Ok(())
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<UdtxConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let mut config: UdtxConfig = serde_yaml::from_str(&content)?;

    for (_name, account) in config.accounts.iter_mut() {
        if let AccountConfig::PrivateKeyEnv { private_key_env } = account {
            if let Ok(val) = std::env::var(private_key_env) {
                let _ = val;
            }
        }
    }

    if let Ok(rpc_url) = std::env::var("UDTX_RPC_URL") {
        config.network.rpc = Some(rpc_url);
    }

    config.validate()?;
    Ok(config)
}

pub fn load_profile<P: AsRef<Path>>(path: P) -> Result<ProfileConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let profile: ProfileConfig = serde_yaml::from_str(&content)?;
    profile.validate()?;
    Ok(profile)
}

pub fn resolve_profile_path<P: AsRef<Path>>(project_root: P, profile_name: &str) -> PathBuf {
    project_root.as_ref().join("profiles").join(format!("{}.yaml", profile_name))
}

pub fn load_config_with_profile<P: AsRef<Path>>(config_path: P) -> Result<(UdtxConfig, ProfileConfig), ConfigError> {
    let config = load_config(&config_path)?;
    let project_root = config_path.as_ref().parent().unwrap_or_else(|| Path::new("."));
    let profile_path = resolve_profile_path(project_root, &config.network.profile);

    if !profile_path.exists() {
        return Err(ConfigError::ProfileNotFound(format!(
            "Profile '{}' not found at expected path: {}",
            config.network.profile,
            profile_path.display()
        )));
    }

    let profile = load_profile(profile_path)?;
    Ok((config, profile))
}

pub fn default_config(project_name: &str) -> UdtxConfig {
    let mut accounts = HashMap::new();
    accounts.insert(
        "owner".to_string(),
        AccountConfig::PrivateKeyEnv {
            private_key_env: "OWNER_PRIVKEY".to_string(),
        },
    );

    UdtxConfig {
        version: 1,
        project: ProjectConfig {
            name: project_name.to_string(),
        },
        network: NetworkConfig {
            profile: "devnet".to_string(),
            rpc: None,
        },
        accounts,
        contracts: ContractsConfig {
            source: ContractSourceConfig::DeployedArtifacts {
                scripts_json: "./artifacts/devnet-scripts.json".to_string(),
            },
        },
        token: TokenConfig {
            kind: TokenKind::Xudt,
            symbol: "XD".to_string(),
            decimals: 8,
            supply_policy: SupplyPolicyConfig {
                mode: SupplyMode::Tracked,
                fixed_after_issue: Some(FixedAfterIssueConfig {
                    enabled: true,
                    target_amount: Some("1000000".to_string()),
                }),
            },
            authorities: AuthoritiesConfig {
                mint: Some("owner".to_string()),
                metadata: Some("owner".to_string()),
                access: Some("owner".to_string()),
            },
        },
        access_control: Some(AccessControlConfig {
            enabled: true,
            mode: AccessMode::Blacklist,
        }),
        scenario: vec![],
    }
}

pub fn default_devnet_profile() -> ProfileConfig {
    let mut system_scripts = HashMap::new();
    system_scripts.insert(
        "secp256k1_blake160".to_string(),
        ScriptRef {
            code_hash: "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8".to_string(),
            hash_type: "type".to_string(),
        },
    );

    let mut contracts = HashMap::new();
    contracts.insert(
        "sudt".to_string(),
        ContractRef {
            code_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hash_type: "data1".to_string(),
            outpoint: OutpointRef {
                tx_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                index: 0,
            },
        },
    );
    contracts.insert(
        "xudt".to_string(),
        ContractRef {
            code_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hash_type: "data1".to_string(),
            outpoint: OutpointRef {
                tx_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                index: 0,
            },
        },
    );
    contracts.insert(
        "access_list".to_string(),
        ContractRef {
            code_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hash_type: "data1".to_string(),
            outpoint: OutpointRef {
                tx_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                index: 0,
            },
        },
    );

    ProfileConfig {
        name: "devnet".to_string(),
        rpc_url: "http://127.0.0.1:8114".to_string(),
        network_type: NetworkType::Devnet,
        system_scripts,
        contracts,
    }
}

pub fn init_project<P: AsRef<Path>>(project_path: P, project_name: &str) -> Result<(), ConfigError> {
    let path = project_path.as_ref();

    fs::create_dir_all(path.join("profiles"))?;
    fs::create_dir_all(path.join("artifacts"))?;

    let config = default_config(project_name);
    let config_yaml = serde_yaml::to_string(&config)?;
    fs::write(path.join("udtx.yaml"), config_yaml)?;

    let profile = default_devnet_profile();
    let profile_yaml = serde_yaml::to_string(&profile)?;
    fs::write(path.join("profiles").join("devnet.yaml"), profile_yaml)?;

    Ok(())
}
