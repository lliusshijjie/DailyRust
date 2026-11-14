// 第 23 课示例：Result 与 ? 运算符
//
// 结构：① 原理验证 —— Result 基本形态、? 的提前返回、From 错误转换、Box<dyn Error>
//       ② 语法演示 —— map/map_err、and_then/or_else、unwrap_or_else、collect、transpose、ok/err
//       ③ 应用实战 —— 配置解析、批量解析、安全除法
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 23_result_and_question_mark.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::num::ParseIntError;

// 自定义错误：通过 From 让 ? 自动转换
#[derive(Debug)]
enum AppError {
    Parse(ParseIntError),
}

impl From<ParseIntError> for AppError {
    fn from(error: ParseIntError) -> Self {
        Self::Parse(error)
    }
}

// ? 在 Result 函数里：遇到 Err 直接 return Err
fn parse_and_double(text: &str) -> Result<i32, ParseIntError> {
    let value: i32 = text.parse()?;
    Ok(value * 2)
}

// ? + From：ParseIntError -> AppError
fn double(text: &str) -> Result<i32, AppError> {
    let value: i32 = text.parse()?;
    Ok(value * 2)
}

// Box<dyn Error>：标准库为 E: Error 提供 From<E>，所以 ? 能直接转
fn parse_boxed(text: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let value: i32 = text.parse()?;
    Ok(value)
}

fn half(value: i32) -> Result<i32, String> {
    if value % 2 == 0 {
        Ok(value / 2)
    } else {
        Err(String::from("odd"))
    }
}

fn divide(left: i32, right: i32) -> Result<i32, String> {
    if right == 0 {
        Err(String::from("divide by zero"))
    } else {
        Ok(left / right)
    }
}

struct Config {
    host: String,
    port: u16,
}

fn parse_config(input: &str) -> Result<Config, String> {
    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;

    for line in input.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("bad line: {line}"))?;
        match key {
            "host" => host = Some(value.to_string()),
            "port" => {
                port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|error| format!("bad port: {error}"))?,
                )
            }
            _ => return Err(format!("unknown key: {key}")),
        }
    }

    Ok(Config {
        host: host.ok_or_else(|| String::from("missing host"))?,
        port: port.ok_or_else(|| String::from("missing port"))?,
    })
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 ? 遇到 Err 提前返回
    println!(
        "parse={:?} bad={}",
        parse_and_double("10"),
        parse_and_double("x").is_err()
    );
    // 预测：parse=Ok(20) bad=true

    // 1.2 ? 通过 From 自动转换错误类型
    println!("from_conversion={:?}", double("5"));
    // 预测：from_conversion=Ok(10)

    if let Err(AppError::Parse(error)) = double("x") {
        println!("app_error={error}");
        // 预测：app_error=invalid digit found in string
    }

    // 1.3 Option -> Result：ok_or_else 保留失败语义
    let maybe = Some(4);
    println!("ok_or={:?}", maybe.ok_or_else(|| String::from("missing")));
    // 预测：ok_or=Ok(4)

    // 1.4 Box<dyn Error>：把具体错误擦除成 trait 对象
    println!(
        "boxed={:?} boxed_err={}",
        parse_boxed("7"),
        parse_boxed("x").is_err()
    );
    // 预测：boxed=Ok(7) boxed_err=true

    println!("===== 2. SYNTAX =====");

    // 2.1 map 与 map_err：分别变换成功值/错误值
    let ok: Result<i32, &str> = Ok(5);
    let err: Result<i32, &str> = Err("boom");
    println!(
        "map={:?} map_err={:?}",
        ok.map(|value| value * 2),
        err.map_err(|error| format!("bad: {error}"))
    );
    // 预测：map=Ok(10) map_err=Err("bad: boom")

    // 2.2 and_then：链式返回 Result 的操作
    println!("and_then={:?}", Ok::<i32, String>(8).and_then(half));
    // 预测：and_then=Ok(4)

    // 2.3 or_else：只在 Err 时尝试恢复
    println!(
        "or_else={:?}",
        Err::<i32, &str>("x").or_else(|_| Ok::<i32, &str>(0))
    );
    // 预测：or_else=Ok(0)

    // 2.4 unwrap_or_else：惰性默认值
    println!(
        "unwrap_or_else={}",
        Err::<i32, &str>("x").unwrap_or_else(|error| error.len() as i32)
    );
    // 预测：unwrap_or_else=1

    // 2.5 collect 成 Result：遇到第一个 Err 就短路
    let parsed: Result<Vec<i32>, _> = ["1", "x", "3"]
        .iter()
        .map(|text| text.parse::<i32>())
        .collect();
    println!("collect_err={}", parsed.is_err());
    // 预测：collect_err=true

    // 2.6 transpose：Result<Option<T>, E> -> Option<Result<T, E>>
    println!(
        "transpose={:?}",
        Ok::<Option<i32>, &str>(Some(1)).transpose()
    );
    // 预测：transpose=Some(Ok(1))

    // 2.7 ok/err：丢弃另一半
    println!(
        "ok_err={:?} {:?}",
        Ok::<i32, &str>(1).ok(),
        Err::<i32, &str>("e").err()
    );
    // 预测：ok_err=Some(1) Some("e")

    println!("===== 3. APPLICATION =====");

    // 3.1 配置解析：? + ok_or_else 串起校验
    match parse_config("host=localhost\nport=8080") {
        Ok(config) => println!("config={} {}", config.host, config.port),
        Err(error) => println!("config_err={error}"),
    }
    // 预测：config=localhost 8080

    println!("config_bad={}", parse_config("host=localhost").is_err());
    // 预测：config_bad=true

    // 3.2 批量解析：collect 短路
    let batch: Result<Vec<i32>, _> = ["1", "2", "x"]
        .iter()
        .map(|text| text.parse::<i32>())
        .collect();
    println!("batch_err={}", batch.is_err());
    // 预测：batch_err=true

    // 3.3 安全除法：可恢复错误用 Result
    println!("divide={:?} zero={:?}", divide(10, 2), divide(1, 0));
    // 预测：divide=Ok(5) zero=Err("divide by zero")
}
