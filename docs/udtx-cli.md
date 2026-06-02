# UDTX CLI 使用指南

UDTX 是 `ckb-standard-udt-contracts` 项目的配套 CLI 工具，用于在 CKB 链上发行、转移和管理 sUDT/xUDT token。它基于 `ckb-sdk-rust` 构建，支持本地 devnet 测试和主网/测试网操作。

## 构建

UDTX CLI 使用 Cargo 构建，不需要额外的前端依赖：

```bash
# 开发构建
cargo build --bin udtx

# Release 构建（推荐日常使用）
cargo build --bin udtx --release
```

构建产物位于 `target/release/udtx`（或 `target/debug/udtx`）。

> **依赖提示**：构建需要 OpenSSL 开发头文件。如果环境缺少，可能需要在 `OPENSSL_DIR` 或 `PKG_CONFIG_PATH` 中指定路径。

## 快速开始

### 1. 初始化项目

```bash
udtx init --name my-token
```

这会生成两个文件：
- `udtx.yaml` — 项目级配置（网络、账户、token 默认参数）
- `profiles/devnet.yaml` — devnet 专用的 RPC 和合约引用配置

### 2. 配置账户

编辑 `udtx.yaml`，填入你的账户信息。支持两种形式：

**环境变量私钥（推荐，避免明文保存）**：
```yaml
accounts:
  owner:
    private_key_env: OWNER_PRIVKEY
```

**直接地址（只读场景）**：
```yaml
accounts:
  alice:
    address: "ckt1..."
```

### 3. 启动本地 Devnet

使用 `@offckb/cli` 管理本地 devnet：

```bash
npm install -g @offckb/cli
offckb node
```

第一次运行会初始化 devnet 配置，之后启动节点和 miner 即可。

启动后，确保 `profiles/devnet.yaml` 中的 `rpc_url` 指向正确的节点地址（默认 `http://127.0.0.1:8114`）。

### 4. 部署合约并填写 Profile

**devnet 的合约配置默认是占位符**，因为每个人本地启动的 devnet 都是全新的链，上面并没有预先部署任何合约。你需要先自己部署合约，然后把部署信息填回 `profiles/devnet.yaml`。

#### 构建合约

```bash
rustup target add riscv64imac-unknown-none-elf
make build MODE=release
```

> **注意**：`sudt-meta` 构建前需要设置 `SUDT_CODE_HASH` 环境变量（即 sudt 合约的 data_hash），`xudt-meta` 需要 `XUDT_CODE_HASH` 和 `ACCESS_LIST_CODE_HASH`。Makefile 会自动处理这些依赖，直接 `make build` 即可。

构建完成后，合约二进制位于 `build/release/`：

| 合约 | 二进制路径 |
|------|-----------|
| sudt | `build/release/sudt` |
| xudt | `build/release/xudt` |
| access-list | `build/release/access-list` |
| sudt-meta | `build/release/sudt-meta` |
| xudt-meta | `build/release/xudt-meta` |

