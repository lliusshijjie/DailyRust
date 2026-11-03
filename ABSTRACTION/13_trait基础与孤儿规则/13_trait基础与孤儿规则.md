# 第 13 课 · trait 基础与孤儿规则

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`13_traits_and_orphan_rule.rs`

## 一、底层原理

### 13.1 trait 是"能力契约"，不是继承

trait 定义一组类型可以实现的能力：

```rust
trait Shape {
    fn name(&self) -> &'static str;      // 必须实现
    fn area(&self) -> f64;               // 必须实现

    fn describe(&self) -> String {       // 默认实现
        format!("{} with area {:.2}", self.name(), self.area())
    }
}
```

- trait 只描述**行为**，不携带数据；实现类型自己决定字段布局
- 一个类型可以实现多个 trait；trait 也可以有 supertrait（13.3）
- trait 里的方法、关联常量、关联类型都是**公开契约**，没有 `pub(crate)` 这种收窄
- 与继承的区别：没有"父类字段/构造器"，复用靠组合 + 默认方法，而不是子类

| 机制 | 数据复用 | 行为复用 | 开放扩展 |
|---|---|---|---|
| 类继承 | 有（父类字段） | 有（虚方法） | 子类 |
| 接口/抽象类 | 无/有 | 有 | 实现者 |
| Rust trait | 无 | 默认方法 | 任何本地类型 + 本地 trait |

### 13.2 静态分发与动态分发

trait 有两种使用方式，底层机制完全不同：

| | 泛型约束 `T: Shape` | `dyn Shape` |
|---|---|---|
| 分发 | 编译期单态化，静态调用 | 运行期 vtable，间接调用 |
| 值大小 | 具体类型大小 | 胖指针（数据 + vtable） |
| 集合 | 同质（`Vec<T>`） | 异构（`Vec<Box<dyn Trait>>`） |
| 代码体积 | 每个具体类型一份 | 一份 |
| 内联 | 可以 | 通常不行 |
| 约束 | 无额外要求 | trait 必须 dyn 兼容 |

```rust
fn total_area<T: Shape>(items: &[T]) -> f64 { ... }        // 单态化
let shapes: Vec<Box<dyn Shape>> = vec![...];                // vtable
```

`dyn Shape` 是**未定长类型**，只能放在指针后面：`&dyn Shape`、`Box<dyn Shape>`、
`Rc<dyn Shape>`。胖指针的两个字分别是数据指针和 vtable 指针。

### 13.3 默认方法、supertrait 与关联项

**默认方法**可以有方法体，能调用必须方法；实现者可以覆盖：

```rust
impl Shape for Square {
    fn name(&self) -> &'static str { "square" }
    fn area(&self) -> f64 { self.side * self.side }
    fn describe(&self) -> String { format!("square: {:.2}", self.area()) } // 覆盖
}
```

**supertrait** 表达"实现 A 必须先实现 B"：

```rust
trait Named: std::fmt::Display {
    fn name(&self) -> &'static str;
    fn named(&self) -> String {
        format!("{} ({})", self.name(), self)   // 可以用 Display 的能力
    }
}
```

实现 `Named` 的类型必须同时实现 `Display`；泛型 `T: Named` 里也能调用 `Display` 的方法。

**关联常量/函数**挂在 trait 上，而不是实例上：

```rust
trait Sides { const COUNT: u32; }
<Circle as Sides>::COUNT;        // 0

trait Factory {
    fn create() -> Self;         // 关联函数：没有 self
}
```

关联函数和关联常量会让 trait **不能**做 `dyn` 对象（13.5），所以示例把 `Sides`
拆成了单独的 trait。

### 13.4 孤儿规则（orphan rule）与一致性（coherence）

孤儿规则回答一个问题：**谁能给某个类型实现某个 trait？**

> `impl<P...> ForeignTrait<T...> for T0` 只有在"trait 本地"或"类型本地（且类型参数被本地类型覆盖）"
> 时才允许。

目的是保证**一致性（coherence）**：不同 crate 不能对同一类型 + 同一 trait 写出冲突的实现，
从而让编译器可以做全局唯一的静态分发，也保证下游升级依赖时行为可预测。

| 写法 | 结果 | 原因 |
|---|---|---|
| `impl Display for MyType` | ✅ | 类型本地（外部 trait） |
| `impl MyTrait for String` | ✅ | trait 本地（外部类型） |
| `impl<T> MyTrait for T` | ✅ | trait 本地，blanket impl |
| `impl<T> Display for MyWrapper<T>` | ✅ | 本地泛型类型覆盖了 T |
| `impl Display for String` | ❌ E0117 | trait 和类型都外部 |
| `impl<T> Display for T` | ❌ E0210 | 外部 trait + 裸类型参数 |
| `impl<T> Display for Vec<T>` | ❌ E0117 | 外部 trait + 外部类型 |

