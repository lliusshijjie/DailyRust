# 第 21 课 · 智能指针：Box、Rc、RefCell

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`21_smart_pointers.rs`

## 一、底层原理

### 21.1 智能指针是"拥有所有权的指针"

普通引用 `&T` 只借用，不拥有；智能指针则**拥有**数据，并在 `Drop` 时负责释放：

| 类型 | 所有权 | 可变性 | 线程安全 | 大小 | 典型用途 |
|---|---|---|---|---|---|
| `Box<T>` | 独占 | `DerefMut` | `Send`/`Sync` 取决于 T | 1 个字 | 递归类型、大对象、trait 对象 |
| `Rc<T>` | 共享（引用计数） | 共享只读 | **不是** `Send`/`Sync` | 1 个字 | 单线程共享只读数据 |
| `Arc<T>` | 共享（原子计数） | 共享只读 | `Send`/`Sync` 取决于 T | 1 个字 | 跨线程共享 |
| `RefCell<T>` | 独占（内含可变性） | 运行期借用检查 | **不是** `Sync` | T + 借用标志 | 单线程内部可变性 |
| `Cell<T>` | 独占（内含可变性） | `get`/`set`（要求 `Copy`） | **不是** `Sync` | 同 T | 计数器、标志位 |

它们都实现了 `Deref`（`Cell` 除外），所以能像普通值一样调用方法。
`Box`/`Rc`/`Arc` 的所有权转移只是移动一个指针，克隆 `Rc`/`Arc` 只是计数 +1。

### 21.2 `Box<T>`：独占堆分配

```rust
let mut boxed = Box::new(5);
*boxed += 1;                  // DerefMut
```

- `Box::new(value)` 把值放到堆上，栈上只留一个指针
- 单所有者：`Box` 移动后原变量失效；drop 时释放堆内存
- 递归类型必须用 `Box`（或 `Rc`）打断无限大小：

```rust
enum List { Cons(i32, Box<List>), Nil }   // 没有 Box 会 E0072
```

- `Box<[T]>` 是"拥有所有权的切片"（胖指针），`Box<str>` 同理
- `Box<dyn Trait>` 是 trait 对象的拥有形式（第 16 课）
- `Box::leak` 返回 `&'static mut T`，主动泄漏；`Box::into_raw`/`from_raw` 是 unsafe 的
  FFI 工具

### 21.3 `Rc<T>`：单线程引用计数

`Rc<T>` 在堆上分配一个"控制块 + 值"：

```
Rc 指针 ──▶ ┌──────────────┐
            │ strong: 2    │
            │ weak: 0      │
            │ value: T     │
            └──────────────┘
```

- `Rc::clone` 只把 `strong` 加 1，O(1)，不复制值
- `strong` 归零时值被 drop；`weak` 归零时控制块被释放
- `Rc::strong_count`/`weak_count`/`ptr_eq`
- `Rc::try_unwrap`：只有最后一个强引用时才能取回 `T`
- `Rc::get_mut`：唯一所有者时拿到 `&mut T`
- `Rc::make_mut`：写时复制（有多个引用时先克隆）
- **`Rc` 不是 `Send`/`Sync`**：不能跨线程，编译器会报 E0277

### 21.4 `RefCell<T>`：运行期借用检查

`RefCell` 把借用规则从编译期搬到运行期：

```rust
let cell = RefCell::new(1);
{
    let mut value = cell.borrow_mut();   // 运行期登记"可变借用"
    *value += 1;
}                                        // 守卫 drop，释放登记
let value = cell.borrow();               // 共享借用
```

- `borrow()` 返回 `Ref<T>`，`borrow_mut()` 返回 `RefMut<T>`；守卫 drop 时归还
- 编译期允许多个 `borrow`；`borrow_mut` 要求当前没有其他借用，否则 **panic**
- `try_borrow`/`try_borrow_mut` 返回 `Result`，冲突时给 `Err` 而不是 panic
- `RefCell` 不是 `Sync`：不能跨线程共享
- 代价是一次运行期借用标志读写；没有 vtable/堆分配

### 21.5 `Cell<T>`：按值进出的内部可变性

```rust
let cell = Cell::new(10);
cell.set(cell.get() + 1);      // 要求 T: Copy
```

- `get` 复制出值，`set` 写回；`replace` 返回旧值；`take` 用 `Default` 换出
- 不能从 `Cell` 里拿引用（否则会破坏规则）；所以只适合 `Copy` 小值或
  `Option<T>`/`Vec<T>` 这种能整体换进换出的类型
- `Cell` 不是 `Sync`；单线程用

### 21.6 `Arc<T>`：跨线程引用计数

`Arc` 与 `Rc` 语义相同，但引用计数用**原子操作**，因此是 `Send + Sync`（当 `T` 满足时）：

```rust
let shared = Arc::new(Mutex::new(0));
let clone = Arc::clone(&shared);
```

- 原子操作比普通计数贵；单线程优先 `Rc`
- `Arc<T>` 只提供共享只读；要修改内部值需要 `Mutex`/`RwLock`/原子类型
  （第 33、35 课）
