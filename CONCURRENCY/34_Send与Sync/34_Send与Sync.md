# 第 34 课 · Send 与 Sync

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`34_send_and_sync.rs`

## 一、底层原理

### 34.1 两个标记 trait 的定义

```rust
pub unsafe auto trait Send {}
pub unsafe auto trait Sync {}
```

- `Send`：类型的**所有权**可以转移到另一个线程
- `Sync`：类型的**共享引用** `&T` 可以出现在多个线程中
- 两者都是 **auto trait**：没有方法，编译器根据字段自动推导
- 两者都是 **unsafe trait**：手写 `unsafe impl` 要自己保证线程安全

一个等价关系：

```text
T: Sync  ⇔  &T: Send
```

也就是说，`T` 能被多个线程同时"读"，当且仅当把 `&T` 传给另一个线程是安全的。

### 34.2 组合规则

- **结构体/枚举**：所有字段都 `Send` 才 `Send`；所有字段都 `Sync` 才 `Sync`
- **引用**：`&T: Send` 当且仅当 `T: Sync`；`&mut T: Send` 当且仅当 `T: Send`
- **容器**：`Box<T>`/`Vec<T>`/`Option<T>` 随 `T`
- **`PhantomData<T>`**：自动 trait 跟随 `T`（哪怕它是零大小）
- **裸指针** `*const T`/`*mut T`：既不是 `Send` 也不是 `Sync`
- **闭包**：由捕获的每个变量共同决定

常见类型矩阵：

| 类型 | `Send` | `Sync` | 说明 |
|---|---|---|---|
| `i32`/`String` | ✅ | ✅ | 普通值 |
| `Rc<T>` | ❌ | ❌ | 引用计数非原子 |
| `Cell<T>` | T: Send | ❌ | 内部可变性非同步 |
| `RefCell<T>` | T: Send | ❌ | 运行期借用检查非同步 |
| `Mutex<T>` | T: Send | T: Send | 互斥提供同步 |
| `RwLock<T>` | T: Send | T: Send + Sync | 读者共享 `&T` |
| `Arc<T>` | T: Send + Sync | T: Send + Sync | 原子引用计数 |
| `MutexGuard<'_, T>` | ❌ | — | 不能跨线程转移 |
| `*const T`/`*mut T` | ❌ | ❌ | 裸指针无安全保证 |
| `Box<dyn Trait>` | ❌ | ❌ | 默认不含 auto trait |
| `Box<dyn Trait + Send + Sync>` | ✅ | ✅ | 显式加上边界 |

### 34.3 为什么 `Rc`/`Cell`/`RefCell` 不行

- `Rc<T>` 的强/弱计数是普通整数，不是原子操作；两个线程同时 clone/drop 会数据竞争
- `Cell<T>` 允许通过 `&Cell<T>` 修改值，没有同步；多个线程同时写会竞争
- `RefCell<T>` 的借用标志是普通整数，多个线程同时 `borrow_mut` 会破坏规则
- 解决：
  - `Rc` → `Arc`
  - `Cell`/`RefCell` → `Mutex`/`RwLock`/原子类型
  - 单线程内部用 `Rc<RefCell<T>>`；跨线程用 `Arc<Mutex<T>>`

### 34.4 闭包与 Future 的 `Send`

- 闭包的 `Send`/`Sync` 由**捕获值**决定；`move` 本身不会让闭包变成 `Send`
- `thread::spawn` 要求 `F: Send + 'static`、`T: Send + 'static`
- `thread::scope` 的闭包不要求 `'static`，但仍要求 `Send`
- `async` 块生成的 Future：如果某个**跨 `.await` 持有的值**不是 `Send`，Future 就不是 `Send`
- 典型事故：`std::sync::MutexGuard` 跨 `.await` → Future 不满足 `Send`，
  `tokio::spawn` 编译失败；改用 `tokio::sync::Mutex` 或缩短 guard 作用域

### 34.5 trait 对象

`Box<dyn Trait>` 默认**不**实现 `Send`/`Sync`，因为 trait 对象擦除了具体类型：

```rust
let boxed: Box<dyn Work> = Box::new(1);
// Box<dyn Work> 不是 Send

let boxed: Box<dyn Work + Send + Sync> = Box::new(1);
// 显式加上 auto trait 边界后才可以跨线程
```

- 错误场景：`Box<dyn Error>` 跨线程 → 需要 `Box<dyn Error + Send + Sync>`
- 库 API 返回 trait 对象时，按需要加 `+ Send + Sync`
- auto trait 边界是 trait 对象类型的一部分，不能事后"补"

