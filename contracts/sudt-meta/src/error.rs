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
            Self::SysIndexOutOfBound => 9,
            Self::SysItemMissing => 10,
            Self::SysLengthNotEnough => 11,
            Self::SysEncoding => 12,
            Self::SysWaitFailure => 13,
            Self::SysInvalidFd => 14,
            Self::SysOtherEndClosed => 15,
            Self::SysMaxVmsSpawned => 16,
            Self::SysMaxFdsCreated => 17,
            Self::SysTypeIdError => 18,
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
