use ckb_std::error::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    SysIndexOutOfBound,
    SysItemMissing,
    SysLengthNotEnough,
    SysEncoding,
    SysWaitFailure,
    SysInvalidFd,
    SysOtherEndClosed,
    SysMaxVmsSpawned,
    SysMaxFdsCreated,
    SyscallUnknown,
    InvalidArgs,
    InvalidMetaData,
    MetaMissing,
    MetaNotUnique,
    MetaLockNotAllowed,
    AuthorityMissing,
    AuthorityFailed,
    UnsupportedAuthorityLocation,
    InvalidShardData,
    InvalidShardSet,
}

impl Error {
    pub const fn code(self) -> i8 {
        match self {
            Self::SysIndexOutOfBound => 1,
            Self::SysItemMissing => 2,
            Self::SysLengthNotEnough => 3,
            Self::SysEncoding => 4,
            Self::SysWaitFailure => 5,
            Self::SysInvalidFd => 6,
            Self::SysOtherEndClosed => 7,
            Self::SysMaxVmsSpawned => 8,
            Self::SysMaxFdsCreated => 9,
            Self::SyscallUnknown => 10,
            Self::InvalidArgs => 11,
            Self::InvalidMetaData => 12,
            Self::MetaMissing => 13,
            Self::MetaNotUnique => 14,
            Self::MetaLockNotAllowed => 15,
            Self::AuthorityMissing => 16,
            Self::AuthorityFailed => 17,
            Self::UnsupportedAuthorityLocation => 18,
            Self::InvalidShardData => 19,
            Self::InvalidShardSet => 20,
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
