use crate::config::{AccountConfig, NetworkType, ProfileConfig};
use crate::error::TokenCliError;
use ckb_hash::blake2b_256;
use ckb_types::core::ScriptHashType;
use ckb_types::packed::{Byte32, Bytes, Script};
use ckb_types::prelude::*;
use secp256k1::{Message, PublicKey, SecretKey};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use zeroize::{Zeroize, ZeroizeOnDrop};

const SECP256K1_BLAKE160_CODE_HASH: &str =
    "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8";

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AccountKey {
    #[zeroize(skip)]
    pub address: String,
    #[zeroize(skip)]
    pub lock_script: Script,
    pub private_key: [u8; 32],
}

pub struct KeyManager {
    accounts: HashMap<String, AccountKey>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn load_account(
        &mut self,
        name: &str,
        config: &AccountConfig,
        profile: &ProfileConfig,
    ) -> Result<&AccountKey, TokenCliError> {
        if self.accounts.contains_key(name) {
            return self.accounts.get(name).ok_or_else(|| TokenCliError::AuthFailed {
                role: format!("account '{}' could not be retrieved", name),
            });
        }

        let account = match config {
            AccountConfig::PrivateKeyEnv { private_key_env } => {
                let hex_str = std::env::var(private_key_env).map_err(|_| {
                    TokenCliError::AuthFailed {
                        role: format!(
                            "account '{}' (env var '{}' not set)",
                            name, private_key_env
                        ),
                    }
                })?;
                let pk_bytes = parse_private_key_hex(&hex_str).map_err(|e| {
                    TokenCliError::AuthFailed {
                        role: format!("account '{}': {}", name, e),
                    }
                })?;
                derive_account(&pk_bytes, profile)?
            }
            AccountConfig::Address { address } => AccountKey {
                private_key: [0u8; 32],
                address: address.clone(),
                lock_script: address_to_lock_script(address, profile)?,
            },
        };

        self.accounts.insert(name.to_string(), account);
        self.accounts
            .get(name)
            .ok_or_else(|| TokenCliError::AuthFailed {
                role: format!("account '{}' could not be cached", name),
            })
    }

    pub fn get_account(&self, name: &str) -> Option<&AccountKey> {
        self.accounts.get(name)
    }

