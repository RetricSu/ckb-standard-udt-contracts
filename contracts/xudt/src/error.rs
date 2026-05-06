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
    InvalidShardData,
    AccessDenied,
    ExtensionFailed,
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
            Self::AmountEncoding => 30,
            Self::AmountOverflow => 31,
            Self::SupplyOverflow => 32,
            Self::SupplyUnderflow => 33,
            Self::InvalidMetaData => 40,
            Self::MetaMissing => 41,
            Self::MetaNotUnique => 42,
            Self::MetaInputMissing => 43,
            Self::MetaOutputMissing => 44,
            Self::MetaStateMismatch => 45,
            Self::AuthorityMissing => 50,
            Self::AuthorityFailed => 51,
            Self::UnsupportedAuthorityLocation => 52,
            Self::InvalidShardData => 60,
            Self::AccessDenied => 61,
            Self::ExtensionFailed => 70,
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
