// 第 19 课示例：迭代器与惰性求值
//
// 结构：① 原理验证 —— 适配器不产生元素、短路、无限迭代器、自定义 Iterator
//       ② 语法演示 —— map/filter/zip/chain/flat_map/fold/collect Result/peekable
//       ③ 应用实战 —— Fibonacci 迭代器、词频统计、解析求和
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 19_iterators_and_laziness.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::cell::Cell;
use std::collections::BTreeMap;

// 自定义 Iterator：实现 next 即可
struct Countdown {
    current: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.current == 0 {
            None
        } else {
            let value = self.current;
            self.current -= 1;
            Some(value)
        }
    }
}

// 无限迭代器：Fibonacci 永远返回 Some
struct Fibonacci {
    a: u64,
    b: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let value = self.a;
        self.a = self.b;
        self.b = value + self.b;
        Some(value)
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 惰性：适配器只构建状态机，不产生元素
    let visited = Cell::new(0);
    let mapped = (1..=3).map(|value| {
        visited.set(visited.get() + 1);
        value * 2
    });
    println!("lazy_before={}", visited.get());
    // 预测：lazy_before=0
    let doubled: Vec<i32> = mapped.collect();
    println!("lazy_after={} doubled={:?}", visited.get(), doubled);
    // 预测：lazy_after=3 doubled=[2, 4, 6]

    // 1.2 短路：find 找到就停，inspect 只看到前 4 个
    let mut checks = 0;
    let found = (1..100)
        .inspect(|_| checks += 1)
        .find(|value| *value > 3);
    println!("short_circuit={:?} visited={}", found, checks);
    // 预测：short_circuit=Some(4) visited=4

    // 1.3 无限迭代器 + take：按需截断
    let powers: Vec<u32> = std::iter::successors(Some(1u32), |value| Some(value * 2))
        .take(5)
        .collect();
    println!("infinite={:?}", powers);
    // 预测：infinite=[1, 2, 4, 8, 16]

    // 1.4 自定义 Iterator：next 返回 None 结束
    let countdown: Vec<u32> = Countdown { current: 3 }.collect();
    println!("countdown={:?}", countdown);
    // 预测：countdown=[3, 2, 1]

    println!("===== 2. SYNTAX =====");

    // 2.1 适配器链：filter -> map
    let pipeline: Vec<i32> = (1..=5)
        .filter(|value| value % 2 == 1)
        .map(|value| value * value)
        .collect();
    println!("pipeline={:?}", pipeline);
    // 预测：pipeline=[1, 9, 25]

    // 2.2 zip：按较短的一侧截断
    let zipped: Vec<(i32, char)> = [1, 2, 3].into_iter().zip(['a', 'b']).collect();
    println!("zipped={:?}", zipped);
    // 预测：zipped=[(1, 'a'), (2, 'b')]

    // 2.3 chain：拼接两个迭代器
    let chained: Vec<i32> = (1..=2).chain(5..=6).collect();
    println!("chained={:?}", chained);
    // 预测：chained=[1, 2, 5, 6]

    // 2.4 flat_map：展开嵌套结构
    let flat: String = "ab cd"
        .split_whitespace()
        .flat_map(|word| word.chars())
        .collect();
    println!("flat={}", flat);
    // 预测：flat=abcd

    // 2.5 fold：把迭代器折叠成单值
    let folded = (1..=4).fold(0, |acc, value| acc + value);
    println!("folded={}", folded);
    // 预测：folded=10

    // 2.6 collect 成 Result：遇到 Err 立即短路
    let parsed: Result<Vec<i32>, _> = ["1", "x", "3"]
        .iter()
        .map(|text| text.parse::<i32>())
        .collect();
    println!("parse_err={}", parsed.is_err());
    // 预测：parse_err=true

    // 2.7 peekable：先看下一个元素，不消费
    let mut iter = [1, 2, 3].into_iter().peekable();
    let peeked = iter.peek().copied();
    let next = iter.next();
    println!("peek={:?} {:?}", peeked, next);
    // 预测：peek=Some(1) Some(1)

    println!("===== 3. APPLICATION =====");

    // 3.1 Fibonacci 迭代器：take 截断无限序列
    let fib: Vec<u64> = Fibonacci { a: 1, b: 1 }.take(6).collect();
    println!("fib={:?}", fib);
    // 预测：fib=[1, 1, 2, 3, 5, 8]

    // 3.2 词频统计：split_whitespace + 迭代器
    let text = "rust iterators are lazy";
    let words: Vec<&str> = text.split_whitespace().collect();
    let chars: usize = words.iter().map(|word| word.len()).sum();
    println!("words={} chars={}", words.len(), chars);
    // 预测：words=4 chars=20

    // 3.3 解析 + 过滤 + 求和 一条流水线
    let even_total: i32 = ["1", "2", "3", "4"]
        .iter()
        .map(|text| text.parse::<i32>().unwrap())
        .filter(|value| value % 2 == 0)
        .sum();
    println!("even_total={}", even_total);
    // 预测：even_total=6

    // 3.4 BTreeMap 词频：fold 携带可变累加器
    let counts: BTreeMap<&str, usize> = "a b a c b a"
        .split_whitespace()
        .fold(BTreeMap::new(), |mut map, word| {
            *map.entry(word).or_insert(0) += 1;
            map
        });
    println!("counts={:?}", counts);
    // 预测：counts={"a": 3, "b": 2, "c": 1}
}
