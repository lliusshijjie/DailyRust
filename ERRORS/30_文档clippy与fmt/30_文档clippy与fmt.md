# 第 30 课 · 文档、clippy 与 fmt 工作流

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`30_docs_clippy_fmt.rs`

## 一、底层原理

### 30.1 三条工具链各管什么

| 工具 | 作用 | 驱动方式 | 配置 |
|---|---|---|---|
| `rustfmt` | 统一代码格式 | `cargo fmt` 调用 rustfmt | `rustfmt.toml` |
| `clippy` | 静态检查/风格/性能 lint | `cargo clippy` 调用 clippy-driver | `clippy.toml`、属性 |
| `rustdoc` | 生成文档、运行文档测试 | `cargo doc` / `cargo test --doc` | 文档注释、`RUSTDOCFLAGS` |

- 三者都不改变程序语义：fmt 只改排版，clippy 只报告（或按建议修复），
  rustdoc 只读取注释/生成文档
- 它们都是**质量门禁**：CI 里失败应该阻止合并
- 工具版本要固定（`rust-toolchain.toml`），避免不同机器格式/lint 结果漂移

### 30.2 文档即契约

Rust 的文档注释会被 rustdoc 提取，并且代码块会被当作**文档测试**执行：

```rust
/// 解析配置。
///
/// # Errors
///
/// 缺少 host 或端口为 0 时返回 [`ConfigError`]。
///
/// # Panics
///
/// 本函数不会 panic。
///
/// # Examples
///
/// ```
/// let config = parse_config("host=localhost\nport=8080").unwrap();
/// assert_eq!(config.port, 8080);
/// ```
pub fn parse_config(input: &str) -> Result<Config, ConfigError> { ... }
```

- `///` 是外层文档（给紧随其后的 item）
- `//!` 是内层文档（给所在模块/crate），通常放在文件顶部
- 文档代码块默认**编译并运行**；可用 `no_run`、`ignore`、`compile_fail`、
  `should_panic` 调整
- 文档测试让示例不会过期；但二进制 crate 的 doctest 无法直接引用 crate 私有项，
  复杂逻辑应放到库 crate（`src/lib.rs`）
- `RUSTDOCFLAGS="-D warnings" cargo doc` 可以把坏链接/坏文档升级为错误

### 30.3 clippy 的 lint 体系

clippy 在 rustc 之上增加大量 lint，按类别分组：

| 类别 | 默认级别 | 说明 |
|---|---|---|
| `correctness` | deny | 几乎肯定是 bug |
| `suspicious` | warn | 可疑写法 |
| `style` | warn | 惯用风格 |
| `complexity` | warn | 可以简化 |
| `perf` | warn | 可能有性能问题 |
| `pedantic` | allow | 更严格，按需开启 |
| `nursery` | allow | 实验性 |
| `restriction` | allow | 禁用某些特性，按项目选择 |
| `cargo` | allow | Cargo.toml 相关 |

关键用法：

- `cargo clippy --all-targets --all-features -- -D warnings`：CI 常用组合
- `--all-targets` 包含 lib/bin/tests/examples/benches；不加会漏掉测试代码
- `#[allow(clippy::lint_name)]` 局部豁免；`#![allow(...)]` crate 级
- 不要 `#[allow(clippy::all)]` 一把梭；豁免要写理由
- `cargo clippy --fix` 可以自动应用建议，但必须 review diff
- `clippy.toml` 配置阈值（如 `too-many-arguments-threshold`）

### 30.4 rustfmt 的规则

- `cargo fmt` 按 `rustfmt.toml` 重排代码；`cargo fmt -- --check` 只检查不修改
- 格式是确定性的：同一版本 rustfmt + 同一配置 → 同一结果
- 编辑器保存时格式化 + CI `--check` 是常见组合
- 部分选项是 **nightly-only**（`unstable_features = true`），stable CI 上不生效
- 直接对单文件运行要带 edition：`rustfmt --edition 2024 file.rs`
- 格式化提交最好独立，避免和逻辑改动混在一起产生巨大 diff

### 30.5 质量门禁的顺序

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

顺序理由：

1. fmt 最快，先失败先修，避免后续 diff 噪音
2. clippy 编译一次，尽早发现可疑代码
3. test 跑行为验证
4. doc 检查文档/链接（可与 test 并行）

### 30.6 单文件示例怎么映射

本仓库没有 `Cargo.toml`，示例文件本身演示：

