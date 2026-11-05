# 第 15 课 · 关联类型、const 泛型与 GAT

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`15_associated_types_const_generics.rs`

## 一、底层原理

### 15.1 关联类型：由实现者选择的"输出类型"

trait 可以声明一个"类型占位符"，具体类型由实现者填：

```rust
trait Collection {
    type Item;                          // 关联类型声明
    fn first(&self) -> Option<&Self::Item>;
}

impl<T> Collection for Vec<T> {
    type Item = T;                      // 实现者选择具体类型
    fn first(&self) -> Option<&T> { ... }
}
```

访问方式：

```rust
Self::Item                      // trait/impl 内部
<Vec<i32> as Collection>::Item  // 完全限定
C::Item                         // 泛型约束中
```

关联类型和泛型参数的区别是**谁来做选择**：

| | 泛型参数 `trait Convert<T>` | 关联类型 `trait Collection { type Item; }` |
|---|---|---|
| 谁选类型 | 调用方/实现方都可以写多个 impl | 实现方为每个类型选**唯一**一个 |
| 同一目标类型的 impl 数 | 可以有多个（`Convert<f64>`、`Convert<u32>`） | 只能有一个（重复 impl 会 E0119） |
| 访问 | `Convert<f64>` | `Collection::Item` / `C::Item` |
| 典型例子 | `From<T>`、`AsRef<T>` | `Iterator::Item`、`Deref::Target`、`Add::Output` |

选择原则：

- 一个类型对某 trait 只有**唯一自然输出** → 关联类型（如迭代器的元素类型）
- 调用方需要选择不同输入/输出组合 → 泛型参数（如 `From<A>`、`From<B>`）
- 关联类型参与约束：`I: Iterator<Item = u32>`、`C::Item: Display`

### 15.2 const 泛型：把"值"变成类型参数

```rust
struct Buffer<const N: usize> {
    data: [u8; N],
}

fn sum_array<const N: usize>(values: [i32; N]) -> i32 { ... }
```

- `N` 是**编译期常量**，进入类型：`Buffer<4>` 与 `Buffer<8>` 是不同类型
- `N` 可以直接用于数组长度、循环上界、容量计算
- 每个具体 `N` 单态化出一份代码（和类型参数一样）
- const 泛型的值**不占运行时字段**；`size_of::<Buffer<4>>()` 就是 4

目前稳定支持的 const 参数类型：

| 类型 | 状态 |
|---|---|
| 整数（`usize`、`u8` 等） | 稳定 |
| `bool`、`char` | 稳定 |
| 引用、字符串、结构体 | 不稳定 |
| 类型级算术（`N + 1`） | 不稳定（`generic_const_exprs`） |

const 参数也可以有**默认值**：

```rust
struct DefaultBuffer<const N: usize = 3> { data: [u8; N] }
DefaultBuffer::<3>;      // 显式
size_of::<DefaultBuffer>(); // 使用默认 3
```

### 15.3 GAT：带生命周期参数的关联类型

普通关联类型不能带生命周期：

```rust
trait Lender {
    type Item;                       // 只能是一个固定类型
    fn lend(&self) -> Self::Item;    // 无法表达"借用 self 的 &str"
}
```

如果 `Item` 需要借用 `self`，它的生命周期必须随 `&self` 变化——这正是 **GAT
（generic associated type）** 解决的问题：

```rust
trait Lender {
    type Item<'a> where Self: 'a;    // 带生命周期参数的关联类型
    fn lend(&self) -> Self::Item<'_>;
}

impl Lender for Words {
    type Item<'a> = &'a str where Self: 'a;
    fn lend(&self) -> Self::Item<'_> { self.data[0].as_str() }
}
```

要点：

- GAT 在 Rust 1.65 稳定
- `where Self: 'a` 是**必需**的：它把关联类型的生命周期限制在 `Self` 有效的范围内；
  漏写会报 "missing required bound on `Item`"
- GAT 可以带 trait 约束：
  `type Iter<'a>: Iterator<Item = &'a i32> where Self: 'a;`