> **注意**：`always_success` 不是本仓库的合约，它是 CKB 生态中常见的无验证脚本。如果你需要部署它（例如 `sudt-meta` 要求 metadata cell 使用 always_success lock），可以从 [ckb-system-scripts](https://github.com/nervosnetwork/ckb-system-scripts) 获取对应二进制，或使用 offckb 内置的 always_success 脚本。

#### 使用 offckb 部署（推荐）

[offckb](https://github.com/nervosnetwork/offckb) 提供了更便捷的部署命令，支持批量部署并自动记录部署结果：

```bash
npm install -g @offckb/cli
offckb deploy --target build/release --network devnet --output deployment
```

部署完成后，`deployment/` 目录下会生成记录文件，包含每个合约的 `tx_hash`、`index` 和 `data_hash`（即 profile 中的 `code_hash`）。

#### 填写部署信息

将部署得到的 `data_hash`、`hash_type` 和 `outpoint`（tx_hash + index）填入 `profiles/devnet.yaml` 的对应字段。

**填写示例**：

```yaml
contracts:
  sudt:
    code_hash: '0xd74751bfcf6b3050a99d33ba3c17e86ec807b72fd4feadcf6a639c538817d0c7'
    hash_type: data2
    outpoint:
      tx_hash: '0xcaeb3a9a1f8524e1fa5e08d1c49ecf27a2616e165c7e9831befa2848285eeeb8'
      index: 0
```

所有合约配置都必须填写正确，`udtx doctor` 才能通过合约引用检查。

### 5. 环境检查

```bash
udtx doctor
```

`doctor` 会依次检查：
- 配置文件合法性
- RPC 连通性（链名、区块高度）
- 账户余额
- 合约引用是否能在链上找到对应的 live cell

如果全部通过，说明环境就绪。

### 6. 发行 Token（Dry-Run 预览）

在实际发送交易前，强烈建议先用 `--dry-run` 预览：

```bash
udtx token issue \
  --token-type sudt \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 8 \
  --supply 1000000 \
  --owner owner \
  --dry-run
```

输出示例：
```
Token Issue Preview
  Token Type: Sudt
  Name: My Token
  Symbol: MTK
  Decimals: 8
  Initial Supply: 1000000
  Transaction Hash: 0xae3a3113...
```

确认无误后，去掉 `--dry-run` 正式发送：

```bash
udtx token issue \
  --token-type sudt \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 8 \
  --supply 1000000 \
  --owner owner
```

### 7. 查询 Token 信息

```bash
udtx token info --owner owner
```

### 8. 转移 Token

```bash
udtx token transfer \
  --to "ckt1..." \
  --amount 1000 \
  --owner owner \
  --dry-run
```

### 9. Mint 增发

```bash
udtx token mint \
  --amount 500000 \
  --owner owner \
  --dry-run
```

### 10. Burn 销毁

```bash
udtx token burn \
  --amount 100000 \
  --owner owner \
  --dry-run
```

## 配置文件详解

### `udtx.yaml`（项目配置）

```yaml
version: 1
project:
  name: my-token
network:
  profile: devnet          # 引用 profiles/ 下的网络配置
  rpc: null                # 可覆盖 profile 中的 RPC 地址
accounts:
  owner:
    private_key_env: OWNER_PRIVKEY
contracts:
  source:
    mode: deployed-artifacts
    scripts_json: ./artifacts/devnet-scripts.json
token:
  kind: sudt               # 默认 token 类型：sudt 或 xudt
  symbol: MTK
  decimals: 8
  supply_policy:
    mode: tracked          # tracked / untracked
    fixed_after_issue:
      enabled: false
  authorities:
    mint: owner
    metadata: owner
    access: owner
access_control:
  enabled: false
  mode: blacklist          # blacklist / whitelist
  addresses: []
```

### `profiles/<name>.yaml`（网络配置）

```yaml
name: devnet
rpc_url: http://127.0.0.1:8114
network_type: devnet
system_scripts:
  secp256k1_blake160:
    code_hash: '0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8'
    hash_type: type
contracts:
  sudt:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data2
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
  xudt:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data2
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
  access_list:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data1
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
  sudt-meta:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data1
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
  xudt-meta:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data1
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
  always_success:
    code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    hash_type: data2
    outpoint:
      tx_hash: '0x0000000000000000000000000000000000000000000000000000000000000000'
      index: 0
```

**关键字段说明**：
- `contracts.<name>.code_hash`：合约的 **CKB data_hash**（注意不是文件 raw hash）。
- `contracts.<name>.hash_type`：`data1`（VM v1，B 扩展指令）或 `data2`（VM v2）。
- `contracts.<name>.outpoint`：合约部署交易的 tx_hash 和 output index。

**devnet 占位符说明**：
devnet 的合约配置默认全部是 `0x0000...` 占位符，因为每个人本地启动的 devnet 都是全新的链，合约需要自行部署后才能使用。部署完成后，请通过链上查询获取真实的 `code_hash`、`hash_type` 和 `outpoint` 填入 profile。testnet 和 mainnet 同理，只是它们的合约通常由项目方或社区统一部署，用户只需获取并填写公开信息即可。

## 命令参考

| 命令 | 说明 |
|------|------|
| `udtx init [--name <name>]` | 初始化项目配置 |
| `udtx doctor` | 环境、配置、链上合约引用综合检查 |
| `udtx env check` | 检查 RPC 连通性和链状态 |
| `udtx token issue [选项]` | 发行新 token |
| `udtx token transfer [选项]` | 转移 token |
| `udtx token mint [选项]` | 增发 token |
| `udtx token burn [选项]` | 销毁 token |
| `udtx token info [选项]` | 查询 token 信息和余额 |
| `udtx access list` | 查看访问控制列表 |
| `udtx access add --address <addr>` | 添加访问控制条目 |
| `udtx access remove --address <addr>` | 移除访问控制条目 |
| `udtx authority show` | 查看当前权限配置 |
| `udtx authority update` | 更新权限配置 |
| `udtx authority drop --yes` | 丢弃权限（需确认） |
| `udtx plan` | 预览计划中的变更 |
| `udtx apply [--yes]` | 应用计划中的变更 |
| `udtx verify` | 验证配置或链上状态 |
| `udtx report [-f markdown\|json]` | 生成报告 |

### `token issue` 参数

```
-t, --token-type <TYPE>    sudt 或 xudt（默认 sudt）
-n, --name <NAME>          Token 名称
-s, --symbol <SYMBOL>      Token 符号
-d, --decimals <DECIMALS>  小数位
-S, --supply <SUPPLY>      初始供应量
-o, --owner <OWNER>        发行者账户名
    --dry-run              预览交易，不发送
```

### `token transfer` 参数

```
-t, --to <TO>              接收方地址
-a, --amount <AMOUNT>      转移数量
-t, --token-type <TYPE>    覆盖 token 类型
-o, --owner <OWNER>        发送方账户名
    --dry-run              预览交易，不发送
```

### `token mint` / `token burn` 参数

```
-a, --amount <AMOUNT>      数量
-t, --token-type <TYPE>    覆盖 token 类型
-o, --owner <OWNER>        操作账户名
    --dry-run              预览交易，不发送
```

## 实际使用经验与注意事项

### 合约部署后如何正确获取 `code_hash`

profile 中的 `code_hash` 必须是 **CKB 节点的 data_hash**（使用 `blake2b-256 + "ckb-default-hash"` personalization），而不是对本地二进制文件直接做 raw blake2b-256。**不要手动计算**，正确做法是：

1. **部署后查询链上数据**：
   ```bash
   ckb-cli rpc get_transaction --tx-hash <tx_hash>
   ```
   查看 outputs_data，再用 `ckb-cli util blake2b --binary-hex <data>` 计算。

2. **直接查询 live cell**：
   ```bash
   ckb-cli rpc get_live_cell --tx-hash <tx_hash> --index <index> --with-data
   ```
   返回结果中直接包含 `data_hash` 字段。

3. **理想情况**：如果 CLI 后续支持 `deploy` 命令，应自动将 `(tx_hash, index, data_hash)` 写回 profile。

### `hash_type` 的选择

- **`data1`**：对应 CKB VM v1，支持 B 扩展指令（`sh3add`、`cpop` 等）。本项目的合约需要 B 扩展，所以之前使用 `data1`。
- **`data2`**：对应 CKB VM v2。如果节点和 SDK 版本支持，部分合约也可以使用 `data2`。
- **选择原则**：以合约实际部署时使用的 hash_type 为准，profile 中的 `hash_type` 必须与链上一致，否则 `udtx doctor` 会报合约引用验证失败。

### `--dry-run` 的重要性

**强烈建议在每次发送交易前都使用 `--dry-run`**。它的作用不仅是预览，还能在本地提前暴露问题：
- 配置错误（账户不存在、余额不足）
- 合约引用不匹配（code_hash / hash_type 错误）
- 交易构造失败（参数非法、依赖缺失）

### Devnet 常见问题

1. **首次启动 offckb devnet**：
   - `offckb node` 首次运行会在当前目录生成 `offckb` 文件夹和配置。
   - 可能需要手动编辑 `ckb.toml`，启用 `Indexer` 和 `Miner` RPC 模块，并配置 `block_assembler`（填你的测试地址）。

2. **区块不产出**：
   - 检查 miner 是否也在运行。offckb 通常需要同时启动节点和 miner。
   - 检查 `block_assembler` 配置是否正确。

3. **余额不足**：
   - offckb devnet 的 genesis 会给预设地址分配大量 CKB（约 4200 万）。确保你的测试账户对应的是 genesis 地址，或者从 genesis 地址转账过去。

### Metadata 支持

本项目的 `sudt` / `xudt` 合约设计强制要求 metadata cell 参与。首次发行 token 时，交易需要同时构造 metadata output cell（包含 name、symbol、decimals、supply_policy、authorities 等），并将 metadata type script hash 作为 UDT type script 的 args。

如果 `udtx token issue` 实际发送失败并返回错误码 **41（MetaMissing）**，说明 CLI 当前版本尚未完整实现 metadata cell 的构造逻辑。此时 `--dry-run` 仍可用于验证交易骨架是否构建正确。

## 故障排查

| 现象 | 可能原因 | 解决方法 |
|------|----------|----------|
| `doctor` 报合约引用失败 | 合约未部署，或 `code_hash` / `hash_type` / `outpoint` 错误 | 先部署合约到链上，然后从链上查询正确的 data_hash、hash_type 和 outpoint 填入 profile |
| `ScriptNotFound` | hash_type 与链上实际不符，或 code_hash 是 raw hash 而非 CKB data_hash | 同上 |
| `MetaMissing` (41) | 交易缺少 metadata cell | token issue 需要同时构造 metadata output；检查 CLI 版本是否支持 |
| `InvalidSupply` (31) | supply delta 与 metadata 不一致 | 检查 token 参数和 metadata 中的 supply_policy |
| 地址解析失败 | 使用了 short format 地址 | 使用 full format 地址（RFC21） |
| `H256` 解析失败 | tx_hash 带 `0x` 前缀 | 某些版本需要去掉前缀；当前代码已修复此问题 |
| 余额显示为 0 | 账户不是 genesis 地址 | 从 devnet genesis 地址转账，或配置正确的 genesis 私钥 |

## 相关资源

- [CKB UDT 标准合约 README](../README.md)
- [CKB 官方文档](https://docs.nervos.org/)
- [offckb CLI](https://github.com/nervosnetwork/offckb)
