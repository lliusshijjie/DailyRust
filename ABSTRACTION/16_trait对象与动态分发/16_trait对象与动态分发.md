# 第 16 课 · trait 对象与动态分发

> 所需工具链：rustc 1.85+（edition 2024） ｜ 讲解：本文件 ｜ 独立代码：`16_trait_objects_and_dispatch.rs`

## 一、底层原理

### 16.1 trait 对象是"胖指针 + vtable"

`dyn Trait` 是**未定长类型**（DST）：编译器不知道具体实现有多大，所以它只能待在指针后面。
一个 trait 对象引用由两个字组成：

```
┌──────────────┐      ┌──────────────────┐
│ data ptr  ───┼─────▶│ 具体类型的值      │
├──────────────┤      └──────────────────┘
│ vtable ptr ──┼─────▶┌──────────────────┐
└──────────────┘      │ drop_in_place    │
                      │ size             │
                      │ align            │
                      │ method 1         │
                      │ method 2 ...     │
                      └──────────────────┘
```

- `size_of::<&dyn Trait>()` = 2 个字（x86_64 上 16 字节）
- `size_of::<Box<dyn Trait>>()` = 2 个字（Box 只存胖指针）
- vtable 里放着析构函数、大小、对齐和每个方法的函数指针
- `Box<dyn Trait>` 的数据在堆上；`&dyn Trait` 可以指向栈上的值

具体类型（如 `Circle` 8 字节）装进 `Box<dyn Speak>` 后变成 16 字节的胖指针，
真正的数据仍在堆上，调用时通过 vtable 找到方法。

### 16.2 动态分发：一次间接调用

```rust
let animals: Vec<Box<dyn Speak>> = vec![Box::new(Dog), Box::new(Cat)];
for animal in &animals {
    animal.speak();          // 运行时查 vtable
}
```

- 每次方法调用多一次**指针间接**（读 vtable 槽位再跳转）
- 通常**不能内联**，优化器看不到具体实现
- 但只有**一份**调用代码，不需要为每个具体类型单态化
- 异构集合（`Vec<Box<dyn Trait>>`）只能用动态分发（或枚举，如果集合封闭）

| 维度 | 泛型 `T: Trait` | 枚举 | `dyn Trait` |
|---|---|---|---|
| 分发 | 静态（单态化） | 静态（`match`） | 动态（vtable） |
| 代码体积 | 每个类型一份 | 一份 | 一份 |
| 异构集合 | 不支持 | 支持（封闭） | 支持（开放） |
| 内联 | 可以 | 可以 | 通常不行 |
| 值大小 | 具体类型大小 | 最大变体 | 胖指针 |
| 扩展 | 任何实现者 | 改枚举 | 任何实现者 |

### 16.3 dyn 兼容性（object safety）的硬约束

`dyn Trait` 要求 trait **dyn 兼容**（旧称 object safe）：

- 方法接收者必须是 `&self`、`&mut self`、`self: Box<Self>`、`self: Rc<Self>`、
  `self: Arc<Self>`、`self: Pin<&mut Self>` 等；**裸 `self` 方法**不行
- 不能有**泛型方法**（vtable 放不下无限多实例）
- 不能返回 `Self`（大小未知）
- 不能有**关联常量**
- 静态方法要么不出现，要么用 `where Self: Sized` 排除
- 关联类型必须**指定**：`dyn Iterator<Item = u32>`，不能只写 `dyn Iterator`
- GAT、RPITIT 一般会让 trait 失去 dyn 兼容性（E0038）

```rust
trait Factory {
    fn create() -> Self where Self: Sized;   // 排除出 vtable，trait 仍可 dyn
    fn describe(&self) -> String;            // 进入 vtable
}
```

`where Self: Sized` 的方法可以在具体类型上调用，但在 `dyn Trait` 上调用会 E0599。

### 16.4 trait 对象的生命周期与 auto trait

trait 对象类型必须带生命周期约束：

```rust
&'a dyn Trait            // 借用的 trait 对象
Box<dyn Trait>           // 默认 'static（在函数签名/结构体字段等位置）
Box<dyn Trait + 'a>      // 允许借用数据
Box<dyn Trait + Send>    // 额外要求 Send
```

