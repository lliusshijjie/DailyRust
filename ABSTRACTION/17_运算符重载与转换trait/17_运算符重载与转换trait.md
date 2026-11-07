# 第 17 课 · 运算符重载与转换 trait

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`17_operators_and_conversions.rs`

## 一、底层原理

### 17.1 运算符就是 trait 方法

Rust 没有"自定义运算符"语法，每个运算符都对应 `std::ops` 里的一个 trait：

| 表达式 | 展开为 | trait |
|---|---|---|
| `a + b` | `Add::add(a, b)` | `Add<Rhs>` |
| `a += b` | `AddAssign::add_assign(&mut a, b)` | `AddAssign<Rhs>` |
| `-a` | `Neg::neg(a)` | `Neg` |
| `a - b` / `a * b` / `a / b` / `a % b` | 对应 `sub`/`mul`/`div`/`rem` | `Sub`/`Mul`/`Div`/`Rem` |
| `a[i]` | `*Index::index(&a, i)` | `Index<Idx>` |
| `*a` | `Deref::deref(&a)` | `Deref` |
| `a == b` | `PartialEq::eq(&a, &b)` | `PartialEq<Rhs>` |
| `a < b` | `PartialOrd::partial_cmp(&a, &b)` | `PartialOrd<Rhs>` |
| `a & b` / `a \| b` / `a ^ b` | `bitand`/`bitor`/`bitxor` | `BitAnd` 等 |
| `!a` | `Not::not(a)` | `Not` |
| `a << b` / `a >> b` | `shl`/`shr` | `Shl`/`Shr` |
| `a()` | `Fn::call`/`FnMut`/`FnOnce` | 第 18 课 |

所以"运算符重载"本质是**为类型实现对应的 trait**：

```rust
impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point { ... }
}
```

`Add` 有两个可定制点：

- `Rhs`（右操作数类型，默认 `Self`）：`impl Add<i32> for Point` 让 `Point + i32` 合法
- `Output`（关联类型）：`Point + Point` 可以返回 `Point`，也可以返回别的类型
  （例如向量 + 向量 = 向量，但矩阵 * 矩阵 = 矩阵，`str + str` 在标准库里返回 `String`）

### 17.2 运算符重载的边界

不能做的事：

- **不能改变优先级/结合性**：`a + b * c` 永远先乘后加
- **不能创建新运算符**
- **不能重载 `&&`/`||` 的短路语义**（它们只接受 `bool`）
- **不能重载 `=`、`..`、`?`、`as`**
- **不能给外部类型实现外部运算符 trait**（孤儿规则，例如 `impl Add for i32` 不行）
- **`Index` 必须返回引用**，不能返回临时值（E0515）

容易忽略的点：

- `Add` **不会**自动带来 `AddAssign`；`+=` 要单独实现（E0368）
- 运算符**按值消费**操作数：`&a + &b` 需要为引用实现 `Add`，否则 E0369
- `Copy` 类型写 `a + b` 之后 `a`、`b` 仍可用；非 `Copy` 类型会被移动

### 17.3 转换 trait 家族

Rust 的转换 trait 分成几层：

| trait | 方向 | 失败 | 典型用途 |
|---|---|---|---|
| `From<T>` | T → Self | 不会失败 | 构造、单位转换 |
| `Into<T>` | Self → T | 不会失败 | `From` 的自动反向（blanket impl） |
| `TryFrom<T>` | T → Self | 可能失败 | 验证、范围检查 |
| `TryInto<T>` | Self → T | 可能失败 | `TryFrom` 的自动反向 |
| `FromStr` | `&str` → Self | 可能失败 | `str::parse` |
| `AsRef<T>` | &Self → &T | 不会失败 | 泛型 API 接收“像 &str 的东西” |
| `AsMut<T>` | &mut Self → &mut T | 不会失败 | 可变借用视图 |
| `Borrow<T>` | &Self → &T | 不会失败 | `HashMap` 键、`Cow` |
| `ToOwned` | &Self → Owned | 不会失败 | `Cow`、切片转 `Vec` |

关键规则：

- **实现 `From` 就自动得到 `Into`**（标准库有 blanket impl），不要手写 `impl Into`
  （会 E0119 冲突）
- `TryFrom` 同理自动得到 `TryInto`；必须指定 `type Error`
- `FromStr::Err` 在调用 `.unwrap()` 时需要 `Debug`
- `AsRef` 只承诺“能借到 `&T`”，没有 `Eq`/`Hash` 一致性要求
- `Borrow` 要求借用后的 `Eq`/`Ord`/`Hash` 与原类型一致（`HashMap` 的 `get` 依赖这一点）
- `as` 是原语之间的**位级/饱和转换**，不做类型安全保证；业务转换优先 `From`/`TryFrom`