几个细节：

- **blanket impl**（`impl<T: Display> MyTrait for T`）是本地 trait 最常见的扩展方式；
  但它会和任何更具体的 `impl MyTrait for Concrete` 冲突（E0119）
- **fundamental 类型**（`Box<T>`、`&T`）在孤儿规则里会"看穿"到内部类型：
  `impl ForeignTrait for Box<MyType>` 允许，因为 `MyType` 是本地类型
- **类型别名不创建本地类型**：`type MyString = String; impl Display for MyString` 仍然是 E0117
- 在库里**新增 blanket impl** 可能让下游的 `impl MyTrait for DownstreamType` 冲突，
  这是 semver 层面的破坏性变更

一致性还禁止同一个 crate 内两个 impl 重叠（E0119），trait 特化（specialization）目前不稳定。

### 13.5 dyn 兼容性（object safety）

`dyn Trait` 需要编译器生成 vtable，因此 trait 必须满足 **dyn 兼容（dyn compatible，
旧称 object safe）**：

- 不能有**关联常量**
- 不能有返回 `Self`（按值）的方法
- 不能有**泛型方法**
- 静态方法（无 self）要么不出现，要么用 `where Self: Sized` 排除
- 方法接收者必须是 `&self`、`&mut self`、`self: Box<Self>`、`self: Rc<Self>`、
  `self: Arc<Self>`、`self: Pin<&mut Self>` 等；**不能用裸 `self`**（除非 `Self: Sized`）

```rust
trait Shape {
    fn area(&self) -> f64;                       // OK
    fn describe(&self) -> String { ... }         // OK
}

trait Sides { const COUNT: u32; }                // 不能 dyn
trait Factory { fn create() -> Self; }           // 不能 dyn
trait Generic { fn convert<T>(&self); }          // 不能 dyn
```

用 `where Self: Sized` 可以把某些方法从 vtable 里排除，trait 本身仍然可以 dyn：

```rust
trait Builder {
    fn build(&self) -> String;
    fn boxed(self) -> Box<Self> where Self: Sized { Box::new(self) }
}
```

`Box<dyn Shape>` 默认还隐含 `'static`：装短生命周期借用要写 `Box<dyn Shape + 'a>`。

### 13.6 derive 与手写 impl 的关系

`#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]` 是**宏生成的普通 impl**，
同样受孤儿规则/一致性约束。它只能生成"逐字段"的机械实现：

- `Debug` 输出类型名和字段；`Display` 不能 derive，需要手写
- `Clone` 是逐字段 `clone`（深或浅取决于字段类型）
- `PartialEq` 逐字段比较；`Eq`/`Hash` 要求字段也满足
- `Copy` 要求字段都 `Copy` 且没有 `Drop`

手写 impl 用于自定义语义（如 `Money` 的 `Display` 格式）。

## 二、具体语法

### 14.1 定义 trait

```rust
trait Summary {
    const MAX_LEN: usize = 280;                  // 关联常量（可默认）
    fn summarize(&self) -> String;               // 必须方法
    fn short(&self) -> String {                  // 默认方法
        self.summarize().chars().take(20).collect()
    }
}
```

### 14.2 实现 trait

```rust
impl Summary for Article {
    fn summarize(&self) -> String { format!("[article] {}", self.title) }
}

impl Summary for Tweet { ... }

impl<T: Summary> Summary for Wrapper<T> { ... }  // 本地泛型类型
```

- 实现块必须列出 trait 的所有**必须项**，否则 E0046
- 默认方法可以覆盖，也可以不写
- 一个类型可以实现多个 trait，一个 trait 可以有多个 impl 块

### 14.3 泛型约束

```rust
fn total_area<T: Shape>(items: &[T]) -> f64 { ... }

fn show<T>(value: &T) -> String
where
    T: Summary + std::fmt::Debug,
{ ... }
```

- `T: Shape` 表示 T 可以用作 Shape；单态化后按具体类型生成代码
- 多约束用 `+`；`where` 子句适合长签名
- 泛型函数里只能调用约束提供的能力

### 14.4 trait 对象

```rust
let shapes: Vec<Box<dyn Shape>> = vec![
    Box::new(Circle { radius: 1.0 }),
    Box::new(Rect { width: 2.0, height: 3.0 }),
];

fn describe(shape: &dyn Shape) -> String { shape.describe() }
```