- 每次方法调用可以使用不同的 `'a`，所以同一个类型能返回"借用本次 `&self`"的视图
- 带 GAT 的 trait **通常不能做 `dyn` 对象**（E0038：GAT 无法放进 vtable）

GAT 的典型用途：

| 场景 | 例子 |
|---|---|
| 借用型迭代器 | `type Iter<'a>: Iterator<Item = &'a T>` |
| 零拷贝视图 | `type View<'a> where Self: 'a` |
| 异步 trait（RPITIT 之前） | `type Fut<'a>: Future<Output = T>` |
| 数据库/解析器行视图 | `fn row(&self) -> Self::Row<'_>` |

### 15.4 三者如何配合

| 机制 | 解决什么 | 选择者 | 生命周期 |
|---|---|---|---|
| 关联类型 | "这个 trait 的输出类型" | 实现者 | 固定 |
| const 泛型 | "这个类型的编译期常量" | 调用方/实现者 | 无 |
| GAT | "随借用变化的关联类型" | 实现者 + 调用方 | 每次调用 |

它们可以组合：`trait FixedArray<const N: usize> { type Item; }`、
`trait Lender { type Item<'a> where Self: 'a; }`、
`struct Matrix<const R: usize, const C: usize>`。

## 二、具体语法

### 16.1 关联类型的定义、实现、访问

```rust
trait Collection {
    type Item;
    fn first(&self) -> Option<&Self::Item>;
}

impl<T> Collection for Vec<T> {
    type Item = T;
    fn first(&self) -> Option<&T> { self.as_slice().first() }
}

<Vec<i32> as Collection>::Item       // i32
Vec::<i32>::Item                     // 也合法（trait 在作用域内）
```

- impl 里必须写 `type Item = ...`，漏写会 E0046
- 多个 trait 都有 `Item` 时，用 `<T as Trait>::Item` 消歧
- 泛型约束里用 `C::Item` 或 `C: Collection<Item = T>`

### 16.2 关联类型作为约束

```rust
fn sum<I: Iterator<Item = u32>>(iter: I) -> u32 { ... }

fn first_item<C: Collection>(collection: &C) -> Option<&C::Item> {
    collection.first()
}

fn show_all<C>(collection: &C) -> String
where
    C: Collection,
    C::Item: Display,
{ ... }
```

- `Item = u32` 是**等式约束**：一个类型只能有一个 `Item`
- 关联类型上还能继续加约束：`C::Item: Display`

### 16.3 const 泛型函数与结构体

```rust
fn sum_array<const N: usize>(values: [i32; N]) -> i32 {
    values.iter().sum()
}

struct Buffer<const N: usize> { data: [u8; N] }

impl<const N: usize> Buffer<N> {
    fn new() -> Self { Self { data: [0; N] } }
    fn capacity(&self) -> usize { N }
}

size_of::<Buffer<4>>()      // 4
```

- `impl` 块必须重复 `const N: usize`（和类型参数一样）
- `N` 可以用于 `[0; N]`、`[T; N]`、数组长度
- `Buffer<4>` 和 `Buffer<8>` 不能互相赋值

### 16.4 const 泛型的默认值与限制

```rust
struct DefaultBuffer<const N: usize = 3> { data: [u8; N] }

size_of::<DefaultBuffer>()   // 3（使用默认值）
```

- 默认值让调用方可以省略 `::<N>`
- 不能写 `[0; N * 2]` 这类类型级算术（稳定版不支持）
- `N` 必须是编译期常量，不能是运行期变量
- const 参数的类型不能是任意类型（稳定版仅整数/bool/char）

### 16.5 GAT 的定义与实现

```rust
trait Lender {
    type Item<'a> where Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
}

impl Lender for Words {
    type Item<'a> = &'a str where Self: 'a;
    fn lend(&self) -> Self::Item<'_> { self.data[0].as_str() }
}
```

- GAT 声明里 `where Self: 'a` 不能省
- impl 里的 `'_` 由 `&self` 的借用推断
- 返回 `&'a str` 时，`'a` 与 `&self` 的借用绑定，调用方不能让它比 `self` 活得更久

