// 第 27 课示例：调试与诊断——dbg!、断言与 Debug
//
// 结构：① 原理验证 —— derive Debug、{:#?} 美化输出、自定义 Debug（脱敏）
//       ② 语法演示 —— assert_eq! 的 panic 消息、cfg!(debug_assertions)、debug_assert!
//       ③ 应用实战 —— finish_non_exhaustive 与敏感字段脱敏、输入校验、断言消息
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 27_debugging_and_assertions.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！
//
// 说明：dbg! 输出到 stderr，并带源码路径；示例用 Debug + 断言消息覆盖等价的诊断手段，
// 避免 stderr 中的中文路径干扰示例输出。

use std::any::Any;
use std::fmt;
use std::thread;

#[derive(Debug)]
struct User {
    name: String,
    age: u32,
}

// 自定义 Debug：敏感字段脱敏
struct Credentials {
    user: String,
    token: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("user", &self.user)
            .field("token", &"***")
            .finish()
    }
}

// 额外字段不展开：打印 .. 表示还有内容
struct AppConfig {
    host: String,
    port: u16,
    token: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("token", &"***")
            .finish_non_exhaustive()
    }
}

// 静音 panic hook，避免断言失败刷屏
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

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        String::from("non-str payload")
    }
}

fn validate(config: &AppConfig) -> Result<(), String> {
    if config.host.is_empty() {
        return Err(String::from("host must not be empty"));
    }
    if config.port == 0 {
        return Err(String::from("port must be non-zero"));
    }
    if config.token.is_empty() {
        return Err(String::from("token must not be empty"));
    }
    Ok(())
}

fn check_positive(value: i32) -> String {
    match catch_silent(std::panic::AssertUnwindSafe(|| {
        assert!(value > 0, "value must be positive, got {value}");
    })) {
        Ok(()) => String::from("ok"),
        Err(payload) => panic_message(payload.as_ref()),
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 derive(Debug)：一次派生，{:?} 可直接打印
    let user = User {
        name: String::from("alice"),
        age: 30,
    };
    assert!(!user.name.is_empty() && user.age > 0);
    println!("debug={:?}", user);
    // 预测：debug=User { name: "alice", age: 30 }

    // 1.2 {:#?}：多行美化输出（适合日志/排查）
    println!("pretty={:#?}", user);
    // 预测：pretty=User {
    // 预测：    name: "alice",
    // 预测：    age: 30,
    // 预测：}

    // 1.3 自定义 Debug：token 不进日志
    let credentials = Credentials {
        user: String::from("alice"),
        token: String::from("secret"),
    };
    assert!(!credentials.user.is_empty() && !credentials.token.is_empty());
    println!("redacted={:?}", credentials);
    // 预测：redacted=Credentials { user: "alice", token: "***" }

    println!("===== 2. SYNTAX =====");

    // 2.1 assert_eq! 失败时打印两边，消息可以捕获用于诊断
    let payload = catch_silent(|| {
        assert_eq!(3, 4);
    })
    .unwrap_err();
    println!("assert_msg={}", panic_message(payload.as_ref()));
    // 预测：assert_msg=assertion `left == right` failed
    // 预测：  left: 3
    // 预测： right: 4

    // 2.2 debug_assertions 在普通构建里开启
    println!("debug_assertions={}", cfg!(debug_assertions));
    // 预测：debug_assertions=true

    // 2.3 debug_assert!：只在 debug 构建执行
    debug_assert!(1 + 1 == 2, "math is broken");
    println!("debug_assert=ok");
    // 预测：debug_assert=ok

    println!("===== 3. APPLICATION =====");

    // 3.1 敏感配置脱敏 + 用 .. 表示还有字段
    let config = AppConfig {
        host: String::from("localhost"),
        port: 8080,
        token: String::from("secret"),
    };
    println!("config={:?}", config);
    // 预测：config=AppConfig { host: "localhost", port: 8080, token: "***", .. }

    // 3.2 输入校验返回 Result，诊断信息写在 Err 里
    println!("validate={:?}", validate(&config));
    // 预测：validate=Ok(())

    // 3.3 断言消息带实际值，排查时不用再猜
    println!("check={} {}", check_positive(5), check_positive(-1));
    // 预测：check=ok value must be positive, got -1
}
