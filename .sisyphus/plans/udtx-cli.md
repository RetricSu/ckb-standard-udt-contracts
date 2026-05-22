# UDTX CLI 推进计划

## TL;DR

> **目标**: 在 `ckb-standard-udt-contracts` 工作区中构建一个 Rust CLI 工具 `udtx`，将脚本集合转变为可复现、可审计、生态友好的工作流。
>
> **交付物**:
> - `udtx` CLI crate（workspace 新成员）
> - 声明式 YAML 配置系统（`udtx.yaml` + `profiles/*.yaml`）
> - `plan -> apply -> verify -> report` 生命周期
> - Markdown + JSON 报告输出
> - 完整文档和模板仓库
>
> **预估工作量**: Large（6-8 周）
> **并行执行**: YES - 4 个 Wave
> **关键路径**: 脚手架 -> Config -> Doctor -> Token(sUDT) -> Token(xUDT) -> AccessList -> Plan/Apply -> Report -> 文档/推广

---

## Context

### 原始需求
用户希望将已有的 devnet practice 和 CLI 设计文档推进为生态可用的工具，包括：
- Rust CLI 实现
- 声明式配置驱动
- plan -> apply -> verify -> report 生命周期
- 生态推广（中量级）

### 访谈总结
**关键决策**:
- **技术栈**: Rust + ckb-sdk-rust（官方 SDK）
- **签名方式**: 环境变量 + 私钥文件（MVP 暂不支持硬件钱包）
- **报告格式**: Markdown + JSON 两者都要
- **合约部署**: CLI 不处理部署，用户通过配置引用已部署合约
- **offckb**: 仅用于本地 devnet 启动和合约部署
- **范围**: 一次性做完（完整 CLI + 生态推广）
- **推广深度**: 中量级（发布 CLI、文档、demo，暂不和钱包/交易所深度合作）

### 研究发现
**代码库结构**:
- Workspace 含 21 个成员：5 个合约 + 2 个 lib + 9 个测试插件 + 1 个测试 crate
- `lib/types`: 共享类型（SudtMeta, XudtMeta, Authority, AccessListShard 等），支持 std/no-std
- `lib/script-utils`: no_std 脚本端工具，CLI 不能直接依赖
- Makefile 驱动构建，使用 `@@INSERTION_POINT@@` 自动生成新 crate
- 无现有 CI/CD

**CKB CLI 生态模式**:
- ckb-cli: trait-based subcommand 架构，支持 batch + interactive 模式
- offckb: Commander.js 扁平命令结构，migration-based 部署追踪
- ckb-sdk-rust: 分层 transaction builder（build_base -> build_balanced -> build_unlocked）
- Terraform 模式: plan artifact -> apply -> state tracking

**测试基础设施**:
- ckb-testtool 0.14.0 用于集成测试
- Fixture 模式（XudtFixture, SudtFixture）
- 无现有 CI

### Metis 审查
**识别的差距**（已处理）:
- 已有设计文档覆盖 80% 需求，需在此基础上实现
- 需明确 lock script 支持范围（MVP 仅 secp256k1-blake160）
- 需明确 xUDT 扩展支持范围（MVP 仅 AccessList）
- 需验证 ckb-sdk-rust 的 xUDT 支持
- 需处理依赖版本冲突风险

---

## Work Objectives

### Core Objective
构建一个 Rust CLI 工具 `udtx`，让开发者通过声明式配置而非自定义脚本来操作 sUDT/xUDT 代币，产出可审计的报告，降低生态采用门槛。

### Concrete Deliverables
- `udtx/` CLI crate（workspace 成员）
- `udtx.yaml` 配置格式定义和验证
- `profiles/devnet.yaml`, `profiles/testnet.yaml` 模板
- 命令实现：init, doctor, chain, token, access, authority, plan, apply, verify, report
- Markdown 报告生成器
- JSON 报告生成器
- README 文档
- 模板仓库（用于快速开始）
- GitHub Actions CI 配置

### Definition of Done
- [ ] `udtx init` 创建有效项目脚手架
- [ ] `udtx doctor` 验证所有依赖和配置
- [ ] `udtx token create` 在 devnet 上创建 sUDT/xUDT
- [ ] `udtx token mint` 成功增发代币
- [ ] `udtx token transfer` 成功转账
- [ ] `udtx access enable/disable/add/remove` 管理访问控制
- [ ] `udtx plan` 生成可审计的执行计划
- [ ] `udtx apply` 执行计划并等待确认
- [ ] `udtx verify` 验证链上状态
- [ ] `udtx report` 生成 Markdown + JSON 报告
- [ ] 所有命令支持 `--dry-run`
- [ ] CI 通过（build + test + clippy + fmt）

### Must Have
- sUDT 创建、增发、转账
- xUDT 创建、增发、转账
- AccessList 启用/禁用、添加/删除条目
- 权限轮换/删除
- plan/apply/verify/report 生命周期
- devnet 支持（通过 offckb）
- 环境变量和私钥文件签名
- Markdown + JSON 报告

### Must NOT Have (Guardrails)
- **不部署合约**：CLI 不处理合约部署，只引用已部署合约
- **不支持自定义 lock script**：MVP 仅 secp256k1-blake160
- **不支持自定义 xUDT 扩展**：MVP 仅 AccessList
- **不添加 GUI/TUI/Web 界面**
- **不替代 offckb/ckb-ccc**：在其之上组合
- **不实现批量操作**：v1 仅单操作，批量通过 shell 脚本或 plan 文件
- **不添加自动 fee bumping**

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: NO（无现有 CI，需新建）
- **Automated tests**: YES (Tests-after)
- **Framework**: cargo test + ckb-testtool（用于集成测试）
- **Agent-Executed QA**: 每个任务包含可执行的 QA 场景

