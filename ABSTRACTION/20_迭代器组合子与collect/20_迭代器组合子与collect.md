# 第 20 课 · 迭代器组合子与 collect

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`20_iterator_adapters_and_collect.rs`

## 一、底层原理

### 20.1 组合子 = 惰性适配器 + 消费者

迭代器的世界里有两类东西：

| 类别 | 作用 | 例子 |
|---|---|---|
| 适配器（adapter） | 返回新迭代器，不执行 | `map`、`filter`、`zip`、`scan` |
| 消费者（consumer） | 驱动 `next()`，产出结果 | `collect`、`fold`、`sum`、`any` |

`collect` 本身不是魔法，它只是调用目标类型的 `FromIterator::from_iter`：

```rust
pub trait FromIterator<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}
```

所以"能 `collect` 成什么"完全由**目标类型的 `FromIterator` 实现**决定。
和它配套的是 `Extend`：

```rust
pub trait Extend<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T);
}
```

- `collect` 创建新容器
- `extend` 往已有容器追加，复用已有分配
- 很多容器同时实现两者；`partition` 依赖 `Default + Extend`

`collect::<Vec<_>>()` 会参考 `size_hint` 预分配，但不是"精确长度"保证。

### 20.2 `collect` 的目标类型

| 目标 | 效果 |
|---|---|
| `Vec<T>` | 保序、可重复 |
| `VecDeque<T>` | 双端队列 |
| `BTreeSet<T>` / `HashSet<T>` | 去重；BTree 有序、Hash 无序 |
| `BTreeMap<K,V>` / `HashMap<K,V>` | 键值；重复键**后者覆盖前者** |
| `String` | 从 `char` 或 `&str` 拼接 |
| `Result<Vec<T>, E>` | 遇到第一个 `Err` 短路 |
| `Option<Vec<T>>` | 遇到 `None` 短路 |
| `Box<[T]>` / `Rc<[T]>` / `Arc<[T]>` | 定长切片容器 |
| 自定义类型 | 实现 `FromIterator` 即可 |

几个重要行为：

```rust
let map: BTreeMap<i32, &str> = [(1, "a"), (1, "b")].into_iter().collect();
// {1: "b"}：重复键由后者覆盖

let parsed: Result<Vec<i32>, _> = ["1", "x"].iter()
    .map(|s| s.parse::<i32>())
    .collect();   // Err(ParseIntError)
```

- `BTreeMap` 按 key 排序，`HashMap` 顺序不确定
- `Result`/`Option` 的 `collect` 会在第一个失败处停止，不再消费后面的元素
- `collect::<String>()` 的 `Item` 可以是 `char` 或 `&str`

### 20.3 组合子分类

| 类别 | 方法 | 说明 |
|---|---|---|
| 变换 | `map`、`filter`、`filter_map`、`flat_map`、`flatten`、`scan`、`inspect` | 一对一/一对多/带状态 |
| 组合 | `chain`、`zip`、`unzip`、`partition` | 拼接/配对/拆分 |
| 折叠 | `fold`、`reduce`、`try_fold`、`sum`、`product` | 聚合成单值 |
| 选择 | `find`、`position`、`any`、`all`、`min`、`max`、`min_by_key`、`max_by_key` | 短路/极值 |
| 截断 | `take`、`skip`、`step_by`、`take_while`、`skip_while` | 惰性截取 |
| 工具 | `once`、`empty`、`repeat`、`repeat_with`、`successors`、`from_fn` | 构造迭代器 |

设计上的对称性：

- `filter().map()` 与 `filter_map()`：后者少一层适配器，闭包返回 `Option`
- `map().flatten()` 与 `flat_map()`：后者一次完成"映射 + 展开"
- `fold` 与 `reduce`：`reduce` 用第一个元素当初始值，空迭代器返回 `None`
- `fold` 与 `try_fold`：后者可以在中途用 `Err`/`None` 短路

### 20.4 自定义 `FromIterator` / `Extend`

