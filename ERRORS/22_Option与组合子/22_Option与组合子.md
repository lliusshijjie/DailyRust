# 第 22 课 · Option 与组合子

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`22_option_and_combinators.rs`

## 一、底层原理

### 22.1 `Option<T>` 是"缺失值"的显式建模

```rust
pub enum Option<T> {
    None,
    Some(T),
}
```

Rust 没有 `null`。可能缺失的值必须写成 `Option<T>`，调用方必须处理 `None`。
这比"用 `-1`/空串/`nullptr` 表示缺失"更安全：类型系统强迫你面对"没有值"的分支。

| 写法 | 语义 | 风险 |
|---|---|---|
| `Option<T>` | 有值 / 无值 | 必须显式处理 |
| C 的 `NULL` | 无值 | 解引用空指针 |
| Java 的 `null` | 无值 | `NullPointerException` |
| 哨兵值 `-1`/`""` | 魔法值 | 与真实值混淆 |

### 22.2 niche 优化：`Option` 可以不占额外空间

如果 `T` 有"无效位模式"，`Option<T>` 会用这个无效值表示 `None`：

| 类型 | 大小 | 机制 |
|---|---|---|
| `Option<&T>` | 同 `&T` | 空指针 |
| `Option<Box<T>>` | 同 `Box<T>` | 空指针 |
| `Option<NonZeroU32>` | 4 | `0` 无效 |
| `Option<bool>` | 1 | `2` 无效 |
| `Option<u8>` | 2 | 没有 niche，需要标签 |

所以 `Option<&T>` 就是"可空指针"的零成本抽象；而 `Option<u8>` 会多 1 字节。
嵌套的 `Option<Option<T>>` 通常需要更多标签（除非内层有 niche）。

### 22.3 `Option` 的三种消费方式

| 方式 | 写法 | 适用 |
|---|---|---|
| 模式匹配 | `match opt { Some(v) => ..., None => ... }` | 需要分支逻辑 |
| 组合子 | `opt.map(...).and_then(...)` | 链式转换 |
| `?` 传播 | `let value = opt?;` | 函数返回 `Option` |

`?` 在 `Option` 函数里遇到 `None` 会提前返回 `None`；在 `Result` 函数里遇到 `None`
需要先转成 `Result`（`ok_or`/`ok_or_else`）。

### 22.4 组合子分类

| 类别 | 方法 | 说明 |
|---|---|---|
| 查询 | `is_some`、`is_none`、`contains`、`is_some_and` | 不取负载 |
| 引用视图 | `as_ref`、`as_mut`、`as_deref` | 借用而非移动 |
| 变换 | `map`、`map_or`、`map_or_else`、`and_then`、`filter`、`flatten` | 链式处理 |
| 组合 | `and`、`or`、`or_else`、`zip` | 合并两个 `Option` |
| 取值 | `unwrap`、`expect`、`unwrap_or`、`unwrap_or_else`、`unwrap_or_default` | 拿到 `T` |
| 可变 | `take`、`replace`、`get_or_insert`、`get_or_insert_with`、`insert` | 操作 `&mut Option<T>` |
| 迭代 | `iter`、`into_iter` | 0/1 个元素的迭代器 |
| 转换 | `ok_or`、`ok_or_else`、`transpose` | 与 `Result` 互转 |

惰性/立即求值的区别很重要：

- `map_or(default, f)`：`default` **立即求值**
- `map_or_else(default_fn, f)`：`default_fn` 只在 `None` 时调用
- `or(other)`：`other` **立即求值**
- `or_else(f)`：`f` 只在 `None` 时调用
- `ok_or(err)`：`err` 立即求值；`ok_or_else(f)` 惰性

### 22.5 `Option` 与所有权

- `Option<T>` 是 `Copy` 当且仅当 `T: Copy`；`Option<&T>` 是 `Copy`
- `opt.as_ref()` 把 `Option<T>` 借成 `Option<&T>`，避免移动
- `opt.take()` 从 `&mut Option<T>` 取出值并留下 `None`
- `opt.replace(value)` 换入新值并返回旧值
- 对 `Option<T>` 按值匹配会移动 `T`；只想读就匹配 `&opt`

### 22.6 `Option` vs `Result`

| | `Option<T>` | `Result<T, E>` |
|---|---|---|
| 语义 | 有/无 | 成功/失败 + 原因 |
| 失败信息 | 无 | 有 `E` |
| 典型场景 | 查找、可选配置 | IO、解析、业务错误 |
| `?` | 只能用于 `Option` 函数 | 只能用于 `Result` 函数（或 `From` 转换） |

"找不到"用 `Option`；"为什么失败"用 `Result`。不要用 `Option` 承载错误原因，
也不要用 `Result` 表达只是"可能没有"的语义。

## 二、具体语法

### 23.1 构造

```rust
let some = Some(42);
let none: Option<i32> = None;
let defaulted: Option<i32> = Default::default();   // None
let from_value = Option::from(7);                   // Some(7)
let parsed: Option<i32> = "42".parse().ok();
```

