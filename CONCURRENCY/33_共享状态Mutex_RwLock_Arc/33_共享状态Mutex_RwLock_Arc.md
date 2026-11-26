# 第 33 课 · 共享状态：Mutex、RwLock 与 Arc

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`33_mutex_rwlock_arc.rs`

## 一、底层原理

### 33.1 三个角色

| 类型 | 解决什么 | 代价 |
|---|---|---|
| `Arc<T>` | 多线程**共享所有权**（原子引用计数） | 原子增减；只读访问 |
| `Mutex<T>` | 多线程**互斥访问**可变数据 | 加锁/解锁、争用阻塞 |
| `RwLock<T>` | 多读**共享**、单写**独占** | 读写锁状态、可能的写者饥饿 |

组合：

```rust
let shared = Arc::new(Mutex::new(0));
```

- `Arc` 提供"大家都能持有"，`Mutex` 提供"同一时刻只有一个人能改"
- `Arc<T>` 本身不提供可变性；要修改内部值需要 `Mutex`/`RwLock`/原子类型
- `Arc<Mutex<T>>` 是跨线程共享可变状态的标准组合

### 33.2 `Arc<T>`：原子引用计数

```
Arc 指针 ──▶ ┌──────────────┐
             │ strong: 2    │
             │ weak: 0      │
             │ value: T     │
             └──────────────┘
```

- `Arc::clone` 只把 strong 加 1，O(1)，不复制值
- `Arc::strong_count`/`ptr_eq`/`try_unwrap`/`get_mut`
- `Arc::make_mut`：写时复制（有别的引用时先 clone）
- `Weak<T>` 打破循环引用（第 21 课）
- `Arc<T>: Send + Sync` 当且仅当 `T: Send + Sync`
- `strong_count` 在并发下只是近似值，不能当同步原语

### 33.3 `Mutex<T>`：互斥锁

```rust
let mutex = Mutex::new(0);
let mut guard = mutex.lock().unwrap();
*guard += 1;
// guard 离开作用域时自动解锁
```

- `lock()` 返回 `LockResult<MutexGuard<T>>`；`MutexGuard` 实现 `Deref`/`DerefMut`
- guard drop 时解锁；**锁的作用域就是 guard 的生命周期**
- `try_lock()` 不阻塞，返回 `TryLockError::WouldBlock`/`Poisoned`
- `MutexGuard` **不是 `Send`**：不能把 guard 移到别的线程
- `Mutex<T>: Sync` 需要 `T: Send`；`Mutex<T>: Send` 需要 `T: Send`
- 不可重入：同一线程重复 `lock()` 会死锁
- 底层是 OS 原语（Linux futex、Windows SRWLock），无争用时走原子快路径

### 33.4 `RwLock<T>`：读写锁

```rust
let lock = RwLock::new(vec![1, 2, 3]);
let sum: i32 = lock.read().unwrap().iter().sum();
*lock.write().unwrap() = vec![4, 5];
```

- 多个读者可以**同时**持有读锁
- 写者独占：没有任何读者/写者时才能拿到写锁
- `try_read`/`try_write` 非阻塞
- 读锁/写锁同样会中毒
- 具体调度策略（读者优先/写者优先）**依赖平台**，不要假设
- 不支持读锁升级为写锁；递归读锁可能死锁
- 读多写少才可能比 `Mutex` 快；写多时 overhead 更大

### 33.5 锁中毒（poisoning）

如果线程在**持有 guard 时 panic**，锁会被标记为 poisoned：

```rust
let mutex = Arc::new(Mutex::new(6));
// 线程里 lock 后 panic
match mutex.lock() {
    Ok(guard) => ...,
    Err(error) => {
        let guard = error.into_inner();   // 仍可取出数据
        // 检查数据是否一致
    }
}
```

- 中毒是**建议性**的：数据不一定损坏，但可能处于半更新状态
- 处理策略：
  1. 传播错误（让上层决定）
  2. `into_inner()` 检查不变量后继续
  3. `clear_poison()` 清除标记（确认数据可用）
  4. 直接 abort（数据关键）
- `panic = "abort"` 下没有 unwinding，也就不会中毒

### 33.6 死锁与锁顺序

经典死锁：

```
线程 A: lock(m1) -> lock(m2)
线程 B: lock(m2) -> lock(m1)
```

避免方法：

