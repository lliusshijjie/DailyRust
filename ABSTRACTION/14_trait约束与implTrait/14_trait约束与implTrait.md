# 第 14 课 · trait 约束、impl Trait 与 where

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`14_trait_bounds_and_impl_trait.rs`

## 一、底层原理

### 14.1 约束是编译期契约

trait 约束写在签名上，告诉编译器"这个类型参数能用哪些能力"：

```rust
fn show<T: Display>(value: T) -> String { value.to_string() }
```

约束只在类型检查阶段起作用，**不进入运行期**。单态化之后，每个具体类型都生成一份
专门的代码（第 12 课），约束本身被完全擦除。

约束可以出现在很多位置：

| 位置 | 例子 |
|---|---|
| 泛型参数内联 | `fn f<T: Display>(x: T)` |
| `where` 子句 | `where T: Display + Clone` |
| `impl` 块 | `impl<T: Display> Wrapper<T>` |
| 关联类型 | `T: Iterator<Item = u32>` |
| 生命周期 | `T: 'a`、`'a: 'b` |
| `impl Trait` 内 | `x: impl Display`、`-> impl Iterator<Item = u32>` |

### 14.2 `impl Trait` 参数（APIT）：匿名泛型

```rust
fn apit_show(value: impl Display) -> String { value.to_string() }

// 约等价于
fn generic_show<T: Display>(value: T) -> String { value.to_string() }
```

- APIT 是一个**匿名类型参数**，同样走单态化，没有动态分发
- 不能 turbofish：`apit_show::<i32>(1)` 会报 E0107
- 多个 `impl Trait` 参数是**不同的**类型参数：
  `fn pair(a: impl Display, b: impl Display)` 的 `a`、`b` 可以是不同类型
- APIT 支持关联类型约束：`impl Iterator<Item = u32>`
- 需要显式指定类型参数、或在文档里引用这个类型时，用普通泛型更合适

### 14.3 `impl Trait` 返回值（RPIT）：不透明类型

```rust
fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}
```

调用方只知道"返回值实现了 `Iterator<Item = u32>`"，不知道 `Filter<Range<u32>, ...>`
这个具体类型。RPIT 的关键性质：

- **一个函数只能有一个隐藏类型**：所有返回路径必须返回同一具体类型，否则 E0308
- **静态分发**：没有 vtable、没有装箱，编译器仍然知道真实类型（只是调用方不能命名）
- **可以返回闭包/迭代器**：这些类型本来无法手写
- **不能直接放进结构体字段**：`struct S { it: impl Iterator<...> }` 不合法（E0562）；
  需要存起来就用泛型参数或 `Box<dyn Iterator<...>>`
- **auto trait 会泄漏**：隐藏类型实现了 `Send`/`Sync`，调用方就能用；
  为了 API 稳定，通常显式写 `+ Send`/`+ Sync`

### 14.4 RPIT 的生命周期捕获

```rust
fn words<'a>(text: &'a str) -> impl Iterator<Item = &'a str> + 'a { ... }
```

- **Rust 2024**：RPIT 默认捕获所有在作用域内的生命周期参数
- **Rust 2021 及更早**：只捕获在返回类型/约束里显式提到的生命周期，需要 `+ 'a`
- `+ 'a` 表示"返回的不透明类型本身也不能比 `'a` 活得更久"
- Rust 1.82 起可以用 `use<...>` 精确控制捕获：

```rust
fn f<'a>(x: &'a i32) -> impl Display + use<> { *x }   // 不捕获 'a
```

如果返回的迭代器借用了局部变量，编译器会拒绝（E0597）；它必须借用参数或拥有数据。

### 14.5 `where` 子句：约束的排版方式

```rust
fn sum_iter<I>(iter: I) -> u32
where
    I: Iterator<Item = u32>,
{
    iter.sum()
}
```

`where` 和"内联约束"在语义上完全一样，只是排版不同：

- 多参数、长约束、关联类型约束时，`where` 明显更易读
- `where` 可以写生命周期约束、HRTB、`Self: Sized`
- 约束顺序不影响语义

```rust
fn total_len<I>(iter: I) -> usize
where
    I: Iterator,
    I::Item: AsRef<str>,
{ ... }
```