    pub fn sign(&self, name: &str, message: &[u8]) -> Result<Vec<u8>, TokenCliError> {
        let account = self.accounts.get(name).ok_or_else(|| TokenCliError::AuthMissing {
            role: format!("account '{}' not loaded", name),
        })?;

        if account.private_key == [0u8; 32] {
            return Err(TokenCliError::AuthFailed {
                role: format!("account '{}' is address-only and cannot sign", name),
            });
        }

        let secret_key = SecretKey::from_slice(&account.private_key).map_err(|e| {
            TokenCliError::AuthFailed {
                role: format!("invalid private key for '{}': {}", name, e),
            }
        })?;

        let msg = Message::from_digest_slice(message).map_err(|e| TokenCliError::AuthFailed {
            role: format!("invalid message hash for '{}': {}", name, e),
        })?;

        let secp = secp256k1::Secp256k1::new();
        let sig = secp.sign_ecdsa_recoverable(&msg, &secret_key);
        let (recovery_id, bytes) = sig.serialize_compact();

        let mut full_sig = Vec::with_capacity(65);
        full_sig.extend_from_slice(&bytes);
        full_sig.push(recovery_id.to_i32() as u8);
        Ok(full_sig)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_private_key_hex(hex_str: &str) -> Result<[u8; 32], String> {
    let trimmed = hex_str.trim();
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let without_prefix = without_prefix.strip_prefix("0X").unwrap_or(without_prefix);

    if without_prefix.len() != 64 {
        return Err(format!(
            "private key must be 64 hex digits, got {}",
            without_prefix.len()
        ));
    }

    let mut bytes = [0u8; 32];
    hex::decode_to_slice(without_prefix, &mut bytes)
        .map_err(|e| format!("invalid hex: {}", e))?;
    Ok(bytes)
}

fn derive_account(
    private_key: &[u8; 32],
    profile: &ProfileConfig,
) -> Result<AccountKey, TokenCliError> {
    let secp = secp256k1::Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(private_key).map_err(|e| TokenCliError::AuthFailed {
            role: format!("invalid private key: {}", e),
        })?;

    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let compressed_pk = public_key.serialize();

    let hash = blake2b_256(&compressed_pk);
    let args = &hash[..20];

    let code_hash = Byte32::from_slice(
        &hex::decode(SECP256K1_BLAKE160_CODE_HASH.trim_start_matches("0x"))
            .map_err(|e| TokenCliError::AuthFailed {
                role: format!("invalid code hash hex: {}", e),
            })?,
    )
    .map_err(|e| TokenCliError::AuthFailed {
        role: format!("invalid code hash bytes: {}", e),
    })?;

    let lock_script = Script::new_builder()
        .code_hash(code_hash)
        .hash_type(ScriptHashType::Type)
        .args(Bytes::from(args.to_vec()))
        .build();

    let address = lock_script_to_address(&lock_script, profile.network_type.clone())?;

    Ok(AccountKey {
        private_key: *private_key,
        address,
        lock_script,
    })
}

fn lock_script_to_address(
    lock_script: &Script,
    network: NetworkType,
) -> Result<String, TokenCliError> {
    let payload_bytes = {
        let mut v = Vec::with_capacity(22);
        v.push(0x01);
        v.push(0x00);
        v.extend_from_slice(&lock_script.args().raw_data());
        v
    };

    let hrp = match network {
        NetworkType::Mainnet => "ckb",
        NetworkType::Testnet | NetworkType::Devnet => "ckt",
    };

    bech32::encode(hrp, payload_bytes.to_base32(), bech32::Variant::Bech32m)
        .map_err(|e| TokenCliError::TxBuild {
            message: format!("bech32 encoding failed: {}", e),
        })
}

fn address_to_lock_script(
    address: &str,
    profile: &ProfileConfig,
) -> Result<Script, TokenCliError> {
    let (hrp, data, variant) = bech32::decode(address).map_err(|e| TokenCliError::TxBuild {
        message: format!("invalid address '{}': {}", address, e),
    })?;

    let expected_hrp = match profile.network_type {
        NetworkType::Mainnet => "ckb",
        NetworkType::Testnet | NetworkType::Devnet => "ckt",
    };

    if hrp != expected_hrp {
        return Err(TokenCliError::TxBuild {
            message: format!(
                "address network mismatch: expected '{}' prefix, got '{}'",
                expected_hrp, hrp
            ),
        });
    }

    if variant != bech32::Variant::Bech32m {
        return Err(TokenCliError::TxBuild {
            message: "address uses wrong bech32 variant".to_string(),
        });
    }

    let bytes: Vec<u8> = from_base32(&data).map_err(|e| TokenCliError::TxBuild {
        message: format!("address base32 decode failed: {}", e),
    })?;

    if bytes.len() != 22 || bytes[0] != 0x01 || bytes[1] != 0x00 {
        return Err(TokenCliError::TxBuild {
            message: "address is not a short-format secp256k1-blake160 address".to_string(),
        });
    }

    let code_hash = Byte32::from_slice(
        &hex::decode(SECP256K1_BLAKE160_CODE_HASH.trim_start_matches("0x")).map_err(|e| {
            TokenCliError::TxBuild {
                message: format!("invalid code hash hex: {}", e),
            }
        })?,
    )
    .map_err(|e| TokenCliError::TxBuild {
        message: format!("invalid code hash bytes: {}", e),
    })?;

    Ok(Script::new_builder()
        .code_hash(code_hash)
        .hash_type(ScriptHashType::Type)
        .args(Bytes::from(bytes[2..].to_vec()))
        .build())
}

pub fn load_private_key_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<[u8; 32], TokenCliError> {
    let mut contents = fs::read_to_string(path).map_err(|e| TokenCliError::AuthFailed {
        role: format!("failed to read private key file: {}", e),
    })?;

    let result = parse_private_key_hex(&contents).map_err(|e| TokenCliError::AuthFailed {
        role: format!("invalid private key in file: {}", e),
    });

    contents.zeroize();
    result
}

pub fn load_private_key_from_env(env_var: &str) -> Result<[u8; 32], TokenCliError> {
    let hex_str = std::env::var(env_var).map_err(|_| TokenCliError::AuthFailed {
        role: format!("environment variable '{}' not set", env_var),
    })?;
    parse_private_key_hex(&hex_str).map_err(|e| TokenCliError::AuthFailed {
        role: format!("invalid private key in '{}': {}", env_var, e),
    })
}

pub fn derive_address(
    private_key: &[u8; 32],
    network: NetworkType,
) -> Result<String, TokenCliError> {
    let secp = secp256k1::Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(private_key).map_err(|e| TokenCliError::AuthFailed {
            role: format!("invalid private key: {}", e),
        })?;

    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let compressed_pk = public_key.serialize();
    let hash = blake2b_256(&compressed_pk);
    let args = &hash[..20];

    let code_hash = Byte32::from_slice(
        &hex::decode(SECP256K1_BLAKE160_CODE_HASH.trim_start_matches("0x")).map_err(|e| {
            TokenCliError::AuthFailed {
                role: format!("invalid code hash hex: {}", e),
            }
        })?,
    )
    .map_err(|e| TokenCliError::AuthFailed {
        role: format!("invalid code hash bytes: {}", e),
    })?;

    let lock_script = Script::new_builder()
        .code_hash(code_hash)
        .hash_type(ScriptHashType::Type)
        .args(Bytes::from(args.to_vec()))
        .build();

    lock_script_to_address(&lock_script, network)
}

pub fn sign_message(message: &[u8], private_key: &[u8; 32]) -> Result<Vec<u8>, TokenCliError> {
    let secp = secp256k1::Secp256k1::new();
    let secret_key =
        SecretKey::from_slice(private_key).map_err(|e| TokenCliError::AuthFailed {
            role: format!("invalid private key: {}", e),
        })?;

    let msg = Message::from_digest_slice(message).map_err(|e| TokenCliError::AuthFailed {
        role: format!("invalid message hash: {}", e),
    })?;

    let sig = secp.sign_ecdsa_recoverable(&msg, &secret_key);
    let (recovery_id, bytes) = sig.serialize_compact();

    let mut full_sig = Vec::with_capacity(65);
    full_sig.extend_from_slice(&bytes);
    full_sig.push(recovery_id.to_i32() as u8);
    Ok(full_sig)
}

trait ToBase32 {
    fn to_base32(&self) -> Vec<bech32::u5>;
}

impl ToBase32 for [u8] {
    fn to_base32(&self) -> Vec<bech32::u5> {
        let mut out = Vec::with_capacity(self.len() * 8 / 5 + 1);
        let mut buffer: u16 = 0;
        let mut bits: u8 = 0;

        for &byte in self {
            buffer = (buffer << 8) | u16::from(byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                let val = ((buffer >> bits) & 0x1f) as u8;
                out.push(bech32::u5::try_from_u8(val).unwrap_or(bech32::u5::try_from_u8(0).unwrap()));
            }
        }

        if bits > 0 {
            let val = ((buffer << (5 - bits)) & 0x1f) as u8;
            out.push(bech32::u5::try_from_u8(val).unwrap_or(bech32::u5::try_from_u8(0).unwrap()));
        }

        out
    }
}

pub fn from_base32(data: &[bech32::u5]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(data.len() * 5 / 8);
    let mut buffer: u16 = 0;
    let mut bits: u8 = 0;

    for &u5 in data {
        buffer = (buffer << 5) | u16::from(bech32::u5::to_u8(u5));
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }

    if bits >= 5 {
        return Err("invalid padding".to_string());
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_private_key_hex_with_prefix() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let pk = parse_private_key_hex(hex).unwrap();
        assert_eq!(pk.len(), 32);
        assert_eq!(pk[0], 0x12);
        assert_eq!(pk[31], 0xef);
    }

    #[test]
    fn test_parse_private_key_hex_without_prefix() {
        let hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let pk = parse_private_key_hex(hex).unwrap();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_parse_private_key_hex_wrong_length() {
        let hex = "0x1234";
        assert!(parse_private_key_hex(hex).is_err());
    }

    #[test]
    fn test_derive_address_roundtrip() {
        let pk_hex = "0x6109170b275a85a849c715debbe82f0ea1e95c2c65dbd0a2df13c00c5a9ff4d2";
        let pk = parse_private_key_hex(pk_hex).unwrap();

        let address = derive_address(&pk, NetworkType::Devnet).unwrap();
        assert!(address.starts_with("ckt1"));

        let address2 = derive_address(&pk, NetworkType::Devnet).unwrap();
        assert_eq!(address, address2);
    }

    #[test]
    fn test_key_manager_load_and_sign() {
        let pk_hex = "0x6109170b275a85a849c715debbe82f0ea1e95c2c65dbd0a2df13c00c5a9ff4d2";
        let _pk = parse_private_key_hex(pk_hex).unwrap();

        let mut km = KeyManager::new();

        let profile = ProfileConfig {
            name: "devnet".to_string(),
            rpc_url: "http://127.0.0.1:8114".to_string(),
            network_type: NetworkType::Devnet,
            system_scripts: Default::default(),
            contracts: Default::default(),
        };

        let config = AccountConfig::PrivateKeyEnv {
            private_key_env: "TEST_UDTX_PK".to_string(),
        };

        std::env::set_var("TEST_UDTX_PK", pk_hex);

        let account = km.load_account("owner", &config, &profile).unwrap();
        assert!(account.address.starts_with("ckt1"));

        let msg = [0u8; 32];
        let sig = km.sign("owner", &msg).unwrap();
        assert_eq!(sig.len(), 65);
    }

    #[test]
    fn test_address_only_account_cannot_sign() {
        let mut km = KeyManager::new();
        let profile = ProfileConfig {
            name: "devnet".to_string(),
            rpc_url: "http://127.0.0.1:8114".to_string(),
            network_type: NetworkType::Devnet,
            system_scripts: Default::default(),
            contracts: Default::default(),
        };

        let config = AccountConfig::Address {
            address: "ckt1qyq2g49lzgt63949kmtklrxmtpytwf0v7etsjg2j4h".to_string(),
        };

        let account = km.load_account("watch", &config, &profile).unwrap();
        assert!(account.private_key == [0u8; 32]);

        let msg = [0u8; 32];
        assert!(km.sign("watch", &msg).is_err());
    }
}