- `Arc<Mutex<T>>` 是跨线程共享可变状态的经典组合

### 21.7 组合模式

| 组合 | 语义 |
|---|---|
| `Rc<Vec<T>>` | 多个所有者共享只读数据 |
| `Rc<RefCell<T>>` | 多个所有者共享**可变**数据（单线程） |
| `Rc<RefCell<Vec<T>>>` | 共享可变集合（待办、图） |
| `Arc<Mutex<T>>` | 跨线程共享可变状态 |
| `Rc<Weak<T>>` | 反向引用，打破循环 |
| `Box<dyn Trait>` | 拥有 trait 对象 |

### 21.8 循环引用与 `Weak`

`Rc` 只靠强计数决定释放。如果两个 `Rc` 互相指向对方，强计数永远不为零，值**永远不会 drop**：

```rust
struct Node { next: RefCell<Option<Rc<Node>>> }
// a.next = Some(b), b.next = Some(a) → 泄漏
```

`Weak<T>` 是"不增加强计数的引用"：

- `Rc::downgrade(&rc)` 得到 `Weak<T>`
- `weak.upgrade()` 返回 `Option<Rc<T>>`；强引用都释放后返回 `None`
- 父子树里，父持 `Rc` 子、子持 `Weak` 父，即可避免环
- `Rc::weak_count` 可观察弱引用数量

## 二、具体语法

### 22.1 `Box`

```rust
let boxed = Box::new(42);
let value = *boxed;

let mut boxed = Box::new(String::from("hi"));
boxed.push('!');

let slice: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
let object: Box<dyn std::fmt::Display> = Box::new(42);
```

- `Box::new` 分配；`Box::pin` 固定地址（第 37 课）
- `into_boxed_slice` 把 `Vec<T>` 收成 `Box<[T]>`
- `Box<dyn Trait>` 需要 trait dyn 兼容

### 22.2 `Rc`

```rust
let first = Rc::new(String::from("shared"));
let second = Rc::clone(&first);

Rc::strong_count(&first);        // 2
Rc::weak_count(&first);          // 0
Rc::ptr_eq(&first, &second);     // true

drop(second);
Rc::strong_count(&first);        // 1
```

- 用 `Rc::clone(&x)` 而不是 `x.clone()`（后者也可，但前者更明确表示"加计数"）
- `Rc::try_unwrap`/`get_mut` 只在唯一所有者时成功

### 22.3 `RefCell`

```rust
let cell = RefCell::new(vec![1]);
cell.borrow_mut().push(2);

let borrowed = cell.borrow();
let blocked = cell.try_borrow_mut().is_err();   // true
drop(borrowed);
cell.try_borrow_mut().unwrap().push(3);
```

- 守卫必须在冲突操作前 drop（或作用域结束）
- `borrow_mut()` 冲突会 panic；线上代码优先 `try_borrow_mut`
- 不要跨 `.await` 持有 `RefCell` 守卫（`Ref` 不是 `Send`）

### 22.4 `Cell`

```rust
let counter = Cell::new(0);
counter.set(counter.get() + 1);

let slot = Cell::new(Some(String::from("x")));
let taken = slot.take();            // 取出 Option，留下 None
```

- `get` 要求 `Copy`；非 `Copy` 用 `take`/`replace`
- 没有借用守卫，开销更低，但能力也更弱

### 22.5 `Arc`

```rust
let shared = Arc::new(vec![1, 2, 3]);
let clone = Arc::clone(&shared);

let counter = Arc::new(std::sync::Mutex::new(0));
```

- 跨线程共享只读用 `Arc<T>`；共享可选用 `Arc<Mutex<T>>`
- `Arc::strong_count` 是原子读，但并发下只是近似值

### 22.6 `Weak`

```rust
let strong = Rc::new(1);
let weak = Rc::downgrade(&strong);
weak.upgrade();          // Some(Rc)
drop(strong);
weak.upgrade();          // None
```

- `Weak` 不阻止值 drop；升级失败返回 `None`
- 适合父指针、观察者、缓存索引

### 22.7 组合

```rust
let todo = Rc::new(RefCell::new(Vec::new()));
let handle = Rc::clone(&todo);
handle.borrow_mut().push(String::from("task"));
```

- 多个句柄共享同一个 `Vec`，修改通过 `borrow_mut`
- 读的时候用 `borrow()`；不要在持有守卫时回调可能再次借用的代码

## 三、实际应用与最佳实践

### 23.1 递归列表：`Box`

```rust
let list = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));
sum_list(&list);      // 3
```

`Box` 把递归变成"指针指向下一个节点"，大小固定。

### 23.2 共享可变待办：`Rc<RefCell<Vec<T>>>`

```rust
let todo = Rc::new(RefCell::new(Vec::new()));
let handle = Rc::clone(&todo);
handle.borrow_mut().push(String::from("write lesson"));
```

多个模块/回调持有同一个待办列表；注意每次操作只持有短暂守卫。

### 23.3 父子树：`Rc` + `Weak`

```rust
struct Node {
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,
}
```

父 → 子用强引用，子 → 父用弱引用；这样树整体 drop 时不会形成环。
`weak_count` 可以观察反向引用数量。

