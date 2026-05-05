use ckb_testtool::{
    ckb_error::Error,
    ckb_types::{
        bytes::Bytes,
        core::{Cycle, TransactionView},
    },
    context::Context,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

pub mod fixtures;
pub mod metadata_builders;

// The exact same Loader code from capsule's template, except that
// now we use MODE as the environment variable
const TEST_ENV_VAR: &str = "MODE";

pub enum TestEnv {
    Debug,
    Release,
}

impl FromStr for TestEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(TestEnv::Debug),
            "release" => Ok(TestEnv::Release),
            _ => Err("no match"),
        }
    }
}

pub struct Loader(PathBuf);

impl Default for Loader {
    fn default() -> Self {
        let test_env = match env::var(TEST_ENV_VAR) {
            Ok(val) => val.parse().expect("test env"),
            Err(_) => TestEnv::Release,
        };
        Self::with_test_env(test_env)
    }
}

impl Loader {
    fn with_test_env(env: TestEnv) -> Self {
        let load_prefix = match env {
            TestEnv::Debug => "debug",
            TestEnv::Release => "release",
        };
        let mut base_path = match env::var("TOP") {
            Ok(val) => {
                let mut base_path: PathBuf = val.into();
                base_path.push("build");
                base_path
            }
            Err(_) => {
                let mut base_path = PathBuf::new();
                // cargo may use a different cwd when running tests, for example:
                // when running debug in vscode, it will use workspace root as cwd by default,
                // when running test by `cargo test`, it will use tests directory as cwd,
                // so we need a fallback path
                base_path.push("build");
                if !base_path.exists() {
                    base_path.pop();
                    base_path.push("..");
                    base_path.push("build");
                }
                base_path
            }
        };

        base_path.push(load_prefix);
        Loader(base_path)
    }

    pub fn load_binary(&self, name: &str) -> Bytes {
        let mut path = self.0.clone();
        path.push(name);
        let result = fs::read(&path);
        if result.is_err() {
            panic!("Binary {:?} is missing!", path);
        }
        result.unwrap().into()
    }
}

// This helper method runs Context::verify_tx, but in case error happens,
// it also dumps current transaction to failed_txs folder.
pub fn verify_and_dump_failed_tx(
    context: &Context,
    tx: &TransactionView,
    max_cycles: u64,
) -> Result<Cycle, Error> {
    let result = context.verify_tx(tx, max_cycles);
    let should_dump = matches!(env::var("DUMP_FAILED_TXS").as_deref(), Ok("1"));
    if result.is_err() && should_dump {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("failed_txs");
        match std::fs::create_dir_all(&path) {
            Ok(()) => {
                let file_name = format!("0x{:x}.json", tx.hash());
                let file_path = path.join(file_name);
                match context.dump_tx(tx) {
                    Ok(mock_tx) => match serde_json::to_string_pretty(&mock_tx) {
                        Ok(json) => match std::fs::write(&file_path, json) {
                            Ok(()) => println!("Failed tx written to {:?}", file_path),
                            Err(err) => {
                                eprintln!("Failed to write tx dump {:?}: {}", file_path, err)
                            }
                        },
                        Err(err) => eprintln!("Failed to serialize dumped tx: {}", err),
                    },
                    Err(err) => eprintln!("Failed to dump tx: {}", err),
                }
            }
            Err(err) => eprintln!("Failed to create failed_txs dir {:?}: {}", path, err),
        }
    }
    result
}
