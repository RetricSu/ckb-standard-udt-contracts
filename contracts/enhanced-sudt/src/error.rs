#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Syscall,
    InvalidArgs,
    AmountEncoding,
    AmountOverflow,
    SupplyOverflow,
    SupplyUnderflow,
    InvalidMetaData,
    MetaMissing,
    MetaNotUnique,
    MetaInputMissing,
    MetaOutputMissing,
    MetaStateMismatch,
    AuthorityMissing,
    AuthorityFailed,
    UnsupportedAuthorityLocation,
    MetaLockNotAllowed,
}

impl Error {
    pub const fn code(self) -> i8 {
        match self {
            Self::Syscall => 1,
            Self::InvalidArgs => 2,
            Self::AmountEncoding => 3,
            Self::AmountOverflow => 4,
            Self::SupplyOverflow => 5,
            Self::SupplyUnderflow => 6,
            Self::InvalidMetaData => 7,
            Self::MetaMissing => 8,
            Self::MetaNotUnique => 9,
            Self::MetaInputMissing => 10,
            Self::MetaOutputMissing => 11,
            Self::MetaStateMismatch => 12,
            Self::AuthorityMissing => 13,
            Self::AuthorityFailed => 14,
            Self::UnsupportedAuthorityLocation => 15,
            Self::MetaLockNotAllowed => 16,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}
