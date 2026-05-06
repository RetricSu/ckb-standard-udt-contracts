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
            Self::AmountEncoding => 12,
            Self::AmountOverflow => 13,
            Self::SupplyOverflow => 14,
            Self::SupplyUnderflow => 15,
            Self::InvalidMetaData => 16,
            Self::MetaMissing => 17,
            Self::MetaNotUnique => 18,
            Self::MetaInputMissing => 19,
            Self::MetaOutputMissing => 20,
            Self::MetaStateMismatch => 21,
            Self::AuthorityMissing => 22,
            Self::AuthorityFailed => 23,
            Self::UnsupportedAuthorityLocation => 24,
            Self::MetaLockNotAllowed => 25,
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
    }
}
