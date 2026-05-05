use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=ENHANCED_SUDT_CODE_HASH");

    let raw = env::var("ENHANCED_SUDT_CODE_HASH").expect(
        "ENHANCED_SUDT_CODE_HASH must be set to the 64-character enhanced-sudt Data2 code hash",
    );
    let bytes = parse_code_hash(&raw);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let constants = format!(
        "pub const ENHANCED_SUDT_CODE_HASH: [u8; 32] = {:?};\n",
        bytes
    );
    fs::write(out_dir.join("generated_constants.rs"), constants)
        .expect("write generated enhanced-sudt-meta constants");
}

fn parse_code_hash(raw: &str) -> [u8; 32] {
    if raw.len() != 64 {
        panic!("ENHANCED_SUDT_CODE_HASH must be exactly 64 hex characters");
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
        _ => panic!("ENHANCED_SUDT_CODE_HASH contains a non-hex character"),
    }
}
