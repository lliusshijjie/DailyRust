# 第 29 课 · Cargo 工程化：features、workspace 与 MSRV

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`29_cargo_features.rs`

## 一、底层原理

### 29.1 Cargo 在构建流程中的位置

```
Cargo.toml ──▶ 依赖解析（resolver）──▶ Cargo.lock
     │                                   │
     ├── feature 选择 ──▶ rustc --cfg feature="..."
     ├── target/profile ──▶ rustc 参数（opt-level、lto、panic...）
     └── workspace 成员 ──▶ 多个 crate 的编译图
```

- **Cargo** 是构建编排器 + 依赖解析器；真正编译仍是 `rustc`
- **`Cargo.toml`** 是清单：包元数据、依赖、features、profile、workspace
- **`Cargo.lock`** 锁定精确版本，保证可复现；二进制/应用应提交，库通常不发布它
- **`target/`** 存放编译产物；workspace 共享一个 `target/`
- **features** 最终变成传给 rustc 的 `--cfg feature="name"`
- **MSRV**（Minimum Supported Rust Version）由 `rust-version` 声明

### 29.2 features 的编译期模型

```toml
[features]
default = ["text"]
text = []
json = ["dep:serde_json"]

[dependencies]
serde_json = { version = "1", optional = true }
```

对应代码：

```rust
#[cfg(feature = "json")]
fn encode(...) -> String { /* json 实现 */ }

#[cfg(not(feature = "json"))]
fn encode(...) -> String { /* text 实现 */ }
```

关键规则：

1. **features 必须是可加的（additive）**：开启更多 feature 只能增加能力，
   不能改变已有语义；否则 feature 合并会互相破坏
2. **feature 统一（unification）**：依赖图里只要有一个 crate 开启某 feature，
   所有依赖同一 crate 的地方都会看到它
3. **default features**：`default = [...]`；下游可用 `default-features = false` 关闭
4. **可选依赖**：`optional = true` 默认生成同名隐式 feature；
   用 `dep:serde_json` 可以只暴露 `json` feature，避免隐式名
5. **弱依赖 feature**`foo?/bar`：只有 `foo` 已启用时才启用 `foo` 的 `bar`
6. **resolver**：edition 2024 默认 resolver 3；它改善 feature 统一与 MSRV 兼容选择
7. **`cfg!` vs `#[cfg]`**：
   - `#[cfg]` 在编译期**删除**不满足的代码
   - `cfg!(...)` 是运行期 `bool`，**两个分支都会编译/类型检查**
   - 所以可选依赖只在 `#[cfg]` 分支里使用才安全

Cargo 还会向 rustc 传 `--check-cfg`，拼错 feature 名会触发 `unexpected_cfgs` 警告；
直接用 rustc 编译时没有这份元数据，示例用 `#![allow(unexpected_cfgs)]` 关闭。

### 29.3 workspace：多 crate 工程

```toml
[workspace]
members = ["crates/core", "crates/cli"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
```

成员：

```toml
[package]
name = "app-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
serde.workspace = true
```

- 一个 workspace 共享一个 `Cargo.lock`、一个 `target/`、一套 profile
- `[workspace.dependencies]` 统一版本，成员用 `workspace = true` 引用
- `[workspace.package]` 统一元数据（version/edition/rust-version/license）
- `cargo build -p app-core`、`cargo test --workspace`、`cargo run -p app-cli`
- `default-members` 控制不带 `-p` 时默认构建哪些成员
- 成员之间用 `path` 依赖；**发布时**还要写 `version`，否则 `cargo publish` 失败
- `[profile.*]` 只在 workspace 根生效，成员里的 profile 会被忽略

### 29.4 MSRV：最低支持的 Rust 版本

```toml
[package]
rust-version = "1.85"
```

- edition 2024 本身要求 Rust 1.85+，所以 MSRV 不能低于 1.85
- `rust-version` 是**声明**，Cargo 会在当前工具链低于它时拒绝构建
- 验证要用**最低版本工具链**实际跑：`cargo +1.85.0 check --all-features`
- `cargo-msrv` 可以自动找最低可用版本
- CI 建议矩阵：`stable` + `MSRV`（可选 beta）
- 依赖的 MSRV 会抬高你的 MSRV；升级依赖时用锁文件/CI 兜底
- `--ignore-rust-version` 可以强行构建，但不保证能编译
- `rust-toolchain.toml` 可以固定开发工具链，但 CI 仍要显式测 MSRV

### 29.5 profiles 与构建优化

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

[profile.dev]
opt-level = 1
```

- profile 只写在 workspace 根
- `lto`/`codegen-units` 影响优化与编译时间
- `strip` 减小二进制；`panic = "abort"` 减小体积但禁用 `catch_unwind`
- 自定义 profile 需要 `inherits`

### 29.6 常用工程命令

```bash
cargo build --release
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo tree -e features
cargo tree -d                 # 重复依赖版本
cargo metadata --no-deps
cargo package --list
cargo publish --dry-run
cargo update -p serde --precise 1.0.200
```

- `--all-features` 是 feature 矩阵的"最大集合"编译检查
- `--no-default-features` 检查最小集合
- `cargo tree -e features` 看 feature 从哪来
- `cargo tree -d` 发现同一 crate 的多个版本
- `cargo deny`/`cargo audit` 做许可证与安全审计（外部工具）

## 二、具体语法

### 30.1 `Cargo.toml` 基本结构

```toml
[package]
name = "app"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "..."
license = "MIT OR Apache-2.0"

