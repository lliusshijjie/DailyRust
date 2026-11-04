// 第 14 课示例：trait 约束、impl Trait 与 where
//
// 结构：① 原理验证 —— APIT 与泛型等价、RPIT 隐藏具体类型、where 与关联类型约束
//       ② 语法演示 —— 多约束、?Sized、关联类型、Send 约束、Box<dyn> 对比、RPIT 闭包
//       ③ 应用实战 —— 返回借用迭代器、top_n、泛型渲染
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 14_trait_bounds_and_impl_trait.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::fmt::{Debug, Display};

// 显式泛型参数：可以 turbofish 指定 T
fn generic_show<T: Display>(value: T) -> String {
    value.to_string()
}

// impl Trait 参数：匿名泛型，行为等价，但不能 turbofish
fn apit_show(value: impl Display) -> String {
    value.to_string()
}

// impl Trait 返回值：调用方只知道"能迭代 u32"，不知道具体适配器类型
fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}

// 返回借用的迭代器：输出生命周期绑定到输入
fn words<'a>(text: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    text.split_whitespace()
}

// where 子句 + 关联类型约束：I 的 Item 必须是 u32
fn sum_iter<I>(iter: I) -> u32
where
    I: Iterator<Item = u32>,
{
    iter.sum()
}

// where 子句 + 多个约束 + Sum
fn where_sum<T>(values: &[T]) -> T
where
    T: Copy + std::iter::Sum<T>,
{
    values.iter().copied().sum()
}

// 关联类型上用 AsRef：Item 只要能当 &str 用
fn total_len<I>(iter: I) -> usize
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    iter.map(|item| item.as_ref().len()).sum()
}

// 多个 trait 约束
fn describe<T: Display + Debug>(value: T) -> String {
    format!("{} ({:?})", value, value)
}

// ?Sized：允许 str、切片等未定长类型
fn debug_len<T: ?Sized + Debug>(value: &T) -> usize {
    format!("{:?}", value).len()
}

// RPIT + Send：调用方可以要求跨线程能力
fn range_iter() -> impl Iterator<Item = u32> + Send {
    1..=3
}

// 对照：Box<dyn> 是动态分发，类型可命名，但有分配与间接调用
fn boxed_iter() -> Box<dyn Iterator<Item = u32>> {
    Box::new(1..=3)
}

// RPIT 返回闭包：具体闭包类型不可命名，但调用方知道它是 FnMut
fn counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

// 应用：返回借用输入的惰性迭代器
fn parse_numbers(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.split_whitespace()
        .filter_map(|word| word.parse().ok())
}

// 应用：泛型排序 + 截断
fn top_n<T: Ord + Clone>(values: &[T], n: usize) -> Vec<T> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| b.cmp(a));
    sorted.truncate(n);
    sorted
}

// 应用：接受任意可 Display 的切片
fn render_all<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 显式泛型与 impl Trait 参数在调用侧等价
    println!("generic={} apit={}", generic_show(42), apit_show(42));
    // 预测：generic=42 apit=42

    println!("generic_str={} apit_str={}", generic_show("x"), apit_show("x"));
    // 预测：generic_str=x apit_str=x

    // 1.2 RPIT：只暴露"能迭代 u32"，隐藏 filter 的具体类型
    println!("evens={:?}", evens(10).collect::<Vec<_>>());
    // 预测：evens=[0, 2, 4, 6, 8]

    // 1.3 返回借用输入的迭代器
    println!("words={:?}", words("alpha beta").collect::<Vec<_>>());
    // 预测：words=["alpha", "beta"]

    // 1.4 where + 关联类型约束
    println!("sum={}", sum_iter(1..=5));
    // 预测：sum=15

    println!("===== 2. SYNTAX =====");

    // 2.1 where + 多约束
    println!("where_sum={}", where_sum(&[1u32, 2, 3, 4, 5]));
    // 预测：where_sum=15

    // 2.2 关联类型约束：I::Item: AsRef<str>
    println!(
        "assoc={}",
        total_len(vec![String::from("abc"), String::from("de")].into_iter())
    );
    // 预测：assoc=5

    // 2.3 多个 trait 约束
    println!("multi={}", describe(7));
    // 预测：multi=7 (7)

    // 2.4 ?Sized：str 与切片都能传
    println!("sized={} {}", debug_len("abc"), debug_len(&[1, 2, 3]));
    // 预测：sized=5 9

    // 2.5 RPIT + Send
    println!("range={:?}", range_iter().collect::<Vec<_>>());
    // 预测：range=[1, 2, 3]

    // 2.6 Box<dyn> 对照
    println!("boxed={:?}", boxed_iter().collect::<Vec<_>>());
    // 预测：boxed=[1, 2, 3]

    // 2.7 RPIT 返回闭包
    let mut next = counter();
    println!("counter={} {} {}", next(), next(), next());
    // 预测：counter=1 2 3

    println!("===== 3. APPLICATION =====");

    // 3.1 惰性解析 + 借用返回
    println!(
        "parsed={:?}",
        parse_numbers("1 2 x 3").collect::<Vec<_>>()
    );
    // 预测：parsed=[1, 2, 3]

    // 3.2 泛型 top_n
    println!("top={:?}", top_n(&[5, 9, 3, 7, 1], 3));
    // 预测：top=[9, 7, 5]

    // 3.3 泛型渲染
    println!("rendered={}", render_all(&[1, 2, 3]));
    // 预测：rendered=1,2,3
}