### QA Policy
每个任务 MUST 包含 agent-executed QA scenarios：
- **CLI 命令**: Bash 执行命令，验证输出和退出码
- **配置文件**: 验证文件存在性和内容
- **报告输出**: 验证 Markdown/JSON 格式和内容
- **链上状态**: 通过 RPC 查询验证

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - 可立即并行):
├── Task 1: CLI crate 脚手架（Cargo.toml, main.rs, clap 设置）
├── Task 2: 配置系统（udtx.yaml schema, profiles, 验证）
├── Task 3: 错误类型和日志系统
├── Task 4: RPC 客户端封装和链连接
└── Task 5: 密钥管理（环境变量 + 私钥文件）

Wave 2 (Core Commands - 依赖 Wave 1):
├── Task 6: Doctor 命令（依赖检查、RPC 验证）
├── Task 7: Chain 命令（up/down/reset via offckb）
├── Task 8: Token create 命令（sUDT 基础）
├── Task 9: Token mint 命令（sUDT 基础）
├── Task 10: Token transfer 命令（sUDT 基础）
└── Task 11: sUDT 集成测试

Wave 3 (Advanced Features - 依赖 Wave 2):
├── Task 12: xUDT create 命令（含 metadata）
├── Task 13: xUDT mint 命令
├── Task 14: xUDT transfer 命令
├── Task 15: AccessList 命令（enable/disable/add/remove）
├── Task 16: Authority 命令（rotate/drop）
├── Task 17: xUDT 集成测试
└── Task 18: AccessList 集成测试

Wave 4 (Lifecycle & Reports - 依赖 Wave 3):
├── Task 19: Plan 命令（生成执行计划）
├── Task 20: Apply 命令（执行计划）
├── Task 21: Verify 命令（验证链上状态）
├── Task 22: Report 命令（Markdown + JSON）
├── Task 23: Plan/Apply 集成测试
└── Task 24: Report 验证

Wave 5 (Docs & Release - 依赖 Wave 4):
├── Task 25: README 和文档
├── Task 26: 模板仓库
├── Task 27: GitHub Actions CI
├── Task 28: 发布准备（版本、changelog）
└── Task 29: 生态推广材料

