# 第 10 课 · 字符串：String、&str 与 UTF-8

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`10_strings_and_utf8.rs`

## 一、底层原理

### 10.1 三种字符串类型

| 类型 | 大小（x86_64） | 所有权 | 可变 | 用途 |
|---|---|---|---|---|
| `str` | 未定长（DST） | 无（只能借） | 否 | UTF-8 字节序列本身 |
| `&str` | 16 字节（ptr + len） | 借用 | 否 | 只读视图、函数参数 |
| `String` | 24 字节（ptr + len + cap） | 拥有 | 是 | 可增长的堆字符串 |

- `str` 是**未定长类型**，不能按值存在，只能出现在指针后面（`&str`、`Box<str>`、`Rc<str>`）
- `&str` 是"胖指针"：数据地址 + 字节长度，**不是** C 的 `char*`（C 没有长度、也没有
  UTF-8 保证）
- `String` 是拥有所有权的堆缓冲，布局和 `Vec<u8>` 一样（三个字），但**始终保证内容是合法 UTF-8**
- 字符串字面量的类型是 `&'static str`；`String` 可以通过 `Deref` 当成 `str` 用，
  `&String` 会自动退化为 `&str`

```rust
let owned: String = String::from("hi");   // 堆上
let borrowed: &str = &owned;              // 借用，不复制
let literal: &'static str = "hi";         // 静态数据
```

### 10.2 UTF-8 编码：字节 ≠ 字符

UTF-8 是**变长编码**：一个 Unicode 标量值占 1–4 个字节。

| 内容 | 字节数 | 字符数 |
|---|---|---|
| `"rust"` | 4 | 4 |
| `"中文"` | 6 | 2 |
| 一个 emoji | 4 | 1 |

因此 `str::len()` 返回的是**字节数**，不是字符数；字符数要 `chars().count()`（O(n)）。

更麻烦的是**字符边界**：不能随便按字节切片。

```rust
let s = "中文";
&s[0..3]     // "中"：合法（3 字节恰好是一个字符）
&s[0..2]     // panic：切在 UTF-8 字符中间
s[0]         // 编译错误：str 不能用整数索引
```

`s[0]` 被禁止（E0277），原因有三层：

1. UTF-8 是变长的，"第 0 个字符"在内存里不一定是第 0 个字节
2. 即使按字节索引，返回 1 个字节也无法构成 `char`（可能是多字节字符的一部分）
3. 按字符索引需要 O(n) 扫描，语言不想把它伪装成 O(1)

需要按字节访问时用 `as_bytes()`；需要按字符访问时用 `chars()`/`char_indices()`；
需要安全切片时用 `s.get(range)`（返回 `Option<&str>`，边界非法时为 `None`）。

### 10.3 `String` 的堆缓冲与 UTF-8 不变量

`String` 的结构等价于：

```
ptr ──▶ 堆上的 UTF-8 字节
len    当前字节长度
cap    已分配容量（≥ len）
```

- 增长时按摊还策略重新分配（通常翻倍），`with_capacity`/`reserve` 可预留
- 所有安全 API 都保持"内容合法 UTF-8"这一不变量：
  `push(char)`、`push_str(&str)`、`insert`、`replace_range` 都不会让你拼出半个字符
- 绕过检查的入口都是 `unsafe`：`String::from_utf8_unchecked`、`as_mut_vec`
- `Vec<u8>` 是任意字节，`String` 是"验证过的 UTF-8"；两者可以零拷贝互转：

| 转换 | 行为 |
|---|---|
| `String::into_bytes()` | 零拷贝，把缓冲交给 `Vec<u8>` |
| `String::as_bytes()` | 借用为 `&[u8]` |
| `String::from_utf8(vec)` | 校验，返回 `Result<String, FromUtf8Error>` |
| `String::from_utf8_lossy(&bytes)` | 非法字节替换为 `U+FFFD`，返回 `Cow<str>` |
| `unsafe { String::from_utf8_unchecked(vec) }` | 不校验，调用者负责合法性 |

`FromUtf8Error::utf8_error()` 返回底层的 `Utf8Error`，它提供 `valid_up_to()`
（第一个非法字节的位置）和 `error_len()`，解析网络报文/文件时很有用。

### 10.4 `char`、字形簇与 Unicode 的边界

- `char` 是 **Unicode 标量值**（4 字节），不是字形（grapheme）
- 一个"看起来的字符"可能由多个 `char` 组成：`"e" + 组合重音` 是 2 个 `char`（3 字节），
  但只显示为一个 `é`
- emoji 的 ZWJ 序列（如家庭 emoji）可能由多个 `char` 拼成
- `chars().count()` 数的是标量值数量，**不是**视觉字符数