- `dyn Trait` 只能通过指针使用
- `&dyn Trait` 借用；`Box<dyn Trait>` 拥有；`Rc<dyn Trait>` 共享
- 调用走 vtable，不能内联；异构集合的唯一选择（或者用枚举，如果集合封闭）

### 14.5 blanket impl

```rust
trait Printable { fn printed(&self) -> String; }

impl<T: std::fmt::Display> Printable for T {
    fn printed(&self) -> String { self.to_string() }
}

42i32.printed();      // "42"
```

- 本地 trait 可以为所有满足约束的类型统一实现
- 之后不能再写 `impl Printable for MyType`（会 E0119 冲突）
- 标准库的 `impl<T: Display> ToString for T` 就是这种模式

### 14.6 扩展 trait（extension trait）

```rust
trait Second { fn second(&self) -> Option<i32>; }

impl Second for Vec<i32> {
    fn second(&self) -> Option<i32> { self.get(1).copied() }
}

vec![1, 2, 3].second();     // Some(2)
```

- trait 本地、类型外部（`Vec<i32>`），孤儿规则允许
- 这是给外部类型"加方法"的标准做法；不需要包装类型
- 注意方法名冲突：trait 方法与固有方法同名时，固有方法优先

### 14.7 UFCS（完全限定语法）

```rust
Shape::area(&circle);              // 显式调用 trait 方法
<Circle as Shape>::name(&circle);  // 完全限定
<Tag as Named>::named(&tag);
```

- 当固有方法与 trait 方法同名、或多个 trait 提供同名方法时，用 UFCS 消歧
- 关联常量同理：`<Circle as Sides>::COUNT`

### 14.8 supertrait

```rust
trait Named: std::fmt::Display {
    fn name(&self) -> &'static str;
}
```

- 实现 `Named` 必须先实现 `Display`，否则 E0277
- `T: Named` 的泛型代码可以同时用 `name()` 和 `to_string()` 等 Display 能力

### 14.9 derive

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Config { host: String, port: u16 }
```

- derive 只生成机械实现；需要业务语义时手写
- `Display`、`Ord`（自定义排序）、`FromStr` 等通常手写
- `#[derive]` 生成的 impl 同样参与一致性检查

## 三、实际应用与最佳实践

### 15.1 异构集合：`Summary` + `Box<dyn Summary>`

```rust
let items: Vec<Box<dyn Summary>> = vec![
    Box::new(Article { title: String::from("Rust traits") }),
    Box::new(Tweet { user: String::from("alice"), text: String::from("hello") }),
];
summarize_all(&items);   // 调用各自实现，统一返回 String
```

当集合里混有不同具体类型、且行为由 trait 统一时，trait 对象是自然选择。
如果集合是**封闭**的（类型种类固定），枚举 + `match` 往往更高效（第 09 课）。

### 15.2 扩展 trait：`Second`

```rust
trait Second { fn second(&self) -> Option<i32>; }
impl Second for Vec<i32> { ... }
```

适合给第三方类型补小工具方法。工程上如果方法很多、语义很重，建议用 newtype
包装（第 08 课），避免 trait 方法污染所有 `Vec<i32>` 的调用点。

### 15.3 本地类型实现外部 trait：`Display for Money`

```rust
struct Money(i64);
impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}
```

孤儿规则允许"外部 trait + 本地类型"，这是自定义输出格式、序列化、比较语义的常见入口。

### 15.4 选择泛型还是 trait 对象

| 场景 | 选择 |
|---|---|
| 同质集合、热路径、需要内联 | 泛型 `T: Trait` |
| 异构集合、插件、运行时装配 | `Box<dyn Trait>` |
| 封闭类型集合、需要穷尽匹配 | 枚举 |
| 库的公共 API | 优先泛型；需要异构时提供 `dyn` 版本 |

### 15.5 最佳实践清单

1. **先定义行为，再决定用泛型还是 dyn**：别为了"灵活"到处 `Box<dyn>`
2. **默认方法保持短小**，复杂逻辑放到实现或独立函数
3. **supertrait 只在确实需要父能力时使用**，避免约束膨胀
4. **扩展 trait 用本地 trait**，遵守孤儿规则，不要试图给外部类型实现外部 trait
5. **blanket impl 谨慎**：它会阻止更具体的实现，也可能影响下游
6. **让 trait 保持 dyn 兼容**，除非确定不需要 trait 对象
7. **对象安全的方法用 `&self`/`&mut self`**；需要 `Self` 的方法加 `where Self: Sized`
8. **方法名冲突用 UFCS**，不要依赖隐式优先级
9. **derive 只用于机械语义**；`Display`/`Ord`/`FromStr` 手写
10. **公开 trait 是 API 契约**：加必须方法会破坏所有实现者，优先给默认实现

