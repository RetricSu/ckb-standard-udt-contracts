use ckb_std::error::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Syscall,
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
            Self::Syscall => 1,
            Self::InvalidArgs => 2,
            Self::InvalidMetaData => 3,
            Self::MetaMissing => 4,
            Self::MetaNotUnique => 5,
            Self::MetaLockNotAllowed => 6,
            Self::AuthorityMissing => 7,
            Self::AuthorityFailed => 8,
            Self::UnsupportedAuthorityLocation => 9,
            Self::InvalidShardData | Self::InvalidShardSet => 3,
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
