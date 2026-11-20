// 第 28 课示例：错误处理策略与 API 边界设计
//
// 结构：① 原理验证 —— newtype 边界校验、Option vs Result、内部不变量 panic
//       ② 语法演示 —— 领域/服务/应用三层错误、fail-fast、收集全部错误、重试、降级
//       ③ 应用实战 —— 配置加载流水线、错误链、校验聚合、退出码
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 28_error_strategy_and_api.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;
use std::thread;

// 边界 newtype：端口一旦构造成功就保证非零
#[derive(Debug)]
struct Port(u16);

impl Port {
    fn new(value: u16) -> Result<Self, ConfigError> {
        if value == 0 {
            Err(ConfigError::InvalidPort(value))
        } else {
            Ok(Port(value))
        }
    }

    fn value(&self) -> u16 {
        self.0
    }
}

// 领域错误：只描述业务语义，不泄露 IO/解析细节
#[derive(Debug)]
enum ConfigError {
    MissingHost,
    MissingPort,
    InvalidPort(u16),
    ParsePort(ParseIntError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingHost => write!(f, "missing host"),
            ConfigError::MissingPort => write!(f, "missing port"),
            ConfigError::InvalidPort(value) => write!(f, "invalid port: {value}"),
            ConfigError::ParsePort(_) => write!(f, "invalid port number"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::ParsePort(source) => Some(source),
            _ => None,
        }
    }
}

impl From<ParseIntError> for ConfigError {
    fn from(error: ParseIntError) -> Self {
        ConfigError::ParsePort(error)
    }
}

#[derive(Debug)]
struct Config {
    host: String,
    port: Port,
}

fn load_config(input: &str) -> Result<Config, ConfigError> {
    let mut host: Option<String> = None;
    let mut port: Option<Port> = None;

    for line in input.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "host" => host = Some(value.to_string()),
                "port" => {
                    let raw: u16 = value.parse()?;
                    port = Some(Port::new(raw)?);
                }
                _ => {}
            }
        }
    }

    Ok(Config {
        host: host.ok_or(ConfigError::MissingHost)?,
        port: port.ok_or(ConfigError::MissingPort)?,
    })
}

// 服务层：加上"在哪个文件失败"的上下文
#[derive(Debug)]
struct LoadError {
    path: String,
    source: ConfigError,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to load {}", self.path)
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

fn load(path: &str, input: &str) -> Result<Config, LoadError> {
    load_config(input).map_err(|error| LoadError {
        path: path.to_string(),
        source: error,
    })
}

// 应用层：把服务错误与校验错误统一成边界错误
#[derive(Debug)]
enum AppError {
    Load(LoadError),
    Validation(Vec<FieldError>),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Load(_) => write!(f, "load failed"),
            AppError::Validation(errors) => {
                write!(f, "validation failed with {} errors", errors.len())
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Load(source) => Some(source),
            AppError::Validation(_) => None,
        }
    }
}

impl From<LoadError> for AppError {
    fn from(error: LoadError) -> Self {
        AppError::Load(error)
    }
}

