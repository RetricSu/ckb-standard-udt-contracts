use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::load_cell_type_hash,
};

use crate::{
    amount::load_cell_amount,
    cells::bound_type_hash,
    error::ScriptError,
    supply::{classify_supply_delta, SupplyDelta},
};

pub fn matches_bound_type_script(
    type_script: &Script,
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> bool {
    if type_script.hash_type() != ScriptHashType::Data2.into() {
        return false;
    }
    if type_script.args().raw_data().as_ref() != meta_type_hash {
        return false;
    }

    let actual_code_hash: [u8; 32] = type_script.code_hash().unpack();
    &actual_code_hash == code_hash
}

pub fn sum_token_amount(
    source: Source,
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> Result<u128, ScriptError> {
    let expected_type_hash = bound_type_hash(meta_type_hash, code_hash);
    sum_amount_by_type_hash(source, &expected_type_hash)
}

pub fn transaction_token_delta(
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> Result<SupplyDelta, ScriptError> {
    let expected_type_hash = bound_type_hash(meta_type_hash, code_hash);
    let input = sum_amount_by_type_hash(Source::Input, &expected_type_hash)?;
    let output = sum_amount_by_type_hash(Source::Output, &expected_type_hash)?;
    classify_supply_delta(input, output)
}

fn sum_amount_by_type_hash(
    source: Source,
    expected_type_hash: &[u8; 32],
) -> Result<u128, ScriptError> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        let type_hash = match load_cell_type_hash(index, source) {
            Ok(type_hash) => type_hash,
            Err(SysError::IndexOutOfBound) => return Ok(total),
            Err(_) => return Err(ScriptError::SyscallUnknown),
        };

        if type_hash.as_ref() == Some(expected_type_hash) {
            let amount = load_cell_amount(index, source)?.ok_or(ScriptError::SyscallUnknown)?;
            total = total
                .checked_add(amount)
                .ok_or(ScriptError::AmountOverflow)?;
        }

        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ckb_std::ckb_types::{
        bytes::Bytes,
        core::ScriptHashType,
        packed::{Byte32, Script},
    };

    fn token_script(hash_type: ScriptHashType, code_hash: [u8; 32], args: [u8; 32]) -> Script {
        Script::new_builder()
            .code_hash(Byte32::from_slice(&code_hash).expect("byte32"))
            .hash_type(hash_type)
            .args(Bytes::from(args.to_vec()).pack())
            .build()
    }

    #[test]
    fn bound_type_script_matches_only_data2_meta_args_and_code_hash() {
        let meta_type_hash = [1u8; 32];
        let code_hash = [2u8; 32];

        let matching = token_script(ScriptHashType::Data2, code_hash, meta_type_hash);
        let data_hash_type = token_script(ScriptHashType::Data, code_hash, meta_type_hash);
        let wrong_code_hash = token_script(ScriptHashType::Data2, [3u8; 32], meta_type_hash);
        let wrong_args = token_script(ScriptHashType::Data2, code_hash, [4u8; 32]);

        assert!(matches_bound_type_script(
            &matching,
            &meta_type_hash,
            &code_hash
        ));
        assert!(!matches_bound_type_script(
            &data_hash_type,
            &meta_type_hash,
            &code_hash
        ));
        assert!(!matches_bound_type_script(
            &wrong_code_hash,
            &meta_type_hash,
            &code_hash
        ));
        assert!(!matches_bound_type_script(
            &wrong_args,
            &meta_type_hash,
            &code_hash
        ));
    }
}
