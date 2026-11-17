# 第 25 课 · panic、unwrap 与 unwind

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`25_panic_unwrap_unwind.rs`

## 一、底层原理

### 25.1 panic 是什么

`panic!` 表示"程序进入了不可继续的状态"：

```rust
panic!("boom");
```

运行期行为（默认 unwind 模式）：

1. 构造 panic payload（`Box<dyn Any + Send>`）
2. 调用 panic hook（默认 hook 打印消息与位置）
3. 沿着调用栈**逐层展开（unwind）**，执行每一层的 `Drop`
4. 如果没人 `catch_unwind`，线程结束；主线程结束时进程以非零码退出（通常是 101）

panic 不是异常控制流：它表达的是"不变量被破坏/到达 unreachable 状态"，
不应该用来做正常的错误分支。可恢复错误用 `Result`（第 23、24 课）。

panic payload 的常见类型：

| 写法 | payload 类型 |
|---|---|
| `panic!("literal")` | `&'static str` |
| `panic!("{}", value)` | `String` |
| `panic_any(value)` | 任意 `Send + 'static` 类型 |

### 25.2 unwinding vs abort

Rust 有两种 panic 运行时：

| 模式 | 行为 | 体积/性能 | `catch_unwind` |
|---|---|---|---|
| `unwind`（默认） | 逐层展开栈、执行 Drop | 稍大、稍慢 | 可捕获 |
| `abort` | 立即终止进程 | 更小、更快 | 不可捕获 |

配置方式：

```toml
[profile.release]
panic = "abort"
```

或者编译参数 `-C panic=abort`。abort 模式下二进制更小、启动/异常路径更快，
但会失去"线程隔离"和 `catch_unwind` 能力。嵌入式、WASM、部分服务端会选择 abort。

**双重 panic**：如果 panic 在 unwind 过程中又从 `Drop` 里 panic，进程会直接 abort，
因为继续展开不安全。

### 25.3 `catch_unwind`：在边界处隔离 panic

```rust
pub fn catch_unwind<F, R>(f: F) -> Result<R, Box<dyn Any + Send>>
where
    F: FnOnce() -> R + UnwindSafe;
```

- 捕获闭包里的 panic，返回 `Err(payload)`；正常返回 `Ok(value)`
- 需要 `UnwindSafe` 约束（见 25.5）
- payload 可以用 `downcast_ref::<T>()` 还原
- `resume_unwind(payload)` 可以重新抛出
- 只应在**边界**使用：FFI、插件、线程池、测试框架；不要用它做普通控制流

`catch_unwind` 不会阻止 panic hook 打印消息；示例里先 `set_hook` 静音再捕获，
主要是为了保持输出干净。

### 25.4 panic hook

默认 hook 打印 `thread 'main' panicked at ...` 和消息；可以通过环境变量
`RUST_BACKTRACE=1` 打印回溯。

自定义 hook：

```rust
let previous = std::panic::take_hook();
std::panic::set_hook(Box::new(|info| {
    let message = info.payload().downcast_ref::<&str>().copied().unwrap_or("?");
    eprintln!("panic: {message}");
}));
// ... 触发 panic ...
std::panic::set_hook(previous);   // 恢复默认
```

- hook 是**全局**的，必须 `Send + Sync + 'static`
- hook 里不要再 panic（否则 abort）
- hook 适合日志、上报、结构化错误；不适合做恢复
- 多线程测试里修改全局 hook 会互相影响，要串行化或避免

### 25.5 `UnwindSafe`：跨 unwind 边界的安全性

`catch_unwind` 要求闭包捕获的类型是 `UnwindSafe`：

- `&T`（T: RefUnwindSafe）通常是 `UnwindSafe`
- `&mut T` **不是** `UnwindSafe`
- `RefCell<T>`/`Cell<T>` 含内部可变性，不是 `RefUnwindSafe`
- `AssertUnwindSafe(value)` 可以显式绕过检查

为什么？panic 可能让数据停在"半更新"状态：

```rust
let mut vec = vec![1, 2, 3];
let _ = catch_unwind(|| {
    vec.push(4);          // 修改
    panic!("boom");       // 修改到一半就 panic
});
// vec 现在可能处于不一致状态
```

`AssertUnwindSafe` 的语义是"我保证展开后数据仍然可用/我会自己修复"，
所以它需要你承担正确性责任，不是万能绕过。

### 25.6 `unwrap`、`expect` 与断言

```rust
result.unwrap();                  // Err 时 panic，消息带 Debug
option.expect("config missing");  // None 时 panic，消息自定义
assert!(value > 0);               // false 时 panic
assert_eq!(left, right);          // 不相等时 panic，打印两边
debug_assert!(value > 0);         // release 下不执行
unreachable!("should not happen");// 表达"代码不应到这里"
todo!("not implemented");         // 占位，运行到就 panic
```