- **固定全局锁顺序**（按地址/ID/字段顺序）
- 锁作用域尽量小，不持锁做 IO/回调/await
- 用 `try_lock` + 退避，避免无限等待
- 同一把锁不要递归获取
- 需要超时用 `parking_lot::Mutex::try_lock_for`（std 没有超时 lock）
- 用 `cargo deadlock`/`loom` 等工具辅助（进阶）

### 33.7 性能与替代

- **计数器优先原子类型**（`AtomicUsize::fetch_add`，第 35 课），比 `Mutex` 轻
- 读多写少：`RwLock`；写多：`Mutex`
- 分片锁（sharded locks）降低争用
- `parking_lot`：更快、无 poisoning、支持超时
- `dashmap`：并发哈希表
- 异步：`tokio::sync::Mutex`（可跨 await），std `Mutex` 的 guard 不是 `Send`

### 33.8 async 中的锁

```rust
// 错误：guard 跨 await，Future 不是 Send
let guard = mutex.lock().unwrap();
tokio::time::sleep(...).await;
drop(guard);
```

- std `MutexGuard` 不是 `Send`，跨 `.await` 会让 Future 不满足 `Send`
- 异步共享状态用 `tokio::sync::Mutex`，或把锁作用域缩到 await 之前
- 不要用 std `Mutex` 保护需要 `.await` 的临界区

## 二、具体语法

### 34.1 `Arc`

```rust
let base = Arc::new(1);
let clone = Arc::clone(&base);

Arc::strong_count(&base);      // 2
Arc::ptr_eq(&base, &clone);    // true
drop(clone);
Arc::strong_count(&base);      // 1
```

- 用 `Arc::clone(&x)` 明确表示"加引用"
- `Arc::try_unwrap` 只在最后一个引用时成功
- `Arc::make_mut` 用于写时复制

### 34.2 `Arc<Mutex<T>>`

```rust
let counter = Arc::new(Mutex::new(0));
let mut handles = Vec::new();
for _ in 0..2 {
    let counter = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        for _ in 0..1000 {
            let mut guard = counter.lock().unwrap();
            *guard += 1;
        }
    }));
}
for handle in handles {
    handle.join().unwrap();
}
```

- 每个线程 `Arc::clone`，把克隆移入闭包
- 锁作用域尽量小：`*counter.lock().unwrap() += 1;` 一行也可以
- 保留一个 `Arc` 给主线程读结果

### 34.3 中毒处理

```rust
let lock_result = mutex.lock();
match lock_result {
    Ok(guard) => *guard,
    Err(error) => {
        let guard = error.into_inner();
        *guard
    }
}
```

- `into_inner()` 返回 guard，数据仍可读/写
- 继续之前要确认不变量是否恢复
- `clear_poison()` 清除标记（确认安全时）

### 34.4 `try_lock`

```rust
let guard = mutex.lock().unwrap();
let blocked = mutex.try_lock().is_err();
```

- 非阻塞；适合"拿不到就做别的事"
- 不要在热循环里空转 `try_lock`；加退避或改用阻塞

### 34.5 `RwLock`

```rust
let values = RwLock::new(vec![1, 2, 3]);
let sum: i32 = values.read().unwrap().iter().sum();
*values.write().unwrap() = vec![4, 5];
```

- 读锁共享，写锁独占
- 持有读锁时不要再拿写锁（同线程死锁）
- 写锁内不要做耗时操作，避免读者饥饿

### 34.6 `Arc::make_mut`

```rust
let mut first = Arc::new(vec![1]);
let second = Arc::clone(&first);
Arc::make_mut(&mut first).push(2);
// first = [1, 2], second = [1]
```

- 只有唯一引用时原地修改；否则先 clone
- 适合"读多写少"的配置/快照

### 34.7 锁顺序

```rust
// 固定顺序：先 id 小的，再 id 大的
let (first, second) = if left.id < right.id {
    (&left.balance, &right.balance)
} else {
    (&right.balance, &left.balance)
};
let mut a = first.lock().unwrap();
let mut b = second.lock().unwrap();
```

- 所有路径使用同一顺序
- 文档里写清楚锁层级
- 尽量用一把锁保护一个聚合，减少多锁

## 三、实际应用与最佳实践

### 35.1 共享计数器

```rust
let counter = Arc::new(Mutex::new(0));
// 多线程 += 1
```