```rust
impl FromIterator<i32> for Stats {
    fn from_iter<I: IntoIterator<Item = i32>>(iter: I) -> Self {
        let mut stats = Stats { count: 0, sum: 0, min: i32::MAX, max: i32::MIN };
        for value in iter {
            stats.count += 1;
            stats.sum += value;
            stats.min = stats.min.min(value);
            stats.max = stats.max.max(value);
        }
        stats
    }
}

let stats: Stats = [3, 1, 4, 2].into_iter().collect();
```

`FromIterator` 让自定义类型无缝接入 `collect`；`Extend` 让它可以复用缓冲：

```rust
let mut buffer = vec![1];
buffer.extend(2..=3);      // [1, 2, 3]
```

### 20.5 组合子的开销

- 适配器是普通结构体，单态化后可内联，最终接近手写循环
- `collect` 会预分配（依赖 `size_hint`），比逐个 `push` 稳
- **中间 `collect` 才是额外开销**：`filter().collect::<Vec<_>>().iter().map()` 多分配一次
- `partition`/`unzip` 各创建两个容器，内存是两份
- `scan`/`fold` 只维护一个累加器，不额外分配
- `Box<dyn Iterator>` 每层一次 vtable 间接，热路径避免

### 20.6 组合子与所有权

- `zip` 按值消费两个迭代器；`unzip` 把 `(A, B)` 拆成两个集合
- `partition` 消费原迭代器，产出两个新集合
- `by_ref()` 借用一个迭代器，允许"只消费一部分，之后继续用"
- `scan` 的状态由闭包持有；`try_fold` 的累加器按值传递
- `collect` 消费迭代器；之后不能再用原迭代器

## 二、具体语法

### 21.1 `collect` 到不同容器

```rust
let numbers = vec![3, 1, 3, 2];

let v: Vec<i32> = numbers.iter().copied().collect();
let d: VecDeque<i32> = numbers.iter().copied().collect();
let s: BTreeSet<i32> = numbers.iter().copied().collect();

let counts: BTreeMap<i32, usize> = numbers.iter().fold(BTreeMap::new(), |mut map, value| {
    *map.entry(*value).or_insert(0) += 1;
    map
});
```

- 同一份数据可以多次 `collect`（每次重新遍历）
- `collect` 的目标类型必须能从上下文推断，否则 E0283

### 21.2 `filter_map` / `flat_map`

```rust
let values: Vec<i32> = ["1", "x", "2"]
    .iter()
    .filter_map(|text| text.parse::<i32>().ok())
    .collect();

let letters: String = ["ab", "cd"]
    .iter()
    .flat_map(|text| text.chars())
    .collect();
```

- `filter_map`：闭包返回 `Option`，`None` 被丢弃
- `flat_map`：闭包返回一个迭代器，元素被展平拼接
- `flatten` 是 `flat_map` 的特例（元素本身是迭代器）

### 21.3 `fold` / `reduce` / `scan` / `try_fold`

```rust
let folded = (1..=4).fold(0, |acc, value| acc + value);      // 10

let reduced = (1..=4).reduce(|acc, value| acc + value);      // Some(10)

let scanned: Vec<i32> = (1..=4)
    .scan(0, |state, value| { *state += value; Some(*state) })
    .collect();                                               // [1, 3, 6, 10]

let checked: Result<i32, &str> = [5, -2, 3].into_iter()
    .try_fold(0, |acc, value| if value < 0 { Err("negative") } else { Ok(acc + value) });
```

- `fold` 需要显式初值；`reduce` 用第一个元素，空迭代器返回 `None`
- `scan` 闭包返回 `Option`，`None` 提前结束
- `try_fold` 闭包返回 `Result`/`Option`，失败即短路

### 21.4 `zip` / `unzip` / `partition`

```rust
let zipped: Vec<(i32, char)> = [1, 2].into_iter().zip(['a', 'b']).collect();

let pairs = [(1, 'a'), (2, 'b')];
let (numbers, letters): (Vec<i32>, Vec<char>) = pairs.into_iter().unzip();

let (evens, odds): (Vec<i32>, Vec<i32>) =
    (1..=4).partition(|value| value % 2 == 0);
```

- `zip` 以较短一侧为准
- `unzip`/`partition` 需要目标容器类型标注
- `partition` 的谓词决定元素去左边还是右边