### 34.6 `unsafe impl` 的契约

手写实现只应出现在**自己定义**的类型上，并且必须保证：

```rust
struct MyBox<T> {
    ptr: *mut T,
    _marker: PhantomData<T>,   // 表达"拥有 T"
}

unsafe impl<T: Send> Send for MyBox<T> {}
unsafe impl<T: Sync> Sync for MyBox<T> {}
```

安全前提（示例）：

- `MyBox` 独占 `ptr` 指向的对象，没有别名
- `Drop` 正确释放，不 double free
- 不把线程本地状态（如线程 ID、非原子计数）暴露给其他线程
- `PhantomData<T>` 让自动 trait/所有权语义正确

`unsafe impl` 不是"绕过编译器"的万能钥匙；它把证明责任交给了你。
如果只是想让类型**不**实现 `Send`/`Sync`，不要手写负 impl（`impl !Send` 仍不稳定），
用 `PhantomData<Rc<()>>` 或 `PhantomData<*const ()>` 标记即可。

### 34.7 编译期断言

```rust
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

assert_send::<Arc<Mutex<i32>>>();
assert_sync::<Arc<Mutex<i32>>>();
```

- 这是"编译期测试"：不满足就编译失败
- 反向断言可以写另一个函数并配合 `trybuild`/`static_assertions`
- `static_assertions` crate 提供 `assert_impl_all!`/`assert_not_impl_any!`
- 泛型代码里用 `where T: Send + Sync` 把要求写进签名，让调用方尽早发现

## 二、具体语法

### 35.1 编译期断言

```rust
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

assert_send::<i32>();
assert_sync::<i32>();
```

- 放在测试/启动代码里，作为 API 契约的一部分
- 泛型结构体可以写 `assert_send::<Cache<i32>>()` 验证推导

### 35.2 组合与推导

```rust
struct Composite {
    value: i32,
    shared: Arc<Mutex<i32>>,
}

assert_send::<Composite>();
assert_sync::<Composite>();
```

- 所有字段都满足，结构体才自动满足
- 加一个 `Rc` 字段，整个结构体就不再 `Send`

### 35.3 `PhantomData` 控制自动 trait

```rust
struct LocalOnly {
    value: i32,
    _marker: PhantomData<Rc<()>>,
}
```

- `PhantomData<Rc<()>>` 零大小，但让 `LocalOnly` 不实现 `Send`/`Sync`
- `PhantomData<*const ()>` 同样可以
- `PhantomData<fn() -> T>` 则始终 `Send + Sync`（函数指针）
- 适合"逻辑上拥有 T，但物理上不存储 T"的包装类型

### 35.4 `unsafe impl`

```rust
unsafe impl<T: Send> Send for MyBox<T> {}
unsafe impl<T: Sync> Sync for MyBox<T> {}
```

- 只对自己定义的类型实现
- 必须写清楚安全性证明（注释/文档）
- 优先用安全抽象（`Mutex`/`Arc`/原子）而不是手写

### 35.5 闭包与 API 边界

```rust
fn run_worker<F>(worker: F) -> &'static str
where
    F: FnOnce() -> &'static str + Send + 'static,
{
    worker()
}
```

- 库函数把 `Send` 要求写进 `where`，调用方编译期就能发现
- `move` 只是转移捕获，不改变 `Send`
- 捕获 `Rc` 的闭包仍然不是 `Send`

### 35.6 trait 对象

```rust
let boxed: Box<dyn Work + Send + Sync> = Box::new(1);
```

- 默认 `Box<dyn Trait>` 不是 `Send`
- 需要跨线程时显式加 `+ Send + Sync`
- 常见：`Box<dyn Error + Send + Sync>`

## 三、实际应用与最佳实践

### 36.1 泛型线程安全缓存

```rust
struct Cache<T> {
    map: Mutex<BTreeMap<String, T>>,
}

let cache = Arc::new(Cache::new());
let worker_cache = Arc::clone(&cache);
let worker = thread::spawn(move || {
    worker_cache.put("a", 1);
});
```

- `Cache<i32>` 自动 `Send + Sync`（`T: Send`）
- `Arc` 提供共享所有权，`Mutex` 提供互斥
- 用编译期断言确认 `Cache<i32>: Send + Sync`

### 36.2 API 设计的边界

