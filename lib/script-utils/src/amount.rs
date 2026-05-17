use ckb_std::{ckb_constants::Source, error::SysError, syscalls};

use crate::error::ScriptError;

const UDT_AMOUNT_LEN: usize = 16;

pub fn decode_amount(data: &[u8]) -> Result<u128, ScriptError> {
    if data.len() < UDT_AMOUNT_LEN {
        return Err(ScriptError::AmountEncoding);
    }

    let mut raw = [0u8; UDT_AMOUNT_LEN];
    raw.copy_from_slice(&data[..UDT_AMOUNT_LEN]);
    Ok(u128::from_le_bytes(raw))
}

pub fn load_cell_amount(index: usize, source: Source) -> Result<Option<u128>, ScriptError> {
    let mut raw = [0u8; UDT_AMOUNT_LEN];
    decode_loaded_amount(syscalls::load_cell_data(&mut raw, 0, index, source), &raw)
}

fn decode_loaded_amount(
    result: Result<usize, SysError>,
    raw: &[u8; UDT_AMOUNT_LEN],
) -> Result<Option<u128>, ScriptError> {
    match result {
        Ok(len) if len < UDT_AMOUNT_LEN => Err(ScriptError::AmountEncoding),
        Ok(_) => Ok(Some(u128::from_le_bytes(*raw))),
        Err(SysError::LengthNotEnough(len)) if len >= UDT_AMOUNT_LEN => {
            Ok(Some(u128::from_le_bytes(*raw)))
        }
        Err(SysError::LengthNotEnough(_)) => Err(ScriptError::AmountEncoding),
        Err(SysError::IndexOutOfBound) => Ok(None),
        Err(_) => Err(ScriptError::SyscallUnknown),
    }
}

pub fn collect_group_amount(source: Source) -> Result<u128, ScriptError> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        match load_cell_amount(index, source)? {
            Some(amount) => {
                total = total
                    .checked_add(amount)
                    .ok_or(ScriptError::AmountOverflow)?;
                index += 1;
            }
            None => return Ok(total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_little_endian_u128_amount() {
        let amount = 0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00u128;

        assert_eq!(decode_amount(&amount.to_le_bytes()), Ok(amount));
    }

    #[test]
    fn rejects_amount_data_shorter_than_sixteen_bytes() {
        assert_eq!(decode_amount(&[0u8; 15]), Err(ScriptError::AmountEncoding));
    }

    #[test]
    fn ignores_trailing_amount_data_bytes() {
        let amount = 0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00u128;
        let mut data = amount.to_le_bytes().to_vec();
        data.extend_from_slice(&[0xff; 8]);

        assert_eq!(decode_amount(&data), Ok(amount));
    }

    #[test]
    fn maps_length_not_enough_to_amount_prefix() {
        let amount = 0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00u128;
        let raw = amount.to_le_bytes();

        assert_eq!(
            decode_loaded_amount(Err(SysError::LengthNotEnough(24)), &raw),
            Ok(Some(amount))
        );
        assert_eq!(
            decode_loaded_amount(Err(SysError::LengthNotEnough(15)), &raw),
            Err(ScriptError::AmountEncoding)
        );
    }
}