- 默认 `'static`：`Box<dyn Trait>` 出现在函数参数/返回值/结构体字段时，装不下短借用
- 局部 `let` 里编译器可能推断出更短的生命周期，但跨函数边界必须显式写 `+ '_`/`+ 'a`
- `Send`/`Sync` 是 auto trait，需要显式写进 trait 对象类型才能跨线程
- `dyn Any` 隐含 `'static`（`Any: 'static`），不能装借用数据

### 16.5 `dyn Any` 与向下转型

`std::any::Any` 是运行期类型擦除的标准工具：

```rust
let boxed: Box<dyn Any> = Box::new(42i32);
boxed.downcast_ref::<i32>();     // Option<&i32>
boxed.downcast_mut::<i32>();     // Option<&mut i32>
boxed.downcast::<i32>();         // Result<Box<i32>, Box<dyn Any>>
boxed.is::<i32>();               // bool
```

- 向下转型是运行期检查 `TypeId`，失败返回 `None`/`Err`
- 适合"配置表/插件上下文"这类异构容器；滥用会丢失类型安全
- 需要 `'static` 类型；借用数据要用 `+ 'a` 的 trait 对象，而不是 `Any`

### 16.6 常见 trait 对象形态

| 类型 | 用途 |
|---|---|
| `Box<dyn Error>` | 错误类型擦除，`?` 向上传播（第 24 课） |
| `Box<dyn Fn(i32) -> i32>` | 闭包集合、回调表 |
| `Box<dyn Iterator<Item = T>>` | 运行期决定迭代器实现 |
| `Box<dyn Any>` | 异构配置/插件上下文 |
| `Arc<dyn Trait + Send + Sync>` | 跨线程共享的服务对象 |

trait 对象还支持 **upcasting**：`&dyn Sub` 可以转成 `&dyn Super`（`Sub: Super`），
当前工具链已稳定。

## 二、具体语法

### 17.1 创建 trait 对象

```rust
let boxed: Box<dyn Speak> = Box::new(Dog);
let borrowed: &dyn Speak = &dog;
let mut_ref: &mut dyn Speak = &mut dog;
let shared: std::rc::Rc<dyn Speak> = std::rc::Rc::new(Dog);
```

- `dyn Trait` 不能按值使用（E0277）
- 需要拥有就 `Box`/`Rc`/`Arc`，需要借用就 `&`/`&mut`

### 17.2 调用方法

```rust
boxed.speak();        // 自动解引用到 dyn Speak
borrowed.speak();
```

- 方法调用语法与普通类型一致
- 无法调用 `Self: Sized` 的静态方法/泛型方法

### 17.3 异构集合

```rust
let animals: Vec<Box<dyn Speak>> = vec![
    Box::new(Dog),
    Box::new(Cat),
    Box::new(Circle { radius: 1.0 }),
];
```

- 同一个容器里放不同具体类型
- 如果集合封闭，枚举 + `match` 往往更快（第 09 课）

### 17.4 生命周期

```rust
let local = String::from("hello");
let object: Box<dyn Speak + '_> = Box::new(Borrowed { text: &local });
object.speak();
```

- `+ '_` 让 trait 对象借用局部数据
- 函数签名里的 `Box<dyn Trait>` 默认 `'static`，需要借用时写 `Box<dyn Trait + 'a>`

### 17.5 auto trait

```rust
let worker: Box<dyn Fn() + Send + Sync> = Box::new(|| {});
```

- 跨线程传递 trait 对象时，`Send`/`Sync` 必须出现在类型里
- `Arc<dyn Trait + Send + Sync>` 是共享服务对象的常见形态

### 17.6 Any 与 downcast

```rust
use std::any::Any;

let boxed: Box<dyn Any> = Box::new(42i32);
if let Some(value) = boxed.downcast_ref::<i32>() {
    println!("{}", value);
}
```

- `downcast_ref` 借引用，`downcast` 拿走 `Box`
- 类型不匹配返回 `None`/`Err`，不会 panic
- 只适用于 `'static` 类型

### 17.7 `dyn Fn` / `dyn Iterator`

