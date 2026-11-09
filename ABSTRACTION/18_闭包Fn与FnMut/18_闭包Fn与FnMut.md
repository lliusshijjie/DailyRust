# 第 18 课 · 闭包：Fn / FnMut / FnOnce

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`18_closures_and_fn_traits.rs`

## 一、底层原理

### 18.1 闭包是"匿名结构体 + 调用 trait"

闭包不是函数指针的语法糖。编译器会为每个闭包表达式生成一个**独一无二的匿名类型**，
把捕获的变量存成字段，并实现相应的调用 trait：

```rust
let base = 40;
let read = || base + 2;
// 约等于：
struct ReadClosure<'a> { base: &'a i32 }
impl Fn<()> for ReadClosure<'_> { ... }
```

由此可以推出：

- 每个闭包表达式的类型都不同，**无法手写命名**；只能用泛型、`impl Fn`、
  `Box<dyn Fn>`、`&dyn Fn` 来传递
- 捕获的变量就是闭包对象的字段：闭包大小 ≈ 捕获值大小（无捕获 = 零大小）
- 调用 `f()` 展开成 `Fn::call(&f, ())` / `FnMut::call_mut(&mut f, ())` /
  `FnOnce::call_once(f, ())`
- 泛型 + 单态化让闭包调用可以内联；只有 `Box<dyn Fn>` 才有分配和 vtable 开销

闭包捕获变量的三种方式由**使用方式**决定：

| 闭包体内怎么用 | 捕获方式 | 实现 trait |
|---|---|---|
| 只读 | `&T` | `Fn` + `FnMut` + `FnOnce` |
| 修改 | `&mut T` | `FnMut` + `FnOnce` |
| 移动/消耗 | `T` | `FnOnce` |
| 加 `move` | 强制按值 | 取决于体内用法 |

### 18.2 Fn / FnMut / FnOnce 的能力层级

三个 trait 是**超 trait 链**：

```
Fn: FnMut: FnOnce
```

- 任何 `Fn` 都可以当 `FnMut`/`FnOnce` 用
- 任何 `FnMut` 都可以当 `FnOnce` 用
- 反过来不行

| trait | 调用方法接收者 | 能调用几次 | 典型闭包 |
|---|---|---|---|
| `Fn` | `&self` | 任意次 | 只读捕获 |
| `FnMut` | `&mut self` | 任意次（需要 `&mut`） | 修改捕获 |
| `FnOnce` | `self` | 一次 | 消耗捕获值 |

```rust
fn call_fn<F: Fn() -> i32>(f: F) -> i32 { f() }           // 只读闭包
fn call_fn_mut<F: FnMut()>(mut f: F) { f(); f(); }        // 可变闭包
fn call_fn_once<F: FnOnce() -> String>(f: F) -> String { f() }  // 消费闭包
```

`FnMut` 的调用需要**可变绑定**（`let mut f = ...`），因为 `call_mut` 要 `&mut self`；
`FnOnce` 调用会**移动闭包**，所以只能调一次。

### 18.3 `move`：把捕获值搬进闭包

```rust
let text = String::from("hi");
let borrowed = || text.len();       // 借用 text
let moved = move || text.len();     // 把 text 移进闭包
```

`move` 的语义是"**创建闭包时按值捕获**"，不是"每次调用都移动"。它解决三类问题：

1. **返回闭包**：闭包要活得比函数栈帧长，不能借用局部变量
2. **跨线程**：`thread::spawn` 要求闭包 `'static`，必须 move
3. **保存所有权**：闭包需要独立拥有捕获值（如缓存、配置）

```rust
fn make_counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || { count += 1; count }
}
```

如果返回的闭包借用局部变量，会报 E0373（"closure may outlive the current function"）；
加 `move` 即可。

捕获 `Copy` 类型的 move 闭包本身也可以是 `Copy`：

```rust
let captured = 10;
let first = move || captured;
let second = first;      // 闭包 Copy
first(); second();       // 两个都能用
```

### 18.4 不相交捕获（disjoint capture）

Rust 2021 起，闭包**按字段**捕获，而不是捕获整个结构体：

