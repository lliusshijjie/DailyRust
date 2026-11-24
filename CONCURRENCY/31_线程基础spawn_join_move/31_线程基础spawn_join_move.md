# 第 31 课 · 线程基础：spawn、join 与 move

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`31_threads_spawn_join_move.rs`

## 一、底层原理

### 31.1 线程是 OS 线程

`std::thread` 是 1:1 的 OS 线程封装：

```rust
let handle = std::thread::spawn(|| 21 * 2);
let value = handle.join().unwrap();   // 42
```

`spawn` 的签名：

```rust
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static;
```

三个约束分别意味着：

| 约束 | 含义 |
|---|---|
| `FnOnce() -> T` | 线程体是一个可调用一次的函数/闭包 |
| `Send` | 闭包捕获的数据可以**转移**到另一个线程 |
| `'static` | 捕获的数据不能借用调用栈上的短生命周期引用 |
| `T: Send + 'static` | 线程返回值也要能跨线程转移、且不含短借用 |

- 每个线程有独立栈（默认约 2 MiB，平台相关），可用 `Builder::stack_size` 调整
- `JoinHandle::join` 阻塞等待线程结束，返回 `Result<T, Box<dyn Any + Send>>`
- **丢弃 `JoinHandle` 会 detach 线程**：之后无法 join；主线程结束时进程退出，
  detached 线程会被直接终止
- 线程调度由 OS 决定，**执行顺序/打印顺序不确定**

### 31.2 `Send` 与 `Sync`

两者都是自动 trait（auto trait），由类型结构自动推导：

| trait | 含义 | 反例 |
|---|---|---|
| `Send` | 所有权可转移到另一个线程 | `Rc<T>` |
| `Sync` | `&T` 可被多个线程同时共享 | `RefCell<T>`、`Cell<T>` |

常见组合：

| 类型 | `Send` | `Sync` | 说明 |
|---|---|---|---|
| `i32`/`String` | ✅ | ✅ | 普通值 |
| `Vec<T>` | T: Send | T: Sync | 随元素 |
| `Rc<T>` | ❌ | ❌ | 引用计数非原子 |
| `Arc<T>` | T: Send + Sync | T: Send + Sync | 原子引用计数 |
| `RefCell<T>` | T: Send | ❌ | 运行期借用检查非线程安全 |
| `Cell<T>` | T: Send | ❌ | 内部可变性 |
| `Mutex<T>` | T: Send | T: Send | 提供同步 |
| `MutexGuard<'_, T>` | ❌ | ✅/— | 不能跨线程转移 |

派生规则：

- `&T: Send` 当且仅当 `T: Sync`
- `&mut T: Send` 当且仅当 `T: Send`
- 闭包的 `Send` 由它捕获的每个值决定

所以 `Rc`/`RefCell` 跨线程会编译失败（E0277），这是数据竞争在类型层面的防线。

### 31.3 `'static` 与 scoped threads

`spawn` 要求 `'static`，因为线程可能比调用者活得久：

```rust
let value = String::from("x");
let handle = std::thread::spawn(|| value.len());
// E0373: closure may outlive the current function, but it borrows `value`
```

`'static` 不是"永远存活"，而是"不包含非 `'static` 的借用"：拥有所有权的数据、
`&'static` 数据都满足。

三种解法：

1. `move` 把所有权交给线程（数据必须 `Send`）
2. `Arc<T>` 共享所有权（读多写少；写要 `Mutex`）
3. `std::thread::scope`（Rust 1.63+）：scoped threads 可以借用栈数据

```rust
let values = vec![1, 2, 3];
let sum = thread::scope(|scope| {
    let first = scope.spawn(|| values[0] + values[1]);
    let second = scope.spawn(|| values[2]);
    first.join().unwrap() + second.join().unwrap()
});
```

`scope` 会等待所有 scoped 线程结束；闭包不需要 `'static`，但仍要求 `Send`。

### 31.4 join 与 panic

- `join()` 返回 `Ok(value)` 或 `Err(payload)`（线程 panic）
- worker panic **不会**终止进程（默认 unwind）；主线程可以继续
- 如果 `JoinHandle` 被丢弃，panic 仍会打印 hook 消息，但没人拿到 `Err`
- `panic = "abort"` 下任何线程 panic 都会终止进程
- `join()` 消耗 `JoinHandle`，**不能 join 两次**（E0382）
- 主线程 panic 会结束进程；worker panic 只结束该线程

### 31.5 线程创建与配置

```rust
let handle = std::thread::Builder::new()
    .name("worker-1".into())
    .stack_size(4 * 1024 * 1024)
    .spawn(|| 3 + 4)
    .expect("spawn");
```

