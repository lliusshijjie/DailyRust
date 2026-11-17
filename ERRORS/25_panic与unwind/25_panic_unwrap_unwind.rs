// 第 25 课示例：panic、unwrap 与 unwind
//
// 结构：① 原理验证 —— catch_unwind、payload 向下转型、unwrap/assert panic、线程 panic、hook 捕获
//       ② 语法演示 —— panic_any、resume_unwind、unwrap_or_else
//       ③ 应用实战 —— 插件隔离、输入校验 vs 不变量断言
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 25_panic_unwrap_unwind.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！
//
// 注意：示例默认用 unwind 运行时；若用 -C panic=abort 编译，catch_unwind 无法捕获。

use std::panic::{self, AssertUnwindSafe, UnwindSafe};
use std::sync::{Arc, Mutex};
use std::thread;

// 静音 panic hook 后捕获 panic，避免 stderr 干扰示例输出
fn catch_silent<F, R>(function: F) -> thread::Result<R>
where
    F: FnOnce() -> R + UnwindSafe,
{
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(function);
    panic::set_hook(previous);
    result
}

// 线程 panic：默认只终止该线程，主线程通过 join 拿到 Err
fn thread_panics() -> bool {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let handle = thread::spawn(|| panic!("worker"));
    let result = handle.join();
    panic::set_hook(previous);
    result.is_err()
}

// 自定义 hook 记录 panic 消息
fn capture_hook() -> Vec<String> {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&messages);
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|text| (*text).to_string())
            .unwrap_or_else(|| String::from("non-str payload"));
        captured.lock().unwrap().push(message);
    }));
    let _ = panic::catch_unwind(|| panic!("hooked"));
    panic::set_hook(previous);
    let result = messages.lock().unwrap().clone();
    result
}

// 插件隔离：把 panic 转成普通错误，不让它炸掉调用方
fn run_plugin<F>(plugin: F, input: i32) -> Result<i32, String>
where
    F: FnOnce(i32) -> i32 + UnwindSafe,
{
    match catch_silent(move || plugin(input)) {
        Ok(value) => Ok(value),
        Err(_) => Err(String::from("plugin panicked")),
    }
}

// 输入校验用 Result，不变量破坏才 panic
fn validate_age(age: i32) -> Result<i32, String> {
    if (0..=150).contains(&age) {
        Ok(age)
    } else {
        Err(format!("invalid age: {age}"))
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 catch_unwind：捕获 panic，拿到 payload
    let result = catch_silent(|| panic!("boom"));
    let message = result
        .as_ref()
        .err()
        .and_then(|payload| payload.downcast_ref::<&str>().copied());
    println!("caught={} payload={:?}", result.is_err(), message);
    // 预测：caught=true payload=Some("boom")

    // 1.2 unwrap 在 Err 上 panic
    println!(
        "unwrap_panic={}",
        catch_silent(|| Err::<i32, &str>("bad").unwrap()).is_err()
    );
    // 预测：unwrap_panic=true

    // 1.3 assert! 失败 panic
    println!(
        "assert_panic={}",
        catch_silent(|| {
            let value = -1;
            assert!(value > 0, "value must be positive");
            value
        })
        .is_err()
    );
    // 预测：assert_panic=true

    // 1.4 &mut 捕获不是 UnwindSafe：需要 AssertUnwindSafe 显式承担风险
    let mut counter = 0;
    let caught = catch_silent(AssertUnwindSafe(|| {
        counter += 1;
        panic!("mut");
    }));
    println!("mut_caught={} counter={}", caught.is_err(), counter);
    // 预测：mut_caught=true counter=1

    // 1.5 线程 panic：只终止该线程，join 返回 Err
    println!("thread_panic={}", thread_panics());
    // 预测：thread_panic=true

    // 1.6 panic hook 可以记录消息（日志/上报）
    println!("hook={:?}", capture_hook());
    // 预测：hook=["hooked"]

    println!("===== 2. SYNTAX =====");

    // 2.1 panic_any：携带任意 Send + 'static 的 payload
    let result = catch_silent(|| panic::panic_any(42));
    let payload = result
        .err()
        .and_then(|payload| payload.downcast_ref::<i32>().copied());
    println!("panic_any={:?}", payload);
    // 预测：panic_any=Some(42)

    // 2.2 resume_unwind：清理后重新抛出
    let first: thread::Result<()> = catch_silent(|| panic!("resume"));
    let resumed = catch_silent(AssertUnwindSafe(|| {
        if let Some(payload) = first.err() {
            panic::resume_unwind(payload);
        }
    }))
    .is_err();
    println!("resumed={}", resumed);
    // 预测：resumed=true

    // 2.3 unwrap_or_else：可恢复错误优先用 Result 组合子
    println!(
        "unwrap_or_else={}",
        Err::<i32, &str>("bad").unwrap_or_else(|error| error.len() as i32)
    );
    // 预测：unwrap_or_else=3

    println!("===== 3. APPLICATION =====");

    // 3.1 插件隔离：panic 被转换为 Err
    let ok = run_plugin(|value| value * 2, 5);
    let bad = run_plugin(|_| panic!("broken plugin"), 1);
    println!("plugin={:?} bad={:?}", ok, bad);
    // 预测：plugin=Ok(10) bad=Err("plugin panicked")

    // 3.2 输入校验用 Result；不变量断言才 panic
    println!(
        "valid={:?} invalid={:?}",
        validate_age(30),
        validate_age(200)
    );
    // 预测：valid=Ok(30) invalid=Err("invalid age: 200")
}
