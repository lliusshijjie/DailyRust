# 第 28 课 · 错误处理策略与 API 边界设计

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`28_error_strategy_and_api.rs`

## 一、底层原理

### 28.1 错误是 API 契约的一部分

函数签名里的 `Result<T, E>` 不只是"可能失败"，它还是**公开契约**：

- 调用方必须处理 `E`
- `E` 的变体、`Display` 文本、`source` 链都属于 API 表面
- 库的错误枚举一旦公开，新增/删除变体就是破坏性变更（所以用 `#[non_exhaustive]`）

一个边界函数通常只有四种结局：

| 结局 | 类型 | 例子 |
|---|---|---|
| 成功 | `T` | 返回解析结果 |
| 可能没有 | `Option<T>` | 查找、可选配置 |
| 可恢复失败 | `Result<T, E>` | 解析失败、IO 失败、校验失败 |
| 程序 bug | `panic!` | 不变量被破坏、unreachable |

把"用户输入非法"写成 panic，或把"程序 bug"写成 `Result`，都是边界设计错误。

### 28.2 分层错误模型

典型分层：

```
adapter/CLI/HTTP
    │  AppError（边界错误：用户可读、可映射退出码/状态码）
    ▼
service/domain
    │  ServiceError（业务语义：NotFound/Conflict/Validation）
    ▼
repository/infra
    │  InfraError（IO/DB/网络：io::Error、sqlx::Error...）
```

原则：

1. **每层只暴露本层语义**：领域层不返回 `io::Error`，而是包装成 `ConfigError`
2. **在边界处转换**：`From` 实现让 `?` 自动升层；没有 `From` 就 `map_err`
3. **保留 `source`**：包装时不要丢掉底层错误
4. **应用层统一错误**：`AppError` 汇总各来源，便于 CLI/HTTP 映射
5. **库用具体枚举，应用可用 `anyhow`/`Box<dyn Error>`**：库的调用方需要 `match`

### 28.3 错误分类与处理策略

| 错误类别 | 例子 | 策略 |
|---|---|---|
| 校验失败 | 端口为 0、年龄越界 | 返回 `Result`，收集全部字段错误 |
| 未找到 | 用户不存在 | `Option` 或 `Result::NotFound` |
| 冲突 | 唯一键重复 | `Result::Conflict`，不重试 |
| 瞬态 | 网络超时、锁竞争 | 有限重试 + 退避 |
| 永久 | 权限不足、格式错误 | 直接失败，不重试 |
| 程序 bug | 不变量破坏 | `assert!`/`panic!` |

常见策略：

| 策略 | 适用 | 风险 |
|---|---|---|
| 传播（`?`） | 本层无法处理 | 错误上下文丢失 |
| 就地处理 | 有明确恢复动作 | 吞掉错误 |
| 重试 | 瞬态、幂等 | 放大故障、重复副作用 |
| 降级/默认值 | 非关键依赖 | 掩盖问题 |
| 收集全部错误 | 表单校验 | 内存/延迟 |
| fail-fast | 流水线、启动检查 | 只看到第一个问题 |
| abort/panic | 不变量破坏 | 进程退出 |

重试规则：

- 只重试**瞬态**且**幂等**的操作
- 有次数上限与总超时
- 退避 + 抖动，避免惊群
- 校验错误、权限错误不要重试
- 重试要可观测（次数、原因）

### 28.4 API 边界设计

**边界校验 + newtype**：

```rust
struct Port(u16);

impl Port {
    fn new(value: u16) -> Result<Self, ConfigError> {
        if value == 0 { Err(ConfigError::InvalidPort(value)) } else { Ok(Port(value)) }
    }
}
```

- 构造成功即保证"非零"，后续函数不用重复校验
- 非法状态在类型层面不可表示（'make illegal states unrepresentable'）
- 解析/转换放在边界（`TryFrom`、`FromStr`），内部函数接收已校验类型

其他原则：

- **库不要 `unwrap`/`panic` 处理外部输入**；返回 `Result`
- **不要暴露内部错误类型**；用领域错误包装
- **错误类型加 `#[non_exhaustive]`**（公开枚举），为新增变体留余地
- **跨线程/异步错误要 `Send + Sync + 'static`**
- **文档写 `# Errors` / `# Panics`**，说明何时返回什么、何时 panic
- **错误消息包含上下文**：操作、路径、字段、值（注意脱敏）
- **`From` 转换保持一一对应**；同一源错误落多个变体时用 `map_err`

### 28.5 日志与可观测性