- `Builder::spawn` 返回 `io::Result<JoinHandle<T>>`，线程创建失败可处理
- `thread::spawn` 创建失败会 panic
- `thread::current().name()` 读取线程名（`Option<&str>`）
- `available_parallelism()` 返回 `io::Result<NonZeroUsize>`，提示可用并行度
- `thread::sleep` 只让出时间片，不是同步原语
- `thread::yield_now` 是调度提示，不保证任何顺序

### 31.6 共享数据的策略

| 需求 | 方案 |
|---|---|
| 把值交给线程并取回结果 | `move` + `join` |
| 多线程只读共享 | `Arc<T>`（T: Sync） |
| 多线程读写共享 | `Arc<Mutex<T>>` / `Arc<RwLock<T>>`（第 33 课） |
| 线程间传消息 | `mpsc` channel（第 32 课） |
| 借用栈数据、短期并行 | `thread::scope` |
| 只读静态数据 | `&'static` / `OnceLock` |

Rust 不允许 `&mut` 同时被多个线程持有；需要共享可变状态时用同步原语。

## 二、具体语法

### 32.1 `spawn` 与 `join`

```rust
let handle = std::thread::spawn(|| 21 * 2);
let value = handle.join().unwrap();
```

- `join` 阻塞直到线程结束
- 返回值通过 `JoinHandle<T>` 带回
- 不要忘记 join；否则线程被 detach

### 32.2 `move` 闭包

```rust
let text = String::from("hello");
let handle = std::thread::spawn(move || text.len());
let len = handle.join().unwrap();   // 5
// text 已被移动，不能再用
```

- `move` 把捕获值的所有权移入闭包
- `Copy` 类型是复制，不是移动
- `move` 只移动捕获的变量；如果捕获的是引用，仍然借用

### 32.3 `thread::scope`

```rust
let values = vec![1, 2, 3];
let sum = thread::scope(|scope| {
    let first = scope.spawn(|| values[0] + values[1]);
    let second = scope.spawn(|| values[2]);
    first.join().unwrap() + second.join().unwrap()
});
```

- scoped 线程的生命周期被 `scope` 限制，可以借用外部栈数据
- `scope` 结束前自动等待所有 scoped 线程
- 如果 scoped 线程 panic 且没有 join，`scope` 会在结束时 panic
- 适合分块计算、短期并行，不需要 `Arc`

### 32.4 `Builder`

```rust
let handle = std::thread::Builder::new()
    .name("worker-1".into())
    .stack_size(4 * 1024 * 1024)
    .spawn(|| (std::thread::current().name().unwrap_or("?").to_string(), 3 + 4))
    .expect("spawn");
```

- `name` 便于日志/调试
- `stack_size` 控制栈大小（深递归/大局部变量）
- 返回 `io::Result`，可处理创建失败

### 32.5 `Send`/`Sync` 编译错误

```rust
use std::rc::Rc;

let value = Rc::new(1);
std::thread::spawn(move || *value);
// E0277: `Rc<i32>` cannot be sent between threads safely
```

```rust
use std::cell::RefCell;

let cell = RefCell::new(0);
std::thread::scope(|scope| {
    scope.spawn(|| *cell.borrow_mut() += 1);
});
// E0277: `RefCell<i32>` cannot be shared between threads safely
```

- `Rc` 需要 `Arc`
- `RefCell`/`Cell` 需要 `Mutex`/`RwLock`/原子类型
- 编译错误是保护，不是障碍

### 32.6 并行度查询

```rust
let parallelism = std::thread::available_parallelism()
    .map(|value| value.get())
    .unwrap_or(1);
```

- 容器/CI 里可能返回 1
- 不要用固定线程数假设机器核数

## 三、实际应用与最佳实践

### 33.1 并行求和

```rust
let data = vec![1, 2, 3, 4, 5];
let (left, right) = data.split_at(data.len() / 2);
let total = thread::scope(|scope| {
    let left_sum = scope.spawn(|| left.iter().sum::<i32>());
    let right_sum = scope.spawn(|| right.iter().sum::<i32>());
    left_sum.join().unwrap() + right_sum.join().unwrap()
});
```

按块切分、每个线程算局部结果、最后合并；没有共享可变状态，也不需要锁。

### 33.2 分块词频

```rust
let (head, tail) = lines.split_at(1);
let words = thread::scope(|scope| {
    let first = scope.spawn(|| head.iter().map(|line| line.split_whitespace().count()).sum::<usize>());
    let second = scope.spawn(|| tail.iter().map(|line| line.split_whitespace().count()).sum::<usize>());
    first.join().unwrap() + second.join().unwrap()
});
```