```rust
let left_len = || pair.left.len();   // 只借用 pair.left
pair.right.push('!');                // 修改 pair.right 不冲突
```

- 2021 之前：闭包捕获整个 `pair`，上面代码会冲突
- 2021 之后：只捕获用到的字段，`pair.right` 可以独立修改
- `move` 闭包也只移动用到的字段

这个变化让闭包与结构体字段的借用规则更自然，但也意味着闭包大小/生命周期
取决于具体用了哪些字段。

### 18.5 闭包的存储与传递

闭包类型不可命名，因此有三种存储方式：

| 方式 | 写法 | 特点 |
|---|---|---|
| 泛型参数 | `struct Handler<F: Fn(i32) -> i32> { f: F }` | 零成本、可内联、每个 F 一份代码 |
| `impl Trait` | `fn make() -> impl Fn() -> i32` | 静态分发、类型对调用方隐藏 |
| trait 对象 | `Box<dyn Fn(i32) -> i32>` | 异构集合、动态分发、堆分配 |

`&dyn Fn` 借用 trait 对象；`Box<dyn FnMut>` 调用时需要 `&mut`；
`Box<dyn FnOnce()>` 调用会消耗 Box，只能调一次。

### 18.6 函数指针与非捕获闭包

- 无捕获闭包可以**强制转换**为 `fn` 指针：`let f: fn(i32) -> i32 = |x| x + 1;`
- 函数项（`fn add_one`）本身实现 `Fn` 系列 trait
- `fn` 指针是 `Copy`、大小固定；带捕获的闭包大小取决于捕获值
- 带捕获的闭包**不能**转成 `fn` 指针

### 18.7 性能

- 闭包捕获内联在闭包对象里，没有堆分配
- 泛型调用经单态化后通常能内联
- `Box<dyn Fn>` 有一次堆分配 + 每次调用一次 vtable 间接
- 迭代器适配器（`map`/`filter`/`fold`）就是用闭包 + 泛型实现的，编译后等价于循环

## 二、具体语法

### 19.1 语法形式

```rust
let no_args = || 42;
let one_arg = |x| x + 1;
let two_args = |x: i32, y: i32| x + y;
let block = |x| { let doubled = x * 2; doubled + 1 };
let moved = move || text.len();
```

- 参数类型通常可省略（由调用处推断）；有歧义时写注解
- 体是表达式时用 `expr`，多语句用 `{ ... }`
- `move` 写在 `||` 之前

### 19.2 trait 约束

```rust
fn call_fn<F: Fn() -> i32>(f: F) -> i32 { f() }

fn call_fn_mut<F: FnMut()>(mut f: F) {
    f();
    f();
}

fn call_fn_once<F: FnOnce() -> String>(f: F) -> String { f() }

fn apply(value: i32, f: impl Fn(i32) -> i32) -> i32 { f(value) }
```

- 只读闭包优先用 `Fn`；需要修改状态用 `FnMut`；只调一次用 `FnOnce`
- `impl Fn` 是 APIT，等价于匿名泛型
- `Fn(i32) -> i32` 是 `Fn<(i32,), Output = i32>` 的语法糖

### 19.3 调用

```rust
let mut count = 0;
let mut increment = || { count += 1; count };
increment();       // 需要 mut
increment();

let consume = move || String::from("x");
consume();         // 之后 consume 不可用
```

- `Fn` 闭包可用 `&` 调用
- `FnMut` 闭包调用需要 `&mut`
- `FnOnce` 闭包调用后失效

### 19.4 返回闭包

```rust
fn make_adder(offset: i32) -> impl Fn(i32) -> i32 {
    move |value| value + offset
}

fn make_counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || { count += 1; count }
}
```

- 返回闭包必须 `move`（否则借用局部变量，E0373）
- 返回 `impl Fn`/`impl FnMut`/`impl FnOnce` 都可以
- 需要存进集合/结构体时用 `Box<dyn Fn>`

### 19.5 存储

