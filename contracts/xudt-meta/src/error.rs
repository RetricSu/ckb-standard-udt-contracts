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
    SysTypeIdError,
    InvalidArgs,
    InvalidTypeId,
    InvalidMetaData,
    InvalidSupply,
    ImmutableSupplyMode,
    AuthorityMissing,
    AuthorityFailed,
    AccessListRequired,
    AccessModeTokenCells,
}

impl Error {
    pub const fn code(self) -> i8 {
        match self {
            Self::InvalidArgs => 1,
            Self::InvalidTypeId => 2,
            Self::InvalidMetaData => 3,
            Self::InvalidSupply => 4,
            Self::ImmutableSupplyMode => 5,
            Self::AuthorityMissing => 6,
            Self::AuthorityFailed => 7,
            Self::SyscallUnknown => 8,
            Self::AccessListRequired => 9,
            Self::AccessModeTokenCells => 10,
            Self::SysIndexOutOfBound => 11,
            Self::SysItemMissing => 12,
            Self::SysLengthNotEnough => 13,
            Self::SysEncoding => 14,
            Self::SysWaitFailure => 15,
            Self::SysInvalidFd => 16,
            Self::SysOtherEndClosed => 17,
            Self::SysMaxVmsSpawned => 18,
            Self::SysMaxFdsCreated => 19,
            Self::SysTypeIdError => 20,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}

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
            SysError::TypeIDError => Self::SysTypeIdError,
            SysError::Unknown(_) => Self::SyscallUnknown,
        }
    }
}