- `None` 需要类型标注（或由上下文推断）
- `Option::from(x)` 等价于 `Some(x)`（`impl From<T> for Option<T>`）
- `parse().ok()` 把 `Result` 转成 `Option`，丢弃错误信息

### 23.2 查询

```rust
opt.is_some();
opt.is_none();
opt.contains(&42);
opt.is_some_and(|value| value > 0);
```

- `is_some_and` 避免"`map(...).unwrap_or(false)`"的样板
- `contains` 要求 `T: PartialEq`

### 23.3 引用视图

```rust
let owned = Some(String::from("hello"));
let borrowed: Option<&str> = owned.as_deref();   // Option<String> -> Option<&str>
let by_ref: Option<&String> = owned.as_ref();    // 不移动
let by_mut: Option<&mut String> = owned.as_mut();
```

- `as_ref`/`as_mut` 是 `Option<&T>`/`Option<&mut T>`
- `as_deref` 做 `Deref` 转换（`Option<String>` → `Option<&str>`、`Option<Box<T>>` → `Option<&T>`）
- 在只想读的场景用引用视图，避免 `Option<T>` 被移动

### 23.4 变换与链式

```rust
Some(5).map(|value| value * 2);                       // Some(10)
Some("42").and_then(|text| text.parse::<i32>().ok()); // Some(42)
None.or_else(|| Some(0));                              // Some(0)
Some(5).filter(|value| *value > 10);                   // None
Some(Some(3)).flatten();                               // Some(3)
Some(1).zip(Some('a'));                                // Some((1, 'a'))
```

- `map` 的闭包返回普通值；`and_then` 的闭包返回 `Option`
- `filter` 保留满足谓词的 `Some`，否则变 `None`
- `flatten` 把 `Option<Option<T>>` 压平
- `zip` 两个都有值才是 `Some((a, b))`

### 23.5 取值

```rust
opt.unwrap();                       // None 时 panic
opt.expect("missing value");        // panic 时带消息
opt.unwrap_or(0);                   // None 时给默认值
opt.unwrap_or_else(|| compute());   // 默认值惰性计算
opt.unwrap_or_default();            // 要求 T: Default
```

- 生产代码里 `unwrap` 只用于"逻辑上不可能为 None"的位置，并写清理由
- `unwrap_or_else` 适合默认值需要分配/计算的场景
- `?` 是最常用的"提前返回"

### 23.6 可变操作

```rust
let mut slot: Option<i32> = None;
slot.get_or_insert(7);                 // None 时插入 7，返回 &mut i32
slot.get_or_insert_with(|| 7);         // 惰性插入
slot.take();                           // 取出并留下 None
slot.replace(9);                       // 换入 9，返回旧值
```

- `get_or_insert_with` 是缓存/惰性初始化的常用写法
- `take` 需要 `&mut Option<T>`，常用在状态机/所有权转移

### 23.7 与 `Result` 互转

```rust
opt.ok_or("missing")?;                  // Option -> Result（立即求值错误）
opt.ok_or_else(|| format!("missing"))?; // 错误惰性构造

let transposed: Result<Option<i32>, &str> =
    Some(Ok::<i32, &str>(1)).transpose();
```

- `ok_or`/`ok_or_else` 把 `None` 变成 `Err`
- `transpose`：`Option<Result<T, E>>` ↔ `Result<Option<T>, E>`
- `?` 不能跨类型自动转换，必须显式 `ok_or`

### 23.8 `Option` 作为迭代器

```rust
for value in Some(1) { ... }        // 迭代 0 或 1 次
Some(1).into_iter().sum::<i32>();   // 1
opt.iter();                          // Iterator<Item = &T>
```

- `Option<T>` 实现 `IntoIterator`，可以无缝接进迭代器链
- `filter_map` 接受返回 `Option` 的闭包，就是为这个场景设计的

## 三、实际应用与最佳实践

### 24.1 配置链：查表 → 解析 → 默认值

```rust
let port: Option<u16> = config.get("port").and_then(|raw| raw.parse().ok());
let port = port.unwrap_or(8080);
```

`and_then` 把"可选值 + 可失败解析"串起来；`unwrap_or` 给默认值。
需要错误原因时改用 `Result` + `ok_or_else`。

### 24.2 查找：`find` + `copied`

```rust
fn find_user<'a>(users: &'a [&'a str], name: &str) -> Option<&'a str> {
    users.iter().copied().find(|user| *user == name)
}
```

"找不到"是正常业务分支，返回 `Option` 而不是抛异常/返回空串。

### 24.3 安全解析：`filter_map`

```rust
let total: i32 = ["10", "x", "30"]
    .iter()
    .filter_map(|text| text.parse::<i32>().ok())
    .sum();
```

非法项自动跳过；需要记录失败原因时改成 `map` + `collect::<Result<...>>`。

### 24.4 缓存：`get_or_insert_with`

```rust
let mut cache: Option<String> = None;
let value = cache.get_or_insert_with(|| String::from("computed"));
```

