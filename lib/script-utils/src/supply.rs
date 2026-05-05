use crate::error::ScriptError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyDelta {
    Increase(u128),
    Decrease(u128),
    Unchanged,
}

pub fn classify_supply_delta(input: u128, output: u128) -> Result<SupplyDelta, ScriptError> {
    if output > input {
        Ok(SupplyDelta::Increase(
            output
                .checked_sub(input)
                .ok_or(ScriptError::AmountOverflow)?,
        ))
    } else if input > output {
        Ok(SupplyDelta::Decrease(
            input
                .checked_sub(output)
                .ok_or(ScriptError::AmountOverflow)?,
        ))
    } else {
        Ok(SupplyDelta::Unchanged)
    }
}

pub fn apply_supply_delta(current_supply: u128, delta: SupplyDelta) -> Result<u128, ScriptError> {
    match delta {
        SupplyDelta::Increase(value) => current_supply
            .checked_add(value)
            .ok_or(ScriptError::SupplyOverflow),
        SupplyDelta::Decrease(value) => current_supply
            .checked_sub(value)
            .ok_or(ScriptError::SupplyUnderflow),
        SupplyDelta::Unchanged => Ok(current_supply),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_supply_delta_direction() {
        assert_eq!(classify_supply_delta(10, 25), Ok(SupplyDelta::Increase(15)));
        assert_eq!(classify_supply_delta(25, 10), Ok(SupplyDelta::Decrease(15)));
        assert_eq!(classify_supply_delta(10, 10), Ok(SupplyDelta::Unchanged));
    }

    #[test]
    fn applies_supply_delta_to_current_supply() {
        assert_eq!(apply_supply_delta(10, SupplyDelta::Increase(5)), Ok(15));
        assert_eq!(apply_supply_delta(10, SupplyDelta::Decrease(5)), Ok(5));
        assert_eq!(apply_supply_delta(10, SupplyDelta::Unchanged), Ok(10));
    }

    #[test]
    fn rejects_supply_overflow_and_underflow() {
        assert_eq!(
            apply_supply_delta(u128::MAX, SupplyDelta::Increase(1)),
            Err(ScriptError::SupplyOverflow)
        );
        assert_eq!(
            apply_supply_delta(0, SupplyDelta::Decrease(1)),
            Err(ScriptError::SupplyUnderflow)
        );
    }
}
