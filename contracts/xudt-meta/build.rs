use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=XUDT_CODE_HASH");
    println!("cargo:rerun-if-env-changed=ACCESS_LIST_CODE_HASH");

    let xudt = load_hash("XUDT_CODE_HASH", "xudt");
    let access_list = load_hash("ACCESS_LIST_CODE_HASH", "access-list");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let constants = format!(
        "pub const XUDT_CODE_HASH: [u8; 32] = {:?};\n\
         pub const ACCESS_LIST_CODE_HASH: [u8; 32] = {:?};\n",
        xudt, access_list
    );
    fs::write(out_dir.join("generated_constants.rs"), constants)
        .expect("write generated xudt-meta constants");
}

fn load_hash(env_name: &str, contract_name: &str) -> [u8; 32] {
    match env::var(env_name) {
        Ok(raw) => parse_code_hash(env_name, &raw),
        Err(_) if is_contract_target() => {
            panic!("{env_name} must be set to the 64-character {contract_name} Data2 code hash")
        }
        Err(_) => {
            println!("cargo:warning={env_name} not set; using zero code hash for host build");
            [0u8; 32]
        }
    }
}

fn is_contract_target() -> bool {
    env::var("TARGET")
        .map(|target| target == "riscv64imac-unknown-none-elf")
        .unwrap_or(false)
}

fn parse_code_hash(env_name: &str, raw: &str) -> [u8; 32] {
    if raw.len() != 64 {
        panic!("{env_name} must be exactly 64 hex characters");
    }

    let mut bytes = [0u8; 32];
    for (index, chunk) in raw.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(env_name, chunk[0]);
        let low = hex_value(env_name, chunk[1]);
        bytes[index] = high << 4 | low;
    }
    bytes
}

fn hex_value(env_name: &str, value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("{env_name} contains a non-hex character"),
    }
}
