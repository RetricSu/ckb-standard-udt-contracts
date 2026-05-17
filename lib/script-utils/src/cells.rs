use alloc::vec::Vec;

use ckb_std::ckb_types::{
    bytes::Bytes,
    core::ScriptHashType,
    packed::{Byte32, Script},
    prelude::*,
};

pub fn bound_type_hash(meta_type_hash: &[u8; 32], code_hash: &[u8; 32]) -> [u8; 32] {
    let script = Script::new_builder()
        .code_hash(Byte32::from_slice(code_hash).expect("code hash is byte32"))
        .hash_type(ScriptHashType::Data2)
        .args(Bytes::from(Vec::from(meta_type_hash.as_slice())).pack())
        .build();
    script.calc_script_hash().unpack()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_script(hash_type: ScriptHashType, code_hash: [u8; 32], args: [u8; 32]) -> Script {
        Script::new_builder()
            .code_hash(Byte32::from_slice(&code_hash).expect("byte32"))
            .hash_type(hash_type)
            .args(Bytes::from(args.to_vec()).pack())
            .build()
    }

    #[test]
    fn bound_type_hash_matches_equivalent_data2_script() {
        let meta_type_hash = [1u8; 32];
        let code_hash = [2u8; 32];
        let matching = token_script(ScriptHashType::Data2, code_hash, meta_type_hash);
        let expected: [u8; 32] = matching.calc_script_hash().unpack();

        assert_eq!(bound_type_hash(&meta_type_hash, &code_hash), expected);
    }
}
