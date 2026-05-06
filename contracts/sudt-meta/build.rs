use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=SUDT_CODE_HASH");

    let bytes = match env::var("SUDT_CODE_HASH") {
        Ok(raw) => parse_code_hash(&raw),
        Err(_) if is_contract_target() => {
            panic!("SUDT_CODE_HASH must be set to the 64-character sudt Data2 code hash")
        }
        Err(_) => {
            println!(
                "cargo:warning=SUDT_CODE_HASH not set; using zero code hash for host/library/test build"
            );
            [0u8; 32]
        }
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let constants = format!("pub const SUDT_CODE_HASH: [u8; 32] = {:?};\n", bytes);
    fs::write(out_dir.join("generated_constants.rs"), constants)
        .expect("write generated sudt-meta constants");
}

fn is_contract_target() -> bool {
    env::var("TARGET")
        .map(|target| target == "riscv64imac-unknown-none-elf")
        .unwrap_or(false)
}

fn parse_code_hash(raw: &str) -> [u8; 32] {
    if raw.len() != 64 {
        panic!("SUDT_CODE_HASH must be exactly 64 hex characters");
    }

    let mut bytes = [0u8; 32];
    for (index, chunk) in raw.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0]);
        let low = hex_value(chunk[1]);
        bytes[index] = high << 4 | low;
    }
    bytes
}

fn hex_value(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("SUDT_CODE_HASH contains a non-hex character"),
    }
}
