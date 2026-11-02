// 第 12 课示例：泛型函数与单态化
//
// 结构：① 原理验证 —— 同一份泛型代码对不同 T 生成不同实例（type_name 可见）
//       ② 语法演示 —— 泛型函数/约束、Pair/Either 泛型类型、特化 impl、turbofish、impl Trait 参数
//       ③ 应用实战 —— 通用 Cache<K,V>、min_max、泛型求和
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 12_generics_and_monomorphization.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

// 最简单的泛型函数：T 可以是任何类型
fn identity<T>(value: T) -> T {
    value
}

// 在泛型函数体内打印具体的 T：证明每个具体类型都有自己的实例
fn type_name_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

// 约束写在参数上：只有 PartialOrd 才能比较
fn largest<T: PartialOrd>(values: &[T]) -> Option<&T> {
    let mut largest = values.first()?;
    for value in values {
        if value > largest {
            largest = value;
        }
    }
    Some(largest)
}

// where 子句：多个约束、跨行的复杂签名更易读
fn sum<T>(values: &[T]) -> T
where
    T: Copy + std::iter::Sum<T>,
{
    values.iter().copied().sum()
}

// 泛型结构体：字段类型由调用方决定
struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
    fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    fn swap(self) -> Pair<B, A> {
        Pair {
            first: self.second,
            second: self.first,
        }
    }
}

// 只有 A、B 都能 Display 时才有 show 方法
impl<A: Display, B: Display> Pair<A, B> {
    fn show(&self) -> String {
        format!("{} & {}", self.first, self.second)
    }
}

// 针对特定类型参数的"特化" impl：只有 Pair<i32, i32> 有 sum
impl Pair<i32, i32> {
    fn sum(&self) -> i32 {
        self.first + self.second
    }
}

// 泛型枚举：Left(L) 或 Right(R)，只映射左边
#[derive(Debug)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    fn map_left<F, U>(self, f: F) -> Either<U, R>
    where
        F: FnOnce(L) -> U,
    {
        match self {
            Either::Left(left) => Either::Left(f(left)),
            Either::Right(right) => Either::Right(right),
        }
    }
}

// impl Trait 参数：等价于一个匿名泛型参数
fn join_all(values: impl IntoIterator<Item = i32>) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

// 泛型缓存：约束写在 impl 上，只有可哈希的键才能用
struct Cache<K, V> {
    entries: HashMap<K, V>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get_or_insert_with(&mut self, key: K, make: impl FnOnce() -> V) -> &V {
        self.entries.entry(key).or_insert_with(make)
    }
}

// 泛型算法：任何可比较、可复制的类型都能求最小/最大值
fn min_max<T: Ord + Copy>(values: &[T]) -> Option<(T, T)> {
    let mut iter = values.iter().copied();
    let first = iter.next()?;
    let mut min = first;
    let mut max = first;
    for value in iter {
        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
    }
    Some((min, max))
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 同一份泛型代码，对不同 T 各生成一份实例
    println!("identity={} {}", identity(7), identity("x"));
    // 预测：identity=7 x

    // 1.2 在函数体内观察具体类型：单态化后的 T 是 i32 / &str
    println!("types={} {}", type_name_of(&1i32), type_name_of(&"x"));
    // 预测：types=i32 &str

    // 1.3 约束让泛型函数可以调用 T 的方法
    println!("largest={:?}", largest(&[3, 9, 2]));
    // 预测：largest=Some(9)

    // 1.4 特化 impl：只有 Pair<i32, i32> 能调用 sum
    println!("pair_sum={}", Pair::new(3, 4).sum());
    // 预测：pair_sum=7

    println!("===== 2. SYNTAX =====");

    // 2.1 泛型结构体 + 条件方法
    let pair = Pair::new(3, 4);
    println!("show={}", pair.show());
    // 预测：show=3 & 4

    let swapped = pair.swap();
    println!("swapped={} {}", swapped.first, swapped.second);
    // 预测：swapped=4 3

    // 2.2 泛型枚举 + 泛型方法
    let left: Either<i32, &str> = Either::Left(10);
    let right: Either<i32, &str> = Either::Right("skip");
    println!(
        "either={:?} {:?}",
        left.map_left(|value| value * 2),
        right.map_left(|value| value * 2)
    );
    // 预测：either=Left(20) Right("skip")

    // 2.3 where 子句与泛型求和
    println!("sum={}", sum(&[1, 2, 3, 4, 5]));
    // 预测：sum=15

    // 2.4 impl Trait 参数：调用方传任何 IntoIterator<Item = i32> 都行
    println!("joined={}", join_all(vec![1, 2, 3]));
    // 预测：joined=1,2,3

    // 2.5 turbofish：给泛型函数显式指定类型参数
    println!("parse={}", "42".parse::<i32>().unwrap());
    // 预测：parse=42

    println!("===== 3. APPLICATION =====");

    // 3.1 泛型缓存：命中时不重新计算，未命中才调用闭包
    let mut cache: Cache<String, i32> = Cache::new();
    let first = *cache.get_or_insert_with("a".to_string(), || 1);
    let second = *cache.get_or_insert_with("a".to_string(), || 99);
    let third = *cache.get_or_insert_with("b".to_string(), || 2);
    println!("cache={} {} {}", first, second, third);
    // 预测：cache=1 1 2

    // 3.2 泛型算法：i32 与 char 都能用同一份 min_max
    println!("minmax={:?}", min_max(&[4, 1, 9, 3, 8]));
    // 预测：minmax=Some((1, 9))
}
