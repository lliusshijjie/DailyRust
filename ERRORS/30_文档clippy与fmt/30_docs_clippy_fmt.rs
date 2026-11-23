// 第 30 课示例：文档、clippy 与 fmt 工作流
//
// 结构：① 原理验证 —— 文档注释、#[deprecated]、#[doc(hidden)]、#[must_use]
//       ② 语法演示 —— #![forbid(unsafe_code)]、#[allow(clippy::...)]、#[non_exhaustive]
//       ③ 应用实战 —— 解析/规范化/状态/范围检查的"质量门禁"汇总
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 30_docs_clippy_fmt.rs -o run.exe && .\run.exe
// 真实工程的质量门禁（本仓库没有 Cargo.toml，命令在 Cargo 工程中执行）：
//   cargo fmt --all -- --check
//   cargo clippy --workspace --all-targets --all-features -- -D warnings
//   cargo doc --workspace --no-deps
//   cargo test --workspace --all-features
// 先预测每行输出，再运行对照！

//! 质量工作流示例：文档注释、lint 属性与 clippy 允许项。
//!
//! 真实库工程里，这些文档会被 `cargo doc` 渲染，示例代码会被 `cargo test --doc`
//! 执行；本文件是二进制示例，因此文档示例保持自包含。

#![forbid(unsafe_code)]

use std::fmt;

/// 应用配置。
///
/// # Examples
///
/// ```
/// let host = String::from("localhost");
/// let port: u16 = 8080;
/// assert!(!host.is_empty() && port > 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// 监听地址（已规范化）。
    pub host: String,
    /// 监听端口，必须非零。
    pub port: u16,
}

/// 配置解析错误。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// 缺少 host。
    MissingHost,
    /// 缺少 port。
    MissingPort,
    /// 端口非法。
    InvalidPort(u16),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingHost => write!(f, "missing host"),
            ConfigError::MissingPort => write!(f, "missing port"),
            ConfigError::InvalidPort(value) => write!(f, "invalid port: {value}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// 解析 `host=...\nport=...` 形式的配置。
///
/// # Errors
///
/// 缺少 host/port，或端口为 0 时返回 [`ConfigError`]。
///
/// # Panics
///
/// 本函数不会 panic。
pub fn parse_config(input: &str) -> Result<Config, ConfigError> {
    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;

    for line in input.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "host" => host = Some(normalize_host(value)),
            "port" => {
                let raw: u16 = value.parse().map_err(|_| ConfigError::InvalidPort(0))?;
                if raw == 0 {
                    return Err(ConfigError::InvalidPort(raw));
                }
                port = Some(raw);
            }
            _ => {}
        }
    }

    Ok(Config {
        host: host.ok_or(ConfigError::MissingHost)?,
        port: port.ok_or(ConfigError::MissingPort)?,
    })
}

/// 旧版入口；保留用于兼容，已废弃。
#[deprecated(since = "0.2.0", note = "use parse_config instead")]
pub fn parse_config_legacy(input: &str) -> Result<Config, ConfigError> {
    parse_config(input)
}

/// 内部辅助：去空白并转小写。不会出现在文档里。
#[doc(hidden)]
pub fn normalize_host(host: &str) -> String {
    host.trim().to_ascii_lowercase()
}

/// 返回状态标签。
///
/// 返回值被标记为 `#[must_use]`：忽略它会产生编译警告。
#[must_use]
pub fn status_label(code: u8) -> &'static str {
    match code {
        0 => "Started",
        _ => "Unknown",
    }
}

/// 范围检查：clippy 默认会建议用 `(0..=150).contains(&value)`，
/// 这里显式允许该 lint，演示 `#[allow(clippy::...)]` 的写法。
#[allow(clippy::manual_range_contains)]
pub fn in_range(value: i32) -> bool {
    value >= 0 && value <= 150
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 文档注释描述契约，不影响运行
    let parsed = parse_config("host=LOCALHOST\nport=8080");
    println!("config={:?}", parsed);
    // 预测：config=Ok(Config { host: "localhost", port: 8080 })

    // 1.2 # Errors/# Panics 写清失败语义
    let invalid = parse_config("host=localhost\nport=0").unwrap_err();
    println!("invalid={}", invalid);
    // 预测：invalid=invalid port: 0

    // 1.3 #[deprecated] 提醒迁移；调用方用 allow 局部豁免
    #[allow(deprecated)]
    let legacy = parse_config_legacy("host=localhost\nport=8080").map(|config| config.port);
    println!("legacy={:?}", legacy);
    // 预测：legacy=Ok(8080)

    println!("===== 2. SYNTAX =====");

    // 2.1 #[doc(hidden)] 的辅助函数仍然可调用
    println!("normalized={}", normalize_host("  LOCALHOST "));
    // 预测：normalized=localhost

    // 2.2 #[must_use] 的返回值必须被使用
    println!("status={}", status_label(0));
    // 预测：status=Started

    // 2.3 #[allow(clippy::...)] 局部关闭 lint
    println!("range={}", in_range(30));
    // 预测：range=true

    println!("===== 3. APPLICATION =====");

    // 3.1 质量门禁：四项检查全部通过
    let checks = [
        parse_config("host=localhost\nport=8080").is_ok(),
        normalize_host("  LOCALHOST ") == "localhost",
        status_label(0) == "Started",
        in_range(30),
    ];
    let passed = checks.iter().filter(|ok| **ok).count();
    println!("checks={}/{}", passed, checks.len());
    // 预测：checks=4/4

    // 3.2 真实工程在 Cargo 中执行这些命令
    println!(
        "next=cargo fmt --check && cargo clippy -- -D warnings && cargo doc --no-deps && cargo test"
    );
    // 预测：next=cargo fmt --check && cargo clippy -- -D warnings && cargo doc --no-deps && cargo test
}