```rust
"e\u{301}".len()            // 3 字节
"e\u{301}".chars().count()  // 2 个标量值
```

大小写转换也会改变长度：`"Straße".to_uppercase()` 是 `"STRASSE"`（`ß` → `SS`）。
Unicode 规范化（NFC/NFD）和字形簇切分不在标准库：

- 规范化：`unicode-normalization` crate
- 字形簇：`unicode-segmentation` crate

### 10.5 借用、切片与零拷贝

返回 `&str` 的 API 都**借用原字符串**，不分配：

```rust
let s = String::from("  hello world  ");
let trimmed = s.trim();            // &str，借用 s
let first = s.split_whitespace().next();  // Option<&str>
let found = s.find("world");       // Option<usize>（字节下标）
```

返回 `String` 的 API 会分配：`replace`、`to_uppercase`、`format!`、`join`。
`Cow<'a, str>` 用来表达"能借就借、必须改才分配"（第 07 课）。

函数参数优先用 `&str`：`String`、字面量、子切片都能传；只有需要"追加/扩容"时才用
`&mut String`，需要"拿走所有权"时才用 `String`。

### 10.6 拼接与格式化的成本

| 写法 | 分配 | 所有权 |
|---|---|---|
| `s.push_str("x")` | 可能扩容，不新建 | 保持 `s` |
| `s + "x"` | 复用 `s` 的缓冲 | **移动 `s`** |
| `format!("{a}{b}")` | 新分配 | 借用 `a`/`b` |
| `write!(&mut s, ...)` | 复用 `s` | 借用 `s` |
| `[a, b].join("")` | 新分配 | 借用元素 |

`String + String` 不合法（右边需要 `&str`）；`+` 会移动左边，之后不能再用。
循环里拼接优先 `push_str`/`write!`，避免每轮 `format!` 分配。

## 二、具体语法

### 11.1 创建

```rust
let empty = String::new();
let from_literal = String::from("hello");
let from_slice = "hello".to_string();
let from_format = format!("{}-{}", "Rust", 2024);
let with_capacity = String::with_capacity(64);
let from_bytes = String::from_utf8(vec![104, 105]).unwrap();  // "hi"
```

- 字符串字面量是 `&'static str`，不是 `String`
- `format!` 是最通用的构造方式，接受任意 `Display` 参数

### 11.2 修改与容量

```rust
let mut s = String::from("hello");
s.push(' ');            // 追加 char
s.push_str("world");    // 追加 &str
s.insert(0, '>');       // 在字节位置插入 char（必须是字符边界）
s.replace_range(0..1, "[");  // 按字节范围替换
s.truncate(5);          // 截断到字节长度（必须是字符边界）
s.pop();                // 删除最后一个 char，返回 Option<char>
```

- `insert`/`truncate`/`remove`/`replace_range` 的边界必须是字符边界，否则 panic
- `push` 接 `char`，`push_str` 接 `&str`
- `clear()` 只把 len 置 0，容量保留
- `reserve`/`shrink_to_fit` 控制容量

### 11.3 索引、切片与迭代

```rust
let s = String::from("a中b");

s.len();                    // 5 字节
s.chars().count();          // 3 个字符
s.as_bytes();               // &[u8]

&s[1..4];                   // "中"（合法边界）
s.get(1..4);                // Some("中")
s.get(1..3);                // None（切在字符中间）

for ch in s.chars() { ... }             // Unicode 标量值
for byte in s.as_bytes() { ... }        // 字节
for (i, ch) in s.char_indices() { ... } // 字节下标 + 字符
```

### 11.4 查找、替换、拆分

```rust
s.contains("中");            // 子串是否存在
s.starts_with("a");          // 前缀
s.find("b");                 // Option<usize>（字节下标）
s.replace("a", "x");         // 新 String

s.split(',');                // 按分隔符
s.split_whitespace();        // 按空白
s.split_once('=');           // Option<(&str, &str)>
s.lines();                   // 按行（处理 \n 与 \r\n）
s.trim();                    // 去掉首尾空白，返回 &str
```

- `split*`/`trim`/`lines` 返回的都是**借用原串的切片**
- `replace` 返回新的 `String`
- `lines()` 会去掉行尾的 `\r`，`split('\n')` 不会

### 11.5 转换

```rust
"42".parse::<i32>();         // Result<i32, ParseIntError>
42.to_string();              // String
String::from("hi");          // &str -> String
s.as_str();                  // &str
s.into_bytes();              // Vec<u8>（零拷贝）
String::from_utf8(bytes);    // Result
```

- `parse` 依赖 `FromStr`，错误类型可以用 `?` 传播
- `to_string` 通过 `Display` 分配新字符串
- 数字格式化用 `format!`/`write!` 的格式参数