第一次访问才计算；之后直接复用。`Option<T>` 是最简单的单值缓存。

### 24.5 最佳实践清单

1. **用 `Option` 表达"可能没有"**，不要用哨兵值
2. **优先组合子/`?`**，少写 `match Some/None`
3. **`map` 与 `and_then` 分清**：闭包返回 `Option` 就用 `and_then`
4. **默认值惰性**：能 `unwrap_or_else`/`or_else` 就别用立即求值版本
5. **只读用 `as_ref`/`as_deref`**，避免移动
6. **`take`/`replace` 处理 `&mut Option<T>`**
7. **`ok_or_else` 转 `Result`**，保留错误原因
8. **`flatten` 压平嵌套 `Option`**，不要写 `and_then(|x| x)`
9. **`filter_map` 适合"解析失败就跳过"**
10. **`unwrap` 只用于不可能失败的位置**，并写注释说明为什么

## 常见陷阱

1. **`unwrap()` 遇到 `None` panic**：生产代码要处理 `None` 或用 `unwrap_or`
2. **`?` 用在非 `Option`/`Result` 函数（E0277）**：`fn main()` 里不能直接 `opt?`；
   改成返回 `Option`/`Result` 的函数，或 `match`
3. **`Option<Option<T>>` 不会自动压平**：`Some(Some(3))` 需要 `.flatten()`
4. **`and_then` 闭包返回非 `Option`**：需要返回普通值时用 `map`
5. **`map` 里返回 `Option` 得到 `Option<Option<T>>`**：该用 `and_then`
6. **`or` 的右值立即求值**：昂贵构造用 `or_else`
7. **`map_or` 的默认值立即求值**：需要惰性用 `map_or_else`
8. **`ok_or` 的错误值立即求值**：需要构造 String 时用 `ok_or_else`
9. **`take` 后原 `Option` 变 `None`**：之后不能再用旧值
10. **`as_ref` 与移动混淆**：按值 `match` 会移动，`as_ref` 不会
11. **`Option<&T>` 是 Copy，`Option<String>` 不是**：按值传递会移动
12. **`unwrap_or_default` 要求 `T: Default`**：自定义类型没实现就编译失败
13. **`is_some_and` 替代 `map(...).unwrap_or(false)`**：更清晰且不分配
14. **`filter` 放在 `map` 之后**：先映射再过滤可能多做事；能先过滤就先生成
15. **`Option` 当迭代器时语义是 0/1 个元素**：`for x in opt` 可能一次都不执行
16. **用 `Option` 承载错误原因**：丢失"为什么失败"；该用 `Result`
17. **`Some(None)` 与 `None` 混淆**：语义不同（"有值且值为空" vs "没有值"）
18. **非 niche 类型的 `Option` 有额外字节**：高频小结构体注意布局
19. **忽略 `#[must_use]` 警告**：`opt.map(...)` 的结果不处理通常意味着 bug
20. **`expect` 消息太随意**：panic 信息应能定位调用点和原因

## 完整示例

见 `22_option_and_combinators.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ERRORS/22_Option与组合子/22_option_and_combinators.rs" -o run.exe
./run.exe
```

示例共 19 行输出，覆盖：niche 布局、`Some`/`None`、`Default`/`Option::from`、
`?` 传播、`map`/`and_then`/`or_else`/`filter`/`map_or`/`map_or_else`、`take`、
`as_deref`、`zip`/`flatten`/`transpose`、`Option` 迭代，最后用配置链、查找、
安全解析和 `get_or_insert_with` 缓存收尾。

## 练习

1. 写 `fn parse_positive(text: &str) -> Option<i32>`，用 `?` 传播解析失败，
   再拒绝非正数。
2. 从 `BTreeMap<&str, &str>` 读取 `port` 与 `host`，解析失败时用默认值；
   比较 `unwrap_or` 与 `unwrap_or_else` 的求值时机。
3. 实现 `fn find_user<'a>(users: &'a [&'a str], name: &str) -> Option<&'a str>`，
   分别测试命中与未命中。
4. 用 `get_or_insert_with` 实现一个"只计算一次"的配置缓存，并统计闭包调用次数。
5. 比较 `Some(Some(3)).flatten()` 与 `Some(3)` 的类型/语义差异。
6. 用 `transpose` 在 `Option<Result<T, E>>` 与 `Result<Option<T>, E>` 之间转换。
7. 用 `filter_map` 解析一组字符串并求和；再改成 `collect::<Result<...>>`，比较行为。
8. 写一个返回 `Option` 的查找函数，在 `main` 里分别用 `match`、`if let`、`?`
   （封装到另一个函数）三种方式处理。

## 参考

- The Rust Programming Language: 第 6 章「枚举和模式匹配」、第 9 章「错误处理」
- std 文档：[`Option`](https://doc.rust-lang.org/std/option/enum.Option.html)
- Rust by Example: [Option](https://doc.rust-lang.org/rust-by-example/std/option.html)
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