### 17.4 孤儿规则与转换

转换 trait 都是外部 trait，能不能实现取决于孤儿规则：

| 写法 | 结果 | 原因 |
|---|---|---|
| `impl From<MyType> for String` | ✅ | 本地类型出现在 trait 参数里 |
| `impl From<u32> for MyType` | ✅ | `Self` 是本地类型 |
| `impl From<u32> for String` | ❌ E0117 | trait 和类型都外部 |
| `impl TryFrom<u64> for UserId` | ✅ | `Self` 本地 |
| `impl AsRef<str> for MyType` | ✅ | `Self` 本地 |

`From<MyType> for String` 看起来"给外部类型实现外部 trait"，但 `MyType` 是本地类型，
出现在 `From<MyType>` 的参数里，所以合法。真正禁止的是**参数里也没有本地类型**的情况。

### 17.5 `Deref` 与方法解析

```rust
impl Deref for Wrapper {
    type Target = str;
    fn deref(&self) -> &str { &self.0 }
}

let wrapper = Wrapper(String::from("hello"));
wrapper.len();              // 自动 deref 到 str::len
wrapper.starts_with('h');   // 同理
```

- `Deref` 让 `Wrapper` 自动获得 `Target` 的方法（方法解析沿 deref 链查找）
- `&Wrapper` 可以**deref coercion** 成 `&str`（函数参数/赋值处自动转换）
- `DerefMut` 提供可变版本
- `Deref` 不是继承：它只是"自动解引用"，不应为了复用方法而滥用；
  需要行为复用用 trait

### 17.6 比较与排序

| trait | 方法 | 要求 |
|---|---|---|
| `PartialEq` | `eq`/`ne` | 可为部分等价（浮点 `NaN`） |
| `Eq` | 标记 | 等价关系（自反/对称/传递） |
| `PartialOrd` | `partial_cmp` | 可能返回 `None`（不可比） |
| `Ord` | `cmp` | 全序，`sort`/`BTreeMap` 需要 |
| `Hash` | `hash` | 必须与 `Eq` 一致 |

- `#[derive(PartialEq, Eq, PartialOrd, Ord)]` 按字段声明顺序逐字段比较
- 浮点只实现 `PartialEq`/`PartialOrd`，不能 `Eq`/`Ord`（`NaN` 不满足全序）
- `Hash` 与 `Eq` 不一致会导致 `HashMap`/`HashSet` 行为错误
- 自定义比较要保证与 `Eq`/`Ord` 的语义一致

## 二、具体语法

### 18.1 算术运算符

```rust
impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point { ... }
}

impl Add<i32> for Point {
    type Output = Point;
    fn add(self, value: i32) -> Point { ... }
}

impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Point { ... }
}
```

- `Rhs` 默认是 `Self`；需要混合类型时写 `Add<T>`
- `Output` 可以是任意类型（可以是 `Self`，也可以是别的）
- 非 `Copy` 类型按值消费，`a + b` 之后 `a` 不可用

### 18.2 赋值运算符

```rust
impl AddAssign for Point {
    fn add_assign(&mut self, other: Point) {
        self.x += other.x;
        self.y += other.y;
    }
}
```

- 赋值 trait 返回 `()`，直接修改 `self`
- `Add`/`AddAssign` 各自独立，需要成对实现

### 18.3 `Index` / `IndexMut`

```rust
impl Index<usize> for Point {
    type Output = i32;
    fn index(&self, index: usize) -> &i32 {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("index out of range"),
        }
    }
}

point[0];        // 7
```

- 必须返回 `&Self::Output`
- 越界可以 panic，也可以自己设计 `get` 返回 `Option`
- `IndexMut` 提供 `&mut` 版本，且要求先实现 `Index`
- 实现 `Index<Range<usize>>` 可以让自定义容器支持切片语法

### 18.4 `Deref` / `DerefMut`

```rust
struct Wrapper(String);

impl Deref for Wrapper {
    type Target = str;
    fn deref(&self) -> &str { &self.0 }
}
```

- `Target` 是解引用后的类型
- 支持方法解析、deref coercion、`*wrapper`
- `DerefMut` 同理，返回 `&mut Target`

### 18.5 比较

```rust
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score { points: u32 }

scores.sort();
```