- **在边界记录一次**：每层都 log 会重复、放大日志量
- 记录错误链（`source`）、操作、关键参数、请求 id
- 用户消息与调试信息分开：用户看 `Display`，日志看 `Debug` + 链
- 不要把密码/token/身份证号写进错误或日志
- 结构化字段优于字符串拼接（`tracing`）

### 28.6 CLI / 服务边界

- CLI：错误 → 用户可读消息 + 非零退出码（通常 1；用法错误可用 2）
- `fn main() -> Result<(), E>` 会在 `Err` 时打印 `Debug` 并返回非零
- 服务：错误 → HTTP 状态码/错误码；不要把内部错误细节直接返回给客户端
- 默认不向用户展示 backtrace；调试时用 `RUST_BACKTRACE=1`

### 28.7 测试错误路径

- 不只测 happy path：每个错误变体至少一个测试
- 断言错误变体（`matches!`）而不是只断言 `is_err()`
- 断言 `source` 链保留
- 测试重试次数、退避上限、降级行为
- 测试错误消息（用户可见的部分）

## 二、具体语法

### 29.1 newtype 边界校验

```rust
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
}
```

- 私有字段 + 构造函数，保证不变量
- 需要读取时提供 `value()` 方法
- 也可以用 `TryFrom<u16>`/`FromStr` 接入标准转换

### 29.2 分层错误 + `From`

```rust
enum ConfigError { MissingHost, InvalidPort(u16), ParsePort(ParseIntError) }

struct LoadError { path: String, source: ConfigError }

enum AppError { Load(LoadError), Validation(Vec<FieldError>) }

impl From<LoadError> for AppError { ... }
```

- 每层 `Display` 只写本层信息
- `source()` 指向下一层
- `?` 通过 `From` 自动升层
- 应用层可以汇总多种来源（加载错误 + 校验错误）

### 29.3 fail-fast vs 收集全部错误

```rust
// fail-fast：遇到第一个 Err 就停
let parsed: Result<Vec<Port>, ConfigError> = inputs
    .iter()
    .map(|text| { let raw: u16 = text.parse()?; Port::new(raw) })
    .collect();

// 收集全部：返回 Vec<FieldError>
fn validate_form(name: &str, age: i32) -> Result<(), Vec<FieldError>> { ... }
```

- 流水线/启动检查用 fail-fast（快速失败、省资源）
- 表单/批量导入用收集全部（一次告诉用户所有问题）
- 收集全部要注意去重、上限、顺序稳定

### 29.4 重试

```rust
fn retry_fetch(attempts: u32) -> Result<u32, &'static str> {
    for attempt in 1..=attempts {
        if attempt == 3 {
            return Ok(attempt);
        }
    }
    Err("retries exhausted")
}
```

- 真实实现要区分错误类型：只重试瞬态错误
- 加退避（指数/抖动）与总超时
- 非幂等操作（扣款、下单）重试要幂等键

### 29.5 降级/默认值

```rust
let config = load("missing.conf", "")
    .or_else(|_| Ok::<Config, LoadError>(fallback_config()));
```

- 只对"非关键依赖"降级
- 降级要记录日志/指标，否则问题被掩盖
- 默认值必须是安全的（不能悄悄放大权限）

### 29.6 边界上的 trait 对象

```rust
fn run() -> Result<(), Box<dyn Error + Send + Sync>> { ... }
```

- 应用入口/混合错误源可以用 `Box<dyn Error + Send + Sync>`
- 库公开 API 优先具体枚举，调用方可 `match`
- `Box<dyn Error>` 丢失类型，需要 `downcast_ref` 才能还原

### 29.7 退出码与文档

```rust
fn main() -> Result<(), Box<dyn Error>> {
    // ...
    Ok(())
}
```

- `Err` 时进程以非零退出（通常 101；`Termination` 实现决定）
- 需要自定义退出码时用 `std::process::ExitCode`
- 文档写：

```rust
/// # Errors
/// 当端口为 0 或缺少 host 时返回 `ConfigError`。
///
/// # Panics
/// 当内部不变量被破坏时 panic（正常输入不会触发）。
fn load_config(input: &str) -> Result<Config, ConfigError> { ... }
```

## 三、实际应用与最佳实践

### 30.1 配置加载流水线

```
load_config (domain) -> load (service, path context) -> run (app, validation) -> CLI
```

示例输出：

```
error_chain=load failed | failed to load app.conf | invalid port: 0
```

- `AppError` 只写"应用层失败"
- `LoadError` 写"哪个文件"
- `ConfigError` 写"哪个字段/为什么"
- `source` 链把三层串起来，日志一次打印完整根因

