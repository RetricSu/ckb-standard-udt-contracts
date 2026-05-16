# Single Script Authority Dedup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deduplicate repeated authority checks within `sudt-meta` and `xudt-meta` script invocations while preserving existing authorization behavior.

**Architecture:** Introduce a small per-invocation authority verifier in each metadata update module. The verifier caches results for authority descriptors it has already checked and exposes the same specific, fallback, and any-authority semantics currently implemented by local helper functions. Cross-script authority checks remain independent.

**Tech Stack:** Rust no_std CKB contracts, `alloc::vec::Vec`, `standard-udt-types::metadata::Authority`, existing `standard_udt_script_utils::authority::check_authority`, `ckb-testtool` integration tests.

---

## File Structure

- Modify `contracts/sudt-meta/src/update.rs`: add a local `AuthorityVerifier`, route update and destroy authority decisions through one verifier per validation call.
- Modify `contracts/xudt-meta/src/update.rs`: add a local `AuthorityVerifier`, route update and destroy authority decisions through one verifier per validation call.
- Test with existing integration suites in `tests/src/tests/sudt_meta` and `tests/src/tests/xudt_meta`; no new behavioral tests are required because this refactor must not change observable behavior.
- Keep `docs/superpowers/specs/2026-05-16-single-script-authority-dedup-design.md` and this plan in the same commit as the refactor.

---

### Task 1: Refactor sUDT Meta Authority Checks

**Files:**
- Modify: `contracts/sudt-meta/src/update.rs`

- [ ] **Step 1: Add `alloc::vec::Vec` import**

Add this near the top of `contracts/sudt-meta/src/update.rs`:

```rust
use alloc::vec::Vec;
```

- [ ] **Step 2: Add an authority verifier**

Add this helper near the existing authority helper functions:

```rust
struct AuthorityVerifier {
    checked: Vec<(Authority, bool)>,
}

impl AuthorityVerifier {
    fn new() -> Self {
        Self {
            checked: Vec::new(),
        }
    }

    fn require(&mut self, authority: Option<&Authority>) -> Result<(), Error> {
        let authority = authority.ok_or(Error::AuthorityMissing)?;
        match self.check(authority) {
            Ok(true) => Ok(()),
            Ok(false) => Err(Error::AuthorityFailed),
            Err(error) => Err(error),
        }
    }

    fn require_with_fallback(
        &mut self,
        authority: Option<&Authority>,
        mint_authority: Option<&Authority>,
    ) -> Result<(), Error> {
        if let Some(authority) = authority {
            match self.check(authority) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(Error::AuthorityFailed) => {
                    if mint_authority.is_none() {
                        return Err(Error::AuthorityFailed);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        self.require(mint_authority)
    }

    fn check(&mut self, authority: &Authority) -> Result<bool, Error> {
        if let Some((_, result)) = self
            .checked
            .iter()
            .find(|(checked_authority, _)| checked_authority == authority)
        {
            return Ok(*result);
        }

        let result = check_authority(authority)?;
        self.checked.push((authority.clone(), result));
        Ok(result)
    }
}
```

- [ ] **Step 3: Route `validate_update` through one verifier**

At the start of `validate_update`, after supply validation and before authority
branches, create:

```rust
let mut verifier = AuthorityVerifier::new();
```

Replace:

```rust
require_authority(input.mint_authority.as_ref())?;
```

with:

```rust
verifier.require(input.mint_authority.as_ref())?;
```

Replace:

```rust
require_authority_with_mint_fallback(
    input.metadata_authority.as_ref(),
    input.mint_authority.as_ref(),
)?;
```

with:

```rust
verifier.require_with_fallback(
    input.metadata_authority.as_ref(),
    input.mint_authority.as_ref(),
)?;
```

- [ ] **Step 4: Route `validate_destroy` through the verifier**

Replace:

```rust
require_authority(input.mint_authority.as_ref())
```

with:

```rust
let mut verifier = AuthorityVerifier::new();
verifier.require(input.mint_authority.as_ref())
```

- [ ] **Step 5: Remove replaced free helper functions**

Remove `require_authority` and `require_authority_with_mint_fallback` if they
are no longer used.

- [ ] **Step 6: Verify sUDT metadata tests**

Run:

```bash
make build MODE=debug CONTRACT=sudt-meta
MODE=debug cargo test -p tests sudt_meta -- --nocapture
```