- 需要全序时 `derive(Ord)`；只要部分比较用 `PartialOrd`
- 手写 `Ord` 时务必与 `PartialOrd`/`Eq` 一致
- 需要按某个字段排序时用 `sort_by_key`

### 18.6 `From` / `Into`

```rust
impl From<Celsius> for Fahrenheit {
    fn from(value: Celsius) -> Self { ... }
}

let f: Fahrenheit = Celsius(100.0).into();
```

- 只实现 `From`，`Into` 自动获得
- 转换应保持"不失败"；可能失败用 `TryFrom`
- 双向转换要分别实现两个 `From`（避免循环 `into`）

### 18.7 `TryFrom` / `TryInto`

```rust
impl TryFrom<i32> for Even {
    type Error = &'static str;
    fn try_from(value: i32) -> Result<Self, Self::Error> { ... }
}

let even: Even = 4i32.try_into().unwrap();
```

- 必须指定 `Error`
- 失败信息应可读，调用方用 `?`、`unwrap`、`match` 处理
- 标准库为整数类型间提供了大量 `TryFrom`

### 18.8 `FromStr`

```rust
impl FromStr for Version {
    type Err = &'static str;
    fn from_str(text: &str) -> Result<Self, Self::Err> { ... }
}

let version: Version = "1.2".parse().unwrap();
```

- `str::parse::<T>()` 就是 `T::from_str`
- `Err` 需要 `Debug` 才能在 `unwrap` 里打印
- 解析错误要区分"缺少分隔符/数字格式错误"等

### 18.9 `AsRef` / `Borrow`

```rust
fn text_len<T: AsRef<str>>(value: T) -> usize {
    value.as_ref().len()
}

text_len("abc");                  // 3
text_len(String::from("hello"));  // 5
```

- `AsRef` 让 API 接收"能当 `&str` 用"的多种类型
- `Borrow` 用于 `HashMap` 键等场景，额外要求 `Eq`/`Hash` 一致
- 不要为"转换"实现 `AsRef`，除非它确实只是廉价借用

### 18.10 `Display` 与 `ToString`

```rust
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.1}, {:.1})", self.x, self.y)
    }
}
```

- 实现 `Display` 自动获得 `ToString`
- `Debug` 给开发者看，`Display` 给用户看
- 不要为同一个类型同时写矛盾的 `Display` 语义

## 三、实际应用与最佳实践

### 19.1 `Point` / `Vector2`：运算符表达数学语义

```rust
let sum = a + b;         // Vector2 + Vector2
let scaled = a * 2.0;    // Vector2 * f64
a.dot(b);                // 不是运算符，用方法
```

运算符应该对应**领域里自然的运算**：向量加法、标量乘法、点积（点积不是"乘法"，
所以用方法而不是 `*`）。

### 19.2 单位转换：`Celsius` → `Fahrenheit`

```rust
let fahrenheit: Fahrenheit = Celsius(100.0).into();
```

`From` 表达"无损/不失败的转换"；两个单位都保留独立类型，避免混用。
转换方向明确时，`From` + `Into` 让调用点读起来像自然语言。

### 19.3 校验转换：`Even` / `UserId`

```rust
let even: Even = 4i32.try_into().unwrap();
Even::try_from(3).is_err();      // true
```

`TryFrom` 把"值必须满足不变量"放进构造过程；调用方拿到的一定是合法值。
ID、金额、范围、枚举字符串解析都适合用 `TryFrom`/`FromStr`。

### 19.4 解析与排序：`Version` / `Score`

```rust
let version: Version = "1.2".parse().unwrap();
scores.sort();                   // Score: Ord
```

`FromStr` + `Ord` 让自定义类型融入标准库生态：`parse`、`sort`、`BTreeMap` 都能直接用。

### 19.5 最佳实践清单

1. **运算符只用于领域里自然的运算**，不要为了炫技重载
2. **`Add` 与 `AddAssign` 成对实现**，让 `+=` 也符合直觉
3. **`Rhs` 用具体类型**（`Add<i32>`）表达混合运算，不要滥用泛型
4. **`Index` 越界行为要明确**：panic 或提供 `get`
5. **`Deref` 只用于智能指针/透明包装**，不要用它模拟继承
6. **转换优先 `From`/`TryFrom`**，不要写 `as` 做业务转换
7. **只实现 `From`，不要手写 `Into`**（自动 blanket impl）
8. **可能失败的转换用 `TryFrom`/`FromStr`**，并给出清晰的 `Error`
9. **`AsRef` 用于廉价借用**，`Borrow` 才用于键/哈希语义
10. **比较 trait 保持一致**：`Eq`/`Ord`/`Hash` 不能互相矛盾