### 14.6 关联类型约束

`Iterator` 的 `Item` 是**关联类型**，约束写法是 `Item = u32`：

```rust
fn sum_iter<I: Iterator<Item = u32>>(iter: I) -> u32
// 或
fn sum_iter<I>(iter: I) -> u32 where I: Iterator<Item = u32>
```

也可以用关联类型参与进一步约束：

```rust
where
    I: Iterator,
    I::Item: AsRef<str>,
```

关联类型和泛型参数的区别：一个类型对同一个 trait 只能有一个 `Item`，所以用等式约束；
泛型参数可以由调用方自由选择（第 15 课展开）。

### 14.7 `?Sized`

类型参数默认隐含 `Sized`（因为要按值传递）。`?Sized` 放宽这个要求：

```rust
fn debug_len<T: ?Sized + Debug>(value: &T) -> usize {
    format!("{:?}", value).len()
}

debug_len("abc");          // T = str
debug_len(&[1, 2, 3]);     // T = [i32; 3]
```

`str`、`[T]`、`dyn Trait` 都是未定长类型，只能通过引用/指针使用。

### 14.8 RPITIT 与 async fn in trait

Rust 1.75 起，trait 方法也能返回 `impl Trait`（RPITIT）：

```rust
trait Numbers {
    fn nums(&self) -> impl Iterator<Item = u32>;
}
```

`async fn` 在 trait 里也是同一机制（返回一个隐藏的 `Future`）。
它们让 trait 可以表达"返回迭代器/异步"的能力，但也会影响 dyn 兼容性：
带 RPITIT 的 trait 通常不能直接做 `dyn` 对象。

## 二、具体语法

### 15.1 泛型参数与约束

```rust
fn f<T: Display>(value: T) -> String { ... }
fn g<T, U>(a: T, b: U) -> String where T: Display, U: Display { ... }
fn h<T: Display + Debug + Clone>(value: T) { ... }
```

### 15.2 `where` 子句

```rust
fn f<T>(value: T) -> String
where
    T: Display + Debug,
{
    format!("{} {:?}", value, value)
}

impl<T> Wrapper<T>
where
    T: Clone,
{ ... }
```

- 逗号分隔多个约束
- 生命周期约束也写在 `where`：`where 'a: 'b, T: 'a`
- `Self: Sized` 用来把方法排除出 vtable

### 15.3 APIT

```rust
fn show(value: impl Display) -> String { ... }
fn pair(a: impl Display, b: impl Display) -> String { ... }
fn iter_sum(values: impl IntoIterator<Item = u32>) -> u32 { ... }
```

- 不能写 `impl Display + Clone` 后再 turbofish
- 需要类型推断歧义时，APIT 帮不上忙，改用泛型参数

### 15.4 RPIT

```rust
fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}

fn words<'a>(text: &'a str) -> impl Iterator<Item = &'a str> + 'a { ... }

fn counter() -> impl FnMut() -> u32 { ... }

fn numbers() -> impl Iterator<Item = u32> + Send { 1..=3 }
```

- 返回闭包、迭代器、`async` 块时最有用
- 想存储返回值：用泛型结构体或 `Box<dyn Trait>`
- 需要 `Send`/`Sync` 时显式写出来

### 15.5 关联类型约束

```rust
fn sum<I: Iterator<Item = u32>>(iter: I) -> u32 { ... }

fn total_len<I>(iter: I) -> usize
where
    I: Iterator,
    I::Item: AsRef<str>,
{ ... }
```

### 15.6 `?Sized`

```rust
fn show<T: ?Sized + Debug>(value: &T) { ... }
```

- 只能出现在类型参数/约束中，且通常只用于引用接收
- 不要给不需要 `?Sized` 的函数乱加，否则会影响调用方推断

### 15.7 HRTB（高阶 trait 约束）

```rust
fn apply<F>(f: F) -> String
where
    F: for<'a> Fn(&'a str) -> &'a str,
{ ... }
```

`for<'a>` 表示"对任意生命周期都成立"，闭包/回调 API 里常见。

### 15.8 RPITIT

