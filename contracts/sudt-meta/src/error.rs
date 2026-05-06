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
    SysTypeIdError,
    SyscallUnknown,
    InvalidArgs,
    InvalidTypeId,
    InvalidMetaData,
    InvalidSupply,
    ImmutableSupplyMode,
    AuthorityMissing,
    AuthorityFailed,
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
            Self::InvalidTypeId => 21,
            Self::InvalidMetaData => 30,
            Self::InvalidSupply => 31,
            Self::ImmutableSupplyMode => 32,
            Self::AuthorityMissing => 50,
            Self::AuthorityFailed => 51,
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

#[cfg(test)]
mod tests {
    use super::Error;
    use ckb_std::error::SysError;

    #[test]
    fn sys_errors_map_to_specific_variants() {
        assert_eq!(
            Error::from(SysError::IndexOutOfBound),
            Error::SysIndexOutOfBound
        );
        assert_eq!(Error::from(SysError::ItemMissing), Error::SysItemMissing);
        assert_eq!(
            Error::from(SysError::LengthNotEnough(32)),
            Error::SysLengthNotEnough
        );
        assert_eq!(Error::from(SysError::Encoding), Error::SysEncoding);
        assert_eq!(Error::from(SysError::WaitFailure), Error::SysWaitFailure);
        assert_eq!(Error::from(SysError::InvalidFd), Error::SysInvalidFd);
        assert_eq!(
            Error::from(SysError::OtherEndClosed),
            Error::SysOtherEndClosed
        );
        assert_eq!(
            Error::from(SysError::MaxVmsSpawned),
            Error::SysMaxVmsSpawned
        );
        assert_eq!(
            Error::from(SysError::MaxFdsCreated),
            Error::SysMaxFdsCreated
        );
        assert_eq!(Error::from(SysError::TypeIDError), Error::SysTypeIdError);
    }
}