[dependencies]
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tempfile = "3"

[features]
default = ["text"]
text = []
json = ["dep:serde_json"]

[dependencies.serde_json]
version = "1"
optional = true
```

### 30.2 feature 定义与使用

```toml
[features]
default = ["text"]
text = []
json = ["dep:serde_json"]
```

```rust
#[cfg(feature = "json")]
fn encode(...) -> String { ... }

#[cfg(not(feature = "json"))]
fn encode(...) -> String { ... }
```

- `default` 是默认开启的 feature 列表
- `dep:serde_json` 只启用依赖，不生成隐式 feature
- 下游：`app = { version = "0.1", default-features = false, features = ["json"] }`

### 30.3 `cfg!` 与 `#[cfg]`

```rust
if cfg!(feature = "json") {
    // 两个分支都会编译
}

#[cfg(feature = "json")]
{
    // 不满足时整段代码被删除
}
```

- `cfg!` 适合"两个分支都可用、只是行为不同"
- `#[cfg]` 适合"依赖可选 crate/模块，未启用时不能编译"
- 混用时要确保 `cfg!` 分支不引用未启用的依赖

### 30.4 用 rustc 验证 feature

Cargo 工程直接：

```bash
cargo run --features json
cargo check --no-default-features --features json
```

本仓库没有 `Cargo.toml`，用 rustc 模拟：

```bash
# 默认（无 feature）
rustc --edition 2024 -D warnings 29_cargo_features.rs -o run.exe

# PowerShell 会吞掉 --cfg 参数里的引号；用响应文件最稳
# args.rsp（ASCII 编码）：
#   --edition
#   2024
#   --cfg
#   feature="json"
#   -D
#   warnings
#   29_cargo_features.rs
#   -o
#   run_json.exe
rustc @args.rsp
```

- `--cfg feature="json"` 正是 Cargo 传参的形式
- 直接用 rustc 时没有 Cargo 的 `--check-cfg` 元数据，示例加了
  `#![allow(unexpected_cfgs)]`；真实工程不要加
- 默认/feature 两种构建最好都跑，避免"只在一个 feature 下能编译"

### 30.5 workspace 继承

```toml
[workspace]
members = ["crates/core", "crates/cli"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
```

```toml
[package]
name = "app-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
```

- 统一版本/元数据，避免成员漂移
- 成员发布时仍需要 `version`（workspace 继承可满足）
- path 依赖 + version 才能发布

### 30.6 MSRV 声明与验证

```toml
[package]
rust-version = "1.85"
```

```bash
rustup toolchain install 1.85.0
cargo +1.85.0 check --workspace --all-features
cargo +1.85.0 test --workspace
# 或
cargo msrv --verify
```

- CI 加一个 MSRV job
- 升级依赖/使用新语法前先问：MSRV 还成立吗
- `cargo update` 可能引入更高 MSRV 的依赖；锁定 + CI 兜底

### 30.7 lints 与发布

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
```

成员：

```toml
[lints]
workspace = true
```

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo publish --dry-run
```

## 三、实际应用与最佳实践

### 31.1 feature-gated 后端

```rust
#[cfg(feature = "json")]
fn encode(name: &str, age: u32) -> String {
    format!(r#"{{"name":"{name}","age":{age}}}"#)
}

#[cfg(not(feature = "json"))]
fn encode(name: &str, age: u32) -> String {
    format!("name={name};age={age}")
}
```

- 业务代码只调用 `encode`，不关心后端
- 默认 feature 保证"最小可用"
- `json` feature 才引入可选依赖/额外代码
- 两种构建都要在 CI 编译（`--no-default-features`、`--all-features`）

本课示例实测：

默认构建（12 行）：

```
json=false text=true
backend=text
cfg_bool=false
selected=text
encode=name=alice;age=30
payload=alice,30
encoded=name=alice;age=30
format=text
msrv=1.85
```

`json` 构建（12 行，关键差异）：

```
json=true text=false
backend=json
cfg_bool=true
selected=json
encode={"name":"alice","age":30}
encoded={"name":"alice","age":30}
format=json
```

### 31.2 workspace 分层

```
workspace/
├── Cargo.toml            # [workspace]
├── Cargo.lock
├── crates/
│   ├── core/             # 领域逻辑，无 IO
│   │   └── Cargo.toml
│   └── cli/              # 入口，依赖 core
│       └── Cargo.toml
└── target/
```

- `core` 不依赖 `cli`，依赖方向单向
- `cli` 用 `core = { path = "../core" }`
- 共享依赖放 `[workspace.dependencies]`
- 统一 `rust-version`/edition/license
- 发布时 `core` 先发，`cli` 依赖其版本

