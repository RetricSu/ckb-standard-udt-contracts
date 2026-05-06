#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptError {
    SyscallUnknown,
    AmountEncoding,
    AmountOverflow,
    SupplyOverflow,
    SupplyUnderflow,
    MetaMissing,
    MetaNotUnique,
    MetaInputMissing,
    MetaOutputMissing,
    MetaLockNotAllowed,
    MetaStateMismatch,
    AuthorityMissing,
    AuthorityFailed,
    UnsupportedAuthorityLocation,
}

impl ScriptError {
    pub const fn code(self) -> i8 {
        match self {
            Self::SyscallUnknown => 1,
            Self::AmountEncoding => 2,
            Self::AmountOverflow => 3,
            Self::SupplyOverflow => 4,
            Self::SupplyUnderflow => 5,
            Self::MetaMissing => 6,
            Self::MetaNotUnique => 7,
            Self::MetaInputMissing => 8,
            Self::MetaOutputMissing => 9,
            Self::MetaLockNotAllowed => 10,
            Self::MetaStateMismatch => 11,
            Self::AuthorityMissing => 12,
            Self::AuthorityFailed => 13,
            Self::UnsupportedAuthorityLocation => 14,
        }
    }
}

impl From<ScriptError> for i8 {
    fn from(error: ScriptError) -> Self {
        error.code()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_nonzero_and_stable() {
        assert_eq!(ScriptError::SyscallUnknown.code(), 1);
        assert_eq!(ScriptError::AmountEncoding.code(), 2);
        assert_eq!(ScriptError::AmountOverflow.code(), 3);
        assert_eq!(ScriptError::SupplyOverflow.code(), 4);
        assert_eq!(ScriptError::SupplyUnderflow.code(), 5);
        assert_eq!(ScriptError::MetaMissing.code(), 6);
        assert_eq!(ScriptError::MetaNotUnique.code(), 7);
        assert_eq!(ScriptError::MetaInputMissing.code(), 8);
        assert_eq!(ScriptError::MetaOutputMissing.code(), 9);
        assert_eq!(ScriptError::MetaLockNotAllowed.code(), 10);
        assert_eq!(ScriptError::MetaStateMismatch.code(), 11);
        assert_eq!(ScriptError::AuthorityMissing.code(), 12);
        assert_eq!(ScriptError::AuthorityFailed.code(), 13);
        assert_eq!(ScriptError::UnsupportedAuthorityLocation.code(), 14);
    }

    #[test]
    fn converts_script_error_to_exit_code() {
        let code: i8 = ScriptError::AuthorityFailed.into();

        assert_eq!(code, 13);
    }
}
