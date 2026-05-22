# Devnet Practice Summary and Discussion Directions

## Context

We ran a full end-to-end practice around `ckb-standard-udt-contracts` using
`offckb` devnet to reduce cost and improve reproducibility.

Scope covered:

- sUDT: metadata create, mint, transfer;
- xUDT: metadata create, mint, transfer;
- xUDT access-list: enable blacklist, add/remove entry, and blocked-transfer test.

## Practical Findings

### What Worked Well

- contract modularity is strong (`sudt`, `sudt-meta`, `xudt`, `xudt-meta`, `access-list`);
- tracked supply invariants and authority checks are robust;
- blocked transfer under blacklist mode is deterministic and testable;
- devnet loop is productive: reset/redeploy/replay can be done quickly.

### Main Frictions

- user workflow is still script-centric and requires deep chain knowledge;
- deployment/runtime artifacts are usable but not yet product-grade for broad teams;
- low-level verification errors are accurate but not always business-readable;
- ecosystem integration needs a standardized compatibility report.

## Supply Model Discussion

Current model provides:

- tracked/untracked supply modes;
- authority-governed mint/protocol-burn updates;
- fixed supply achievable by minting target amount and dropping mint authority.

Current model does not provide:

- built-in hard-cap (`max_supply`) mode at contract level.

Suggested short-term product path:

- expose `fixed_after_issue` in tooling (mint to target then drop authority);
- include strict verify checks to prove no further authorized mint is possible.

## Why We Propose a CLI + Config Product Layer

A productized orchestration layer can make this contract suite easier to adopt
without changing on-chain semantics:

- one declarative config for token policy and scenario;
- one deterministic lifecycle: `plan -> apply -> verify -> report`;
- one audit artifact for users and ecosystem integrators.

This directly addresses the main adoption gap: reducing operational complexity,
not adding another custom script.

## Discussion Directions With Maintainers

### Direction 1: Standardized Tooling Interface

- agree on a stable artifact contract (`scripts.json` + run/report schema);
- align failure-code mapping for user-facing diagnostics;
- publish a canonical reference scenario for sUDT/xUDT/access-list.

### Direction 2: Ecosystem Compatibility Kit

- provide machine-readable operation matrix (mint/transfer/blocked transfer);
- provide script identity outputs for wallets/indexers;
- define a compatibility badge process for third-party projects.

### Direction 3: Supply Policy Roadmap

- short term: tooling-level fixed supply mode (`fixed_after_issue`);
- medium term: evaluate optional on-chain hard-cap extension proposal;
- keep backward-compatible migration story for current tracked/untracked users.

## Suggested Forum Reply (Draft)

We ran a complete devnet practice on this contract suite and got stable results
for both sUDT and xUDT, including access-list blacklist add/remove and expected
blocked transfers.

Our main feedback is positive on protocol design (modularity, authority model,
supply invariants), while the biggest adoption gap is still the developer
workflow. Today, the workflow is still script-heavy for most teams.

We are considering building a productized CLI + config orchestration layer with
`plan -> apply -> verify -> report`, so teams can operate the suite through
business-level config instead of custom scripts.

On supply: we understand the current design intentionally has tracked/untracked
modes and no native hard-cap mode; practical fixed supply can be done by minting
then dropping mint authority. We think this is workable now, and we are happy to
discuss whether a future optional hard-cap extension is valuable.

If helpful, we can share:

- a reproducible devnet scenario template;
- a machine-readable compatibility report schema;
- concrete UX feedback from converting script workflows into a tool product.

## Expected Collaboration Outcome

- lower onboarding cost for app teams;
- stronger confidence for ecosystem integrators;
- clearer path from protocol capability to real-world product adoption.