### 31.3 MSRV 与 CI

```yaml
# 伪代码
jobs:
  msrv:
    steps:
      - uses: dtolnay/rust-toolchain@1.85.0
      - run: cargo check --workspace --all-features
  stable:
    steps:
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --all-features
```

- MSRV job 必须真正用最低版本编译
- `--all-features` 覆盖最大代码面
- `--no-default-features` 覆盖最小代码面
- 依赖升级后重跑 MSRV job

### 31.4 最佳实践清单

1. **feature 保持可加**，不要做互斥开关
2. **默认 feature 提供最小可用能力**
3. **可选依赖用 `dep:`**，避免隐式 feature 名
4. **两种极端都编译**：`--all-features` 与 `--no-default-features`
5. **`cfg!` 只用于两分支都能编译的场景**
6. **workspace 统一 edition/rust-version/license**
7. **profile/lints 只写在 workspace 根**
8. **MSRV 用最低工具链实际验证**，并进 CI
9. **提交 `Cargo.lock`**（应用/workspace），保证可复现
10. **发布前 `cargo package`/`--dry-run`**，检查 path 依赖与文件列表

## 常见陷阱

1. **feature 非可加**：开 `json` 改变 `text` 行为，feature 合并后语义错乱
2. **互斥 feature**：`feature = "a"` 与 `"b"` 不能同时开，依赖图一合并就炸
3. **`cfg!` 里引用可选依赖**：未启用时两个分支都编译，直接失败
4. **feature 名拼错**：`cfg` 静默为 false；靠 `--check-cfg`/`--all-features` 发现
5. **`default-features = false` 后代码仍假设默认能力**：编译失败或行为缺失
6. **可选依赖隐式 feature 与自定义 feature 重名**：用 `dep:` 明确边界
7. **忘记 feature 统一**：一个依赖开启后全局生效，测试要覆盖组合
8. **workspace 成员写 profile**：被忽略；profile 放根
9. **成员各自 `Cargo.lock`**：workspace 只用根锁文件
10. **path 依赖没写 version**：`cargo publish` 失败
11. **MSRV 只写在文档里**：没有 CI 验证，迟早被新语法/依赖破坏
12. **依赖升级抬高 MSRV**：`cargo update` 后 MSRV job 失败
13. **edition 2024 却声明 `rust-version = "1.80"`**：edition 本身要求 1.85
14. **resolver 版本不匹配**：feature 统一/MSRV 解析行为不同
15. **`--all-features` 通过就以为没问题**：还要测 `--no-default-features`
16. **dev-dependencies 的 feature 影响正常构建**：理解 resolver 的作用域
17. **`[patch]` 写在成员里**：只在 workspace 根生效
18. **发布包含 `target/`/临时文件**：`cargo package --list` 检查
19. **`unsafe_code = "forbid"` 与依赖冲突**：lint 只管本 crate
20. **`cargo update` 不带 `-p` 全量升级**：不可控；按需升级并测 MSRV

## 完整示例

见 `29_cargo_features.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
# 默认构建
rustc --edition 2024 -D warnings "ERRORS/29_Cargo工程化/29_cargo_features.rs" -o run.exe
./run.exe

# json feature（响应文件方式，避免 PowerShell 吞引号）
rustc @args.rsp
```

示例默认构建共 12 行输出，覆盖：`cfg!(feature)`、feature-gated 模块/常量/函数、
`cfg!` 与 `#[cfg]` 的差异、统一编码接口、MSRV 常量。启用 `json` 后同样 12 行，
但 `json=true`、`backend=json`、`encode` 输出 JSON。

## 练习

1. 写一个 `Cargo.toml`，定义 `default = ["text"]`、`text`、`json` 三个 feature。
2. 用 `#[cfg(feature = "json")]` 切换两套编码实现，分别编译运行。
3. 把 `serde_json` 设为 optional，用 `json = ["dep:serde_json"]` 暴露 feature。
4. 建一个 workspace：`core` + `cli`，共享 `[workspace.dependencies]` 与 `rust-version`。
5. 声明 `rust-version = "1.85"`，用 `cargo +1.85.0 check --all-features` 验证。
6. 用 `cargo tree -e features` 观察某个 feature 从哪个依赖开启。
7. 分别用 `--all-features` 与 `--no-default-features` 编译，修复暴露的问题。
8. 解释 feature 统一为什么会让"互斥 feature"失效，并给出可加的设计方案。

## 参考

- The Cargo Book: [Features](https://doc.rust-lang.org/cargo/reference/features.html)、
  [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)、
  [Rust version](https://doc.rust-lang.org/cargo/reference/rust-version.html)、
  [Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- rustc: [`--check-cfg`](https://doc.rust-lang.org/rustc/command-line-arguments.html#--check-cfg)
- Cargo Book: [Resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)
- 工具：`cargo-msrv`、`cargo-deny`、`cargo-audit`、`cargo-semver-checks`