```rust
trait Numbers {
    fn nums(&self) -> impl Iterator<Item = u32>;
}

impl Numbers for S {
    fn nums(&self) -> impl Iterator<Item = u32> { 1..=3 }
}
```

### 15.9 `impl Trait` + Send / 生命周期

```rust
fn produce<'a>(input: &'a str) -> impl Iterator<Item = &'a str> + Send + 'a {
    input.split_whitespace()
}
```

多个约束用 `+`；`Send`/`Sync` 是 auto trait，必须显式写才能进入 API 契约。

## 三、实际应用与最佳实践

### 16.1 惰性解析：返回借用输入的迭代器

```rust
fn parse_numbers(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.split_whitespace().filter_map(|word| word.parse().ok())
}
```

- 不分配 `Vec`，调用方 `collect()` 时才决定容器
- 返回类型借用了 `text`，生命周期绑定到参数
- 比"返回 `Vec<u32>`"少一次分配，但调用方不能保存超出 `text` 生命周期的迭代器

### 16.2 返回闭包：类型无法命名

```rust
fn counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || { count += 1; count }
}
```

闭包类型是编译器生成的匿名类型，手写不出来；RPIT 是返回它的标准方式。
调用方只知道它是 `FnMut() -> u32`，无法访问闭包内部状态。

### 16.3 泛型算法：`top_n` 与 `render_all`

```rust
fn top_n<T: Ord + Clone>(values: &[T], n: usize) -> Vec<T> { ... }
fn render_all<T: Display>(items: &[T]) -> String { ... }
```

约束只写真正需要的能力：`Ord + Clone` 用于排序/复制，`Display` 用于渲染。
泛型 + 单态化让同一份代码服务所有满足约束的类型。

### 16.4 选择哪种写法

| 需求 | 推荐 |
|---|---|
| 参数接受任意实现类型 | `impl Trait` 参数（或泛型） |
| 参数需要 turbofish/显式类型 | 普通泛型参数 |
| 返回迭代器/闭包，隐藏具体类型 | `impl Trait` 返回值 |
| 返回值要存进结构体/异构集合 | `Box<dyn Trait>` 或泛型结构体 |
| 需要跨线程 | `impl Trait + Send` / `Box<dyn Trait + Send>` |
| 公开 API 想保留实现细节 | RPIT（调用方无法命名，改实现更容易） |

### 16.5 最佳实践清单

1. **参数优先 APIT**：类型名不重要的地方写 `impl Trait`，签名更短
2. **需要 turbofish/显式类型时用泛型参数**
3. **返回迭代器/闭包用 RPIT**，避免 `Box<dyn Iterator>` 的分配与间接调用
4. **RPIT 的所有返回路径保持同一具体类型**，否则用 `Box<dyn Trait>` 或枚举
5. **跨线程返回显式加 `+ Send`**，不要依赖 auto trait 泄漏
6. **返回借用数据时写清生命周期**（`+ 'a` 或 `+ '_`），2024 下默认捕获所有
7. **需要存进结构体/集合时**用泛型结构体或 `Box<dyn Trait>`
8. **`where` 用于多约束/关联类型**，提高可读性
9. **`?Sized` 只给确实要收 `str`/切片/dyn 的函数**
10. **公开 API 的 RPIT 要谨慎**：隐藏类型是 semver 契约的一部分（auto traits、生命周期）

## 常见陷阱

1. **RPIT 多分支返回不同类型（E0308）**：`if c { iter.map(..) } else { iter.filter(..) }`
   是两个不同具体类型；用 `Box<dyn Iterator>` 或枚举统一
2. **APIT 不能 turbofish（E0107）**：`f::<i32>(1)` 对 `fn f(x: impl Display)` 不合法；
   改成普通泛型
3. **RPIT 不能放结构体字段（E0562）**：`struct S { it: impl Iterator<...> }`；
   改成泛型字段或 `Box<dyn Iterator<...>>`
4. **返回借用局部的迭代器（E0597）**：迭代器借用了函数内局部变量；
   改成返回拥有数据的 `Vec`，或借用参数
5. **忘记 `+ Send`**：返回的迭代器要跨线程时编译失败；显式加 `+ Send`
6. **`impl Trait` 隐藏类型后调用不了固有方法**：只能调用约束里的方法；
   需要更多方法就把约束写全
