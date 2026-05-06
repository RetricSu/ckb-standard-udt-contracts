use ckb_std::error::SysError;
use standard_udt_types::error::Error as TypesError;

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
            Self::InvalidArgs => 20,
            Self::InvalidMetaData => 30,
            Self::MetaMissing => 40,
            Self::MetaNotUnique => 41,
            Self::AuthorityMissing => 50,
            Self::AuthorityFailed => 51,
            Self::UnsupportedAuthorityLocation => 52,
            Self::InvalidShardData => 60,
            Self::InvalidShardSet => 61,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}

impl From<TypesError> for Error {
    fn from(_error: TypesError) -> Self {
        Self::InvalidMetaData
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
