# 第 23 课 · Result 与 ? 运算符

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`23_result_and_question_mark.rs`

## 一、底层原理

### 23.1 `Result` 是"可恢复错误"的显式建模

```rust
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

- `Ok(T)` 表示成功并携带值，`Err(E)` 表示失败并携带原因
- 调用方必须处理 `Err`，否则 `#[must_use]` 警告会提醒你
- 与 `Option` 的区别：`Option` 只说"没有值"，`Result` 还说"为什么失败"
- 与 panic 的区别：`Result` 是**值**，可以传播、组合、记录；panic 是控制流中断

| 场景 | 选择 |
|---|---|
| 查找不到、可选配置 | `Option<T>` |
| 解析失败、IO 失败、业务校验失败 | `Result<T, E>` |
| 程序员错误、不变量被破坏 | `panic!`/`assert!` |
| 程序启动阶段的致命错误 | `panic!`/`expect` |

Rust 没有异常；`Result` 把错误变成普通返回值，因此错误路径和正常路径一样可测试、
可组合、可穷尽匹配。

### 23.2 `?` 的展开：提前返回 + 错误转换

`expr?` 大致展开为：

```rust
match expr {
    Ok(value) => value,
    Err(error) => return Err(From::from(error)),
}
```

三个关键点：

1. **提前返回**：遇到 `Err` 直接 `return Err(...)`，后面的代码不执行
2. **自动转换**：`From::from(error)` 把错误类型转成函数返回类型要求的 `E`
3. **只能用于兼容的返回类型**：函数必须返回 `Result`（或 `Option`，或实现了
   `FromResidual` 的类型）

```rust
fn parse_and_double(text: &str) -> Result<i32, ParseIntError> {
    let value: i32 = text.parse()?;   // Err 时直接返回
    Ok(value * 2)
}
```

### 23.3 错误转换：`From` 是 `?` 的桥梁

`?` 不会做任意转换，它只调用 `From::from`。所以：

- 标准库已为常见错误提供转换（如 `io::Error` → `Box<dyn Error>`）
- 自定义错误要实现 `From<SourceError> for MyError`
- 没有合适的 `From` 时，用 `map_err` 显式转换

```rust
#[derive(Debug)]
enum AppError { Parse(ParseIntError) }

impl From<ParseIntError> for AppError {
    fn from(error: ParseIntError) -> Self { Self::Parse(error) }
}

fn double(text: &str) -> Result<i32, AppError> {
    let value: i32 = text.parse()?;   // ParseIntError -> AppError
    Ok(value * 2)
}
```

`Box<dyn std::error::Error>` 有 blanket impl：
`impl<E: Error + 'static> From<E> for Box<dyn Error>`，所以 `?` 可以直接把
实现了 `Error` 的具体错误装箱成 trait 对象。

### 23.4 `?` 的适用上下文

| 返回类型 | `?` 可以处理 | 备注 |
|---|---|---|
| `Result<T, E>` | `Result` | 错误经 `From` 转换 |
| `Option<T>` | `Option` | `None` 提前返回 |
| `Result<T, E>` | `Option` | **不行**（E0277）；先 `ok_or` |
| `Option<T>` | `Result` | **不行**（E0277） |
| `fn main() -> Result<(), E>` | `Result` | `E: Debug` 用于打印 |
| 闭包（返回 `Result`） | `Result` | 闭包必须声明返回类型/能推断 |
| `fn f() -> ()` | 都不行 | E0277 |

`?` 也能用于 `main`，只要 `main` 返回 `Result`：

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value: i32 = "42".parse()?;
    println!("{}", value);
    Ok(())
}
```

### 23.5 `Result` 的组合子

| 类别 | 方法 | 说明 |
|---|---|---|
| 变换成功值 | `map` | `Ok(T)` → `Ok(U)` |
| 变换错误值 | `map_err` | `Err(E)` → `Err(F)` |
| 链式 | `and_then` | 闭包返回 `Result` |
| 恢复 | `or_else` | 只在 `Err` 时尝试替代 |
| 查询 | `is_ok`、`is_err`、`is_ok_and`、`is_err_and` | 不取负载 |
| 取值 | `unwrap`、`expect`、`unwrap_or`、`unwrap_or_else`、`unwrap_or_default` | 拿到 `T` |
| 丢弃一半 | `ok` → `Option<T>`、`err` → `Option<E>` | 简化处理 |
| 互转 | `transpose` | `Result<Option<T>,E>` ↔ `Option<Result<T,E>>` |
| 迭代 | `collect::<Result<Vec<_>, _>>`、`try_fold`、`try_for_each` | 短路 |
| 观察 | `inspect`、`inspect_err` | 打日志不改变结果 |

和 `Option` 一样，注意惰性/立即求值：

- `map_or`/`unwrap_or` 的默认值立即求值
- `map_or_else`/`unwrap_or_else`/`or_else` 惰性
- `ok_or` 的错误值立即构造；`ok_or_else` 惰性

### 23.6 `Result` 与迭代器

- `collect::<Result<Vec<_>, _>>()`：遇到第一个 `Err` 短路
- `try_fold`/`try_for_each`：在中间某个元素失败时提前停止
- `partition(Result::is_ok)`：把成功/失败分成两组
- `filter_map(|x| x.ok())`：只保留成功值，丢弃错误

## 二、具体语法

### 24.1 构造与查询

```rust
let ok: Result<i32, &str> = Ok(5);
let err: Result<i32, &str> = Err("boom");