- `unwrap` 只用于"逻辑上不可能失败"的位置，并写注释
- `expect` 的消息应该能定位问题（哪个配置、哪个字段）
- 断言适合**内部不变量**；用户输入校验用 `Result`
- `debug_assert!` 只在 debug 构建生效，不能依赖它做业务校验

### 25.7 线程与 panic

- `thread::spawn` 的线程 panic 默认只终止该线程，不影响其他线程
- `JoinHandle::join()` 返回 `Err(payload)`，调用方可以处理
- 主线程 panic 会结束进程（默认退出码 101）
- `panic = "abort"` 下任何线程 panic 都会终止整个进程
- 线程 panic 时若持有 `Mutex`，锁会被标记为 poisoned（第 33 课）

### 25.8 panic vs Result 的选择

| 场景 | 选择 |
|---|---|
| 用户输入非法、文件不存在、网络失败 | `Result` |
| 库函数遇到调用方违反前置条件 | `panic!`/`assert!` |
| 程序启动缺配置/缺依赖 | `expect` 或 `Result` |
| 内部不变量被破坏 | `assert!`/`unreachable!` |
| FFI 边界、插件执行 | `catch_unwind` 隔离 |
| 测试断言 | `assert!`/`assert_eq!` |

库不应该对**可恢复的外部输入** panic；应用入口可以用 `expect` 表达"启动失败"。

## 二、具体语法

### 26.1 `panic!` / `panic_any`

```rust
panic!("boom");
panic!("invalid value: {value}");
std::panic::panic_any(42);
```

- `panic!` 支持格式化参数，生成 `String` payload
- `panic_any` 携带自定义 payload，配合 `downcast_ref` 使用
- `unreachable!`/`todo!`/`unimplemented!` 都是 panic 的变体

### 26.2 `catch_unwind`

```rust
let result = std::panic::catch_unwind(|| -> i32 {
    panic!("boom");
});
assert!(result.is_err());

let message = result
    .err()
    .and_then(|payload| payload.downcast_ref::<&str>().copied());
```

- 闭包正常返回时得到 `Ok(value)`
- `Err(payload)` 里的 payload 是 `Box<dyn Any + Send>`
- 需要静音 hook 时先 `set_hook`，结束后恢复

### 26.3 `resume_unwind`

```rust
let payload = catch_unwind(|| panic!("x")).unwrap_err();
let _ = catch_unwind(AssertUnwindSafe(|| {
    std::panic::resume_unwind(payload);
}));
```

- 在清理资源后重新抛出原 panic
- 保持原 payload，不重新构造

### 26.4 panic hook

```rust
let previous = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let message = info.payload().downcast_ref::<&str>().copied().unwrap_or("?");
    // 记录 message
}));
// ...
std::panic::set_hook(previous);
```

- `info.payload()` 取 payload，`info.location()` 取位置
- hook 里做日志要小心死锁（panic 可能发生在持锁时）

### 26.5 `UnwindSafe`

```rust
let mut value = 0;
let result = catch_unwind(AssertUnwindSafe(|| {
    value += 1;
    panic!("boom");
}));
```

- `&mut` 捕获需要 `AssertUnwindSafe`
- `RefCell`/`Cell` 同理
- 使用前确认展开后数据仍然满足不变量

### 26.6 `unwrap` / `expect`

```rust
let port: u16 = "8080".parse().unwrap();
let config = std::fs::read_to_string("app.conf").expect("read app.conf");
```

- `unwrap` 适合原型、测试、已证明不会失败的位置
- `expect` 的消息写清"在做什么时失败了"
- 生产代码优先 `?`/`unwrap_or_else`

### 26.7 线程 join

```rust
let handle = std::thread::spawn(|| panic!("worker"));
let result = handle.join();
assert!(result.is_err());
```

- join 的 `Err` 携带 panic payload
- 关心线程失败时不要忽略 `join()` 的返回值

### 26.8 `panic = "abort"`

```toml
[profile.release]
panic = "abort"
```

- 不能用 `catch_unwind`
- 双重 panic 直接 abort
- 适合体积敏感/无 unwind 表的平台

## 三、实际应用与最佳实践

### 27.1 插件隔离：把 panic 转成 `Err`

```rust
fn run_plugin<F>(plugin: F, input: i32) -> Result<i32, String>
where
    F: FnOnce(i32) -> i32 + UnwindSafe,
{
    match catch_unwind(move || plugin(input)) {
        Ok(value) => Ok(value),
        Err(_) => Err(String::from("plugin panicked")),
    }
}
```

插件/回调来自外部代码，不能保证不 panic；在边界处捕获并转成普通错误，
避免一个插件炸掉整个进程。

### 27.2 输入校验用 `Result`，不变量用 `assert!`

