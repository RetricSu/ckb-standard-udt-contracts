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
            Self::Syscall => 8,
            Self::AccessListRequired => 9,
            Self::AccessModeTokenCells => 10,
        }
    }
}

impl From<Error> for i8 {
    fn from(error: Error) -> Self {
        error.code()
    }
}