ok.is_ok();
ok.is_err();
ok.is_ok_and(|value| value > 0);
ok.ok();          // Some(5)
err.err();        // Some("boom")
```

### 24.2 `?` 基本用法

```rust
fn parse_and_double(text: &str) -> Result<i32, ParseIntError> {
    let value: i32 = text.parse()?;
    Ok(value * 2)
}
```

- `?` 只能放在返回 `Result`/`Option` 的函数里
- 遇到 `Err` 立即返回；`Drop` 会正常执行
- 不要用 `?` 代替所有 `match`：需要在当前函数里处理错误时用 `match`/`if let`

### 24.3 `From` 转换

```rust
impl From<ParseIntError> for AppError {
    fn from(error: ParseIntError) -> Self { Self::Parse(ParseIntError) }
}

fn double(text: &str) -> Result<i32, AppError> {
    let value: i32 = text.parse()?;
    Ok(value * 2)
}
```

- 一个自定义错误通常要为多个源错误实现 `From`
- `From` impl 要保留原始错误（作为 `source` 字段），不要只留字符串
- 没有 `From` 时用 `map_err`：

```rust
let value: i32 = text
    .parse()
    .map_err(|error| format!("bad number: {error}"))?;
```

### 24.4 `map` / `map_err`

```rust
ok.map(|value| value * 2);                       // Ok(10)
err.map_err(|error| format!("bad: {error}"));    // Err("bad: boom")
```

- `map` 只处理 `Ok`
- `map_err` 只处理 `Err`，用来统一错误类型/添加上下文

### 24.5 `and_then` / `or_else`

```rust
Ok::<i32, String>(8).and_then(half);              // Ok(4)
Err::<i32, &str>("x").or_else(|_| Ok(0));         // Ok(0)
```

- `and_then` 闭包返回 `Result`，用于串接可能失败的操作
- `or_else` 只在 `Err` 时执行，用于降级/重试

### 24.6 `unwrap` 家族

```rust
result.unwrap();                       // Err 时 panic
result.expect("config must exist");    // panic 时带消息
result.unwrap_or(0);                   // Err 时默认值
result.unwrap_or_else(|error| 0);      // 默认值惰性
result.unwrap_or_default();            // 需要 T: Default
```

- `unwrap` 只用于"这里不可能失败"的位置，并写注释
- `expect` 的消息要能定位问题
- 生产代码优先 `?`/`match`/`unwrap_or_else`

### 24.7 `ok` / `err` / `transpose`

```rust
Ok::<i32, &str>(1).ok();                          // Some(1)
Err::<i32, &str>("e").err();                      // Some("e")

Ok::<Option<i32>, &str>(Some(1)).transpose();     // Some(Ok(1))
```

- `ok`/`err` 把 `Result` 降级成 `Option`，丢弃另一半
- `transpose` 在"可选 + 可失败"两种组合间切换

### 24.8 `collect` / `try_fold`

```rust
let parsed: Result<Vec<i32>, _> = ["1", "x", "3"]
    .iter()
    .map(|text| text.parse::<i32>())
    .collect();                                   // Err(ParseIntError)

let sum: Result<i32, String> = [1, 2, 3]
    .iter()
    .try_fold(0, |acc, value| Ok(acc + value));
```

- `collect` 遇到第一个 `Err` 短路
- `try_fold`/`try_for_each` 在闭包返回 `Err` 时停止

### 24.9 `main` 返回 `Result`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value: i32 = "42".parse()?;
    println!("{}", value);
    Ok(())
}
```

- `main` 返回 `Result` 时，`Err` 会打印 `Debug` 并以非零状态退出
- `E` 需要实现 `Debug`（`Termination` 的要求）

## 三、实际应用与最佳实践

### 25.1 配置解析：`?` + `ok_or_else`

```rust
for line in input.lines() {
    let (key, value) = line.split_once('=')
        .ok_or_else(|| format!("bad line: {line}"))?;
    ...
}

Ok(Config {
    host: host.ok_or_else(|| String::from("missing host"))?,
    port: port.ok_or_else(|| String::from("missing port"))?,
})
```

`Option` 的"缺少字段"转成 `Result` 的"配置错误"；`?` 让主流程保持扁平。

### 25.2 批量解析：`collect::<Result<...>>`

```rust
let batch: Result<Vec<i32>, _> = ["1", "2", "x"]
    .iter()
    .map(|text| text.parse::<i32>())
    .collect();
```

- 成功时得到 `Vec<i32>`，失败时得到第一个错误
- 需要"收集所有错误"时用 `fold` 或 `partition`
- 需要"跳过失败项"时用 `filter_map(|x| x.ok())`

### 25.3 安全除法：可恢复错误用 `Result`

