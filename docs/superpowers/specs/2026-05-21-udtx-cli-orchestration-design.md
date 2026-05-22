# UDTX CLI + Config Orchestration Design

## Goal

Build a deliverable tool product on top of `ckb-standard-udt-contracts` that turns
"script collection" into a reproducible, auditable, ecosystem-friendly workflow.

The product form is:

- one CLI carrier (`udtx`);
- one declarative configuration format;
- one execution lifecycle (`plan -> apply -> verify -> report`).

## Problem Statement

Current workflows are technically valid but operationally expensive:

- too many manual steps across node/deploy/metadata/mint/access-list;
- transactions are reproducible only by experienced engineers;
- failure reasons are script-level and hard to map to business intent;
- ecosystem integrators (wallet/indexer/exchange) lack a stable machine-readable artifact.

## Non-Goals

- do not change existing on-chain validation semantics in this phase;
- do not introduce a hard-cap supply mode in contracts in this phase;
- do not replace `offckb`/`@ckb-ccc/core`; compose on top of them;
- do not require users to understand cell-level details for common workflows.

## Users

- protocol developers: need full control and extension points;
- application teams: need low-friction issuance and governance operations;
- ecosystem integrators: need deterministic outputs and compatibility signals.

## Product Principles

- configuration-first: business intent in config, not hardcoded scripts;
- deterministic execution: same config + same environment -> same plan;
- auditable by default: every run outputs tx hashes and state deltas;
- reversible dev loop: devnet reset and replay must be first-class;
- incremental adoption: start with sUDT/xUDT core flows, then add extensions.

## High-Level Architecture

```text
+------------------+      +---------------------+      +--------------------+
| User Config      | ---> | Planner             | ---> | Execution Plan      |
| (token, policy,  |      | - validate schema   |      | - typed steps       |
| authority, flow) |      | - resolve defaults  |      | - preconditions     |
+------------------+      +---------------------+      +--------------------+
                                                            |
                                                            v
+------------------+      +---------------------+      +--------------------+
| Runtime Adapter  | <--- | Executor            | ---> | Verifier            |
| - offckb         |      | - send tx           |      | - on-chain checks   |
| - ccc            |      | - wait commit       |      | - policy checks     |
| - encoder        |      | - collect traces    |      | - supply checks     |
+------------------+      +---------------------+      +--------------------+
                                                            |
                                                            v
                                                     +--------------------+
                                                     | Reporter           |
                                                     | - markdown/json    |
                                                     | - tx timeline      |
                                                     | - compatibility    |
                                                     +--------------------+
```

## CLI Command Surface (MVP)

### Environment and Project

- `udtx init`: create project scaffold and sample config.
- `udtx doctor`: validate dependencies (`offckb`, Rust, Node), RPC, account funds.
- `udtx chain up|down|reset`: manage local devnet lifecycle.

### Contract and Token Operations

- `udtx deploy`: deploy standard contracts and persist script artifacts.
- `udtx token create`: create `sudt-meta` or `xudt-meta`.
- `udtx token mint`: issue tokens according to policy and authority.
- `udtx token transfer`: transfer tokens.
- `udtx access enable|disable`: toggle xUDT access mode.
- `udtx access add|remove`: maintain blacklist/whitelist entries.
- `udtx authority rotate|drop`: rotate or remove authorities.

### Orchestration Lifecycle

- `udtx plan`: compile config into deterministic step plan (no tx send).
- `udtx apply`: execute plan and wait for confirmations.
- `udtx verify`: validate end state against expected assertions.
- `udtx report`: generate markdown/json audit output.

## Configuration Model

### File Layout

- `udtx.yaml`: primary declarative config.
- `profiles/*.yaml`: environment overlays (`devnet`, `testnet`, `mainnet`).
- `artifacts/*.json`: generated deploy and run outputs.

### Minimal Config Example

```yaml
version: 1
project:
  name: demo-standard-udt
network:
  profile: devnet
  rpc: http://127.0.0.1:28114
accounts:
  owner:
    private_key_env: OWNER_PRIVKEY
  user1:
    address: ckt1...
contracts:
  source:
    mode: deployed-artifacts
    scripts_json: ./artifacts/devnet-scripts.json
token:
  kind: xudt
  symbol: XD
  decimals: 8
  supply_policy:
    mode: tracked
    fixed_after_issue:
      enabled: true
      target_amount: "1000000"
  authorities:
    mint: owner_lock
    metadata: owner_lock
    access: owner_lock
access_control:
  enabled: true
  mode: blacklist
scenario:
  - action: mint
    to: owner
    amount: "1000"
  - action: transfer
    from: owner
    to: user1
    amount: "200"
  - action: access.add
    account: user1
  - action: transfer
    from: owner
    to: user1
    amount: "50"
    expect_failure:
      error_code: 61
  - action: access.remove
    account: user1
  - action: transfer
    from: owner
    to: user1
    amount: "100"
```