```rust
fn validate_age(age: i32) -> Result<i32, String> {
    if (0..=150).contains(&age) {
        Ok(age)
    } else {
        Err(format!("invalid age: {age}"))
    }
}
```

用户输入非法是正常业务分支；`assert!` 只用于"程序写错了"的情况。

### 27.3 hook 做日志/上报

```rust
std::panic::set_hook(Box::new(|info| {
    let location = info.location().map(|loc| loc.to_string());
    // 写入日志/上报
}));
```

- 适合崩溃报告、结构化日志
- 不要依赖 hook 恢复程序
- 多线程环境下 hook 是共享的

### 27.4 最佳实践清单

1. **可恢复错误用 `Result`**，panic 留给不变量/不可恢复错误
2. **库不要因外部输入 panic**，把决定权交给调用方
3. **`unwrap` 只用于已证明安全的位置**，并写注释
4. **`expect` 消息写清上下文**
5. **边界处 `catch_unwind`**：FFI、插件、线程池
6. **`&mut`/`RefCell` 跨边界要用 `AssertUnwindSafe` 并说明理由**
7. **panic hook 只做日志/上报**，不要再 panic
8. **不要忽略线程 `join()` 的 `Err`**
9. **`panic = "abort"` 下没有 `catch_unwind`**
10. **双重 panic 会 abort**，`Drop` 里避免 panic

## 常见陷阱

1. **`-C panic=abort` 下 `catch_unwind` 捕获不到**：进程直接终止；确认运行时模式
2. **`&mut T` 捕获不是 `UnwindSafe`（E0277）**：用 `AssertUnwindSafe`，但要保证状态可恢复
3. **`RefCell`/`Cell` 不是 `RefUnwindSafe`（E0277）**：内部可变性同理
4. **`Drop` 里 panic（双重 panic）**：进程 abort；析构函数不要 panic
5. **panic hook 修改后不恢复**：全局状态影响后续代码/测试
6. **hook 里再 panic**：直接 abort
7. **hook 里加锁导致死锁**：panic 可能发生在持锁期间
8. **假设 payload 一定是 `&str`**：格式化 `panic!` 产生 `String`；用 `downcast_ref`
9. **`catch_unwind` 当控制流**：性能差、语义混乱；用 `Result`
10. **`unwrap` 处理用户输入**：生产环境会 panic
11. **`assert!` 校验外部输入**：应该返回 `Result`
12. **`debug_assert!` 当业务校验**：release 下不执行
13. **`unreachable!` 用在可能到达的分支**：运行期 panic
14. **`todo!` 留在生产代码**：运行到就 panic
15. **FFI 边界让 panic 穿过 `extern "C"`**：未定义行为/终止；用 `catch_unwind` 包住
16. **忽略线程 `join` 的 `Err`**：后台线程失败被吞掉
17. **主线程 panic 退出码**：默认 101，脚本/容器要按非零处理
18. **panic 消息包含敏感信息**：日志/hook 会上报；注意脱敏
19. **嵌套 `catch_unwind` 以为能捕获 abort**：abort 不可捕获
20. **Mutex 在 panic 后 poisoning**：后续 `lock()` 返回 `Err`（第 33 课）

## 完整示例

见 `25_panic_unwrap_unwind.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ERRORS/25_panic与unwind/25_panic_unwrap_unwind.rs" -o run.exe
./run.exe
```

示例共 14 行输出，覆盖：`catch_unwind` 与 payload 向下转型、`unwrap`/`assert!` panic、
`AssertUnwindSafe`、线程 panic 与 `join`、panic hook 捕获、`panic_any`、
`resume_unwind`、插件隔离和输入校验。

## 练习

1. 用 `catch_unwind` 捕获 `panic!("boom")`，打印 payload 的 `&str` 内容。
2. 写一个捕获 `&mut` 状态的 `AssertUnwindSafe` 示例，说明为什么需要它。
3. 自定义 panic hook，把 panic 消息记录到 `Arc<Mutex<Vec<String>>>`，再恢复默认 hook。
4. 实现 `run_plugin`，把插件 panic 转成 `Result::Err`。
5. 启动一个会 panic 的线程，用 `join()` 拿到 `Err` 并打印 payload。
6. 用 `-C panic=abort` 编译带 `catch_unwind` 的程序，观察行为差异并解释。
7. 比较 `unwrap`、`expect`、`?`、`unwrap_or_else` 四种处理方式。
8. 解释为什么 `Drop` 里 panic 会导致 abort，并给出一个安全替代方案。

## 参考

- The Rust Programming Language: 第 9.1 节「不可恢复的错误与 panic!」
- std 文档：[`std::panic`](https://doc.rust-lang.org/std/panic/index.html)、
  [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- The Rustonomicon: [Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html)
- Cargo Book: [Profiles — panic](https://doc.rust-lang.org/cargo/reference/profiles.html#panic)
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
