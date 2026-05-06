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
            Self::SyscallUnknown => 1,
            Self::InvalidArgs => 2,
            Self::InvalidMetaData => 3,
            Self::MetaMissing => 4,
            Self::MetaNotUnique => 5,
            Self::MetaLockNotAllowed => 6,
            Self::AuthorityMissing => 7,
            Self::AuthorityFailed => 8,
            Self::UnsupportedAuthorityLocation => 9,
            Self::InvalidShardData | Self::InvalidShardSet => 3,
            Self::SysIndexOutOfBound => 10,
            Self::SysItemMissing => 11,
            Self::SysLengthNotEnough => 12,
            Self::SysEncoding => 13,
            Self::SysWaitFailure => 14,
            Self::SysInvalidFd => 15,
            Self::SysOtherEndClosed => 16,
            Self::SysMaxVmsSpawned => 17,
            Self::SysMaxFdsCreated => 18,
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