### 16.6 GAT 带 trait 约束

```rust
trait Iterable {
    type Iter<'a>: Iterator<Item = &'a i32> where Self: 'a;
    fn iter(&self) -> Self::Iter<'_>;
}

impl Iterable for Numbers {
    type Iter<'a> = std::slice::Iter<'a, i32> where Self: 'a;
    fn iter(&self) -> Self::Iter<'_> { self.0.iter() }
}
```

- GAT 的约束写在 `:` 后面，生命周期约束写在 `where`
- 这样 `self.iter()` 可以直接当 `Iterator` 用，元素是 `&i32`

### 16.7 GAT 与 trait 对象

```rust
let _: Box<dyn Iterable>;   // E0038：GAT 不能进 vtable
```

- 带 GAT 的 trait 通常不是 dyn 兼容的
- 需要运行时多态时，改用 `Box<dyn Iterator<Item = ...>>` 或枚举

## 三、实际应用与最佳实践

### 17.1 `Collection`：统一容器的"首元素"接口

```rust
let values = vec![10, 20, 30];
first_item(&values);              // Some(10)
Collection::len(&values);         // 3
```

关联类型让 `first_item` 能返回"容器元素类型"的引用，而不需要为每种容器写一个函数。
`Collection::len(&values)` 用 UFCS 避免与 `Vec::len` 的固有方法混淆。

### 17.2 `Matrix<R, C>`：行列数进入类型

```rust
let mut matrix = Matrix::<2, 3>::zeros();
matrix.set(0, 0, 1.5);
matrix.set(1, 2, 5.0);
matrix.sum();                     // 6.5
matrix.rows(); matrix.cols();     // 2, 3
```

`R`、`C` 编译期确定后，数组维度、循环上界、越界检查都可以被优化；
不同尺寸的矩阵是不同类型，误用会在编译期被发现。

### 17.3 GAT 零拷贝视图

```rust
let grid = Grid { rows: vec![vec![1, 2], vec![4, 8], vec![10, 20]] };
let sums: Vec<i32> = (0..3).map(|i| grid.row(i).sum()).collect();
// [3, 12, 30]
```

`row()` 返回借用该行的迭代器，不复制数据。没有 GAT 时，要么返回 `&Vec<i32>`
（暴露内部结构），要么返回 `Box<dyn Iterator>`（分配 + 动态分发）。

### 17.4 最佳实践清单

1. **每个类型只有唯一输出类型时用关联类型**，否则用泛型参数
2. **关联类型约束优先写等式**：`Iterator<Item = u32>` 比额外泛型参数更清晰
3. **const 泛型用于"编译期已知的尺寸/长度"**：矩阵、固定缓冲、协议常量
4. **const 泛型不要塞运行期数据**：`N` 必须是 const
5. **GAT 的 `where Self: 'a` 不要漏**，它是借用型关联类型的前提
6. **GAT 返回借用时保持零拷贝**，不要把 `&str` 变成 `String`
7. **带 GAT 的 trait 不要指望 `dyn`**；需要异构就换 `Box<dyn>`/枚举
8. **关联类型过多时考虑拆 trait**，一个 trait 只表达一组内聚能力
9. **const 泛型参数和类型参数一样要重复写在 `impl` 上**
10. **公开 API 的关联类型/const 参数也是 semver 契约**，改动会破坏下游

## 常见陷阱

1. **重复 impl 不同关联类型（E0119）**：`impl A for i32 { type Item = u8; }` 与
   `type Item = u16` 冲突；需要多输出就用泛型参数