### 21.5 选择类组合子

```rust
(1..=10).find(|value| value % 7 == 0);       // Some(7)
(1..=10).any(|value| value == 5);             // true
(1..=10).all(|value| value > 0);              // true
words.iter().min_by_key(|word| word.len());   // 最短
words.iter().max_by_key(|word| word.len());   // 最长
```

- `find`/`any`/`all` 短路
- `min_by_key`/`max_by_key` 的 key 函数可能被多次调用，应保持廉价、无副作用
- `min`/`max` 要求 `Ord`，浮点不可用

### 21.6 `extend` 复用容器

```rust
let mut buffer = vec![1];
buffer.extend(2..=3);            // [1, 2, 3]
buffer.extend([4, 5]);           // [1, 2, 3, 4, 5]
```

- 与 `collect` 的区别：`extend` 不创建新容器，保留已有分配
- 循环里累积结果时优先 `extend`，而不是反复 `collect`

### 21.7 `once` / `empty` / `repeat`

```rust
std::iter::once(0).chain(1..=2);          // 0,1,2
std::iter::empty::<i32>();                 // 空
std::iter::repeat(7).take(3);              // 7,7,7
std::iter::repeat_with(|| 7).take(3);      // 惰性生成
```

- `once`/`empty` 用来补位、构造可空迭代器
- `repeat` 会立即复制同一个值；`repeat_with` 每次调用闭包
- 无限迭代器必须配合 `take`/`zip` 等截断

### 21.8 自定义 `FromIterator`

```rust
impl FromIterator<i32> for Stats {
    fn from_iter<I: IntoIterator<Item = i32>>(iter: I) -> Self { ... }
}

let stats: Stats = [3, 1, 4, 2].into_iter().collect();
```

- 实现后，`collect::<Stats>()` 即可用
- 也可以实现 `Extend<i32>` 支持 `stats.extend(...)`
- 自定义容器要同时考虑 `size_hint`，让 `collect` 能预分配

## 三、实际应用与最佳实践

### 22.1 配置解析：`filter_map` + `collect`

```rust
let config: BTreeMap<&str, i32> = input
    .lines()
    .filter_map(|line| {
        let (key, value) = line.split_once('=')?;
        Some((key, value.parse().ok()?))
    })
    .collect();
```

非法行被 `?` 自动丢弃，合法行进入 `BTreeMap`；重复键由后出现的值覆盖。

### 22.2 按首字母分组

```rust
let mut groups: BTreeMap<char, Vec<&str>> = BTreeMap::new();
for word in words {
    groups.entry(word.chars().next().unwrap()).or_default().push(word);
}
```

`entry` + `or_default` 是"分组"的标准写法；键借用原数据，零复制。
更复杂的 `group_by`/`chunk_by` 可以用 `itertools` crate。

### 22.3 结果拆分：`unzip` / `partition`

```rust
let (names, scores): (Vec<&str>, Vec<i32>) = data.into_iter().unzip();

let (valid, invalid): (Vec<_>, Vec<_>) = parsed.into_iter().partition(Result::is_ok);
```

一次遍历把结果分到两个容器；`partition(Result::is_ok)` 是"拆分成功/失败"的常用技巧。

### 22.4 最佳实践清单

1. **明确目标容器**：`collect` 类型要写清楚（标注或 turbofish）
2. **能用组合子就别写循环**，但不要为炫技嵌套过多层
3. **避免中间 `collect`**：链式直接消费，减少分配
4. **短路优先**：`find`/`any`/`all` 比 `filter().count()` 高效
5. **`filter_map` 合并过滤与映射**，少一层适配器
6. **`try_fold`/`collect::<Result<...>>` 提前失败**，别让错误拖到最后
7. **`fold` 携带可变累加器**，比分多次 `collect` 更灵活
8. **`extend` 复用已有容器**，循环累积时优先
9. **`partition`/`unzip` 需要类型标注**，否则 E0283
10. **`min_by_key` 的 key 要廉价**，复杂计算先映射再比较

## 常见陷阱

