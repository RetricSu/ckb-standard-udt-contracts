use ckb_std::error::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Syscall,
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
}

impl Error {
    pub const fn code(self) -> i8 {
        match self {
            Self::Syscall => 1,
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
            Self::SysIndexOutOfBound => 17,
            Self::SysItemMissing => 18,
            Self::SysLengthNotEnough => 19,
            Self::SysEncoding => 20,
            Self::SysWaitFailure => 21,
            Self::SysInvalidFd => 22,
            Self::SysOtherEndClosed => 23,
            Self::SysMaxVmsSpawned => 24,
            Self::SysMaxFdsCreated => 25,
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
            SysError::Unknown(_) => Self::Syscall,
            _ => Self::Syscall,
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