Wave FINAL (Verification - 4 个并行审查):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review
├── Task F3: Real manual QA
└── Task F4: Scope fidelity check
```

### Dependency Matrix

| Task | Blocked By | Blocks |
|------|-----------|--------|
| 1 (Scaffold) | - | 2,3,4,5 |
| 2 (Config) | 1 | 6,8,19 |
| 3 (Errors) | 1 | ALL |
| 4 (RPC) | 1 | 6,7,8,9,10 |
| 5 (Keys) | 1 | 8,9,10 |
| 6 (Doctor) | 2,4 | 7,8,9,10 |
| 7 (Chain) | 4 | 11,17,18 |
| 8 (sUDT create) | 2,4,5 | 9,10,11 |
| 9 (sUDT mint) | 4,5,8 | 11 |
| 10 (sUDT transfer) | 4,5,8 | 11 |
| 11 (sUDT tests) | 7,8,9,10 | - |
| 12 (xUDT create) | 2,4,5,8 | 13,14,17 |
| 13 (xUDT mint) | 4,5,12 | 17 |
| 14 (xUDT transfer) | 4,5,12 | 17 |
| 15 (AccessList) | 4,5,12 | 18 |
| 16 (Authority) | 4,5,12 | - |
| 17 (xUDT tests) | 7,12,13,14 | - |
| 18 (AccessList tests) | 7,15 | - |
| 19 (Plan) | 2,8,12 | 20,23 |
| 20 (Apply) | 4,19 | 21,23 |
| 21 (Verify) | 4,20 | 22,23 |
| 22 (Report) | 21 | 24 |
| 23 (Plan tests) | 19,20 | - |
| 24 (Report tests) | 22 | - |
| 25 (Docs) | ALL | - |
| 26 (Template) | 25 | - |
| 27 (CI) | ALL | - |
| 28 (Release) | 27 | - |
| 29 (Promotion) | 28 | - |

---

## TODOs

- [x] 1. CLI Crate Scaffold

  **What to do**:
  - 在 workspace 根目录创建 `udtx/` 目录
  - 创建 `udtx/Cargo.toml`，添加依赖：clap, ckb-sdk, ckb-types, serde, serde_yaml, tokio, anyhow, thiserror, tracing, tracing-subscriber
  - 创建 `udtx/src/main.rs`，设置 clap derive 宏命令结构
  - 创建 `udtx/src/lib.rs`，导出核心模块
  - 更新根 `Cargo.toml`，在 `@@INSERTION_POINT@@` 处添加 `"udtx"`
  - 确保 `cargo check -p udtx` 通过

  **Must NOT do**:
  - 不要添加任何具体命令实现
  - 不要修改现有 crate

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 纯脚手架工作，无复杂逻辑

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 2,3,4,5
  - **Blocked By**: None

  **References**:
  - `Cargo.toml` - workspace 结构，@@INSERTION_POINT@@ 模式
  - `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` - CLI 命令设计
  - ckb-cli 源码 - trait-based subcommand 架构参考

  **Acceptance Criteria**:
  - [ ] `cargo check -p udtx` 成功编译
  - [ ] `cargo run -p udtx -- --help` 显示帮助信息
  - [ ] 根 Cargo.toml 包含 `"udtx"` 成员

  **QA Scenarios**:

  ```
  Scenario: CLI scaffold compiles
    Tool: Bash
    Preconditions: 无
    Steps:
      1. cargo check -p udtx
    Expected Result: 编译成功，无错误
    Evidence: .sisyphus/evidence/task-1-compile.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): scaffold CLI crate`
  - Files: `udtx/Cargo.toml`, `udtx/src/main.rs`, `udtx/src/lib.rs`, `Cargo.toml`

- [x] 2. Configuration System

  **What to do**:
  - 定义 `udtx.yaml` 配置结构（serde 序列化/反序列化）
  - 定义 profile 结构（devnet/testnet/mainnet）
  - 实现配置验证（schema validation）
  - 实现配置加载（从文件 + 环境变量覆盖）
  - 实现 `udtx init` 命令，生成示例配置

  **Must NOT do**:
  - 不要硬编码任何网络特定的值
  - 不要实现配置加密

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 配置解析和验证，标准 Rust 模式

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 6,8,19
  - **Blocked By**: Task 1

  **References**:
  - `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` - 配置模型章节
  - `lib/types/src/metadata/mod.rs` - 类型定义模式

  **Acceptance Criteria**:
  - [ ] `udtx init` 创建 `udtx.yaml` + `profiles/devnet.yaml`
  - [ ] 配置验证拒绝无效输入
  - [ ] 环境变量覆盖正常工作

  **QA Scenarios**:

  ```
  Scenario: Init creates valid config
    Tool: Bash
    Preconditions: 空目录
    Steps:
      1. cargo run -p udtx -- init
      2. cat udtx.yaml
      3. cat profiles/devnet.yaml
    Expected Result: 文件存在，格式正确，包含必要字段
    Evidence: .sisyphus/evidence/task-2-init-config/

  Scenario: Config validation rejects invalid input
    Tool: Bash
    Preconditions: 无效 udtx.yaml（如缺少必要字段）
    Steps:
      1. cargo run -p udtx -- doctor
    Expected Result: 退出码非 0，错误信息指明无效字段
    Evidence: .sisyphus/evidence/task-2-invalid-config.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): configuration system`
  - Files: `udtx/src/config.rs`, `udtx/src/profiles.rs`

- [x] 3. Error Types and Logging

  **What to do**:
  - 定义 `TokenCliError` enum（使用 thiserror）
  - 映射底层错误到产品级错误代码
  - 设置 tracing 日志系统
  - 实现用户友好的错误信息（含修复建议）

  **Must NOT do**:
  - 不要暴露底层 RPC 错误细节给用户
  - 不要在错误信息中泄露私钥

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 错误处理和日志是标准模式

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: ALL
  - **Blocked By**: Task 1

  **References**:
  - `lib/types/src/error.rs` - 现有错误类型
  - ckb-sdk-rust 错误处理模式

  **Acceptance Criteria**:
  - [ ] 所有错误包含可操作的建议
  - [ ] 日志分级正确（info/warn/error/debug）
  - [ ] 私钥不在日志中

  **QA Scenarios**:

  ```
  Scenario: Error messages are helpful
    Tool: Bash
    Preconditions: 无效 RPC URL
    Steps:
      1. cargo run -p udtx -- doctor --rpc http://invalid
    Expected Result: 错误信息包含 "检查 RPC URL 是否正确"
    Evidence: .sisyphus/evidence/task-3-error-msg.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): error handling and logging`
  - Files: `udtx/src/error.rs`, `udtx/src/logger.rs`

- [x] 4. RPC Client Wrapper

  **What to do**:
  - 封装 ckb-sdk-rust 的 RPC 客户端
  - 实现重试机制（3 次，指数退避）
  - 实现超时处理（30 秒）
  - 实现网络类型验证（devnet/testnet/mainnet）

  **Must NOT do**:
  - 不要直接暴露 ckb-sdk-rust 的 API
  - 不要缓存敏感信息

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要理解 CKB RPC 和 SDK 的交互

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 6,7,8,9,10,12,13,14,15,16
  - **Blocked By**: Task 1

  **References**:
  - ckb-sdk-rust 文档和源码
  - offckb 的 RPC 使用模式

  **Acceptance Criteria**:
  - [ ] RPC 调用成功返回数据
  - [ ] 超时后重试 3 次
  - [ ] 网络类型不匹配时拒绝

  **QA Scenarios**:

  ```
  Scenario: RPC connection works
    Tool: Bash
    Preconditions: offckb devnet 运行中
    Steps:
      1. cargo run -p udtx -- doctor
    Expected Result: RPC 连接成功，返回链信息
    Evidence: .sisyphus/evidence/task-4-rpc.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): RPC client wrapper`
  - Files: `udtx/src/rpc.rs`

- [x] 5. Key Management

  **What to do**:
  - 实现环境变量加载（OWNER_PRIVKEY 等）
  - 实现私钥文件加载
  - 实现 Secp256k1 签名
  - 实现地址生成

  **Must NOT do**:
  - 不要明文存储私钥
  - 不要支持硬件钱包（MVP 范围外）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 涉及密码学操作，需谨慎

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 8,9,10,12,13,14,15,16
  - **Blocked By**: Task 1

  **References**:
  - ckb-sdk-rust 的签名模块
  - ckb-cli 的 keystore 模式

  **Acceptance Criteria**:
  - [ ] 环境变量加载正常工作
  - [ ] 私钥文件加载正常工作
  - [ ] 签名产生有效交易

  **QA Scenarios**:

  ```
  Scenario: Key loading works
    Tool: Bash
    Preconditions: 设置 OWNER_PRIVKEY 环境变量
    Steps:
      1. export OWNER_PRIVKEY=0x...
      2. cargo run -p udtx -- doctor
    Expected Result: 地址正确显示
    Evidence: .sisyphus/evidence/task-5-key.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): key management`
  - Files: `udtx/src/keys.rs`

- [x] 6. Doctor Command

  **What to do**:
  - 实现 `udtx doctor` 命令
  - 检查依赖（offckb, Rust, Node）
  - 验证 RPC 连接和链类型
  - 验证账户余额
  - 验证合约引用（code hash 匹配）

  **Must NOT do**:
  - 不要修改任何系统状态
  - 不要尝试修复问题（只报告）

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 主要是检查和报告

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 7,8,9,10
  - **Blocked By**: Tasks 2,4

  **References**:
  - `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` - Doctor 设计

  **Acceptance Criteria**:
  - [ ] `udtx doctor` 通过时退出码 0
  - [ ] `udtx doctor` 失败时退出码非 0
  - [ ] 报告包含所有检查项状态

  **QA Scenarios**:

  ```
  Scenario: Doctor passes on valid setup
    Tool: Bash
    Preconditions: offckb devnet 运行，配置正确
    Steps:
      1. cargo run -p udtx -- doctor
    Expected Result: 所有检查通过，退出码 0
    Evidence: .sisyphus/evidence/task-6-doctor-pass.txt

  Scenario: Doctor fails on invalid setup
    Tool: Bash
    Preconditions: 无效 RPC URL
    Steps:
      1. cargo run -p udtx -- doctor --rpc http://invalid
    Expected Result: 检查失败，退出码非 0
    Evidence: .sisyphus/evidence/task-6-doctor-fail.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): doctor command`
  - Files: `udtx/src/commands/doctor.rs`

- [x] 7. Chain Commands

  **What to do**:
  - 实现 `udtx chain up`（启动 devnet）
  - 实现 `udtx chain down`（停止 devnet）
  - 实现 `udtx chain reset`（重置 devnet）
  - 通过 shell out 到 offckb 实现

  **Must NOT do**:
  - 不要直接管理 CKB 节点进程
  - 不要修改 offckb 的内部状态

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 主要是进程管理

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 11,17,18
  - **Blocked By**: Task 4

  **References**:
  - offckb CLI 文档

  **Acceptance Criteria**:
  - [ ] `udtx chain up` 启动 devnet
  - [ ] `udtx chain down` 停止 devnet
  - [ ] `udtx chain reset` 重置 devnet

  **QA Scenarios**:

  ```
  Scenario: Chain lifecycle works
    Tool: Bash
    Preconditions: 无
    Steps:
      1. cargo run -p udtx -- chain up
      2. curl http://127.0.0.1:8114
      3. cargo run -p udtx -- chain down
    Expected Result: devnet 启动、可访问、停止
    Evidence: .sisyphus/evidence/task-7-chain.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): chain lifecycle commands`
  - Files: `udtx/src/commands/chain.rs`

- [x] 8. sUDT Token Create

  **What to do**:
  - 实现 `udtx token create --type sudt` 命令
  - 构建 sUDT metadata cell
  - 使用 lib/types 的 SudtMeta 序列化
  - 支持 --dry-run

  **Must NOT do**:
  - 不要处理 xUDT（后续任务）
  - 不要处理 AccessList

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要理解 CKB cell 结构和交易构建

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 9,10,11
  - **Blocked By**: Tasks 2,4,5,6

  **References**:
  - `lib/types/src/metadata/token.rs` - SudtMeta 定义
  - `tests/src/test_helpers/metadata.rs` - metadata 构建模式
  - ckb-sdk-rust UdtIssueBuilder

  **Acceptance Criteria**:
  - [ ] `udtx token create --type sudt --dry-run` 显示交易预览
  - [ ] `udtx token create --type sudt` 成功创建 token
  - [ ] 交易可在链上查询

  **QA Scenarios**:

  ```
  Scenario: Create sUDT token
    Tool: Bash
    Preconditions: offckb devnet 运行，账户有余额
    Steps:
      1. cargo run -p udtx -- token create --type sudt --name "Test" --symbol "TST" --decimals 8 --supply 1000000 --owner ckt1... --dry-run
      2. cargo run -p udtx -- token create --type sudt --name "Test" --symbol "TST" --decimals 8 --supply 1000000 --owner ckt1...
    Expected Result: dry-run 显示交易信息，实际执行成功，返回 tx hash
    Evidence: .sisyphus/evidence/task-8-sudt-create/
  ```

  **Commit**: YES
  - Message: `feat(udtx): sUDT token create`
  - Files: `udtx/src/commands/token/create.rs`

- [ ] 9. sUDT Token Mint

  **What to do**:
  - 实现 `udtx token mint` 命令
  - 增发 sUDT 代币
  - 验证 mint authority
  - 支持 --dry-run

  **Must NOT do**:
  - 不要绕过 authority 检查

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要 authority 验证和交易构建

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 4,5,8

  **References**:
  - `tests/src/tests/sudt/mint.rs` - mint 测试模式
  - ckb-sdk-rust UdtIssueBuilder

  **Acceptance Criteria**:
  - [ ] `udtx token mint` 成功增发
  - [ ] 余额正确增加
  - [ ] 无 authority 时失败

  **QA Scenarios**:

  ```
  Scenario: Mint sUDT tokens
    Tool: Bash
    Preconditions: sUDT token 已创建
    Steps:
      1. cargo run -p udtx -- token mint --token <hash> --to ckt1... --amount 1000
    Expected Result: 交易成功，接收方余额增加 1000
    Evidence: .sisyphus/evidence/task-9-sudt-mint.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): sUDT token mint`
  - Files: `udtx/src/commands/token/mint.rs`

- [ ] 10. sUDT Token Transfer

  **What to do**:
  - 实现 `udtx token transfer` 命令
  - 转账 sUDT 代币
  - 处理 change output
  - 支持 --dry-run

  **Must NOT do**:
  - 不要丢失 change

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要 cell 收集和 change 处理

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 4,5,8

  **References**:
  - `tests/src/tests/sudt/lifecycle.rs` - transfer 测试模式
  - ckb-sdk-rust UdtTransferBuilder

  **Acceptance Criteria**:
  - [ ] `udtx token transfer` 成功转账
  - [ ] 发送方余额减少，接收方增加
  - [ ] Change 正确返回

  **QA Scenarios**:

  ```
  Scenario: Transfer sUDT tokens
    Tool: Bash
    Preconditions: sUDT token 已创建，发送方有余额
    Steps:
      1. cargo run -p udtx -- token transfer --token <hash> --to ckt1... --amount 500
    Expected Result: 发送方余额减少 500，接收方增加 500
    Evidence: .sisyphus/evidence/task-10-sudt-transfer.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): sUDT token transfer`
  - Files: `udtx/src/commands/token/transfer.rs`

- [ ] 11. sUDT Integration Tests

  **What to do**:
  - 编写 sUDT 端到端测试
  - 使用 offckb devnet
  - 测试 create, mint, transfer 完整流程
  - 测试错误场景

  **Must NOT do**:
  - 不要修改生产代码

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要设置测试环境和验证链上状态

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Tasks 7,8,9,10

  **References**:
  - `tests/src/tests/sudt/` - 现有测试模式
  - offckb 预 funded 账户

  **Acceptance Criteria**:
  - [ ] 测试通过（create -> mint -> transfer）
  - [ ] 错误场景测试通过

  **QA Scenarios**:

  ```
  Scenario: sUDT full lifecycle test
    Tool: Bash
    Preconditions: offckb devnet 运行
    Steps:
      1. cargo test -p udtx --test sudt_lifecycle
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-11-sudt-tests.txt
  ```

  **Commit**: YES
  - Message: `test(udtx): sUDT integration tests`
  - Files: `udtx/tests/sudt_lifecycle.rs`

- [ ] 12. xUDT Token Create

  **What to do**:
  - 实现 `udtx token create --type xudt` 命令
  - 构建 xUDT metadata cell（含 config flags, access mode 等）
  - 使用 lib/types 的 XudtMeta 序列化
  - 支持 --dry-run

  **Must NOT do**:
  - 不要支持自定义 extensions（MVP 范围外）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: xUDT metadata 更复杂

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 13,14,17
  - **Blocked By**: Tasks 2,4,5,8

  **References**:
  - `lib/types/src/metadata/token.rs` - XudtMeta 定义
  - `tests/src/test_helpers/metadata.rs` - xUDT metadata 构建
  - `tests/src/tests/xudt_meta/creation_supply.rs`

  **Acceptance Criteria**:
  - [ ] `udtx token create --type xudt` 成功创建
  - [ ] 支持 tracked/untracked supply
  - [ ] 支持 access mode 配置

  **QA Scenarios**:

  ```
  Scenario: Create xUDT token
    Tool: Bash
    Preconditions: offckb devnet 运行
    Steps:
      1. cargo run -p udtx -- token create --type xudt --name "XTest" --symbol "XT" --decimals 8 --supply 1000000 --access blacklist --owner ckt1... --dry-run
    Expected Result: dry-run 显示交易信息
    Evidence: .sisyphus/evidence/task-12-xudt-create.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): xUDT token create`
  - Files: `udtx/src/commands/token/create.rs`（扩展）

- [ ] 13. xUDT Token Mint

  **What to do**:
  - 实现 xUDT mint 命令
  - 处理 access mode 检查
  - 支持 --dry-run

  **Must NOT do**:
  - 不要跳过 access check

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要 access mode 验证

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 4,5,12

  **References**:
  - `tests/src/tests/xudt/mint.rs`

  **Acceptance Criteria**:
  - [ ] xUDT mint 成功
  - [ ] Blacklist 模式下 blocked 地址无法接收

  **QA Scenarios**:

  ```
  Scenario: Mint xUDT tokens
    Tool: Bash
    Preconditions: xUDT token 已创建
    Steps:
      1. cargo run -p udtx -- token mint --token <hash> --to ckt1... --amount 1000
    Expected Result: 交易成功
    Evidence: .sisyphus/evidence/task-13-xudt-mint.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): xUDT token mint`
  - Files: `udtx/src/commands/token/mint.rs`（扩展）

- [ ] 14. xUDT Token Transfer

  **What to do**:
  - 实现 xUDT transfer 命令
  - 处理 access mode（whitelist/blacklist）
  - 支持 --dry-run

  **Must NOT do**:
  - 不要绕过 access control

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要 AccessList 验证

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 4,5,12

  **References**:
  - `tests/src/tests/xudt/lifecycle.rs`
  - `tests/src/tests/xudt/access.rs`

  **Acceptance Criteria**:
  - [ ] xUDT transfer 成功
  - [ ] Blacklist 模式下 blocked transfer 被拒绝
  - [ ] Whitelist 模式下未授权 transfer 被拒绝

  **QA Scenarios**:

  ```
  Scenario: Transfer xUDT with access control
    Tool: Bash
    Preconditions: xUDT token 已创建，blacklist 模式
    Steps:
      1. cargo run -p udtx -- access add --token <hash> --address ckt1...
      2. cargo run -p udtx -- token transfer --token <hash> --to ckt1... --amount 100
    Expected Result: 交易失败，错误码 61
    Evidence: .sisyphus/evidence/task-14-xudt-transfer.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): xUDT token transfer`
  - Files: `udtx/src/commands/token/transfer.rs`（扩展）

- [ ] 15. AccessList Commands

  **What to do**:
  - 实现 `udtx access enable/disable` 命令
  - 实现 `udtx access add/remove` 命令
  - 构建 AccessList shard cells
  - 使用 lib/types 的 AccessListShard 序列化

  **Must NOT do**:
  - 不要支持 shard split/merge（MVP 范围外）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要理解 AccessList shard 结构

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 18
  - **Blocked By**: Tasks 4,5,12

  **References**:
  - `lib/types/src/metadata/access_list.rs` - AccessListShard 定义
  - `tests/src/test_helpers/access_list.rs` - shard 构建
  - `tests/src/tests/access_list/`

  **Acceptance Criteria**:
  - [ ] AccessList enable/disable 成功
  - [ ] Add/remove 条目成功
  - [ ] Full-domain 要求满足

  **QA Scenarios**:

  ```
  Scenario: Manage AccessList
    Tool: Bash
    Preconditions: xUDT token 已创建
    Steps:
      1. cargo run -p udtx -- access enable --token <hash> --mode blacklist
      2. cargo run -p udtx -- access add --token <hash> --address ckt1...
      3. cargo run -p udtx -- access remove --token <hash> --address ckt1...
    Expected Result: 所有操作成功
    Evidence: .sisyphus/evidence/task-15-access-list.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): AccessList commands`
  - Files: `udtx/src/commands/access.rs`

- [ ] 16. Authority Commands

  **What to do**:
  - 实现 `udtx authority rotate` 命令
  - 实现 `udtx authority drop` 命令
  - 支持 mint_authority, metadata_authority, access_authority
  - 需要显式确认（--yes）

  **Must NOT do**:
  - 不要允许无确认删除 authority

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 涉及权限变更，需谨慎

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Tasks 4,5,12

  **References**:
  - `lib/types/src/metadata/authority.rs` - Authority 定义
  - `tests/src/tests/xudt_meta/plugin_authority.rs`

  **Acceptance Criteria**:
  - [ ] Authority rotate 成功
  - [ ] Authority drop 需要 --yes 确认
  - [ ] Drop 后无法再 mint

  **QA Scenarios**:

  ```
  Scenario: Rotate and drop authority
    Tool: Bash
    Preconditions: xUDT token 已创建
    Steps:
      1. cargo run -p udtx -- authority rotate --token <hash> --type mint --to ckt1...
      2. cargo run -p udtx -- authority drop --token <hash> --type mint --yes
      3. cargo run -p udtx -- token mint --token <hash> --to ckt1... --amount 100
    Expected Result: rotate 成功，drop 成功，后续 mint 失败
    Evidence: .sisyphus/evidence/task-16-authority.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): authority commands`
  - Files: `udtx/src/commands/authority.rs`

- [ ] 17. xUDT Integration Tests

  **What to do**:
  - 编写 xUDT 端到端测试
  - 测试 create, mint, transfer, access control
  - 测试错误场景

  **Must NOT do**:
  - 不要修改生产代码

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 复杂测试场景

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Tasks 7,12,13,14

  **References**:
  - `tests/src/tests/xudt/`

  **Acceptance Criteria**:
  - [ ] xUDT 完整生命周期测试通过
  - [ ] Access control 测试通过

  **QA Scenarios**:

  ```
  Scenario: xUDT full lifecycle test
    Tool: Bash
    Preconditions: offckb devnet 运行
    Steps:
      1. cargo test -p udtx --test xudt_lifecycle
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-17-xudt-tests.txt
  ```

  **Commit**: YES
  - Message: `test(udtx): xUDT integration tests`
  - Files: `udtx/tests/xudt_lifecycle.rs`

- [ ] 18. AccessList Integration Tests

  **What to do**:
  - 编写 AccessList 端到端测试
  - 测试 enable/disable, add/remove
  - 测试 blacklist/whitelist 模式

  **Must NOT do**:
  - 不要修改生产代码

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要验证 AccessList 行为

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Tasks 7,15

  **References**:
  - `tests/src/tests/access_list/`

  **Acceptance Criteria**:
  - [ ] AccessList 测试通过
  - [ ] Blacklist/whitelist 模式测试通过

  **QA Scenarios**:

  ```
  Scenario: AccessList lifecycle test
    Tool: Bash
    Preconditions: offckb devnet 运行
    Steps:
      1. cargo test -p udtx --test access_list_lifecycle
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-18-access-tests.txt
  ```

  **Commit**: YES
  - Message: `test(udtx): AccessList integration tests`
  - Files: `udtx/tests/access_list_lifecycle.rs`

- [ ] 19. Plan Command

  **What to do**:
  - 实现 `udtx plan` 命令
  - 将配置编译为确定性执行计划
  - 输出 `artifacts/plan.json` 和 `artifacts/plan.md`
  - 验证前置条件

  **Must NOT do**:
  - 不要发送任何交易

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - Reason: 需要理解所有命令的依赖关系

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 20,23
  - **Blocked By**: Tasks 2,8,12

  **References**:
  - Terraform plan 模式
  - `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` - Plan 设计

  **Acceptance Criteria**:
  - [ ] `udtx plan` 生成 plan.json
  - [ ] plan.json 包含所有步骤和依赖
  - [ ] 无效配置时拒绝生成 plan

  **QA Scenarios**:

  ```
  Scenario: Generate execution plan
    Tool: Bash
    Preconditions: 有效配置
    Steps:
      1. cargo run -p udtx -- plan
      2. cat artifacts/plan.json
      3. cat artifacts/plan.md
    Expected Result: 文件存在，格式正确
    Evidence: .sisyphus/evidence/task-19-plan/
  ```

  **Commit**: YES
  - Message: `feat(udtx): plan command`
  - Files: `udtx/src/commands/plan.rs`

- [ ] 20. Apply Command

  **What to do**:
  - 实现 `udtx apply` 命令
  - 读取 plan.json 并执行
  - 发送交易并等待确认
  - 输出 `artifacts/run.json` 和 `artifacts/tx-timeline.json`

  **Must NOT do**:
  - 不要跳过确认提示（除非 --yes）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - Reason: 需要处理交易失败和重试

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 21,23
  - **Blocked By**: Tasks 4,19

  **References**:
  - Terraform apply 模式
  - ckb-sdk-rust 交易发送模式

  **Acceptance Criteria**:
  - [ ] `udtx apply` 执行 plan 中的所有步骤
  - [ ] 交易成功确认
  - [ ] 失败时提供清晰错误信息

  **QA Scenarios**:

  ```
  Scenario: Apply execution plan
    Tool: Bash
    Preconditions: plan.json 已生成
    Steps:
      1. cargo run -p udtx -- apply --yes
      2. cat artifacts/run.json
    Expected Result: 所有步骤执行成功，tx hash 记录
    Evidence: .sisyphus/evidence/task-20-apply/
  ```

  **Commit**: YES
  - Message: `feat(udtx): apply command`
  - Files: `udtx/src/commands/apply.rs`

- [ ] 21. Verify Command

  **What to do**:
  - 实现 `udtx verify` 命令
  - 验证链上状态（余额、metadata、access list）
  - 验证预期失败
  - 输出 `artifacts/verify.json`

  **Must NOT do**:
  - 不要修改链上状态

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 需要 RPC 查询和状态比较

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 22,23
  - **Blocked By**: Tasks 4,20

  **References**:
  - `tests/src/fixtures.rs` - 验证模式

  **Acceptance Criteria**:
  - [ ] `udtx verify` 验证所有断言
  - [ ] 不匹配时报告差异

  **QA Scenarios**:

  ```
  Scenario: Verify chain state
    Tool: Bash
    Preconditions: apply 已执行
    Steps:
      1. cargo run -p udtx -- verify
      2. cat artifacts/verify.json
    Expected Result: 所有断言通过
    Evidence: .sisyphus/evidence/task-21-verify.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): verify command`
  - Files: `udtx/src/commands/verify.rs`

- [ ] 22. Report Command

  **What to do**:
  - 实现 `udtx report` 命令
  - 生成 Markdown 报告（人类可读）
  - 生成 JSON 报告（机器可读）
  - 包含摘要、交易列表、最终状态、兼容性提示

  **Must NOT do**:
  - 不要泄露私钥

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 主要是格式化和输出

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 24
  - **Blocked By**: Task 21

  **References**:
  - `docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` - Report 设计

  **Acceptance Criteria**:
  - [ ] `udtx report` 生成 report.md 和 report.json
  - [ ] Markdown 包含所有必要章节
  - [ ] JSON 可被解析

  **QA Scenarios**:

  ```
  Scenario: Generate reports
    Tool: Bash
    Preconditions: verify 已执行
    Steps:
      1. cargo run -p udtx -- report
      2. cat artifacts/report.md
      3. cat artifacts/report.json
    Expected Result: 文件存在，格式正确
    Evidence: .sisyphus/evidence/task-22-report/
  ```

  **Commit**: YES
  - Message: `feat(udtx): report command`
  - Files: `udtx/src/commands/report.rs`

- [ ] 23. Plan/Apply Integration Tests

  **What to do**:
  - 测试 plan -> apply -> verify 完整生命周期
  - 测试错误恢复
  - 测试 idempotency

  **Must NOT do**:
  - 不要修改生产代码

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - Reason: 端到端测试

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Tasks 19,20

  **References**:
  - 现有测试模式

  **Acceptance Criteria**:
  - [ ] 完整生命周期测试通过
  - [ ] 错误恢复测试通过

  **QA Scenarios**:

  ```
  Scenario: Full lifecycle test
    Tool: Bash
    Preconditions: offckb devnet 运行
    Steps:
      1. cargo test -p udtx --test lifecycle
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-23-lifecycle-tests.txt
  ```

  **Commit**: YES
  - Message: `test(udtx): plan/apply integration tests`
  - Files: `udtx/tests/lifecycle.rs`

- [ ] 24. Report Verification

  **What to do**:
  - 验证报告格式和内容
  - 测试不同场景的报告

  **Must NOT do**:
  - 不要修改生产代码

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 格式验证

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Task 22

  **Acceptance Criteria**:
  - [ ] 报告格式正确
  - [ ] 内容完整

  **QA Scenarios**:

  ```
  Scenario: Report format validation
    Tool: Bash
    Preconditions: report 已生成
    Steps:
      1. cargo test -p udtx --test report
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-24-report-tests.txt
  ```

  **Commit**: YES
  - Message: `test(udtx): report verification`
  - Files: `udtx/tests/report.rs`

- [ ] 25. Documentation

  **What to do**:
  - 编写 README.md（安装、快速开始、命令参考）
  - 编写配置文档
  - 编写示例场景

  **Must NOT do**:
  - 不要重复设计文档内容

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []
  - Reason: 纯文档工作

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: ALL

  **Acceptance Criteria**:
  - [ ] README 包含安装说明
  - [ ] README 包含快速开始
  - [ ] README 包含所有命令示例

  **QA Scenarios**:

  ```
  Scenario: Documentation completeness
    Tool: Bash
    Preconditions: 无
    Steps:
      1. cat udtx/README.md | grep -c "```"
    Expected Result: 包含代码示例
    Evidence: .sisyphus/evidence/task-25-docs.txt
  ```

  **Commit**: YES
  - Message: `docs(udtx): README and documentation`
  - Files: `udtx/README.md`, `udtx/docs/`

- [ ] 26. Template Repository

  **What to do**:
  - 创建模板仓库（用于快速开始）
  - 包含示例配置和场景

  **Must NOT do**:
  - 不要包含私钥

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 模板创建

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Task 25

  **Acceptance Criteria**:
  - [ ] 模板仓库可 clone 并运行
  - [ ] 包含完整示例

  **QA Scenarios**:

  ```
  Scenario: Template works
    Tool: Bash
    Preconditions: 无
    Steps:
      1. git clone <template-repo>
      2. cd udtx-template && udtx doctor
    Expected Result: doctor 通过
    Evidence: .sisyphus/evidence/task-26-template.txt
  ```

  **Commit**: YES
  - Message: `feat(udtx): template repository`
  - Files: `templates/`

- [ ] 27. GitHub Actions CI

  **What to do**:
  - 创建 `.github/workflows/ci.yml`
  - 配置 build, test, clippy, fmt
  - 配置 offckb devnet 测试

  **Must NOT do**:
  - 不要泄露私钥

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: CI 配置

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: ALL

  **Acceptance Criteria**:
  - [ ] CI 通过
  - [ ] 所有检查通过

  **QA Scenarios**:

  ```
  Scenario: CI passes
    Tool: Bash
    Preconditions: 无
    Steps:
      1. act -j test
    Expected Result: 所有任务通过
    Evidence: .sisyphus/evidence/task-27-ci.txt
  ```

  **Commit**: YES
  - Message: `ci(udtx): GitHub Actions workflow`
  - Files: `.github/workflows/ci.yml`

- [ ] 28. Release Preparation

  **What to do**:
  - 版本号管理
  - Changelog
  - 发布脚本

  **Must NOT do**:
  - 不要自动发布

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - Reason: 发布准备

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Task 27

  **Acceptance Criteria**:
  - [ ] 版本号正确
  - [ ] Changelog 完整

  **QA Scenarios**:

  ```
  Scenario: Release ready
    Tool: Bash
    Preconditions: 无
    Steps:
      1. cargo check -p udtx
      2. cargo test -p udtx
    Expected Result: 编译和测试通过
    Evidence: .sisyphus/evidence/task-28-release.txt
  ```

  **Commit**: YES
  - Message: `chore(udtx): release preparation`
  - Files: `udtx/CHANGELOG.md`, `udtx/Cargo.toml`（版本）

- [ ] 29. Ecosystem Promotion

  **What to do**:
  - 编写论坛回复（基于 devnet practice summary）
  - 准备 demo 脚本
  - 准备兼容性报告模板

  **Must NOT do**:
  - 不要过度承诺

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []
  - Reason: 推广材料

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Task 28

  **Acceptance Criteria**:
  - [ ] 论坛回复草稿完成
  - [ ] Demo 脚本可运行

  **QA Scenarios**:

  ```
  Scenario: Promotion materials ready
    Tool: Bash
    Preconditions: 无
    Steps:
      1. cat docs/promotion/forum-reply.md
      2. cat docs/promotion/demo.sh
    Expected Result: 文件存在，内容完整
    Evidence: .sisyphus/evidence/task-29-promotion/
  ```

  **Commit**: YES
  - Message: `docs(udtx): ecosystem promotion materials`
  - Files: `docs/promotion/`

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
  读取计划端到端。验证每个 "Must Have" 的实现存在。验证每个 "Must NOT Have" 不存在。检查证据文件。
  输出: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  运行 `cargo check`, `cargo clippy`, `cargo test`。检查 `as any`, 空 catch, console.log, 注释掉的代码, 未使用的导入。
  输出: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  从干净状态开始，执行每个任务的 QA 场景。测试跨任务集成。
  输出: `Scenarios [N/N pass] | Integration [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  验证每个任务的实现与规格 1:1 匹配。检查范围蔓延。
  输出: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- 每个任务独立 commit
- 格式: `type(udtx): description`
- type: feat, test, docs, ci, chore
- 每个 commit 前运行 `cargo check` 和 `cargo test`

---

## Success Criteria

### Verification Commands
```bash
# 编译
cargo check -p udtx

# 测试
cargo test -p udtx

# 代码质量
cargo clippy -p udtx
cargo fmt -p udtx -- --check

# 端到端测试（需要 offckb devnet）
offckb node &
cargo test -p udtx --test e2e
```

### Final Checklist
- [ ] 所有 "Must Have" 已实现
- [ ] 所有 "Must NOT Have" 已排除
- [ ] 所有测试通过
- [ ] CI 通过
- [ ] 文档完整
- [ ] 模板仓库可用
- [ ] 推广材料准备就绪