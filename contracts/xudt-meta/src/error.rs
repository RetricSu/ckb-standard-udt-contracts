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
    SysTypeIdError,
    SyscallUnknown,
    InvalidArgs,
    DuplicateMetaCell,
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
            Self::SysIndexOutOfBound => 1,
            Self::SysItemMissing => 2,
            Self::SysLengthNotEnough => 3,
            Self::SysEncoding => 4,
            Self::SysWaitFailure => 5,
            Self::SysInvalidFd => 6,
            Self::SysOtherEndClosed => 7,
            Self::SysMaxVmsSpawned => 8,
            Self::SysMaxFdsCreated => 9,
            Self::SysTypeIdError => 10,
            Self::SyscallUnknown => 11,
            Self::InvalidArgs => 20,
            Self::DuplicateMetaCell => 21,
            Self::InvalidMetaData => 30,
            Self::InvalidSupply => 31,
            Self::ImmutableSupplyMode => 32,
            Self::AuthorityMissing => 50,
            Self::AuthorityFailed => 51,
            Self::AccessListRequired => 60,
            Self::AccessModeTokenCells => 61,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}

impl From<TypesError> for Error {
    fn from(error: TypesError) -> Self {
        match error {
            TypesError::InvalidSupply => Self::InvalidSupply,
            _ => Self::InvalidMetaData,
        }
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
