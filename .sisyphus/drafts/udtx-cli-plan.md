# Draft: UDTX CLI 推进计划

## 用户确认的需求

### 技术栈
- **Rust** CLI 实现
- **offckb** 仅负责：启动本地链 / 部署合约到本地链
- CLI 通过配置写入原始基础合约的信息（code hash, hash type 等）
- **用户不需要自己部署合约**，只需要复用已部署的合约来发币

### 范围
- **一次性做完** - 完整 CLI + 生态推广

### 推广深度
- **中量级** - 正式发布 CLI 工具，写文档，做 demo，但暂不和钱包/交易所深度合作

## 关键设计决策

### 合约部署策略
- CLI 不处理合约部署
- 用户通过配置文件引用已部署的合约（devnet/testnet/mainnet 的 scripts.json）
- offckb 用于本地开发时启动 devnet 和部署合约到 devnet
- 对于 testnet/mainnet，用户需要自行部署或使用社区已部署的合约

### CLI 架构
- Rust CLI crate
- 配置驱动（YAML）
- plan -> apply -> verify -> report 生命周期
- 集成 ckb-sdk 或 ccc-rs 进行链上交互

## 已收集信息

### 代码库结构
- Workspace 结构：contracts/, lib/, tests/, tests/plugins/
- Cargo.toml 使用 @@INSERTION_POINT@@ 模式自动生成新 crate
- lib/types: 共享类型库（SudtMeta, XudtMeta, AccessListShard, Authority 等）
- lib/script-utils: 脚本端共享工具
- tests: ckb-testtool 集成测试，fixture 模式

### 关键类型（可用于 CLI）
- `SudtMeta` / `XudtMeta`: 元数据序列化/反序列化
- `Authority` / `AuthorityType`: 权限配置
- `AccessListShard` / `AccessListRange`: 访问列表
- `Extension` / `ExtensionType`: 扩展配置
- Config flags: CONFIG_SUPPLY_TRACKED, CONFIG_ACCESS_ENABLED, etc.

### 构建系统
- Makefile 驱动，支持 MODE=debug/release
- cargo generate 用于生成新合约
- 无 CI/CD 配置

### 测试基础设施
- ckb-testtool 0.14.0 用于集成测试
- 无现有 CI
- 建议为 CLI 添加集成测试

## 关键设计决策（已确认）

### 1. 链交互库
**ckb-sdk-rust**（官方 Rust SDK）
- 成熟稳定，社区支持好
- 提供交易构建、签名、发送的完整流程

### 2. 签名方式
**环境变量 + 私钥文件**
- 环境变量：OWNER_PRIVKEY 等
- 私钥文件：支持从文件加载
- MVP 阶段暂不支持硬件钱包

### 3. 报告格式
**Markdown + JSON 两者都要**
- Markdown：人类可读的审计报告
- JSON：机器可读的兼容性报告

## 研究任务

- [x] 探索代码库结构（已完成）
- [x] 检查测试基础设施（已完成）
- [x] 研究 CKB CLI 生态模式（已完成）
- [x] Metis 差距分析（已完成）

## Metis 差距分析结果

### 关键发现
1. **已有设计文档**：`docs/superpowers/specs/2026-05-21-udtx-cli-orchestration-design.md` 已包含 80% 的设计
2. **CLI 位置**：应作为 workspace 新成员，使用 `@@INSERTION_POINT@@` 插入
3. **lib/types 依赖**：CLI 应依赖 `lib/types` 的 `std` feature
4. **lib/script-utils**：no_std 专用，CLI 不能直接依赖

### 需要额外确认的问题
1. **Lock script 支持**：MVP 仅支持 secp256k1-blake160，Omnilock/ACP 后续版本
2. **xUDT 扩展**：MVP 仅支持 AccessList，其他扩展后续版本
3. **合约引用方式**：config 中应包含 outpoints 还是 type script hashes？
4. **测试策略**：单元测试 + devnet 集成测试（使用 offckb 的 20 个预 funded 账户）

### 技术风险
1. **ckb-sdk-rust xUDT 支持**：需验证 SDK 是否支持 XudtWitnessInput 构造
2. **依赖版本冲突**：ckb-sdk-rust、ckb-types、ckb-jsonrpc-types 版本需对齐
3. **Cell dep 管理**：每笔交易需要正确的 lock script、type script、extension cell deps

### 建议的 Guardrails
- 所有金额计算使用 checked_add/checked_sub
- 所有 mutating 命令支持 --dry-run
- 所有交易前验证 capacity >= occupied_capacity
- 所有网络操作前运行 udtx doctor 验证
- 所有错误信息包含可操作的修复建议