- 文档注释（`///`、`//!`、`# Errors`、`# Panics`、`# Examples`）
- `#[deprecated]`、`#[doc(hidden)]`、`#[must_use]`
- `#![forbid(unsafe_code)]`
- `#[non_exhaustive]`
- `#[allow(clippy::manual_range_contains)]`

而 `cargo fmt`/`cargo clippy`/`cargo doc`/`cargo test` 在真实 Cargo 工程中执行；
本课用临时 Cargo 工程验证了这些命令（见文末）。

## 二、具体语法

### 31.1 文档注释

```rust
//! crate 级文档（文件顶部）

/// 结构体/函数文档（紧贴 item）
pub struct Config {
    /// 字段文档
    pub host: String,
}
```

- `///` 与 `#[doc = "..."]` 等价
- `//!` 只能出现在模块/crate 开头
- 文档里的 `[`Config`]` 是 intra-doc link，rustdoc 会检查

### 31.2 推荐章节

```rust
/// 一行摘要。
///
/// 详细说明。
///
/// # Errors
///
/// 什么情况下返回什么错误。
///
/// # Panics
///
/// 什么情况下 panic（没有就写"不会 panic"）。
///
/// # Safety
///
/// `unsafe fn` 必须写调用者要满足的前提。
///
/// # Examples
///
/// ```
/// assert_eq!(2 + 2, 4);
/// ```
```

- 公开 API 至少写摘要 + `# Errors`/`# Panics`
- `unsafe fn` 必须有 `# Safety`
- 示例代码块会被当作 doctest 执行

### 31.3 文档属性

```rust
#[doc(hidden)]
pub fn internal_helper() {}

#[deprecated(since = "0.2.0", note = "use parse_config instead")]
pub fn parse_config_legacy() {}

#[must_use]
pub fn status_label(code: u8) -> &'static str { ... }
```

- `#[doc(hidden)]`：公开但不出现在文档里（慎用）
- `#[deprecated]`：编译警告 + 迁移说明
- `#[must_use]`：忽略返回值时警告
- `#[non_exhaustive]`：外部不能穷尽匹配，为新增变体留余地

### 31.4 clippy 属性

```rust
#[allow(clippy::manual_range_contains)]
pub fn in_range(value: i32) -> bool {
    value >= 0 && value <= 150
}
```

- 局部 `#[allow]` 优于 crate 级
- 同一 crate 可 `#![deny(clippy::unwrap_used)]`（restriction 类）
- 豁免要写注释说明原因
- `#[expect(clippy::...)]` 比 `allow` 更好：lint 不再触发时会提醒删除

### 31.5 rustfmt 配置

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
use_small_heuristics = "Max"
```

- 稳定选项直接生效；不稳定选项需要 nightly
- `edition` 决定语法解析（2024）
- 团队统一配置，避免每人不同格式

### 31.6 CI 命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo package --list
```

- `--all-features` 覆盖最大代码面；再加一条 `--no-default-features` 覆盖最小面
- `RUSTDOCFLAGS` 让坏链接直接失败
- `cargo package --list` 检查发布内容

## 三、实际应用与最佳实践

### 32.1 质量门禁

```bash
# 本地提交前
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# CI
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

- 本地允许自动修复；CI 只检查
- `--all-targets` 别漏测试/示例
- 工具链固定，避免 lint 集合随版本变化

### 32.2 文档与 doctest

- 公开 API 写 `# Errors`/`# Panics`/`# Examples`
- 复杂示例放库 crate，确保 doctest 能引用 API
- 需要外部环境的示例标 `no_run`
- `RUSTDOCFLAGS="-D warnings"` 检查坏链接
- 文档里的示例要能编译；否则 CI 会失败

### 32.3 clippy 误报处理

1. 先判断是不是真问题
2. 能改就改，别急着 allow
3. 必须 allow 时：局部、加注释、用 `#[expect]`
4. 定期清理 allow，防止掩盖新问题

### 32.4 最佳实践清单

1. **fmt/clippy/test/doc 都进 CI**
2. **clippy 用 `--all-targets --all-features -- -D warnings`**
3. **fmt CI 用 `--check`，本地再自动格式化**
4. **公开 API 写 `# Errors`/`# Panics`/`# Examples`**
5. **doctest 保持可运行**
6. **`#[allow]` 局部化并写理由**
7. **固定工具链版本**
8. **`RUSTDOCFLAGS="-D warnings"` 检查文档链接**
9. **格式提交与逻辑提交分开**
10. **`cargo package --list` 检查发布内容**