7. **关联类型约束写错（E0277）**：`Iterator<Item = String>` 传了 `u32` 迭代器；
   检查 Item 类型
8. **`?Sized` 漏写**：想收 `&str`/`&[T]` 却写了 `T`，编译失败
9. **`where` 子句语法错误**：多个约束用逗号，不是 `+`；`where` 放在返回类型之后
10. **HRTB 写成普通生命周期**：`F: Fn(&str) -> &str` 与 `for<'a> Fn(&'a str) -> &'a str`
    在闭包场景语义不同；需要"任意生命周期"时用 `for<'a>`
11. **RPITIT 让 trait 不 dyn 兼容**：带 `-> impl Trait` 的 trait 通常不能做 `dyn`
12. **`impl Trait` 返回的 auto trait 不稳定**：隐藏类型变了，`Send`/`Sync` 可能变；
    API 里显式声明
13. **返回 `impl Trait` 但调用方想命名**：RPIT 天生不可命名；需要命名就用具体类型/泛型
14. **`Box<dyn Trait>` 默认 `'static`**：借用数据要写 `Box<dyn Trait + '_>`
15. **在 trait 方法里用 APIT**：`fn f(x: impl Display)` 在 trait 里等价于泛型方法，
    会影响 dyn 兼容性；需要对象安全时用泛型参数 + `where Self: Sized`

## 完整示例

见 `14_trait_bounds_and_impl_trait.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/14_trait约束与implTrait/14_trait_bounds_and_impl_trait.rs" -o run.exe
./run.exe
```

示例共 18 行输出，覆盖：APIT 与泛型等价、RPIT 隐藏迭代器类型、借用返回、
`where` + 关联类型约束、多约束、`?Sized`、`+ Send`、`Box<dyn>` 对照、
RPIT 返回闭包、惰性解析、`top_n` 与泛型渲染。

## 练习

1. 写 `fn evens(limit: u32) -> impl Iterator<Item = u32>`，再写一个
   `fn evens_boxed(limit: u32) -> Box<dyn Iterator<Item = u32>>`，比较两者。
2. 写 `fn words<'a>(text: &'a str) -> impl Iterator<Item = &'a str> + 'a`，
   分别用 `&str` 和 `String` 调用。
3. 实现 `fn top_n<T: Ord + Clone>(values: &[T], n: usize) -> Vec<T>`，
   再用 `impl Trait` 参数写一个等价版本，比较签名可读性。
4. 返回一个 `impl FnMut() -> u32` 计数器；解释为什么调用方无法命名这个闭包类型。
5. 给一个返回迭代器的函数加 `+ Send`，并尝试把它放进 `std::thread::spawn`；
   去掉 `+ Send` 观察编译错误。
6. 在 trait 里定义 `fn nums(&self) -> impl Iterator<Item = u32>` 并实现；
   再尝试 `Box<dyn Trait>`，记录错误并解释。
7. 试写 `struct Holder { items: impl Iterator<Item = u32> }`，记录 E0562，
   分别用泛型字段和 `Box<dyn Iterator>` 修复。
8. 比较 `impl Trait` 与 `Box<dyn Trait>` 作为公开 API 返回值时，
   对调用方、二进制体积、semver 的影响。

## 参考

- The Rust Programming Language: 第 10.2 节「trait 作为参数 / 返回实现 trait 的类型」
- Rust Reference: [`impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)、
  [Trait bounds](https://doc.rust-lang.org/reference/trait-bounds.html)、
  [Where clauses](https://doc.rust-lang.org/reference/items/generics.html#where-clauses)
- Rust 2024 Edition Guide: [RPIT lifetime capture](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html)
- Rust 1.82 发布说明（`use<>` 精确捕获）：
  <https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html>
- Rust 1.75 发布说明（RPITIT / async fn in trait）：
  <https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html>
- 错误索引：[E0308](https://doc.rust-lang.org/error_codes/E0308.html)、
  [E0107](https://doc.rust-lang.org/error_codes/E0107.html)、
  [E0562](https://doc.rust-lang.org/error_codes/E0562.html)、
  [E0597](https://doc.rust-lang.org/error_codes/E0597.html)