```rust
let ops: Vec<Box<dyn Fn(i32) -> i32>> = vec![
    Box::new(|x| x + 1),
    Box::new(|x| x * 2),
];

let iter: Box<dyn Iterator<Item = u32>> = Box::new(1..=3);
```

- 闭包类型匿名，`Box<dyn Fn>` 是统一存放的方式
- `dyn Iterator` 必须指定 `Item`

### 17.8 upcasting

```rust
let bird: &dyn Animal = &Bird;
let speak: &dyn Speak = bird;    // Animal: Speak
speak.speak();
```

- 子 trait 对象可以向上转成父 trait 对象
- 依赖当前工具链的 upcasting 支持（已稳定）

### 17.9 设计 dyn 兼容的 trait

```rust
trait Service {
    fn handle(&self, input: &str) -> String;

    fn create() -> Self where Self: Sized;   // 静态构造，不进 vtable
}
```

- 把泛型方法/静态方法/返回 `Self` 的方法用 `where Self: Sized` 排除
- 关联类型在 dyn 时要写具体值
- 需要 GAT/RPITIT 时，放弃 dyn 兼容性

## 三、实际应用与最佳实践

### 18.1 插件注册表

```rust
let plugins: Vec<Box<dyn Plugin>> = vec![Box::new(Double), Box::new(AddOne)];
for plugin in &plugins {
    println!("{}:{}", plugin.name(), plugin.run(5));
}
```

插件接口只依赖 trait，新增实现不需要改注册表代码。代价是每次调用一次 vtable 间接，
以及每个插件一次堆分配（`Box`）。

### 18.2 Any 配置存储

```rust
let mut config: HashMap<&str, Box<dyn Any>> = HashMap::new();
config.insert("threads", Box::new(4i32));
config.insert("name", Box::new(String::from("service")));
config.get("threads").and_then(|v| v.downcast_ref::<i32>());
```

异构配置的经典写法；读取时用 `downcast_ref` 还原类型。生产代码里通常再加一层
`Config` 结构体和默认值，避免到处 `downcast`。

### 18.3 选择 dyn / enum / 泛型

| 场景 | 选择 |
|---|---|
| 插件、运行时注册、开放扩展 | `Box<dyn Trait>` |
| 封闭状态集、需要穷尽匹配 | 枚举 |
| 热路径、同质集合、要内联 | 泛型 `T: Trait` |
| 回调表 | `Box<dyn Fn>` |
| 错误类型擦除 | `Box<dyn Error>` |
| 跨线程共享 | `Arc<dyn Trait + Send + Sync>` |

### 18.4 最佳实践清单

1. **默认用泛型/枚举**，需要运行时异构时才上 `dyn`
2. **热路径避免 dyn**：vtable 调用不能内联，循环里代价明显
3. **插件/回调边界用 dyn**：灵活性和解耦优先
4. **trait 对象默认 `'static`**：跨函数边界借用数据要写 `+ 'a`
5. **跨线程显式写 `Send + Sync`**，不要依赖调用方推断
6. **`Any` 只用于真正的异构容器**，普通业务用具体类型/枚举
7. **downcast 失败不要 panic**，返回 `None`/默认值/错误
8. **dyn 兼容的 trait 把泛型/静态方法放到 `where Self: Sized`**
9. **`Box<dyn Trait>` 有堆分配 + 间接调用**，预算敏感时考虑 `&dyn`
10. **公开 API 返回 `Box<dyn Trait>` 是 semver 契约**：实现者集合会变化

## 常见陷阱

1. **按值使用 `dyn Trait`（E0277）**：`let value: dyn Trait;` 不合法；
   必须 `&dyn`/`Box<dyn>`
2. **trait 不 dyn 兼容（E0038）**：泛型方法、返回 `Self`、关联常量、GAT、RPITIT；
   拆 trait 或用 `where Self: Sized`
3. **调用 `Self: Sized` 方法（E0599）**：`dyn Trait` 上没有这个方法；
   在具体类型上调用
4. **`Box<dyn Trait>` 默认 `'static`（E0597）**：函数签名/结构体字段里装短借用失败；
   写 `Box<dyn Trait + 'a>`