### 11.6 格式化

```rust
format!("{name}={value}");
format!("{:>8}", "hi");       // 右对齐
format!("{:.2}", 3.14159);    // 两位小数
format!("{:#x}", 255);        // 0xff
write!(&mut buf, "{}", value).unwrap();  // 追加，不新建
```

- `Display`（`{}`）给人看，`Debug`（`{:?}`）给开发者看（带引号/转义）
- `write!` 需要 `use std::fmt::Write;`，错误类型是 `fmt::Error`

### 11.7 Unicode 操作

```rust
"abc".chars().rev().collect::<String>();  // "cba"
"Straße".to_uppercase();                  // "STRASSE"
ch.len_utf8();                            // 该字符的 UTF-8 字节长度
char::from_u32(0x4E2D);                   // Option<char>
```

- 反转字符串必须按 `char`（或字形簇）反转，按字节反转会产生非法 UTF-8
- 大小写转换可能改变长度，转换后不要沿用旧下标

### 11.8 OS 字符串

```rust
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
```

- `OsString`/`OsStr` 用于"操作系统字符串"：Windows 上可能不是合法 UTF-8
- `Path`/`PathBuf` 是路径专用包装，`to_string_lossy()` 把非 UTF-8 部分替换后再显示
- 跨平台代码不要假设 `&str` 能无损表示所有路径

## 三、实际应用与最佳实践

### 12.1 词频统计：键借用原文，零复制

```rust
let text = "the cat and the dog and the bird";
let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
for word in text.split_whitespace() {
    *counts.entry(word).or_insert(0) += 1;
}
```

`split_whitespace` 产出 `&str`，`BTreeMap<&str, _>` 直接借用 `text`，不复制任何单词。
需要按出现次数排序时，把 `(&str, usize)` 收集到 `Vec` 再 `sort_by_key`。

### 12.2 CSV 逐行解析

```rust
for line in data.lines().skip(1) {
    if let Some((_, score)) = line.split_once(',') {
        total += score.parse::<i32>().unwrap_or(0);
    }
}
```

`lines()` 处理 `\r\n`，`split_once` 返回两个借用切片，`parse` 做类型转换。
真实工程里建议用 `csv` crate 处理引号、转义和表头。

### 12.3 回文判断：按字符而不是字节

```rust
let cleaned: Vec<char> = text.chars()
    .filter(|c| c.is_alphanumeric())
    .flat_map(|c| c.to_lowercase())
    .collect();
cleaned.iter().eq(cleaned.iter().rev())
```

过滤标点、大小写折叠、按 `char` 比较。按字节比较会把多字节字符拆坏。

### 12.4 用 `write!` 复用缓冲区

```rust
let mut report = String::new();
writeln!(report, "name={}", "daily").unwrap();
writeln!(report, "lessons={}", 10).unwrap();
```

循环里拼接报告时，`write!` 比每轮 `format!` 少一次分配。
需要最终字符串时再 `report` 整体使用。

### 12.5 UTF-8 校验与容错

```rust
String::from_utf8(bytes)        // 严格：Err 时可用 valid_up_to 定位
String::from_utf8_lossy(&bytes) // 容错：非法字节替换为 U+FFFD
```

网络/文件输入优先严格校验；展示层可以用 lossy。不要用
`from_utf8_unchecked` 绕过检查，除非上游已经严格保证。

### 12.6 最佳实践清单

1. **参数用 `&str`**，返回借用片段也用 `&str`；需要拥有时才用 `String`
2. **区分字节与字符**：`len()` 是字节，`chars().count()` 才是字符数
3. **切片前确认边界**：不可信输入用 `get`，不要直接 `&s[a..b]`
4. **遍历优先 `chars()`/`char_indices()`**，不要按字节拼字符
5. **反转/截断按字符**，不要按字节操作 UTF-8
6. **大量拼接用 `push_str`/`write!`**，避免循环里 `format!`
7. **`+` 只用于 `String + &str`**，并注意它会移动左边
8. **`Cow<str>` 表达"借用或拥有"**，减少不必要的分配
9. **严格校验用 `from_utf8`**，展示用 `from_utf8_lossy`
10. **路径/OS 字符串用 `PathBuf`/`OsString`**，不要假设一定是 UTF-8

## 常见陷阱

1. **`s[0]` 不能编译（E0277）**：字符串没有整数索引；用 `chars().nth(0)`、
   `s.as_bytes()[0]` 或 `s.get(0..1)`
2. **按字节切片切在字符中间会 panic**：`&"中文"[0..2]`；用 `get` 或
   `char_indices` 找边界