## 常见陷阱

1. **CI 里 `cargo fmt` 不带 `--check`**：把 CI 工作区改了但没失败
2. **clippy 不带 `--all-targets`**：测试/示例里的问题漏掉
3. **clippy 不带 `-D warnings`**：有警告也算通过
4. **`#[allow(clippy::all)]` 一把梭**：把真正的 bug 也屏蔽
5. **`#[allow]` 不写理由**：后人不敢删
6. **工具链不固定**：不同 rustc 的 lint/格式结果不同
7. **`rustfmt.toml` 用 nightly 选项**：stable CI 不生效
8. **`///` 与 `//!` 用错位置**：文档挂到错误 item/模块
9. **doctest 引用二进制私有项**：编译失败；把逻辑放库
10. **`no_run` 掩盖运行期错误**：该跑的还是要在测试里跑
11. **公开 API 没写 `# Errors`/`# Panics`**：调用方不知道失败语义
12. **坏 intra-doc link**：`cargo doc` 不失败，文档里是断链
13. **`#[doc(hidden)]` 滥用**：公开 API 变得不可发现
14. **`#[deprecated]` 不写迁移路径**：调用方不知道怎么改
15. **忽略 `#[must_use]` 警告**：返回值被丢弃通常是 bug
16. **格式提交混在逻辑 PR 里**：review 噪音巨大
17. **`cargo clippy --fix` 不看 diff**：可能改错语义
18. **只跑 `cargo test` 不跑 doctest**：`cargo test` 默认含 doctest，但 `--tests` 不含
19. **`missing_docs` 没开**：公开 API 缺文档
20. **发布包含临时文件**：`cargo package --list` 检查

## 完整示例

见 `30_docs_clippy_fmt.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ERRORS/30_文档clippy与fmt/30_docs_clippy_fmt.rs" -o run.exe
./run.exe
```

示例共 11 行输出，覆盖：文档注释与 `# Errors`/`# Panics`、`#[deprecated]`、
`#[doc(hidden)]`、`#[must_use]`、`#![forbid(unsafe_code)]`、
`#[allow(clippy::...)]`、`#[non_exhaustive]`，以及四项质量门禁的汇总。

本课实测：

- `rustc --edition 2024 -D warnings`：零警告，11 行输出与注释预测一致
- `rustfmt --edition 2024 --check`：通过
- `clippy-driver --edition 2024 -D warnings`：零警告
- 临时 Cargo 工程：`cargo fmt -- --check`、`cargo clippy --all-targets -- -D warnings` 通过；
  `cargo test` 2 个单元测试 + 2 个 doctest 通过；`cargo doc --no-deps` 成功

## 练习

1. 给一个公开函数补 `///` 文档、`# Errors`、`# Panics` 和可运行的 `# Examples`。
2. 用 `#[deprecated(note = "...")]` 标记旧函数，并在调用处用 `#[allow(deprecated)]`。
3. 开启 `#![forbid(unsafe_code)]` 与 `#![deny(missing_docs)]`，修复暴露的问题。
4. 用 `#[allow(clippy::...)]` 局部豁免一个 lint，并改写成 `#[expect]` 版本。
5. 写 `rustfmt.toml`，分别用 `cargo fmt` 与 `cargo fmt -- --check` 验证。
6. 在临时 Cargo 工程里跑完整门禁：fmt/clippy/test/doc。
7. 制造一个坏 intra-doc link，用 `RUSTDOCFLAGS="-D warnings" cargo doc` 让它失败。
8. 解释 `--all-targets`、`--all-features`、`-D warnings` 各自的作用。

## 参考

- The rustfmt Book：[Configuration](https://rust-lang.github.io/rustfmt/)
- Clippy 文档：[Lint list](https://rust-lang.github.io/rust-clippy/master/)、
  [Configuration](https://doc.rust-lang.org/clippy/configuration.html)
- The rustdoc Book：[Documentation tests](https://doc.rust-lang.org/rustdoc/documentation-tests.html)、
  [Lints](https://doc.rust-lang.org/rustdoc/lints.html)
- Cargo Book：[`cargo fmt`](https://doc.rust-lang.org/cargo/commands/cargo-fmt.html)、
  [`cargo clippy`](https://doc.rust-lang.org/cargo/commands/cargo-clippy.html)、
  [`cargo doc`](https://doc.rust-lang.org/cargo/commands/cargo-doc.html)