### 30.2 重试与降级

- 瞬态错误：有限重试 + 退避
- 非关键配置：降级到安全默认值 + 记录
- 校验错误：立即失败，不重试
- 降级路径要有测试，否则上线才发现默认值不可用

### 30.3 校验错误聚合

```rust
let errors = validate_form("", -5).unwrap_err();
// [name: must not be empty, age: must be between 0 and 150]
```

- 一次返回全部字段错误，前端可逐项标红
- 错误顺序稳定，便于测试/展示
- 每个 `FieldError` 带字段名 + 消息，不拼接大字符串

### 30.4 最佳实践清单

1. **边界解析、内部信任**：newtype 保证不变量
2. **库返回具体错误枚举，应用用 `anyhow`/boxed**
3. **公开错误枚举加 `#[non_exhaustive]`**
4. **每层只暴露本层语义**，不泄露 IO/DB 错误
5. **包装错误必须保留 `source`**
6. **可恢复错误用 `Result`，bug 才 panic**
7. **重试只针对瞬态 + 幂等 + 有上限**
8. **降级要记录，默认值要安全**
9. **边界只记录一次日志**，带上下文与错误链
10. **文档写 `# Errors` / `# Panics`，测试覆盖错误路径**

## 常见陷阱

1. **领域层直接返回 `io::Error`**：泄露实现细节，调用方被迫依赖 IO
2. **库公开 `Box<dyn Error>`**：调用方无法 `match`，只能靠字符串
3. **公开错误枚举没有 `#[non_exhaustive]`**：新增变体破坏下游
4. **边界 `unwrap`**：用户输入触发 panic
5. **对用户输入 `assert!`**：应该返回 `Result`
6. **重试校验错误**：永远不会成功，浪费资源
7. **无上限重试**：故障时雪崩
8. **非幂等操作重试**：重复扣款/下单
9. **每层都 log 同一个错误**：日志重复、难以定位
10. **`ok()`/`unwrap_or_default()` 吞错误**：失败被静默忽略
11. **错误消息没有上下文**：只写 "failed"
12. **包装时丢掉 `source`**：日志看不到根因
13. **混用 `Option` 与 `Result`**：缺失 vs 失败语义混乱
14. **错误类型不 `Send + Sync`**：跨线程/异步编译失败
15. **`From` 冲突/歧义**：同一源错误落多个变体，`?` 无法选择
16. **`match` 用 `_ =>` 吞掉新变体**：升级依赖后行为悄悄改变
17. **错误时退出码仍为 0**：脚本/CI 误判成功
18. **把 backtrace 直接返回给用户**：泄露路径/内部结构
19. **错误里带密码/token**：日志与上报系统全量收集
20. **只测 happy path**：错误分支上线才暴露

## 完整示例

见 `28_error_strategy_and_api.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ERRORS/28_错误处理策略与API边界/28_error_strategy_and_api.rs" -o run.exe
./run.exe
```

示例共 16 行输出，覆盖：`Port` newtype 校验、`Option` 查找、内部不变量 panic、
`ConfigError → LoadError → AppError` 三层错误、fail-fast、校验错误聚合、
重试、降级、错误链与 CLI 退出码。

## 练习

1. 为 `Port` 添加 `TryFrom<u16>` 与 `FromStr`，保证非法值无法构造。
2. 设计 `ConfigError`、`LoadError`、`AppError` 三层错误，用 `?` 自动升层。
3. 比较 fail-fast 与收集全部错误：分别实现并测试多字段校验。
4. 给一个瞬态错误加重试：最多 3 次、指数退避（可用计数模拟）。
5. 实现降级：配置加载失败时返回安全默认值，并记录降级原因。
6. 给公开函数补 `# Errors` / `# Panics` 文档，说明每种失败。
7. 把 CLI 入口改成 `fn main() -> Result<(), Box<dyn Error>>`，观察退出码。
8. 写测试覆盖每个错误变体、`source` 链与重试次数上限。

## 参考

- The Rust Programming Language: 第 9 章「错误处理」
- Rust API Guidelines: [Interoperability — error types](https://rust-lang.github.io/api-guidelines/interoperability.html)
- std 文档：[`Result`](https://doc.rust-lang.org/std/result/enum.Result.html)、
  [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
- 生态：`thiserror`（库错误枚举）、`anyhow`（应用错误上下文）、`tracing`（可观测性）
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)、
  [E0004](https://doc.rust-lang.org/error_codes/E0004.html)