## 常见陷阱

1. **孤儿规则（E0117）**：`impl Display for String` 不允许；trait 和类型必须至少一个本地
2. **blanket 外部 trait（E0210）**：`impl<T> Display for T` 不允许；外部 trait 不能为裸类型参数实现
3. **impl 重叠（E0119）**：blanket impl 与具体 impl 冲突；只能二选一
4. **trait 没导入（E0599）**：调用 trait 方法前要 `use` 该 trait；同 crate 的 trait 也要在作用域
5. **trait 不 dyn 兼容（E0038）**：含关联常量、泛型方法、返回 `Self`、裸 `self` 方法；
   用 `where Self: Sized` 排除或拆 trait
6. **supertrait 没实现（E0277）**：`trait A: B` 的实现者必须先实现 B
7. **必须方法漏实现（E0046）**：impl 块没有列全必须项；默认方法可以不写
8. **`Box<dyn Trait>` 默认 `'static`**：装借用要写 `Box<dyn Trait + 'a>`
9. **trait 对象按值使用**：`dyn Trait` 是 DST，不能作为局部变量，必须 `&`/`Box`
10. **固有方法与 trait 方法同名**：固有方法优先；要调 trait 版本用 `UFCS`
11. **给类型定义加多余约束**：`struct S<T: Trait>` 迫使所有使用处满足；约束放 impl
12. **类型别名当本地类型**：`impl Display for MyString`（`MyString = String`）仍是 E0117
13. **blanket impl 后加具体 impl**：E0119；从设计上就要想清楚覆盖范围
14. **关联常量/函数的访问**：用 `<Type as Trait>::ITEM`，不要依赖名字推断
15. **在库里新增 impl 造成下游冲突**：blanket impl 可能让下游已有 impl 失效，
    这是 semver 破坏性变更
16. **`dyn Trait + Send + Sync` 忘记加**：跨线程传递 trait 对象时类型不满足
17. **trait 方法按值 `self`**：默认不能用于 dyn；需要 `Self: Sized`
18. **`impl Trait for &T` 与 `impl Trait for T` 混淆**：两者是不同的实现目标

## 完整示例

见 `13_traits_and_orphan_rule.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/13_trait基础与孤儿规则/13_traits_and_orphan_rule.rs" -o run.exe
./run.exe
```

示例共 18 行输出，覆盖：默认方法与覆盖、supertrait、blanket impl、关联常量、
trait 对象异构集合、泛型约束、`impl Trait` 参数、`&dyn`、泛型类型实现 trait、UFCS、
本地类型实现 `Display`、本地 trait 扩展 `str` 与 `Vec<i32>`、以及 `Summary` 摘要系统。

## 练习

1. 定义 `trait Area { fn area(&self) -> f64; }`，给 `describe` 一个默认实现；
   为圆、矩形、三角形实现，并让其中一个覆盖 `describe`。
2. 定义 `trait Named: std::fmt::Display`，写泛型函数 `fn show<T: Named>(value: &T) -> String`，
   在函数里同时使用 `name()` 与 `to_string()`。
3. 为 `Vec<i32>` 写扩展 trait，提供 `second` 与 `sum_even`；
   解释为什么这在孤儿规则下合法。
4. 尝试 `impl std::fmt::Display for String`，记录 E0117 并解释；
   再尝试 `impl<T> std::fmt::Display for T`，记录 E0210。
5. 给一个 trait 加关联常量和一个泛型方法，尝试 `Box<dyn Trait>`，
   记录 E0038；用 `where Self: Sized` 修复可 dyn 的部分。
6. 定义 `Summary` trait 与三个实现，用 `Vec<Box<dyn Summary>>` 做摘要；
   再改成枚举实现，比较两者的扩展性与体积。
7. 写一个 blanket impl 和一个具体 impl，复现 E0119，并说明为什么编译器不允许重叠。

## 参考

- The Rust Programming Language: 第 10.2 节「trait：定义共同行为」、
  第 17.2 节「为使用不同类型的值而设计的 trait 对象」
- Rust Reference: [Traits](https://doc.rust-lang.org/reference/items/traits.html)、
  [Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html)、
  [Trait implementation coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)
- Rust Reference: [Dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- 错误索引：[E0117](https://doc.rust-lang.org/error_codes/E0117.html)、
  [E0119](https://doc.rust-lang.org/error_codes/E0119.html)、
  [E0210](https://doc.rust-lang.org/error_codes/E0210.html)、
  [E0038](https://doc.rust-lang.org/error_codes/E0038.html)、
  [E0599](https://doc.rust-lang.org/error_codes/E0599.html)
