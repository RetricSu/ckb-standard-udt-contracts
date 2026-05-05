# Agent Constraints

- Create on-chain script crates with the repository's `make generate` / `ckb-script-templates` flow. Do not hand-roll contract `Cargo.toml`, `Makefile`, or `src/main.rs` skeletons.
- Keep generated contract crate structure intact unless a later change has a concrete reason. Prefer editing `src/entry.rs`, modules, and dependencies over replacing template plumbing.
- Core contracts live under `contracts/`. Test-only script fixtures may live under `tests/plugins/`, but they must still be generated from the contract template.
- Shared non-script Rust libraries may be created manually under `lib/`, but they must not pretend to be CKB script crates.
- New implementation names must not include `v1`; legacy references are allowed only under `ref/old` or when discussing old docs.
- Treat `ref/old` as reference material, not a source tree to copy wholesale.
- Keep script boundaries clear: metadata types in `lib/types`, reusable CKB helpers in `lib/script-utils`, and each contract crate responsible for one validation boundary.