- 需要跨线程的 API：`T: Send + 'static` 或 `T: Send + Sync`
- 只在单线程使用的类型：不要加多余的 `Send` 约束
- 返回 trait 对象：按需要 `+ Send + Sync`
- 异步 API：要求 `Future: Send` 时，避免在 `.await` 期间持有非 `Send` 值

### 36.3 最佳实践清单

1. **优先用 `Arc`/`Mutex`/原子**，少手写 `unsafe impl`
2. **`Send` 是转移、`Sync` 是共享引用**，不要混用
3. **闭包的 `Send` 取决于捕获值**，`move` 不解决
4. **跨线程 trait 对象加 `+ Send + Sync`**
5. **泛型 API 把 `Send + Sync` 写进约束**
6. **用编译期断言锁住类型契约**
7. **`PhantomData` 表达所有权/选择退出**
8. **异步注意跨 `.await` 的值**
9. **`Rc<RefCell>` 单线程，`Arc<Mutex>` 跨线程**
10. **手写 `unsafe impl` 必须附安全性证明**

## 常见陷阱

1. **`Rc` 跨线程（E0277）**：换 `Arc`
2. **`RefCell`/`Cell` 跨线程共享（E0277）**：换 `Mutex`/`RwLock`
3. **`Cell` 是 `Send` 但不是 `Sync`**：能 move 到线程，不能 `&Cell` 共享
4. **`MutexGuard` 跨线程（E0277）**：guard 不是 `Send`
5. **`Box<dyn Trait>` 默认不是 `Send`**：加 `+ Send + Sync`
6. **`Arc<T>` 需要 `T: Send + Sync`**：不是有 `Arc` 就线程安全
7. **`RwLock<T>` 的 `Sync` 需要 `T: Send + Sync`**：读者会拿到 `&T`
8. **把 `Send` 当成 `Sync`**：`MutexGuard` 是反例
9. **以为 `move` 让闭包 `Send`**：捕获 `Rc` 仍然不行
10. **Future 跨 `.await` 持非 `Send` 值**：`tokio::spawn` 编译失败
11. **`unsafe impl Send` 没有证明**：把数据竞争引入安全代码
12. **`PhantomData` 方向搞反**：本意退出却写成了 `PhantomData<T>` 的镜像
13. **`&T: Send` 误判**：需要 `T: Sync`
14. **`thread::scope` 闭包不需要 `'static` 但需要 `Send`**
15. **`mpsc::Receiver` 不是 `Sync`**：只能一个消费者
16. **`JoinHandle<T>` 的 `Send` 需要 `T: Send`**
17. **负 impl 语法**：`impl !Send` 仍不稳定；用 PhantomData
18. **`Rc` 藏在结构体字段里**：整个结构体失去 `Send`
19. **trait 对象的 auto trait 被擦除**：`Box<dyn Error>` 不能跨线程
20. **多线程下用线程本地 ID 做 key**：`Send` 不保证逻辑正确

## 完整示例

见 `34_send_and_sync.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "CONCURRENCY/34_Send与Sync/34_send_and_sync.rs" -o run.exe
./run.exe
```

示例共 12 行输出，覆盖：基础类型/`Arc<Mutex>`/组合结构体的编译期断言、
`PhantomData<Rc<()>>` 的零大小与自动 trait 影响、泛型 `Cache<i32>` 的
`Send + Sync` 推导、闭包 `Send` 约束，以及跨线程缓存。

## 练习

1. 用 `assert_send`/`assert_sync` 检查 `i32`、`String`、`Vec<String>`、`Arc<Mutex<i32>>`。
2. 给一个结构体加 `Rc` 字段，验证它不再 `Send`；改成 `Arc` 后恢复。
3. 验证 `Cell<i32>` 是 `Send` 但不是 `Sync`；解释原因。
4. 用 `PhantomData<Rc<()>>` 让一个类型不实现 `Send`，并写编译失败用例。
5. 写一个 `Box<dyn Trait + Send + Sync>` 的跨线程示例。
6. 解释为什么 `MutexGuard` 不是 `Send`，以及为什么 `RwLock<T>: Sync` 需要 `T: Sync`。
7. 写一个手写 `unsafe impl Send` 的包装类型，并说明安全性前提（只写代码）。
8. 用 `static_assertions` 的写法重写编译期断言（只写代码，不要求本机编译）。

## 参考

- The Rust Programming Language: 第 16.4 节「使用 Send 和 Sync 的可扩展并发」
- The Rustonomicon: [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- std 文档：[`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html)、
  [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- crate：`static_assertions`
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
