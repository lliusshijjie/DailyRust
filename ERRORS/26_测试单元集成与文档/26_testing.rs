// 第 26 课示例：测试——单元、集成与文档测试
//
// 结构：① 原理验证 —— cfg(test)、assert 家族、panic 捕获、debug_assert
//       ② 语法演示 —— 表驱动断言、Result 测试、should_panic 校验
//       ③ 应用实战 —— add/divide/clamp/parse_port 的测试套件与汇总
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 26_testing.rs -o run.exe && .\run.exe
// 编译测试（本文件同时包含 #[cfg(test)] mod tests）：
//   rustc --edition 2024 --test 26_testing.rs -o tests.exe && .\tests.exe --test-threads=1
// 文档测试（自包含示例）：
//   rustdoc --edition 2024 --test 26_testing.rs

use std::thread;

/// 两数相加。
///
/// ```
/// assert_eq!(2 + 3, 5);
/// ```
fn add(left: i32, right: i32) -> i32 {
    left + right
}

/// 安全除法：除零返回错误。
fn divide(left: i32, right: i32) -> Result<i32, String> {
    if right == 0 {
        Err(String::from("divide by zero"))
    } else {
        Ok(left / right)
    }
}

/// 把 value 钳制到 [min, max]。
///
/// 调用方必须保证 `min <= max`，否则这是程序 bug，直接 panic。
fn clamp(value: i32, min: i32, max: i32) -> i32 {
    assert!(min <= max, "min must be <= max");
    value.max(min).min(max)
}

/// 解析 TCP 端口。
///
/// ```
/// let port: u16 = "8080".parse().expect("valid port");
/// assert_eq!(port, 8080);
/// ```
fn parse_port(text: &str) -> Result<u16, String> {
    text.parse()
        .map_err(|error| format!("invalid port: {error}"))
}

// 静音 panic hook，避免 should_panic 演示污染输出
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

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        String::from("non-str payload")
    }
}

// 单元测试：可以访问父模块的私有函数
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_numbers() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn divide_by_zero_is_error() {
        assert!(divide(1, 0).is_err());
    }

    #[test]
    fn clamps_values() {
        assert_eq!(clamp(15, 0, 10), 10);
        assert_eq!(clamp(-5, 0, 10), 0);
    }

    #[test]
    #[should_panic(expected = "min must be <= max")]
    fn clamp_rejects_bad_bounds() {
        let _ = clamp(1, 10, 0);
    }

    #[test]
    fn parse_port_accepts_valid() -> Result<(), String> {
        let port = parse_port("8080")?;
        assert_eq!(port, 8080);
        Ok(())
    }

    #[test]
    #[ignore]
    fn slow_check() {
        assert_eq!(add(1, 1), 2);
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 cfg!(test) 在普通构建里是 false；--test 构建里是 true
    println!("cfg_test={}", cfg!(test));
    // 预测：cfg_test=false

    // 1.2 assert_eq 成功时什么都不发生
    assert_eq!(add(2, 3), 5);
    println!("assert_ok={}", add(2, 3) == 5);
    // 预测：assert_ok=true

    // 1.3 断言失败会 panic；测试框架会把它记为失败
    let panic_caught = catch_silent(|| {
        assert!(1 == 2, "math broke");
    })
    .is_err();
    println!("panic_caught={}", panic_caught);
    // 预测：panic_caught=true

    // 1.4 debug_assert 只在 debug 构建生效
    debug_assert!(add(1, 1) == 2);
    println!("debug_assert=ok");
    // 预测：debug_assert=ok

    println!("===== 2. SYNTAX =====");

    // 2.1 表驱动测试：同一断言跑多组输入
    let cases = [(1, 2, 3), (0, 0, 0), (-1, 1, 0)];
    for (left, right, expected) in cases {
        assert_eq!(add(left, right), expected);
    }
    println!("table_checks={}", cases.len());
    // 预测：table_checks=3

    // 2.2 返回 Result 的测试函数：用 ? 传播失败
    fn parse_check() -> Result<(), String> {
        let port = parse_port("8080")?;
        assert_eq!(port, 8080);
        Ok(())
    }
    println!("result_test={:?}", parse_check());
    // 预测：result_test=Ok(())

    // 2.3 should_panic：验证 panic 消息包含预期子串
    let payload = catch_silent(|| clamp(1, 10, 0)).unwrap_err();
    let message = panic_message(payload.as_ref());
    println!(
        "should_panic={}",
        message.contains("min must be <= max")
    );
    // 预测：should_panic=true

    println!("===== 3. APPLICATION =====");

    // 3.1 被测函数的实际输出
    println!(
        "add={} divide={:?} zero={:?}",
        add(2, 3),
        divide(10, 2),
        divide(1, 0)
    );
    // 预测：add=5 divide=Ok(5) zero=Err("divide by zero")

    println!(
        "clamp={} parse={:?} bad={}",
        clamp(15, 0, 10),
        parse_port("8080"),
        parse_port("x").is_err()
    );
    // 预测：clamp=10 parse=Ok(8080) bad=true

    // 3.2 一个小型"测试套件"：汇总通过数
    let checks = [
        ("add", add(2, 3) == 5),
        ("clamp_high", clamp(15, 0, 10) == 10),
        ("clamp_low", clamp(-5, 0, 10) == 0),
        ("divide", divide(10, 2) == Ok(5)),
        ("divide_zero", divide(1, 0).is_err()),
        ("parse", parse_port("8080") == Ok(8080)),
    ];
    let passed = checks.iter().filter(|(_, ok)| *ok).count();
    println!("suite={}/{}", passed, checks.len());
    // 预测：suite=6/6
}
