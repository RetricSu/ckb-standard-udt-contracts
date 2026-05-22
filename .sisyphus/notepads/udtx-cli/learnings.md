# UDTX CLI Learnings

## Conventions

### Workspace Structure
- Root `Cargo.toml` uses `@@INSERTION_POINT@@` marker for automated crate insertion
- New crates should be added at the insertion point, not at the end
- `lib/` is for shared non-script libraries (can be created manually)
- `contracts/` is for on-chain script crates (must use `make generate`)
- CLI crate should be a workspace member, not under `lib/`

### Type System
- `lib/types` supports dual `std`/`no-std` features
- CLI should use `std` feature for host-side operations
- `lib/script-utils` is `no_std` only - CLI cannot depend on it
- Key types: `SudtMeta`, `XudtMeta`, `Authority`, `AccessListShard`, `Extension`

### Build System
- Makefile drives contract builds with `MODE=debug/release`
- `scripts/ckb-data-hash` computes code hashes for dependent contracts
- Test plugins built separately under `tests/plugins/`

## Patterns

### Test Infrastructure
- `ckb-testtool` 0.14.0 for integration tests
- Fixture pattern: `XudtFixture`, `SudtFixture`
- `Loader` struct loads contract binaries from `build/{debug,release}/`
- `MODE` env var controls which build directory to use

### Error Handling
- `ScriptError` enum in `lib/script-utils/src/error.rs`
- Product-level error codes: E_PRECHECK_CAPACITY, E_AUTH_MISSING, etc.
- Use `thiserror` for structured errors in CLI

### Transaction Building
- ckb-sdk-rust: `UdtIssueBuilder`, `UdtTransferBuilder`
- `CapacityBalancer` for fee handling
- `DefaultCellCollector` + `CellQueryOptions` for cell querying
- Amounts are little-endian u128 (16 bytes)

## Decisions

### Technology Stack
- **CLI Framework**: clap v4 with derive macros
- **Chain SDK**: ckb-sdk-rust v5.1.0
- **Config Format**: YAML (human-friendly) + JSON (machine-readable)
- **Logging**: tracing + tracing-subscriber
- **Progress**: indicatif for long operations

### Scope Boundaries
- MVP: secp256k1-blake160 only (no Omnilock/ACP)
- MVP: AccessList only (no custom xUDT extensions)
- CLI does NOT deploy contracts
- No GUI/TUI/Web interface
- No batch operations in v1

### Key Design Choices
- Plan/Apply pattern inspired by Terraform
- Config-driven approach (business intent in config, not scripts)
- Reports in both Markdown and JSON
- Environment variable + private key file for signing (no hardware wallet in MVP)

## Gotchas

### Dependency Management
- ckb-sdk-rust, ckb-types, ckb-jsonrpc-types versions must align
- `lib/types` uses `ckb-types` which may conflict with SDK version
- Must pin versions in workspace Cargo.toml

### Cell Operations
- UDT cell capacity = 8 + len(data) + len(lock) + len(type)
- Wrong capacity calculation = "InsufficientCapacity" error on-chain
- Always calculate occupied capacity before transaction construction

### Authority
- `mint_authority` is the strongest authority
- Can authorize supply changes and acts as fallback
- Authority drop requires explicit `--yes` confirmation
- After drop, no further authorized mint or protocol burn possible

### xUDT Complexity
- xUDT supports up to 16 extensions
- AccessList is one extension; RCE requires SMT proofs
- MVP only supports AccessList
- Extension list must be sorted by `(extension_type, script_hash)`

### Network Types
- Devnet/testnet/mainnet have different genesis blocks
- Wrong network config = wrong address prefix, wrong script hashes
- `udtx doctor` must validate network type matches RPC response

### Offckb Integration
- offckb provides 20 pre-funded accounts for devnet
- CLI shells out to offckb for chain up/down/reset
- offckb manages contract deployment to devnet
- CLI references deployed contracts via config (outpoints or script hashes)

## References

- Design spec: `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md`
- Devnet summary: `docs/superpowers/specs/2026-05-21-devnet-practice-summary-for-forum.md`
- Contract README: `README.md`
- Architecture: `Architecture.md`

## Task 5: Key Management System

### Dependencies Added
- `secp256k1 = { version = "0.29", features = ["recovery", "global-context"] }`
- `ckb-hash = "0.112"`
- `zeroize = { version = "1", features = ["derive"] }`
- `bech32 = "0.9"` (for CKB address encoding/decoding)
- `hex = "0.4"` (for hex parsing)

### Key Design Decisions
- `AccountKey` uses `#[derive(Zeroize, ZeroizeOnDrop)]` to clear private key from memory on drop
- `KeyManager` caches loaded accounts in a `HashMap<String, AccountKey>`
- Address-only accounts (watch-only) store a dummy `[0u8; 32]` private key and refuse to sign
- Secp256k1-blake160 is the only supported lock script (MVP scope)

### Address Encoding
- Short format: `0x01 | 0x00 | blake160(args)` then bech32m encoded
- Mainnet prefix: `ckb`, Testnet/Devnet prefix: `ckt`
- Uses bech32 0.9 API (not 0.11 which has breaking changes)

### CKB Types Gotchas
- `ScriptBuilder::hash_type()` takes `ScriptHashType` directly, not `.into()` (type inference fails)
- `ScriptBuilder::args()` takes `Bytes` directly, no `.pack()` needed

### Test Coverage
- Private key hex parsing (with/without 0x prefix, wrong length)
- Address derivation roundtrip (deterministic)
- KeyManager load from env var + sign
- Address-only account refuses to sign
