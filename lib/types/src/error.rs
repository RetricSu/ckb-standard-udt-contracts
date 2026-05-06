#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Molecule,
    InvalidScriptLocation,
    InvalidScriptShape,
    InvalidScriptHash,
    InvalidConfigFlags,
    InvalidSupply,
    ExtensionsTooMany,
    ExtensionsNotSorted,
    ExtensionsDuplicated,
    MetadataTooLarge,
    AccessListTooLarge,
}

impl From<crate::molecule::error::VerificationError> for Error {
    fn from(_: crate::molecule::error::VerificationError) -> Self {
        Error::Molecule
    }
}
