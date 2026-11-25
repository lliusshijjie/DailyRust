# 第 32 课 · 消息传递：mpsc 通道

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`32_mpsc_channels.rs`

## 一、底层原理

### 32.1 mpsc 模型

`mpsc` = **multi-producer, single-consumer**：

```rust
let (sender, receiver) = std::sync::mpsc::channel();
sender.send(42).unwrap();
let value = receiver.recv().unwrap();
```

- `channel()` 创建**无界**通道：`send` 不阻塞，内存可无限增长
- `sync_channel(n)` 创建**有界**通道：缓冲区满时 `send` 阻塞（背压）
- `Sender<T>` 可以 `clone`，多个生产者
- `Receiver<T>` **不能 clone**，只有一个消费者
- `send(T)` 把 `T` 的所有权移入通道，`recv()` 取回

### 32.2 所有权、`Send` 与线程

- 通道本身不要求 `T: Send`；在**同一线程**里甚至可以传 `Rc`
- 一旦 `Sender<T>` 要移动到另一个线程，就需要 `Sender<T>: Send`，
  也就是 `T: Send`
- `Receiver<T>` 是 `Send`（可以移动到别的线程收），但**不是 `Sync`**（不能共享引用）
- `Sender<T>` 是 `Send + Sync`（可以 clone 给多个线程）

关键错误：

```rust
let (sender, receiver) = mpsc::channel();
let value = Rc::new(1);
std::thread::spawn(move || sender.send(value).unwrap());
// E0277: `Rc<i32>` cannot be sent between threads safely
```

### 32.3 关闭语义

| 事件 | 结果 |
|---|---|
| 所有 `Sender` 被 drop | `recv()` → `Err(RecvError)`；`iter()` 结束 |
| `Receiver` 被 drop | `send()` → `Err(SendError(value))`，值可回收 |
| 通道空但 sender 还在 | `try_recv()` → `Err(TryRecvError::Empty)` |
| sender 全 drop 且空 | `try_recv()` → `Err(TryRecvError::Disconnected)` |
| 超时 | `recv_timeout(d)` → `Err(RecvTimeoutError::Timeout)` |

`iter()`/`for msg in receiver` 会在**所有 sender 都 drop** 后结束；如果主线程自己
还持有 `Sender` 的克隆，循环永远不会结束（常见死锁）。

### 32.4 有界通道与背压

```rust
let (sender, receiver) = mpsc::sync_channel(2);
sender.send(1).unwrap();
sender.send(2).unwrap();
// sender.send(3) 会阻塞，直到 receiver 取走一个
sender.try_send(3);   // Err(TrySendError::Full(3))
```

- `sync_channel(0)` 是**会合通道**：`send` 阻塞到有 `recv` 配对
- 有界通道提供背压，防止生产者把内存打爆
- `try_send` 返回 `Full(value)`/`Disconnected(value)`，可决定丢弃/重试/阻塞

### 32.5 单消费者与线程池

`std::sync::mpsc` 的 `Receiver` 不能 clone，所以：

- **一个 worker**：直接 `move` 给 worker
- **多个 worker 共享一个队列**：需要 `Arc<Mutex<Receiver>>`（锁会把消费串行化）
  或每个 worker 一条通道（由分发器轮询）
- **真正的 MPMC**：用 `crossbeam-channel`/`flume`；`crossbeam` 还提供 `select!`
- 异步场景用 `tokio::sync::mpsc`（有界、async send/recv）

### 32.6 顺序与并发

- **同一个 sender** 发送的消息保持 FIFO
- **多个 sender** 之间的交错顺序不确定
- 需要确定性输出时：收集后排序、按 key 聚合，或只输出总数
- `try_recv` 是"此刻有没有"，不要用它做严格同步；用阻塞 `recv` 或屏障

## 二、具体语法

### 33.1 基本收发

```rust
use std::sync::mpsc;

let (sender, receiver) = mpsc::channel::<i32>();
sender.send(42).unwrap();
assert_eq!(receiver.recv().unwrap(), 42);
```

- `recv()` 阻塞直到有消息或通道关闭
- `send()` 在无界通道上不阻塞
- 发送后原值被移动，不能再用

### 33.2 克隆 Sender（多生产者）