分块处理 + 结果合并是并发任务的常见骨架；数据只读借用，`scope` 足够。

### 33.3 worker panic 隔离

```rust
let good = thread::spawn(|| 1 + 2);
let bad = run_thread_silently(|| -> () { panic!("boom") });
println!("partial={:?} panic={}", good.join(), bad.is_err());
```

- 一个 worker 失败不影响其他 worker
- 用 `join()` 的 `Result` 记录/上报失败
- 需要"全部成功或全部失败"时，在合并阶段统一处理

### 33.4 最佳实践

1. **优先 scoped threads 借用**，避免不必要的 `Arc`
2. **`move` 明确所有权**；注意 `move` 后原变量不可用
3. **每个线程都要 join 或明确 detach 的理由**
4. **worker panic 要处理 `join` 的 `Err`**
5. **共享只读用 `Arc<T>`，共享可选用 `Mutex`**
6. **不要依赖线程调度顺序**；用 join/通道/屏障同步
7. **线程数参考 `available_parallelism`**，不要硬编码
8. **给线程命名**，便于日志和调试
9. **大栈需求用 `Builder::stack_size`**
10. **用 `Send`/`Sync` 让编译器帮你发现数据竞争**

## 常见陷阱

1. **`spawn` 借用局部变量（E0373）**：加 `move`、用 `Arc` 或 `scope`
2. **`Rc` 跨线程（E0277）**：换 `Arc`
3. **`RefCell` 跨线程共享（E0277）**：换 `Mutex`/`RwLock`
4. **忘记 `join`**：线程 detach，主线程退出时被终止
5. **`join` 两次（E0382）**：`join` 消耗 `JoinHandle`
6. **忽略 `join` 的 `Err`**：worker panic 被吞掉
7. **以为 `move` 后还能用原变量**：所有权已转移
8. **`move` 捕获引用仍借用**：`move` 不延长引用生命周期
9. **把 `'static` 理解成"永远活着"**：它只要求无短借用
10. **线程数按 CPU 核数硬编码**：容器/CI 会过载
11. **依赖打印顺序做断言**：并发输出顺序不确定
12. **用 `sleep` 当同步**：时序不可靠；用 join/channel/barrier
13. **在 `Drop`/析构里 join 自己**：死锁
14. **`panic = "abort"` 下 worker panic**：整个进程退出
15. **栈溢出**：线程栈溢出通常直接 abort；深递归用 `stack_size`
16. **`Send` 与 `Sync` 混淆**：`Send` 是转移，`Sync` 是共享引用
17. **`Arc<T>` 要求 `T: Send + Sync`**：不是有了 `Arc` 就万事大吉
18. **`MutexGuard` 跨线程**：它不是 `Send`
19. **线程 ID 当唯一标识永久存储**：ID 可能被复用
20. **detach 线程访问已释放资源**：生命周期必须由所有权/`scope` 保证

## 完整示例

见 `31_threads_spawn_join_move.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "CONCURRENCY/31_线程基础spawn_join_move/31_threads_spawn_join_move.rs" -o run.exe
./run.exe
```

示例共 12 行输出，覆盖：`spawn`/`join` 返回值、worker panic 的 `Err`、
`available_parallelism`、`Builder` 命名线程、`move` 转移 `String`、
`thread::scope` 借用栈数据、并行求和、分块词频与 panic 隔离。

## 练习

1. `spawn` 一个线程计算 `1..=100` 的和，`join` 取回结果。
2. 用 `move` 把 `String` 移入线程，返回其长度；解释为什么原变量不能再使用。
3. 用 `thread::scope` 并行计算一个 `Vec<i32>` 的最大值和最小值。
4. 把数组分成 4 块，用 4 个 scoped 线程求和并合并。
5. 启动一个会 panic 的线程，`join` 后打印 `Err`；确认主线程继续运行。
6. 用 `Builder` 创建命名线程，打印线程名与返回值。
7. 写一段 `Rc` 跨线程代码，阅读 E0277 并改成 `Arc`。
8. 解释 `Send` 与 `Sync` 的区别，并各举一个满足/不满足的类型。

## 参考

- The Rust Programming Language: 第 16.1 节「使用线程同时运行代码」
- std 文档：[`std::thread`](https://doc.rust-lang.org/std/thread/index.html)、
  [`thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)、
  [`JoinHandle`](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
- The Rustonomicon: [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)、
  [E0373](https://doc.rust-lang.org/error_codes/E0373.html)、
  [E0382](https://doc.rust-lang.org/error_codes/E0382.html)
