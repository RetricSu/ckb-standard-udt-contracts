# UDTX CLI Issues and Blockers

## Open Issues

### 1. ckb-sdk-rust xUDT Support Verification
**Status**: OPEN
**Priority**: HIGH
**Description**: Need to verify ckb-sdk-rust v5.1.0 supports xUDT with extensions
**Impact**: Blocks xUDT commands (Tasks 12-14)
**Resolution**: Review SDK source for `XudtWitnessInput` construction

### 2. Dependency Version Alignment
**Status**: OPEN
**Priority**: HIGH
**Description**: ckb-sdk-rust, ckb-types, ckb-jsonrpc-types versions must align with workspace
**Impact**: Blocks all tasks
**Resolution**: Pin compatible versions in workspace Cargo.toml

### 3. Contract Reference Format
**Status**: OPEN
**Priority**: MEDIUM
**Description**: Config should contain outpoints or type script hashes?
**Impact**: Blocks config system (Task 2)
**Resolution**: Use outpoints for direct reference, with code hash verification

### 4. No CI/CD Infrastructure
**Status**: OPEN
**Priority**: MEDIUM
**Description**: No existing GitHub Actions or other CI
**Impact**: Blocks Task 27
**Resolution**: Create new CI workflow from scratch

## Resolved Issues

### 1. CLI Location
**Status**: RESOLVED
**Decision**: New workspace member `udtx/`, not under `lib/`
**Reason**: CLI is a binary tool, not a shared library

### 2. Lock Script Support
**Status**: RESOLVED
**Decision**: MVP only secp256k1-blake160
**Reason**: Keep MVP focused, add Omnilock/ACP in v2

### 3. xUDT Extension Support
**Status**: RESOLVED
**Decision**: MVP only AccessList
**Reason**: Other extensions (RCE) require SMT proofs, too complex for MVP

### 4. Report Format
**Status**: RESOLVED
**Decision**: Both Markdown and JSON
**Reason**: Human-readable + machine-readable for ecosystem integration

## Potential Blockers

### 1. offckb Compatibility
**Risk**: offckb API may change
**Mitigation**: Use shell commands (stable interface) rather than library dependency

### 2. Cell Dep Management
**Risk**: Every transaction needs correct cell deps
**Mitigation**: Maintain registry of known script deployments per network

### 3. Fee Estimation
**Risk**: CKB fee model is complex (cycles + size)
**Mitigation**: Use `estimate_fee_rate` RPC with 20% buffer

### 4. Concurrent Operations
**Risk**: Two plan files targeting same inputs
**Mitigation**: Document as limitation; suggest sequential execution

## Notes

- `lib/script-utils` is `no_std` - CLI cannot use it directly
- Contract code hashes are build-time generated via `include!` macro
- `ckb-testtool` uses `native-simulator` feature for faster testing
- Debug builds include test-oriented allowances behind `#[cfg(debug_assertions)]`