5. **`dyn Any` 装借用数据**：`Any: 'static`，借用类型不满足；改用具体 trait 对象
6. **downcast 类型写错**：返回 `None`/`Err`，不会 panic；检查类型或用 `is::<T>()`
7. **`dyn Clone`**：`Clone::clone` 返回 `Self`，不 dyn 兼容；用 `dyn_clone` crate
   或手写 `clone_box` 方法
8. **trait 对象在热循环里**：每次调用一次间接跳转；用泛型/枚举替代
9. **`Box<dyn Trait>` 以为只有一层指针**：胖指针 + 堆数据，调用要读 vtable
10. **`Rc<dyn Trait>` 跨线程**：`Rc` 不是 `Send`；改用 `Arc<dyn Trait + Send + Sync>`
11. **`dyn Iterator` 漏写 `Item`**：关联类型必须指定，否则 E0191
12. **`dyn A + B`**：只能有一个非 auto trait；多个能力用 supertrait 组合
13. **upcasting 依赖版本**：旧编译器不支持子 trait 转父 trait；
    当前工具链已稳定，但库的 MSRV 要注意
14. **`Box<dyn FnOnce>` 调用**：`FnOnce` 只能调用一次，调用会消耗 Box
15. **把 `Box<dyn Trait>` 当值比较/克隆**：trait 对象没有 `PartialEq`/`Clone` 自动实现；
    需要手动设计
16. **`dyn Trait` 与 `impl Trait` 混淆**：前者运行期擦除，后者编译期隐藏具体类型
17. **supertrait 方法 `where Self: Sized`**：在 dyn 上不可用；vtable 里没有

## 完整示例

见 `16_trait_objects_and_dispatch.rs`，按 **① 原理验证 → ② 语法演示 → ③ 应用实战**
三段组织，先预测输出再运行（示例输出刻意只用 ASCII，规避 Windows 控制台编码问题）：

```bash
rustc --edition 2024 -D warnings "ABSTRACTION/16_trait对象与动态分发/16_trait_objects_and_dispatch.rs" -o run.exe
./run.exe
```

示例共 13 行输出，覆盖：胖指针/Box 大小、具体类型与 trait 对象大小对比、异构
`Vec<Box<dyn Speak>>` 分发、`dyn Any` 向下转型、`Box<dyn Fn>`、`Box<dyn Iterator>`、
带生命周期的 trait 对象、trait upcasting、插件注册表、Any 配置存储。

## 练习

1. 定义 `trait Shape { fn area(&self) -> f64; }`，用 `Vec<Box<dyn Shape>>` 装圆、
   矩形、三角形，求和；再新增一种形状，验证注册表代码不用改。
2. 实现插件注册表：`trait Plugin { fn name(&self) -> &str; fn run(&self, i32) -> i32; }`，
   注册 3 个插件并按名称排序输出。
3. 用 `HashMap<&str, Box<dyn Any>>` 存 `i32`、`f64`、`String`，读取时分别 downcast；
   类型不匹配时返回默认值。
4. 给一个带泛型方法的 trait 加 `where Self: Sized`，让它变成 dyn 兼容；
   验证泛型方法在 `dyn` 上不可调用。
5. 比较 `Vec<Box<dyn Shape>>` 与 `Vec<Shape>` 枚举实现的调用开销/代码体积；
   说明各自适用场景。
6. 写一个 `Box<dyn FnOnce()>` 的回调，调用一次；解释为什么之后不能再用。
7. 用 `Box<dyn Speak + '_>` 装一个借用局部字符串的实现，观察生命周期约束。
8. 尝试 `dyn A + B`（两个普通 trait），记录 E0225，并用 supertrait 改写。

## 参考

- The Rust Programming Language: 第 17.2 节「为使用不同类型的值而设计的 trait 对象」
- Rust Reference: [Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html)、
  [Dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- std 文档：[`std::any::Any`](https://doc.rust-lang.org/std/any/trait.Any.html)、
  [`Box<dyn Error>`](https://doc.rust-lang.org/std/error/trait.Error.html)
- 错误索引：[E0038](https://doc.rust-lang.org/error_codes/E0038.html)、
  [E0277](https://doc.rust-lang.org/error_codes/E0277.html)、
  [E0597](https://doc.rust-lang.org/error_codes/E0597.html)、
  [E0599](https://doc.rust-lang.org/error_codes/E0599.html)
