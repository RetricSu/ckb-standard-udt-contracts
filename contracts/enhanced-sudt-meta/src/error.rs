use ckb_std::error::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidArgs,
    InvalidTypeId,
    InvalidMetaData,
    InvalidSupply,
    ImmutableSupplyMode,
    AuthorityMissing,
    AuthorityFailed,
    Syscall,
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
            Self::Syscall => 8,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}

impl From<SysError> for Error {
    fn from(_: SysError) -> Self {
        Self::Syscall
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use ckb_std::error::SysError;

    #[test]
    fn sys_error_maps_to_syscall() {
        let error = Error::from(SysError::LengthNotEnough(32));

        assert_eq!(error, Error::Syscall);
        assert_eq!(i8::from(error), 8);
    }
}