```rust
struct Handler<F: Fn(i32) -> i32> {
    function: F,
}

impl<F: Fn(i32) -> i32> Handler<F> {
    fn run(&self, value: i32) -> i32 { (self.function)(value) }
}

let boxed: Box<dyn Fn(i32) -> i32> = Box::new(|x| x * 2);
```

- 泛型字段零成本；每个闭包类型一份代码
- `Box<dyn Fn>` 适合异构集合/运行时注册
- `&dyn Fn` 适合临时借用

### 19.6 函数指针

```rust
fn add_one(value: i32) -> i32 { value + 1 }

let pointer: fn(i32) -> i32 = add_one;
let closure_pointer: fn(i32) -> i32 = |value| value + 1;
```

- 无捕获闭包可以转 `fn`
- 有捕获闭包不能转 `fn`（没有地方存捕获值）

### 19.7 `FnOnce` 的消费

```rust
let text = String::from("x");
let consume = move || text;
consume();          // text 被闭包消耗

let boxed: Box<dyn FnOnce() -> String> = Box::new(move || String::from("y"));
boxed();            // Box 被消耗，只能调一次
```

- `FnOnce` 适合"最终处理器/一次性回调"
- 迭代器适配器要求 `FnMut`，不能直接传 `FnOnce`（会被多次调用）

## 三、实际应用与最佳实践

### 20.1 中间件管线

```rust
let pipeline: Vec<Box<dyn Fn(i32) -> i32>> = vec![
    Box::new(|v| v + 1),
    Box::new(|v| v * 3),
    Box::new(|v| v - 2),
];
let result = pipeline.iter().fold(5, |value, step| step(value));  // 16
```

每个中间件是 `Fn(i32) -> i32`，用 `Box<dyn Fn>` 统一存储。
如果中间件数量固定、追求零成本，可以改成泛型嵌套或枚举。

### 20.2 事件处理器列表

```rust
let handlers: Vec<Box<dyn Fn(&str) -> String>> = vec![
    Box::new(|event| format!("log:{}", event)),
    Box::new(|event| format!("metric:{}", event.len())),
];
```

闭包捕获配置、借用事件参数、返回处理结果；适合日志、指标、通知等插件式逻辑。

### 20.3 ID 生成器：`FnMut` 持有状态

```rust
let mut next_id = make_counter();
let ids: Vec<u32> = (0..3).map(|_| next_id()).collect();  // [1, 2, 3]
```

`FnMut` 闭包把自增状态放进闭包字段，调用方只需要一个可变绑定。
需要跨线程共享时改用 `Mutex`/原子类型（第 33、35 课）。

### 20.4 最佳实践清单

1. **只读捕获用 `Fn`**，需要修改状态用 `FnMut`，一次性消费用 `FnOnce`
2. **返回闭包/跨线程/存 `'static` 时用 `move`**
3. **能泛型就别 `Box<dyn Fn>`**：泛型零成本；需要异构集合再用 trait 对象
4. **`FnMut` 闭包变量要 `let mut`**
5. **`FnOnce` 只调用一次**，不要放进会被多次调用的适配器
6. **优先用迭代器适配器**而不是手写闭包循环：可读性更好，编译后等价
7. **注意闭包大小**：捕获大对象时考虑 `Box`/引用，避免拷贝
8. **跨线程闭包必须 `Send + 'static`**（`thread::spawn` 的要求）
9. **不要依赖 2018 的整结构体捕获**，2021+ 是字段级捕获
10. **闭包不是 trait 实现**：需要命名类型/方法时写成结构体 + trait

## 常见陷阱

1. **`FnMut` 闭包忘了 `mut`（E0596）**：`let f = || count += 1; f();` 需要 `let mut f`
2. **`FnOnce` 调用两次（E0382）**：第一次调用已经把闭包移走；只能调一次
3. **可变闭包当 `Fn` 用（E0594）**：修改捕获的闭包只实现 `FnMut`，不能放进 `F: Fn`
4. **返回借用局部的闭包（E0373）**：`fn make() -> impl Fn() { let s = ...; || s.len() }`；
   加 `move`
