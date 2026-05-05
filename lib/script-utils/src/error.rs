#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Syscall,
    AmountEncoding,
    AmountOverflow,
    SupplyOverflow,
    SupplyUnderflow,
    MetaMissing,
    MetaNotUnique,
    MetaInputMissing,
    MetaOutputMissing,
    MetaLockNotAllowed,
    MetaStateMismatch,
    AuthorityMissing,
    AuthorityFailed,
    UnsupportedAuthorityLocation,
}
