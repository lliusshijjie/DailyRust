// 第 07 课示例：生命周期标注
//
// 结构：① 原理验证 —— 生命周期在运行期被擦除、'static 字面量、'static 约束接受拥有者
//       ② 语法演示 —— 省略规则、显式 'a、结构体生命周期、'static 引用、'_ 占位符
//       ③ 应用实战 —— 返回借用片段、零拷贝解析、Cow、返回借用迭代器、静态配置
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 07_lifetimes.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::borrow::Cow;
use std::mem::size_of;

// 显式标注：返回值的生命周期同时绑定到两个输入（取交集的区域）
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() >= y.len() { x } else { y }
}

// 省略规则 1：只有一个输入生命周期，输出自动继承
fn first_word(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or("")
}

// `'_` 是占位符，等价于让编译器按省略规则补全
fn echo(text: &str) -> &'_ str {
    text
}

// `T: 'static` 表示 T 不包含短生命周期的借用；String 这样的拥有者满足它
fn keep<T: 'static>(value: T) -> T {
    value
}

// 结构体中存引用必须声明生命周期参数
struct Excerpt<'a> {
    part: &'a str,
}

impl<'a> Excerpt<'a> {
    fn new(part: &'a str) -> Self {
        Self { part }
    }

    // 省略规则 3：输出生命周期来自 &self
    fn announce(&self) -> &str {
        self.part
    }
}

// 多个字段可以有各自的生命周期
struct Pair<'a, 'b> {
    first: &'a str,
    second: &'b str,
}

// 零拷贝解析：返回的结构体借用输入
struct KeyValue<'a> {
    key: &'a str,
    value: &'a str,
}

fn parse_pair(line: &str) -> Option<KeyValue<'_>> {
    let (key, value) = line.split_once('=')?;
    Some(KeyValue { key, value })
}

// 返回借用输入的迭代器
fn words<'a>(text: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    text.split_whitespace()
}

// 借用或拥有：能借就借，必须改才分配
fn normalize<'a>(input: &'a str) -> Cow<'a, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}

// 静态配置字符串天然是 &'static str
const APP_NAME: &str = "daily-rust";
static GREETING: &str = "hello-static";

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 生命周期只在编译期存在，标注不会改变运行期类型大小
    println!(
        "lifetimes_erased={}",
        size_of::<&i32>() == size_of::<&'static i32>()
    );
    // 预测：lifetimes_erased=true

    // 1.2 字符串字面量的类型就是 &'static str
    let literal: &'static str = "static";
    println!("static_literal_len={}", literal.len());
    // 预测：static_literal_len=6

    // 1.3 两个输入的借用都被纳入同一个 'a，输出取二者中更短的有效区域
    let left = String::from("abc");
    let right = String::from("abcdef");
    println!("longest={}", longest(&left, &right));
    // 预测：longest=abcdef

    // 1.4 `T: 'static` 接受拥有所有权的 String（不是引用也满足）
    println!("static_bound_owned_len={}", keep(String::from("owned")).len());
    // 预测：static_bound_owned_len=5

    println!("===== 2. SYNTAX =====");

    // 2.1 单输入省略：返回 &str 的生命周期与参数一致
    println!("first_word={}", first_word("hello world"));
    // 预测：first_word=hello

    // 2.2 结构体存引用：Excerpt<'a> 的生命周期由 part 决定
    let text = String::from("the quick brown fox");
    let excerpt = Excerpt::new(&text[4..9]);
    println!("excerpt={} announce={}", excerpt.part, excerpt.announce());
    // 预测：excerpt=quick announce=quick

    // 2.3 两个字段各自的生命周期
    let pair = Pair {
        first: "one",
        second: "two",
    };
    println!("pair={} {}", pair.first, pair.second);
    // 预测：pair=one two

    // 2.4 static 项的引用就是 &'static str
    println!("static_ref={}", GREETING);
    // 预测：static_ref=hello-static

    // 2.5 '_ 占位符：显式告诉编译器"这里按省略规则补全"
    println!("placeholder={}", echo("echo"));
    // 预测：placeholder=echo

    println!("===== 3. APPLICATION =====");

    // 3.1 返回输入的片段，不复制
    let line = "key=value";
    let parsed = parse_pair(line).unwrap();
    println!("parsed={} {}", parsed.key, parsed.value);
    // 预测：parsed=key value

    // 3.2 Cow：需要修改时才分配，否则借用
    let borrowed = normalize("already-clean");
    let owned = normalize("needs space");
    println!(
        "cow_borrowed={} cow_owned={}",
        matches!(&borrowed, Cow::Borrowed(_)),
        matches!(&owned, Cow::Owned(_))
    );
    // 预测：cow_borrowed=true cow_owned=true

    // 3.3 返回借用输入的迭代器：元素借用 text，不复制
    let collected: Vec<&str> = words("alpha beta").collect();
    println!("words={:?}", collected);
    // 预测：words=["alpha", "beta"]

    // 3.4 静态配置串：'static 适合常量与全局配置，不适合普通函数参数
    println!("app={}", APP_NAME);
    // 预测：app=daily-rust
}