Expected: all `sudt_meta` tests pass.

---

### Task 2: Refactor xUDT Meta Authority Checks

**Files:**
- Modify: `contracts/xudt-meta/src/update.rs`

- [ ] **Step 1: Add `alloc::vec::Vec` import**

Add this near the top of `contracts/xudt-meta/src/update.rs`:

```rust
use alloc::vec::Vec;
```

- [ ] **Step 2: Add an authority verifier**

Add this helper near the existing authority helper functions:

```rust
struct AuthorityVerifier {
    checked: Vec<(Authority, bool)>,
}

impl AuthorityVerifier {
    fn new() -> Self {
        Self {
            checked: Vec::new(),
        }
    }

    fn require(&mut self, authority: Option<&Authority>) -> Result<(), Error> {
        let authority = authority.ok_or(Error::AuthorityMissing)?;
        match self.check(authority) {
            Ok(true) => Ok(()),
            Ok(false) => Err(Error::AuthorityFailed),
            Err(error) => Err(error),
        }
    }

    fn require_with_fallback(
        &mut self,
        authority: Option<&Authority>,
        mint_authority: Option<&Authority>,
    ) -> Result<(), Error> {
        if let Some(authority) = authority {
            match self.check(authority) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(Error::AuthorityFailed) => {
                    if mint_authority.is_none() {
                        return Err(Error::AuthorityFailed);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        self.require(mint_authority)
    }

    fn require_any(&mut self, authorities: &[Option<&Authority>]) -> Result<(), Error> {
        let mut has_authority = false;
        for authority in authorities.iter().filter_map(|authority| *authority) {
            has_authority = true;
            match self.check(authority) {
                Ok(true) => return Ok(()),
                Ok(false) | Err(Error::AuthorityFailed) => {}
                Err(error) => return Err(error),
            }
        }

        if has_authority {
            Err(Error::AuthorityFailed)
        } else {
            Err(Error::AuthorityMissing)
        }
    }

    fn check(&mut self, authority: &Authority) -> Result<bool, Error> {
        if let Some((_, result)) = self
            .checked
            .iter()
            .find(|(checked_authority, _)| checked_authority == authority)
        {
            return Ok(*result);
        }

        let result = check_authority(authority)?;
        self.checked.push((authority.clone(), result));
        Ok(result)
    }
}
```

- [ ] **Step 3: Route `validate_update` through one verifier**

At the start of `validate_update`, after supply validation and before authority
branches, create:

```rust
let mut verifier = AuthorityVerifier::new();
```

Replace direct calls to:

```rust
require_authority(...)
require_authority_with_mint_fallback(...)
require_any_authority(...)
```

with:

```rust
verifier.require(...)
verifier.require_with_fallback(...)
verifier.require_any(...)
```

- [ ] **Step 4: Route `validate_destroy` through the verifier**

Replace:

```rust
require_authority(input.mint_authority.as_ref())
```

with:

```rust
let mut verifier = AuthorityVerifier::new();
verifier.require(input.mint_authority.as_ref())
```

- [ ] **Step 5: Remove replaced free helper functions**

Remove `require_authority`, `require_authority_with_mint_fallback`, and
`require_any_authority` if they are no longer used.

- [ ] **Step 6: Verify xUDT metadata tests**

Run:

```bash
make build MODE=debug CONTRACT=xudt-meta
MODE=debug cargo test -p tests xudt_meta -- --nocapture
```

Expected: all `xudt_meta` tests pass.

---

### Task 3: Full Regression Verification

**Files:**
- No code changes.

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt
```

Expected: command exits successfully.

- [ ] **Step 2: Build debug artifacts**

Run:

```bash
make build MODE=debug
```

Expected: all debug contract and plugin artifacts build successfully.

- [ ] **Step 3: Run full integration suite**

Run:

```bash
MODE=debug cargo test -p tests -- --nocapture
```

Expected: all integration tests pass.

- [ ] **Step 4: Review diff**

Run:

```bash
git diff -- contracts/sudt-meta/src/update.rs contracts/xudt-meta/src/update.rs docs/superpowers/specs/2026-05-16-single-script-authority-dedup-design.md docs/superpowers/plans/2026-05-16-single-script-authority-dedup.md
```

Expected: diff is limited to single-script authority deduplication and docs.