2. **忘记写 `type Item = ...`（E0046）**：impl 必须列全关联类型
3. **关联类型访问歧义**：多个 trait 都有 `Item` 时用 `<T as Trait>::Item`
4. **GAT 漏写 `where Self: 'a`**：报 "missing required bound on `Item`"
5. **GAT trait 做 `dyn`（E0038）**：GAT 不能进 vtable；改用具体类型/`Box<dyn Iterator>`
6. **const 泛型做类型级算术**：`[0; N * 2]` 稳定版不支持；需要 nightly `generic_const_exprs`
7. **const 参数用运行期变量**：`let n = 4; Buffer::<n>` 不合法，`n` 必须是 const
8. **混淆 `Buffer<4>` 与 `Buffer<8>`**：它们是不同类型，不能互相赋值
9. **const 泛型 impl 漏写参数**：`impl Buffer<N>` 缺 `const N: usize` 不合法
10. **关联类型上忘记加约束**：`C::Item: Display` 不写就调不了 `to_string`
11. **把 GAT 当普通关联类型用**：`type Item<'a> = &'a str` 在 impl 里必须带
    `where Self: 'a` 和生命周期参数
12. **GAT 返回借用超出 `self` 生命周期**：编译器会拒绝；不要试图返回局部数据的借用
13. **const 泛型默认值位置**：默认值要放在参数列表末尾（和类型参数默认值一样）
14. **`N` 参与数组重复时类型不匹配**：`[0u8; N]` 与 `[0; N]` 的元素类型要一致
15. **关联类型和泛型参数混用导致推断困难**：调用处可能需要 turbofish 或显式标注

## 完整示例

见 `15_associated_types_const_generics.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/15_关联类型const泛型与GAT/15_associated_types_const_generics.rs" -o run.exe
./run.exe
```

示例共 14 行输出，覆盖：`Collection::Item` 的类型名与借用、`Convert<T>` 的多次 impl、
const 泛型 `Buffer<N>` 的大小/容量/求和、`sum_array`、关联类型约束、GAT 借用 `&str`、
GAT 迭代器、const 泛型矩阵、GAT 行视图求和。

## 练习

1. 定义 `trait Container { type Item; fn first(&self) -> Option<&Self::Item>; }`，
   为 `Vec<T>` 与 `Option<T>` 实现，比较两者的 `Item`。
2. 用 `trait Convert<T>` 为同一个类型实现 `Convert<f64>` 和 `Convert<u32>`；
   再改写成关联类型版本，观察为什么只能保留一个转换。
3. 写 `fn sum_array<const N: usize>(values: [i32; N]) -> i32`，
   分别用长度 3、5、10 的数组调用，解释单态化。
4. 实现 `Matrix<const R: usize, const C: usize>` 的 `transpose`，
   返回 `Matrix<C, R>`（提示：逐元素复制）。
5. 实现 GAT `trait Lender { type Item<'a> where Self: 'a; fn lend(&self) -> Self::Item<'_>; }`，
   为一个持有 `Vec<String>` 的结构体返回 `&str`。
6. 实现 GAT `trait Iterable { type Iter<'a>: Iterator<Item = &'a i32> where Self: 'a; }`，
   用 `std::slice::Iter` 作为具体类型。
7. 尝试 `Box<dyn Iterable>`，记录 E0038；再改用 `Box<dyn Iterator<Item = i32>>` 修复。
8. 尝试为同一类型写两个不同 `Item` 的 impl，记录 E0119，并解释关联类型与泛型参数的区别。

## 参考

- The Rust Programming Language: 第 19.2 节「高级 trait」（关联类型）、第 13 章（`Iterator::Item`）
- Rust Reference: [Associated items](https://doc.rust-lang.org/reference/items/associated-items.html)、
  [Generic parameters](https://doc.rust-lang.org/reference/items/generics.html)
- Rust 1.65 发布说明（GAT 稳定）：
  <https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html>
- RFC 1598（GAT）：<https://rust-lang.github.io/rfcs/1598-generic_associated_types.html>
- const 泛型 RFC 2000：<https://rust-lang.github.io/rfcs/2000-const-generics.html>
- 错误索引：[E0119](https://doc.rust-lang.org/error_codes/E0119.html)、
  [E0038](https://doc.rust-lang.org/error_codes/E0038.html)、
  [E0046](https://doc.rust-lang.org/error_codes/E0046.html)
