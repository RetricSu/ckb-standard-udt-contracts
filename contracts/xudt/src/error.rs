use ckb_std::error::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    SyscallUnknown,
    SysIndexOutOfBound,
    SysItemMissing,
    SysLengthNotEnough,
    SysEncoding,
    SysWaitFailure,
    SysInvalidFd,
    SysOtherEndClosed,
    SysMaxVmsSpawned,
    SysMaxFdsCreated,
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
    InvalidShardData,
    AccessDenied,
    ExtensionFailed,
}

impl Error {
    pub const fn code(self) -> i8 {
        match self {
            Self::SyscallUnknown => 1,
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
            Self::InvalidShardData => 17,
            Self::AccessDenied => 18,
            Self::ExtensionFailed => 19,
            Self::SysIndexOutOfBound => 20,
            Self::SysItemMissing => 21,
            Self::SysLengthNotEnough => 22,
            Self::SysEncoding => 23,
            Self::SysWaitFailure => 24,
            Self::SysInvalidFd => 25,
            Self::SysOtherEndClosed => 26,
            Self::SysMaxVmsSpawned => 27,
            Self::SysMaxFdsCreated => 28,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}

#[allow(unreachable_patterns)]
impl From<SysError> for Error {
    fn from(error: SysError) -> Self {
        match error {
            SysError::IndexOutOfBound => Self::SysIndexOutOfBound,
            SysError::ItemMissing => Self::SysItemMissing,
            SysError::LengthNotEnough(_) => Self::SysLengthNotEnough,
            SysError::Encoding => Self::SysEncoding,
            SysError::WaitFailure => Self::SysWaitFailure,
            SysError::InvalidFd => Self::SysInvalidFd,
            SysError::OtherEndClosed => Self::SysOtherEndClosed,
            SysError::MaxVmsSpawned => Self::SysMaxVmsSpawned,
            SysError::MaxFdsCreated => Self::SysMaxFdsCreated,
            SysError::Unknown(_) => Self::SyscallUnknown,
            _ => Self::SyscallUnknown,
        }
    }
}