3. **`len()` 当字符数**：`"中文".len()` 是 6，不是 2
4. **`String + String` 不合法**：右边必须是 `&str`；用 `format!` 或 `push_str`
5. **`+` 移动左边**：`let c = a + &b;` 之后 `a` 不可用
6. **`&String` 参数不必要**：用 `&str` 更通用；`&String` 只在需要容量 API 时用
7. **`chars().nth(i)` 在循环里是 O(n²)**：需要下标就 `char_indices` 或先收集
8. **`truncate`/`remove`/`insert` 边界 panic**：必须落在字符边界
9. **大小写转换改变长度**：`ß` → `SS`，转换后旧下标失效
10. **`from_utf8` 失败不处理**：用 `valid_up_to()` 定位非法字节
11. **`from_utf8_lossy` 改变长度**：替换字符是 3 字节，长度可能变长
12. **`unsafe from_utf8_unchecked` 传非法字节**：立即 UB
13. **`push` 传 `&str`**：`push` 只接 `char`，字符串用 `push_str`
14. **`trim` 以为返回 `String`**：它返回 `&str` 借用原串
15. **`split` 后修改原串**：切片借用未结束，编译器会阻止
16. **`replace` 在热循环里分配**：需要就地修改用 `replace_range`
17. **`split('\n')` 留下 `\r`**：跨平台文本用 `lines()`
18. **按字节反转字符串**：会产生非法 UTF-8，必须按 `char`/字形簇
19. **把 `char` 当字形**：组合重音、emoji 序列都是多个 `char`
20. **假设路径一定是 UTF-8**：Windows 路径可能包含非 UTF-8，用 `OsStr`/`Path`

## 完整示例

见 `10_strings_and_utf8.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "BASICS/10_字符串String与UTF8/10_strings_and_utf8.rs" -o run.exe
./run.exe
```

示例共 28 行输出，覆盖：`String` 3 字 / `&str` 2 字布局、`size_of_val` 的 `str` 长度、
UTF-8 字节/字符统计、容量关系、`push`/`push_str`、`+` 的所有权、`format!`、
`char_indices`、安全切片与 `get` 边界、`split`/`lines`/`trim`、`find`/`replace`、
`parse`、Unicode 大小写（`ß` → `SS`）、按字符反转、`&String` → `&str`，
最后用词频统计、CSV 求和、回文判断、`write!` 报告和 UTF-8 校验收尾。

## 练习

1. 写 `fn char_stats(text: &str) -> (usize, usize)`，返回（字节数，字符数），
   分别用 `"rust"`、`"中文"`、`"e\u{301}"` 测试。
2. 实现 `fn truncate_chars(text: &str, n: usize) -> &str`：保留前 n 个字符，
   用 `char_indices` 找边界，要求不 panic。
3. 统计一段英文的词频，并按次数从高到低输出（同次数按字母序），
   要求键使用 `&str` 而不是 `String`。
4. 解析 CSV 文本：首行是表头，后续每行 `name,score`；跳过非法行并汇总分数。
5. 实现 `fn is_palindrome(text: &str) -> bool`，忽略大小写、空白与标点；
   再用一个 emoji 字符串验证"按字符比较"与"按字节比较"的差异。
6. 实现 `fn reverse_words(text: &str) -> String`：单词顺序反转、单词内部不变，
   例如 `"hello world"` → `"world hello"`。
7. 构造一个含非法字节的 `Vec<u8>`，用 `String::from_utf8` 得到错误，
   通过 `err.utf8_error()` 打印 `valid_up_to()` 与 `error_len()`；
   再用 `from_utf8_lossy` 对比长度。
8. 比较两种 API：`fn f(s: &str)` 与 `fn g(s: &String)`，
   分别用 `String`、字面量、子切片调用，说明哪种更通用。

## 参考

- The Rust Programming Language: 第 8.2 节「使用字符串存储 UTF-8 编码的文本」
- std 文档：[`String`](https://doc.rust-lang.org/std/string/struct.String.html)、
  [`str`](https://doc.rust-lang.org/std/primitive.str.html)
- Rust Reference: [String literals](https://doc.rust-lang.org/reference/tokens.html#string-literals)、
  [String type](https://doc.rust-lang.org/std/string/struct.String.html)
- The Rustonomicon: [Exotically sized types](https://doc.rust-lang.org/nomicon/exotic-sizes.html)（DST 与胖指针）
- Unicode 标准：[UTF-8（RFC 3629）](https://www.rfc-editor.org/rfc/rfc3629)
- crate：`unicode-segmentation`（字形簇）、`unicode-normalization`（NFC/NFD）、`csv`（CSV 解析）
- 错误索引：[E0277](https://doc.rust-lang.org/error_codes/E0277.html)