```rust
fn divide(left: i32, right: i32) -> Result<i32, String> {
    if right == 0 {
        Err(String::from("divide by zero"))
    } else {
        Ok(left / right)
    }
}
```

输入校验、除零、越界、解析失败都是"可恢复"的，返回 `Result` 让调用方决定
重试/降级/上报。

### 25.4 最佳实践清单

1. **可恢复错误用 `Result`**，不要用 panic 处理用户输入
2. **`?` 优先于层层 `match`**，但需要在当前位置处理错误时仍用 `match`
3. **自定义错误实现 `From`**，让 `?` 自动转换
4. **错误要保留 `source`/原始信息**，不要只留一句字符串
5. **`map_err` 用于添加/统一上下文**，不要吞掉源错误
6. **`unwrap` 只用于不可能失败的位置**，并写注释说明
7. **`collect::<Result<...>>` 短路**，`partition` 分离成功/失败
8. **`main` 可以返回 `Result`**，用 `?` 简化启动逻辑
9. **错误类型尽量具体**，`Box<dyn Error>` 适合入口/混合错误，不宜到处用
10. **为每种失败定义清晰语义**，不要用 `Result<T, ()>`

## 常见陷阱

1. **`?` 用在返回 `()` 的函数（E0277）**：改成返回 `Result`/`Option`，
   或在 `main` 上写 `-> Result<(), E>`
2. **在 `Result` 函数里 `?` 一个 `Option`（E0277）**：先 `ok_or`/`ok_or_else`
3. **在 `Option` 函数里 `?` 一个 `Result`（E0277）**：转换或改成 `Result`
4. **自定义错误缺少 `From`（E0277）**：实现 `From<E>` 或用 `map_err`
5. **`unwrap` 遇到 `Err` panic**：生产代码要处理错误
6. **`expect` 消息没有上下文**：panic 信息应能定位问题
7. **`map_err` 丢弃源错误**：丢失根因；保留 `source`
8. **`map` 与 `and_then` 混淆**：闭包返回 `Result` 才用 `and_then`
9. **`or_else` 与 `or` 混淆**：`or` 立即求值，`or_else` 惰性
10. **`collect::<Result<...>>` 缺类型标注**：推断不出来；写目标容器
11. **闭包里用 `?` 但闭包不返回 `Result`**：闭包签名要声明返回类型
12. **`Result<(), ()>` 无语义**：至少给出错误枚举或消息
13. **`ok()` 丢弃错误**：需要诊断信息时不要用
14. **`unwrap_or_default` 需要 `Default`**：自定义类型未实现会编译失败
15. **在 `main` 里 `?` 但没有改返回类型（E0277）**：改 `main` 签名
16. **`?` 与 `Drop` 的误解**：提前返回会正常 drop 局部变量，没有泄漏
17. **`try` 块**：稳定版还不支持，别照搬 nightly 写法
18. **`Box<dyn Error>` 无法 `match` 具体类型**：需要分支处理时用自定义枚举
19. **错误链没有 `source()`**：日志里看不到根因
20. **用 `Result` 表达"没有值"**：应该用 `Option`，避免给"缺失"编造错误

## 完整示例

见 `23_result_and_question_mark.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ERRORS/23_Result与问号运算符/23_result_and_question_mark.rs" -o run.exe
./run.exe
```

示例共 19 行输出，覆盖：`?` 提前返回、`From` 错误转换、`Box<dyn Error>`、
`ok_or_else`、`map`/`map_err`、`and_then`/`or_else`、`unwrap_or_else`、
`collect::<Result>`、`transpose`、`ok`/`err`，最后用配置解析、批量解析和安全除法收尾。

## 练习

1. 写 `fn parse_and_double(text: &str) -> Result<i32, ParseIntError>`，用 `?`
   传播解析失败，再乘 2。
2. 定义 `AppError`，为 `ParseIntError` 与 `std::io::Error` 各实现 `From`，
   写一个同时用到两者的函数。
3. 用 `?` + `ok_or_else` 解析 `host=...\nport=...` 配置，缺少字段时报错。
4. 用 `collect::<Result<Vec<_>, _>>` 解析一组字符串；再改成 `filter_map`
   跳过失败项，比较行为。
5. 用 `try_fold` 实现"累加直到遇到第一个负数就返回 `Err`"。
6. 把 `main` 改成 `fn main() -> Result<(), Box<dyn Error>>`，用 `?` 解析一个数字。
7. 实现 `fn divide(a: i32, b: i32) -> Result<i32, String>`，分别用 `match`、
   `?`、`unwrap_or_else` 三种方式调用。
8. 比较 `map_err` 与实现 `From` 两种错误转换方式的适用场景。

## 参考

- The Rust Programming Language: 第 9 章「错误处理」
- std 文档：[`Result`](https://doc.rust-lang.org/std/result/enum.Result.html)、
  [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
- Rust by Example: [Result](https://doc.rust-lang.org/rust-by-example/std/result.html)
- 生态：`thiserror`（自定义错误枚举）、`anyhow`（应用层错误上下文）
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