```rust
let (sender, receiver) = mpsc::channel();
let sender2 = sender.clone();

let first = thread::spawn(move || {
    sender.send((1, 10)).unwrap();
});
let second = thread::spawn(move || {
    sender2.send((2, 30)).unwrap();
});

let messages: Vec<(i32, i32)> = receiver.iter().collect();
first.join().unwrap();
second.join().unwrap();
```

- 每个生产者持有自己的 `Sender` 克隆
- 所有克隆都 drop 后，`receiver.iter()` 结束
- 多生产者顺序不保证；排序/聚合后再断言

### 33.3 `try_recv` / `try_iter` / `recv_timeout`

```rust
match receiver.try_recv() {
    Ok(value) => println!("{value}"),
    Err(TryRecvError::Empty) => println!("empty"),
    Err(TryRecvError::Disconnected) => println!("closed"),
}

let values: Vec<i32> = receiver.try_iter().collect();

let value = receiver.recv_timeout(Duration::from_millis(100));
```

- `try_recv` 立即返回，适合事件循环轮询
- `try_iter` 非阻塞地取完当前可用消息
- `recv_timeout` 带超时，注意时序测试可能 flaky

### 33.4 有界通道

```rust
use std::sync::mpsc::{self, TrySendError};

let (sender, receiver) = mpsc::sync_channel(1);
sender.send(1).unwrap();
match sender.try_send(2) {
    Ok(()) => println!("accepted"),
    Err(TrySendError::Full(value)) => println!("full: {value}"),
    Err(TrySendError::Disconnected(value)) => println!("closed: {value}"),
}
```

- 缓冲区大小 = 尚未被消费的消息上限
- `send` 满时阻塞；`try_send` 满时立即返回
- `sync_channel(0)` 需要接收端同时 `recv` 才能完成 `send`

### 33.5 通道关闭与错误

```rust
let (sender, receiver) = mpsc::channel::<i32>();
drop(receiver);
let error = sender.send(1).unwrap_err();
println!("returned value = {}", error.0);
```

```rust
let (sender, receiver) = mpsc::channel::<i32>();
drop(sender);
assert!(receiver.recv().is_err());
```

- `SendError<T>` 是元组结构体，`error.0` 取回消息
- `RecvError` 表示所有 sender 已 drop
- 生产代码不要对 `send` 无条件 `unwrap`；接收端可能先退出

### 33.6 工作线程流水线

```rust
let (job_sender, job_receiver) = mpsc::channel::<u32>();
let (result_sender, result_receiver) = mpsc::channel::<u32>();

let worker = thread::spawn(move || {
    for job in job_receiver {
        result_sender.send(job * job).unwrap();
    }
});

for job in 1..=4 {
    job_sender.send(job).unwrap();
}
drop(job_sender);

let results: Vec<u32> = result_receiver.iter().collect();
worker.join().unwrap();
```

- job 通道关闭 → worker 的 `for` 结束 → result sender drop → 结果迭代器结束
- 这种"输入通道 + 输出通道"是最常见的 worker 结构
- 多个 worker 时，需要 MPMC 或每个 worker 一条 job 通道

### 33.7 事件计数

```rust
let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
for event in receiver {
    let key = match event {
        Event::Click => "click",
        Event::Move => "move",
    };
    *counts.entry(key).or_insert(0) += 1;
}
```

- 消息用枚举表达"发生了什么"
- 消费端聚合，输出有序（BTreeMap）
- 事件循环可以继续扩展成 actor 模型

## 三、实际应用与最佳实践

### 34.1 Job 流水线

```
main ──job──▶ worker ──result──▶ main
```

- 生产者发送 job，worker 计算结果，主线程收集
- 用 `drop(job_sender)` 通知结束
- 用 `join()` 等待 worker，避免提前退出
- 结果顺序：单 worker 时通常保序，但不要依赖；排序/带 id 更稳

### 34.2 事件计数与聚合

- 多生产者发送事件枚举
- 消费者用 `BTreeMap`/`HashMap` 聚合
- 输出前排序，保证测试确定性
- 需要"每个生产者独立计数"时，在消息里带 producer id

### 34.3 背压

- 无界通道适合短时突发，但要注意内存
- 有界通道 + 阻塞 `send` 把压力传回生产者
- `try_send` + 丢弃/降级适合"允许丢消息"的场景（指标、日志）
- 队列长度是重要监控指标