1. **`collect` 目标不明（E0283）**：`let v = (0..3).collect();` 推断不出来；
   写 `Vec<i32>` 或 `collect::<Vec<_>>()`
2. **`partition` 类型不明（E0283）**：`let (a, b) = iter.partition(...)` 需要标注
   两个容器类型
3. **`HashMap` 重复键覆盖**：后插入的赢；需要聚合就用 `entry`/`fold`
4. **`BTreeMap`/`HashMap` 顺序差异**：需要稳定输出用 BTree；Hash 顺序不保证
5. **`collect::<Result<...>>` 短路**：第一个 `Err` 后面的元素不会被消费
6. **`reduce` 空迭代器返回 `None`**：不要 `unwrap` 而不处理
7. **`scan` 返回 `None` 会提前结束**：状态闭包要明确返回 `Some` 还是 `None`
8. **`flat_map` 闭包返回非迭代器**：必须返回 `IntoIterator`
9. **`filter_map` 闭包返回非 `Option`**：要求 `Option`
10. **`min_by_key` key 有副作用/很贵**：可能被多次调用；先用 `map` 预计算
11. **以为迭代器有 `sort`**：标准库迭代器没有 `sort`；先 `collect::<Vec<_>>()`
    （或 `itertools::sorted`）
12. **中间 `collect` 满天飞**：每多一个 `Vec` 就多一次分配
13. **`by_ref` 用错**：`iter.by_ref().take(2)` 之后可以继续用 `iter`
14. **无限迭代器直接 `collect`**：会挂起；先 `take`
15. **`extend` 与 `collect` 混用**：需要复用容器时用 `extend`
16. **`unzip` 类型标注缺失**：两个目标容器都要能推断
17. **`partition` 以为不分配**：它创建两个新容器
18. **`collect::<String>` 的 Item 类型不对**：必须是 `char`/`&str`/`String` 能拼接的类型
19. **`size_hint` 当精确长度**：自定义迭代器可能给得很松；`collect` 只是参考
20. **`Box<dyn Iterator>` 上做复杂组合**：每层 vtable 间接，热路径不划算

## 完整示例

见 `20_iterator_adapters_and_collect.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/20_迭代器组合子与collect/20_iterator_adapters_and_collect.rs" -o run.exe
./run.exe
```

示例共 19 行输出，覆盖：同一数据 `collect` 成 `Vec`/`VecDeque`/`BTreeSet`/`BTreeMap`、
`String`/`Result`/`Option` 目标、自定义 `Stats` 的 `FromIterator`、
`filter_map`/`flat_map`/`fold`/`reduce`/`scan`/`try_fold`、`unzip`/`partition`、
`extend`、排序截断，以及配置解析、分组和结果拆分。

## 练习

1. 把 `["1", "x", "2", "3"]` 用 `filter_map` 解析成 `Vec<i32>`，再分别
   `collect` 到 `BTreeSet` 与 `HashMap<i32, usize>`。
2. 实现 `struct Stats` 的 `FromIterator<i32>` 与 `Extend<i32>`，用两种方式累积。
3. 用 `try_fold` 实现"遇到第一个负数就返回 `Err`"的求和函数。
4. 用 `scan` 生成前缀和，并用 `fold` 再算一遍，比较两者。
5. 解析 `key=value` 文本到 `BTreeMap`，非法行跳过，重复键后者覆盖。
6. 按首字母分组单词，输出 `BTreeMap<char, Vec<&str>>`。
7. 用 `unzip` 拆分 `(name, score)` 列表，用 `partition` 拆成功/失败结果。
8. 比较 `filter().map()` 与 `filter_map()`、`map().flatten()` 与 `flat_map()`
   在可读性/适配器层数上的差异。

## 参考

- The Rust Programming Language: 第 13.2 节「用迭代器处理元素序列」
- std 文档：[`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html)、
  [`FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)、
  [`Extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html)
- Rust by Example: [Iterators](https://doc.rust-lang.org/rust-by-example/trait/iter.html)
- itertools crate（`sorted`、`group_by`、`chunk_by`、`tuple_windows` 等扩展组合子）
- 错误索引：[E0283](https://doc.rust-lang.org/error_codes/E0283.html)