- 简单场景用 `Mutex` 足够
- 高频计数用 `AtomicUsize`（下一课），避免锁争用
- 计数与业务状态需要一致时，放在同一个锁里

### 35.2 共享缓存

```rust
let cache = Arc::new(Mutex::new(BTreeMap::new()));
```

- 整个 map 一把锁，简单但争用高
- 读多写少可用 `RwLock<BTreeMap>` 或分片
- 高并发缓存考虑 `dashmap`/`moka`

### 35.3 读写配置

```rust
let config = RwLock::new(8080u16);
// 写线程更新，读线程读取
```

- 配置读多写少，`RwLock` 合适
- 更新时尽量整体替换，避免读者看到半更新状态
- 读锁作用域只包住读取，不要跨 IO

### 35.4 最佳实践清单

1. **锁作用域最小化**，guard 用完立刻 drop
2. **不要持锁做 IO/回调/await**
3. **固定锁顺序**，避免死锁
4. **同一把锁不要递归**
5. **计数器优先原子类型**
6. **读多写少用 `RwLock`，写多用 `Mutex`**
7. **处理 poisoning**，不要无脑 `unwrap`
8. **`Arc::clone` 而不是移动原 Arc**
9. **异步用 `tokio::sync::Mutex`**
10. **高争用考虑分片/parking_lot/dashmap**

## 常见陷阱

1. **guard 持有太久**：阻塞其他线程；缩小作用域
2. **guard 跨 `.await`**：Future 不是 `Send`；用异步锁或提前 drop
3. **两把锁顺序相反**：死锁；统一顺序
4. **同一线程重复 lock**：std `Mutex` 不可重入，死锁
5. **`MutexGuard` 跨线程（E0277）**：guard 不是 `Send`
6. **`lock().unwrap()` 遇 poisoning panic**：用 `match`/`into_inner`
7. **忽略 poisoning 继续用数据**：可能读到半更新状态
8. **持读锁再拿写锁**：同线程死锁
9. **以为 `RwLock` 一定更快**：写多/短临界区可能更慢
10. **`Arc` 与 `Mutex` 混淆**：`Arc` 只共享所有权，不提供可变性
11. **`Arc::clone` 当深拷贝**：共享同一份数据
12. **`strong_count` 当同步**：并发下只是近似
13. **用 `Mutex` 做高频计数**：原子类型更合适
14. **锁内 panic**：导致 poisoning；关键区避免 panic
15. **`try_lock` 忙等**：CPU 空转；退避或阻塞
16. **std `Mutex` 想要超时**：没有；用 `parking_lot`
17. **`Mutex<Rc<T>>` 跨线程（E0277）**：`Rc` 不是 `Send`
18. **`RwLock` 递归读**：可能死锁；不要嵌套
19. **锁保护范围过大**：把无关字段也串行化
20. **忘记 join**：主线程退出，worker 被终止

## 完整示例

见 `33_mutex_rwlock_arc.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "CONCURRENCY/33_共享状态Mutex_RwLock_Arc/33_mutex_rwlock_arc.rs" -o run.exe
./run.exe
```

示例共 12 行输出，覆盖：`Arc` 计数/`ptr_eq`、`Arc<Mutex>` 并发计数、
锁中毒与 `into_inner` 恢复、`try_lock`、`Arc<Mutex<Vec>>` 共享日志、
`RwLock` 并发读、`Arc::make_mut` 写时复制、共享缓存与配置读写。

## 练习

1. 用 `Arc<Mutex<i32>>` 开 4 个线程各加 1000，验证最终值。
2. 制造锁中毒：线程持锁 panic，主线程用 `into_inner` 恢复并检查数据。
3. 用 `try_lock` 实现"拿不到锁就跳过"的任务。
4. 用 `RwLock<Vec<i32>>` 实现多读者并发求和、单写者追加。
5. 用 `Arc::make_mut` 实现配置快照的写时复制。
6. 用 `Arc<Mutex<BTreeMap>>` 做多线程词频统计。
7. 写一段两把锁反序获取的代码，说明死锁条件并改成固定顺序。
8. 解释 `MutexGuard` 为什么不是 `Send`，以及 async 中应该用什么锁。

## 参考

- The Rust Programming Language: 第 16.3 节「使用共享状态并发」
- std 文档：[`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)、
  [`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)、
  [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- The Rustonomicon: [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- 生态：`parking_lot`、`dashmap`、`tokio::sync::Mutex`
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