### 34.4 最佳实践清单

1. **明确通道关闭条件**：谁持有 Sender、何时 drop
2. **`Receiver` 只有一个消费者**；多消费者用 crossbeam/分发器
3. **跨线程消息类型必须 `Send`**
4. **对 `send` 的 `Err` 做处理**，不要无条件 `unwrap`
5. **优先有界通道**，除非明确需要无界缓冲
6. **多生产者输出先排序/聚合**，不要依赖交错顺序
7. **用枚举表达消息类型**，避免 `String` 类型标签
8. **worker 结束后 join**，确保结果都处理完
9. **`try_recv` 用于轮询，不用于严格同步**
10. **性能敏感/需要 select 时用 crossbeam-channel**

## 常见陷阱

1. **接收端已 drop，`send().unwrap()` panic**：用 `if let Err` 处理
2. **忘记 drop 原始 Sender**：`for msg in receiver` 永远不结束
3. **主线程持有 Sender 还等 `iter()`**：死锁
4. **`Receiver` clone 编译失败（E0599）**：std mpsc 单消费者；换 crossbeam
5. **无界通道内存暴涨**：生产快于消费；改用 `sync_channel`
6. **`sync_channel` 满时 `send` 阻塞**：同线程先 send 再 recv 会死锁
7. **`sync_channel(0)` 误以为能缓冲**：它是会合通道
8. **多生产者顺序不确定**：断言前排序/聚合
9. **`try_recv` 的 Empty 当成"永远没消息"**：只是此刻为空
10. **`try_iter` 在 sender 还活着时结束**：它只取当前可用，不等待
11. **`recv_timeout` 做时序断言**：CI 上容易 flaky
12. **`Rc` 通过 Sender 跨线程（E0277）**：Sender 移动需要 T: Send
13. **以为 `send` 后还能用原值**：所有权已移入通道
14. **消息类型太大**：通道移动大对象；考虑 `Box`/`Arc`
15. **worker panic 导致通道关闭**：消费端要处理 `Err`
16. **`Sender` 被 panic 展开时 drop**：通道可能提前关闭
17. **用通道做共享状态**：`Mutex`/`RwLock` 更直接；通道适合消息流
18. **多个 worker 抢一个 `Receiver`**：需要锁，吞吐受限；用 MPMC
19. **std 没有 `select`**：多通道等待用 crossbeam-channel
20. **异步代码用 std mpsc**：阻塞 `recv` 会卡住 executor；用 tokio mpsc

## 完整示例

见 `32_mpsc_channels.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "CONCURRENCY/32_消息传递mpsc/32_mpsc_channels.rs" -o run.exe
./run.exe
```

示例共 13 行输出，覆盖：`send`/`recv`、通道关闭、`try_recv` 的 `Empty`、
接收端 drop 后的 `SendError`、克隆 `Sender` 多生产者、`try_iter`、
`sync_channel` 的 `Full`、worker 平方流水线与事件计数。

## 练习

1. 创建通道，发送一个 `String`，接收后打印长度。
2. 克隆 `Sender`，两个线程各发 3 条消息，收集后排序输出。
3. 验证：所有 sender drop 后 `recv()` 返回 `Err`；`try_recv()` 返回 `Disconnected`。
4. 用 `try_recv` 写一个非阻塞事件循环，处理完当前消息后退出。
5. 用 `sync_channel(2)` 演示背压：第 3 次 `try_send` 返回 `Full`。
6. 实现 job/result 双通道 worker，计算一组数字的平方和。
7. 多生产者发送事件枚举，消费端用 `BTreeMap` 计数。
8. 用 crossbeam-channel 的 `select!` 同时等待两个通道（只写代码，不要求本机编译）。

## 参考

- The Rust Programming Language: 第 16.2 节「使用消息传递在线程间传送数据」
- std 文档：[`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html)、
  [`Sender`](https://doc.rust-lang.org/std/sync/mpsc/struct.Sender.html)、
  [`Receiver`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html)
- 生态：`crossbeam-channel`（MPMC/select）、`flume`、`tokio::sync::mpsc`
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)、
  [E0599](https://doc.rust-lang.org/error_codes/E0599.html)