5. **`move` 后原变量不可用**：`let f = move || text;` 之后 `text` 已被移走
6. **`Box<dyn FnMut>` 调用需要 `&mut`**：`let mut f = Box::new(...)`；否则 E0596
7. **把 `FnOnce` 传给 `map`/`filter`**：这些适配器要求 `FnMut`，`FnOnce` 不满足
8. **带捕获闭包转 `fn` 指针**：只有无捕获闭包能转
9. **`impl Fn` 字段**：`struct S { f: impl Fn() }` 不合法（E0562）；用泛型或 `Box<dyn Fn>`
10. **`Box<dyn Fn>` 默认 `'static`**：借用数据要写 `Box<dyn Fn(&str) + '_>`
11. **闭包捕获整个结构体（2018 语义）**：2021+ 按字段捕获，但 `move` 仍会移动用到的字段
12. **闭包与 `Drop`**：捕获值在闭包被 drop 时释放；顺序按字段声明
13. **递归闭包**：闭包无法直接引用自身；用 `fn` 指针、`Rc<RefCell<...>>` 或结构体方法
14. **闭包类型无法命名**：不要试图写类型注解/函数返回具体闭包类型
15. **`Fn`/`FnMut` 的 trait 对象调用**：`&dyn Fn` 可直接调；`&mut dyn FnMut` 需要 `mut`
16. **`move` 闭包捕获引用**：移动的是引用本身，被引用数据的生命周期仍要够长
17. **闭包在热路径分配**：`Box<dyn Fn>` 每次调用一次间接；泛型闭包可内联

## 完整示例

见 `18_closures_and_fn_traits.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/18_闭包Fn与FnMut/18_closures_and_fn_traits.rs" -o run.exe
./run.exe
```

示例共 16 行输出，覆盖：`Fn`/`FnMut`/`FnOnce` 三种约束、函数指针与无捕获闭包、
Copy 闭包、不相交字段捕获、返回 `impl FnMut` 计数器、`Box<dyn Fn>`、泛型 `Handler`、
迭代器适配器，最后用中间件管线、事件处理器和 ID 生成器收尾。

## 练习

1. 写 `fn make_adder(offset: i32) -> impl Fn(i32) -> i32`，返回一个捕获 `offset` 的闭包，
   并测试负偏移。
2. 写 `fn make_counter() -> impl FnMut() -> u32`，从 1 开始计数；
   再写一个 `Fn` 版本的只读计数器，比较两者差异。
3. 实现 `fn apply_twice<F: Fn(i32) -> i32>(value: i32, f: F) -> i32`，
   用同一个闭包连续调用两次。
4. 用泛型 `Handler<F: Fn(i32) -> i32>` 存闭包并调用；再改成 `Box<dyn Fn(i32) -> i32>`，
   比较代码体积与灵活性。
5. 实现中间件管线 `Vec<Box<dyn Fn(i32) -> i32>>`，用 `fold` 依次执行；
   增加一个"记录日志"的中间件（需要捕获 `&mut Vec<String>`，用 `FnMut`）。
6. 写一个 `FnOnce` 闭包消费 `String`，调用一次；再装进 `Box<dyn FnOnce()>` 调用。
7. 用结构体两个字段演示 2021 不相交捕获：闭包只借用 `left` 时，`right` 仍可修改。
8. 比较函数指针 `fn(i32) -> i32`、无捕获闭包、带捕获闭包三种形式的大小和调用方式。

## 参考

- The Rust Programming Language: 第 13.1 节「闭包」、第 13.2 节「迭代器」
- Rust Reference: [Closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html)、
  [Closure traits](https://doc.rust-lang.org/reference/types/closure.html)
- std 文档：[`Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html)、
  [`FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html)、
  [`FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)
- Rust 2021 Edition Guide: [Disjoint closure captures](https://doc.rust-lang.org/edition-guide/rust-2021/disjoint-closure-capture.html)
- 错误索引：[E0373](https://doc.rust-lang.org/error_codes/E0373.html)、
  [E0382](https://doc.rust-lang.org/error_codes/E0382.html)、
  [E0594](https://doc.rust-lang.org/error_codes/E0594.html)、
  [E0596](https://doc.rust-lang.org/error_codes/E0596.html)