## 常见陷阱

1. **`Add` 不自动带来 `AddAssign`（E0368）**：`a += b` 需要单独实现
2. **`&T + &T` 不编译（E0369）**：运算符按值消费；为引用实现 `Add` 或用 `*`
3. **给外部类型实现外部 trait（E0117）**：`impl From<u32> for String` 不允许；
   本地类型出现在 trait 参数里才允许（`From<MyType> for String`）
4. **手写 `Into` 冲突（E0119）**：标准库已有 blanket impl；只实现 `From`
5. **`Index` 返回临时值（E0515）**：必须返回引用；不要返回 `&(self.x + 1)`
6. **`Index` 越界 panic**：需要 `Option` 语义时自己提供 `get` 方法
7. **`Deref` 滥用**：把所有方法都暴露出去，破坏封装；用 trait 组合
8. **比较不一致**：`PartialEq` 与 `Ord`/`Hash` 矛盾会让 `HashMap`/排序行为诡异
9. **浮点 `Eq`/`Ord`**：`NaN` 不满足全序；只实现 `PartialEq`/`PartialOrd`
10. **`TryFrom::Error` 缺少 `Debug`**：`unwrap`/`assert` 需要
11. **`FromStr::Err` 太模糊**：解析失败要能区分原因
12. **`as` 当业务转换**：截断/饱和/丢符号，优先 `From`/`TryFrom`
13. **运算符优先级误判**：`a + b * c` 不会按你的类型改变
14. **`Display` 与 `Debug` 语义混用**：用户输出用 `Display`，内部排查用 `Debug`
15. **`Borrow` 与 `AsRef` 选错**：`HashMap` 键需要 `Borrow` 的等值/哈希一致性

## 完整示例

见 `17_operators_and_conversions.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/17_运算符重载与转换trait/17_operators_and_conversions.rs" -o run.exe
./run.exe
```

示例共 16 行输出，覆盖：`Point + Point`/`Point + i32` 两种 `Rhs`、`Neg`、`AddAssign`、
`Index`、`Deref`、`From`/`Into`、`TryFrom`、`FromStr`、`AsRef`、`Ord` 排序，
最后用 `Vector2` 运算符与单位转换收尾。

## 练习

1. 给 `Point` 实现 `Sub`、`Mul<i32>`、`IndexMut`，并让 `point[0] = 5` 可用。
2. 实现 `Vector2` 的 `Sub`、`Div<f64>`、`AddAssign`，并保持 `dot`/`norm` 方法。
3. 写 `Money(i64)` 的 `AddAssign`，用 `checked_add` 防止溢出；溢出时 panic 还是
   返回 `Result`，说明你的取舍。
4. 实现 `Celsius` ↔ `Fahrenheit` 双向 `From`，再用 `TryFrom<f64>` 拒绝低于绝对零度的值。
5. 实现 `UserId(u64)` 的 `TryFrom<u64>` 与 `FromStr`，拒绝 0。
6. 给 `Version` 实现 `Display` 与 `Ord`，用 `sort` 对多个版本排序并打印。
7. 为自定义 `Matrix` 实现 `Index<(usize, usize)>`，越界时 panic；
   再提供 `get` 返回 `Option`。
8. 比较 `as`、`From`、`TryFrom` 三种转换在截断/失败/类型安全上的差异，各举一例。

## 参考

- The Rust Programming Language: 第 19.2 节「高级 trait」（运算符重载、关联类型）
- Rust Reference: [Operator expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html)
- std 文档：[`std::ops`](https://doc.rust-lang.org/std/ops/index.html)、
  [`std::convert`](https://doc.rust-lang.org/std/convert/index.html)、
  [`std::str::FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html)
- Rust API Guidelines: [Conversions](https://rust-lang.github.io/api-guidelines/naming.html#ad-hoc-conversions-follow-as_-to_-into_-conventions-c-conv)
- 错误索引：[E0117](https://doc.rust-lang.org/error_codes/E0117.html)、
  [E0119](https://doc.rust-lang.org/error_codes/E0119.html)、
  [E0368](https://doc.rust-lang.org/error_codes/E0368.html)、
  [E0369](https://doc.rust-lang.org/error_codes/E0369.html)、
  [E0515](https://doc.rust-lang.org/error_codes/E0515.html)