#[derive(Debug)]
struct FieldError {
    field: &'static str,
    message: String,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

fn validate_form(name: &str, age: i32) -> Result<(), Vec<FieldError>> {
    let mut errors = Vec::new();

    if name.is_empty() {
        errors.push(FieldError {
            field: "name",
            message: String::from("must not be empty"),
        });
    }
    if !(0..=150).contains(&age) {
        errors.push(FieldError {
            field: "age",
            message: String::from("must be between 0 and 150"),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_config(config: &Config) -> Result<(), Vec<FieldError>> {
    let mut errors = Vec::new();

    if config.host.is_empty() {
        errors.push(FieldError {
            field: "host",
            message: String::from("must not be empty"),
        });
    }
    if config.port.value() < 1024 {
        errors.push(FieldError {
            field: "port",
            message: String::from("must be >= 1024"),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn run(path: &str, input: &str) -> Result<Config, AppError> {
    let config = load(path, input)?;
    validate_config(&config).map_err(AppError::Validation)?;
    Ok(config)
}

// 内部不变量：违反就是程序员错误，用断言而非 Result
#[track_caller]
fn require_non_empty(value: &str) {
    assert!(
        !value.is_empty(),
        "internal invariant: value must not be empty"
    );
}

fn catch_silent<F, R>(function: F) -> thread::Result<R>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(function);
    std::panic::set_hook(previous);
    result
}

fn chain(error: &dyn Error) -> Vec<String> {
    let mut messages = vec![error.to_string()];
    let mut current = error.source();
    while let Some(source) = current {
        messages.push(source.to_string());
        current = source.source();
    }
    messages
}

fn find_host<'a>(hosts: &'a BTreeMap<&str, &str>, key: &str) -> Option<&'a str> {
    hosts.get(key).copied()
}

fn retry_fetch(attempts: u32) -> Result<u32, &'static str> {
    for attempt in 1..=attempts {
        if attempt == 3 {
            return Ok(attempt);
        }
    }
    Err("retries exhausted")
}

fn fallback_config() -> Config {
    Config {
        host: String::from("fallback"),
        port: Port(8080),
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 边界 newtype：构造成功即保证非零
    println!(
        "port={:?} invalid={:?}",
        Port::new(8080),
        Port::new(0)
    );
    // 预测：port=Ok(Port(8080)) invalid=Err(InvalidPort(0))

    // 1.2 查找用 Option，解析/校验用 Result
    let mut hosts = BTreeMap::new();
    hosts.insert("host", "localhost");
    println!(
        "lookup={:?} missing={:?}",
        find_host(&hosts, "host"),
        find_host(&hosts, "port")
    );
    // 预测：lookup=Some("localhost") missing=None

    // 1.3 内部不变量用 panic；边界输入用 Result
    let panic_contract = catch_silent(|| require_non_empty("")).is_err();
    println!("panic_contract={}", panic_contract);
    // 预测：panic_contract=true

    println!("===== 2. SYNTAX =====");

    // 2.1 正常路径：三层错误类型都能 ? 转换
    println!("load={:?}", load("app.conf", "host=localhost\nport=8080"));
    // 预测：load=Ok(Config { host: "localhost", port: Port(8080) })

    // 2.2 fail-fast：collect 遇到第一个 Err 就停
    let parsed: Result<Vec<Port>, ConfigError> = ["8080", "0", "9090"]
        .iter()
        .map(|text| {
            let raw: u16 = text.parse()?;
            Port::new(raw)
        })
        .collect();
    println!("fail_fast={:?}", parsed);
    // 预测：fail_fast=Err(InvalidPort(0))

    // 2.3 收集全部校验错误，而不是只报第一个
    let collect_all = validate_form("", -5).unwrap_err();
    println!("collect_all={}", collect_all.len());
    // 预测：collect_all=2

    // 2.4 重试：只对瞬态错误重试，且有次数上限
    println!("retry={:?}", retry_fetch(3));
    // 预测：retry=Ok(3)

    // 2.5 降级：加载失败时回退到安全默认值
    let fallback = load("missing.conf", "").or_else(|_| Ok::<Config, LoadError>(fallback_config()));
    println!("fallback={:?}", fallback);
    // 预测：fallback=Ok(Config { host: "fallback", port: Port(8080) })

    println!("===== 3. APPLICATION =====");

    // 3.1 成功路径
    println!(
        "run={:?}",
        run("app.conf", "host=localhost\nport=8080")
    );
    // 预测：run=Ok(Config { host: "localhost", port: Port(8080) })

    // 3.2 失败路径：错误链保留"应用 -> 文件 -> 字段"三层上下文
    let failed = run("app.conf", "host=localhost\nport=0").unwrap_err();
    println!("error_chain={}", chain(&failed).join(" | "));
    // 预测：error_chain=load failed | failed to load app.conf | invalid port: 0

    // 3.3 校验错误聚合：调用方能一次拿到全部字段问题
    let errors = validate_form("", -5).unwrap_err();
    println!("first_error={}", errors[0]);
    println!("validation={}", AppError::Validation(errors));
    // 预测：first_error=name: must not be empty
    // 预测：validation=validation failed with 2 errors

    // 3.4 CLI 边界：错误映射为非零退出码，而不是把 backtrace 甩给用户
    let exit_code = if run("app.conf", "host=localhost\nport=0").is_err() {
        1
    } else {
        0
    };
    println!("exit_code={}", exit_code);
    // 预测：exit_code=1
}
