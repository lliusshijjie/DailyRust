// 第 24 课示例：自定义错误类型与错误链
//
// 结构：① 原理验证 —— Display/Debug/source、From 转换、Box<dyn Error> downcast、链式遍历
//       ② 语法演示 —— 带路径上下文的 LoadError、三层错误链、Send + Sync trait 对象
//       ③ 应用实战 —— 配置加载错误链、API 错误枚举
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 24_custom_errors_and_chains.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

// 库层错误：具体枚举，保留每种失败语义
#[derive(Debug)]
enum ConfigError {
    MissingHost,
    MissingPort,
    InvalidPort(ParseIntError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingHost => write!(f, "missing host"),
            ConfigError::MissingPort => write!(f, "missing port"),
            ConfigError::InvalidPort(_) => write!(f, "invalid port"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::InvalidPort(source) => Some(source),
            _ => None,
        }
    }
}

// ? 通过 From 自动把 ParseIntError 转成 ConfigError
impl From<ParseIntError> for ConfigError {
    fn from(error: ParseIntError) -> Self {
        ConfigError::InvalidPort(error)
    }
}

// 应用层错误：加上"在哪个文件里失败"的上下文
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

// API 错误：调用方可能想 match 具体变体
#[derive(Debug)]
enum ApiError {
    NotFound(u64),
    Forbidden,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NotFound(id) => write!(f, "user {id} not found"),
            ApiError::Forbidden => write!(f, "forbidden"),
        }
    }
}

impl Error for ApiError {}

fn parse_port(text: &str) -> Result<u16, ConfigError> {
    let port: u16 = text.parse()?;
    Ok(port)
}

fn load_config(input: &str) -> Result<(), ConfigError> {
    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;

    for line in input.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "host" => host = Some(value.to_string()),
                "port" => port = Some(parse_port(value)?),
                _ => {}
            }
        }
    }

    let _ = host.ok_or(ConfigError::MissingHost)?;
    let _ = port.ok_or(ConfigError::MissingPort)?;
    Ok(())
}

fn load(path: &str, input: &str) -> Result<(), LoadError> {
    load_config(input).map_err(|error| LoadError {
        path: path.to_string(),
        source: error,
    })
}

// 从最外层错误开始，沿 source() 打印整条链
fn chain(error: &dyn Error) -> Vec<String> {
    let mut messages = vec![error.to_string()];
    let mut current = error.source();
    while let Some(source) = current {
        messages.push(source.to_string());
        current = source.source();
    }
    messages
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 Display 给用户看，Debug 给开发者看
    let missing = ConfigError::MissingHost;
    println!("display={} debug={:?}", missing, missing);
    // 预测：display=missing host debug=MissingHost

    // 1.2 source() 指向根因
    let invalid = ConfigError::InvalidPort("x".parse::<u16>().unwrap_err());
    println!("source={}", invalid.source().unwrap());
    // 预测：source=invalid digit found in string

    // 1.3 ? + From：ParseIntError -> ConfigError
    println!("from_conversion={}", parse_port("x").is_err());
    // 预测：from_conversion=true

    // 1.4 Box<dyn Error> 可以 downcast 回具体类型
    let boxed: Box<dyn Error> = Box::new(ConfigError::MissingHost);
    println!(
        "downcast={}",
        boxed.downcast_ref::<ConfigError>().is_some()
    );
    // 预测：downcast=true

    // 1.5 手动遍历错误链
    println!("chain={}", chain(&invalid).join(" | "));
    // 预测：chain=invalid port | invalid digit found in string

    println!("===== 2. SYNTAX =====");

    // 2.1 带上下文的错误链：LoadError -> ConfigError -> ParseIntError
    let load_error = load("config.txt", "host=localhost\nport=x").unwrap_err();
    println!("load_display={}", load_error);
    // 预测：load_display=failed to load config.txt

    println!("chain_depth={}", chain(&load_error).len());
    // 预测：chain_depth=3

    println!("chain={}", chain(&load_error).join(" | "));
    // 预测：chain=failed to load config.txt | invalid port | invalid digit found in string

    // 2.2 Send + Sync 的错误对象：可跨线程传递
    let boxed: Box<dyn Error + Send + Sync> = Box::new(ConfigError::MissingPort);
    println!("send_sync={}", boxed);
    // 预测：send_sync=missing port

    println!("===== 3. APPLICATION =====");

    // 3.1 应用层错误链：保留路径上下文与根因
    match load("app.conf", "port=8080") {
        Ok(()) => println!("loaded"),
        Err(error) => println!("error={} chain={}", error, chain(&error).join(" | ")),
    }
    // 预测：error=failed to load app.conf chain=failed to load app.conf | missing host

    // 3.2 API 错误枚举：调用方可以 match 具体变体
    println!(
        "api={} {}",
        ApiError::NotFound(7),
        ApiError::Forbidden
    );
    // 预测：api=user 7 not found forbidden
}