## Supply Policy Semantics (Product Layer)

Contracts currently support tracked/untracked supply and authority-governed supply
updates, but no built-in hard-cap mode. Product-level policy should expose:

- `untracked`: require `current_supply == 0`.
- `tracked`: enforce `current_supply` reconciliation against token deltas.
- `fixed_after_issue`: mint to target then irreversibly drop `mint_authority`.

`fixed_after_issue` is the recommended practical fixed-supply workflow until a
future on-chain hard-cap extension is introduced.

## Execution Lifecycle Details

### Plan Phase

- validate config schema and business invariants;
- resolve profile overlays and secrets;
- materialize concrete scripts, dependencies, and witnesses;
- build an ordered `ExecutionPlan` with idempotency metadata.

Output:

- `artifacts/plan.json`;
- `artifacts/plan.md` (human review).

### Apply Phase

- run step prechecks (capacity, script deps, authority presence, singleton cells);
- submit transactions in strict order;
- wait commit and persist tx hash timeline;
- classify failures into user-readable categories.

Output:

- `artifacts/run.json`;
- `artifacts/tx-timeline.json`.

### Verify Phase

- assert balances, metadata flags, access-list entries;
- assert expected failures occurred at intended steps;
- assert authority state (for fixed supply mode: `mint_authority == None`).

Output:

- `artifacts/verify.json`.

### Report Phase

- generate `artifacts/report.md` and `artifacts/report.json`;
- include summary, tx list, final state, and compatibility hints.

## Error Taxonomy

Map low-level script/runtime errors to stable product codes:

- `E_PRECHECK_CAPACITY`: insufficient CKB for planned outputs/fees;
- `E_PRECHECK_DEP`: missing or mismatched cell dep artifact;
- `E_AUTH_MISSING`: required authority absent;
- `E_AUTH_FAILED`: authority check failed;
- `E_SUPPLY_INVALID`: tracked supply invariant violated;
- `E_ACCESS_DENIED`: access-list blocked transfer (expected in some tests);
- `E_UNEXPECTED_REVERT`: uncategorized chain verification failure.

This taxonomy should be stable across CLI versions for tooling and CI integration.

## Compatibility Strategy

### Upstream Compatibility

- consume existing `scripts.json` deployment artifact format where possible;
- keep `offckb` as default deploy/runtime bridge;
- keep `@ckb-ccc/core` as default tx assembly and wait primitive.

### Ecosystem Compatibility Outputs

Provide a machine-readable report to support wallets/indexers:

- token script identity (`code_hash`, `hash_type`, `args`);
- metadata script identity and latest `current_supply`;
- access mode and shard state;
- tested operation matrix (`mint`, `transfer`, `blocked transfer`, etc.).

## Security and Operational Considerations

- never log private keys or raw secret material;
- support key loading from env or signer plugin only;
- provide `--dry-run` and `plan` review before `apply`;
- require explicit confirmation for authority drop operations;
- include nonce/timestamp and tool version in artifacts for audit traceability.

## Delivery Milestones

### Milestone 1 (2 weeks)

- `init`, `doctor`, `plan`, `apply`;
- devnet support only;
- sUDT and xUDT create/mint/transfer baseline.

### Milestone 2 (2 weeks)

- access-list orchestration (`enable/add/remove`);
- expected-failure assertions;
- `verify` and markdown/json reporting.

### Milestone 3 (2 to 4 weeks)

- authority workflows (`rotate/drop`);
- compatibility report for ecosystem integrators;
- CI template (`GitHub Actions`) for automated scenario runs.

## Adoption Plan

- publish a canonical template repo with one command quickstart;
- publish deterministic demo artifacts for public verification;
- release a compatibility badge based on `udtx verify` results;
- co-design report schema with wallet/indexer maintainers;
- keep CLI install and upgrade path simple and stable.

## Open Questions

- Should `report.json` be versioned independently from CLI?
- Should testnet profile enforce stricter minimum confirmations?
- Should fixed-supply mode require two-phase confirmation by default?
- When to propose on-chain hard-cap extension vs. keep product-layer fixed mode?