### 23.4 最佳实践清单

1. **默认用 `Box` 表达独占堆数据**，递归类型/Trait 对象优先
2. **单线程共享只读用 `Rc`**，跨线程用 `Arc`
3. **需要内部可变性时**：单线程 `RefCell`/`Cell`，多线程 `Mutex`/`RwLock`/原子
4. **`Rc<RefCell<T>>` 是单线程共享可变的最后手段**，能不用就不用
5. **避免 `Rc` 循环**：反向引用一律 `Weak`
6. **`RefCell` 守卫尽量短**，不要跨回调/`.await` 持有
7. **线上代码用 `try_borrow_mut`** 而不是 `borrow_mut`，把 panic 变成可控错误
8. **`Rc::clone` 是浅拷贝**，修改共享数据前想清楚所有权
9. **`Arc` 有原子开销**，单线程不要用
10. **能用借用就不用智能指针**：`&T`/`&mut T` 优先，智能指针是所有权方案

## 常见陷阱

1. **`Rc` 循环泄漏**：两个 `Rc` 互指，drop 永不发生；用 `Weak` 打断环
2. **`RefCell` 重复 `borrow_mut` panic**：运行期 panic，不是编译错误；
   用 `try_borrow_mut` 或缩短守卫作用域
3. **持有 `borrow` 时 `borrow_mut`**：即使 NLL 也救不了（运行期检查）；
   显式 `drop(guard)`
4. **`RefCell` 守卫跨 `.await`**：`Ref` 不是 `Send`，异步代码会编译失败
5. **`Rc` 跨线程（E0277）**：`Rc<i32>` 不满足 `Send`；改用 `Arc`
6. **`Rc::try_unwrap` 失败**：还有别的强引用；先 `drop` 或改 `Arc<Mutex>`
7. **`Rc::get_mut` 返回 None**：存在其他句柄；先确保唯一
8. **把 `Rc::clone` 当深拷贝**：修改会影响所有持有者
9. **`Cell::get` 要求 `Copy`**：`Cell<String>` 不能 `get`；用 `take`/`replace`
10. **递归类型忘记 `Box`（E0072）**：`enum List { Cons(i32, List) }` 无限大小
11. **`Box<dyn Trait>` 的 trait 不 dyn 兼容**：见第 16 课
12. **`RefCell<Vec<T>>` 迭代时修改**：`for x in cell.borrow().iter() { cell.borrow_mut()... }` panic
13. **`Arc<Mutex<T>>` 忘记 `Arc`**：`Mutex` 本身不能跨线程共享所有权
14. **`Rc` 的 `strong_count` 当同步原语**：它只用于调试/观察，不是锁
15. **`Box::leak` 当释放**：它主动泄漏内存，只在需要 `'static` 时使用
16. **`RefCell` 以为零成本**：每次借用都有运行期标志读写
17. **`Weak` 升级后忘记判空**：值可能已经被释放
18. **`Rc<RefCell<T>>` 在热路径频繁借用**：改用 `Cell`/重构数据流

## 完整示例

见 `21_smart_pointers.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/21_智能指针BoxRcRefCell/21_smart_pointers.rs" -o run.exe
./run.exe
```

示例共 16 行输出，覆盖：`Box`/`Rc` 指针大小、`Box` 堆值修改、`Rc` 计数与
`ptr_eq`、`RefCell` 运行期借用冲突、`Rc<RefCell<T>>` 共享可变、`Box` 递归列表、
`Cell`、`try_borrow`、`Rc::try_unwrap`、`Weak` 升级，最后用共享待办和
`Weak` 父子树收尾。

## 练习

1. 用 `Box` 实现单链表 `enum List { Cons(i32, Box<List>), Nil }`，
   写 `sum`/`len`/`push_front`。
2. 用 `Rc<String>` 共享配置，克隆 3 份后打印 `strong_count`，逐个 drop 观察计数。
3. 用 `RefCell<i32>` 实现计数器，分别用 `borrow_mut` 和 `try_borrow_mut` 修改。
4. 用 `Rc<RefCell<Vec<String>>>` 实现待办列表，两个句柄各自添加一项。
5. 实现父子树：父 `Rc` 子、子 `Weak` 父；drop 根后验证 `weak.upgrade()` 为 `None`。
6. 构造一个 `Rc` 循环引用，观察 `Drop` 不执行；用 `Weak` 修复并验证释放。
7. 用 `Cell<bool>` 实现"是否已初始化"标志（`OnceCell` 的手写简化版）。
8. 比较 `Box`、`Rc`、`Arc`、`RefCell`、`Mutex` 的所有权/可变性/线程安全/开销。

## 参考

- The Rust Programming Language: 第 15 章「智能指针」
  （15.1 `Box`、15.4 `Rc`、15.5 `RefCell`、15.6 循环引用）
- std 文档：[`Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)、
  [`Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html)、
  [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html)、
  [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html)、
  [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)、
  [`Weak`](https://doc.rust-lang.org/std/rc/struct.Weak.html)
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)、
  [E0072](https://doc.rust-lang.org/error_codes/E0072.html)
