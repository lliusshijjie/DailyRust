// 第 20 课示例：迭代器组合子与 collect
//
// 结构：① 原理验证 —— 同一份数据 collect 到不同容器、Result/Option 短路、自定义 FromIterator
//       ② 语法演示 —— filter_map/flat_map、fold/reduce/scan/try_fold、unzip/partition、extend
//       ③ 应用实战 —— 配置解析、按首字母分组、结果拆分
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 20_iterator_adapters_and_collect.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::{BTreeMap, BTreeSet, VecDeque};

// 自定义 collect 目标：实现 FromIterator
#[derive(Debug)]
struct Stats {
    count: usize,
    sum: i32,
    min: i32,
    max: i32,
}

impl FromIterator<i32> for Stats {
    fn from_iter<I: IntoIterator<Item = i32>>(iter: I) -> Self {
        let mut stats = Stats {
            count: 0,
            sum: 0,
            min: i32::MAX,
            max: i32::MIN,
        };
        for value in iter {
            stats.count += 1;
            stats.sum += value;
            stats.min = stats.min.min(value);
            stats.max = stats.max.max(value);
        }
        stats
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 同一份数据 collect 到不同容器
    let numbers = vec![3, 1, 3, 2];
    let as_vec: Vec<i32> = numbers.iter().copied().collect();
    let as_deque: VecDeque<i32> = numbers.iter().copied().collect();
    println!("vec={:?} deque={:?}", as_vec, as_deque);
    // 预测：vec=[3, 1, 3, 2] deque=[3, 1, 3, 2]

    let as_set: BTreeSet<i32> = numbers.iter().copied().collect();
    let counts: BTreeMap<i32, usize> = numbers.iter().fold(BTreeMap::new(), |mut map, value| {
        *map.entry(*value).or_insert(0) += 1;
        map
    });
    println!("set={:?} counts={:?}", as_set, counts);
    // 预测：set={1, 2, 3} counts={1: 1, 2: 1, 3: 2}

    // 1.2 String / Result / Option 作为 collect 目标
    let text: String = ['a', 'b', 'c'].into_iter().collect();
    let parsed: Result<Vec<i32>, _> = ["1", "2", "3"]
        .iter()
        .map(|value| value.parse::<i32>())
        .collect();
    let optional: Option<Vec<i32>> = [Some(1), Some(2)].into_iter().collect();
    println!("text={} result={:?} option={:?}", text, parsed, optional);
    // 预测：text=abc result=Ok([1, 2, 3]) option=Some([1, 2])

    // 1.3 自定义 FromIterator：collect 成 Stats
    let stats: Stats = [3, 1, 4, 2].into_iter().collect();
    println!("custom={:?}", stats);
    // 预测：custom=Stats { count: 4, sum: 10, min: 1, max: 4 }

    println!("===== 2. SYNTAX =====");

    // 2.1 filter_map：一次完成过滤 + 映射
    let parsed_values: Vec<i32> = ["1", "x", "2", "3"]
        .iter()
        .filter_map(|text| text.parse::<i32>().ok())
        .collect();
    println!("filter_map={:?}", parsed_values);
    // 预测：filter_map=[1, 2, 3]

    // 2.2 flat_map：把每个元素展开成迭代器再拼接
    let letters: String = ["ab", "cd"]
        .iter()
        .flat_map(|text| text.chars())
        .collect();
    println!("flat_map={}", letters);
    // 预测：flat_map=abcd

    // 2.3 fold / reduce / scan
    let folded = (1..=4).fold(0, |acc, value| acc + value);
    let reduced = (1..=4).reduce(|acc, value| acc + value);
    let scanned: Vec<i32> = (1..=4)
        .scan(0, |state, value| {
            *state += value;
            Some(*state)
        })
        .collect();
    println!("fold={} reduce={:?} scan={:?}", folded, reduced, scanned);
    // 预测：fold=10 reduce=Some(10) scan=[1, 3, 6, 10]

    // 2.4 try_fold：出错立即短路
    let checked: Result<i32, &'static str> = [5, -2, 3].into_iter().try_fold(0, |acc, value| {
        if value < 0 {
            Err("negative")
        } else {
            Ok(acc + value)
        }
    });
    println!("try_fold={:?}", checked);
    // 预测：try_fold=Err("negative")

    // 2.5 unzip：把 (A, B) 拆成两个集合
    let pairs = [(1, 'a'), (2, 'b')];
    let (numbers, letters): (Vec<i32>, Vec<char>) = pairs.into_iter().unzip();
    println!("unzip={:?} {:?}", numbers, letters);
    // 预测：unzip=[1, 2] ['a', 'b']

    // 2.6 partition：按谓词一分为二
    let (evens, odds): (Vec<i32>, Vec<i32>) = (1..=4).partition(|value| value % 2 == 0);
    println!("partition={:?} {:?}", evens, odds);
    // 预测：partition=[2, 4] [1, 3]

    // 2.7 extend：复用已有容器，不重新 collect
    let mut buffer = vec![1];
    buffer.extend(2..=3);
    println!("extend={:?}", buffer);
    // 预测：extend=[1, 2, 3]

    // 2.8 排序 + 截断：迭代器本身不提供 sort
    let mut top = vec![9, 3, 7, 1];
    top.sort_unstable_by(|left, right| right.cmp(left));
    top.truncate(3);
    println!("top={:?}", top);
    // 预测：top=[9, 7, 3]

    println!("===== 3. APPLICATION =====");

    // 3.1 配置解析：过滤非法行，重复键后者覆盖前者
    let input = "a=1\nb=2\nbad line\na=3";
    let config: BTreeMap<&str, i32> = input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key, value.parse().ok()?))
        })
        .collect();
    println!("config={:?}", config);
    // 预测：config={"a": 3, "b": 2}

    // 3.2 按首字母分组
    let words = ["apple", "avocado", "banana", "cherry", "blueberry"];
    let mut groups: BTreeMap<char, Vec<&str>> = BTreeMap::new();
    for word in words {
        groups
            .entry(word.chars().next().unwrap())
            .or_default()
            .push(word);
    }
    println!("groups={:?}", groups);
    // 预测：groups={'a': ["apple", "avocado"], 'b': ["banana", "blueberry"], 'c': ["cherry"]}

    // 3.3 unzip + partition：把结果拆成两部分
    let data = [("alice", 90), ("bob", 85), ("carol", 95)];
    let (names, scores): (Vec<&str>, Vec<i32>) = data.into_iter().unzip();
    println!("names={:?} scores={:?}", names, scores);
    // 预测：names=["alice", "bob", "carol"] scores=[90, 85, 95]

    let parsed_scores: Vec<Result<i32, _>> = ["12", "x", "34"]
        .iter()
        .map(|text| text.parse::<i32>())
        .collect();
    let (valid, invalid): (Vec<_>, Vec<_>) = parsed_scores.into_iter().partition(Result::is_ok);
    let values: Vec<i32> = valid.into_iter().map(|result| result.unwrap()).collect();
    println!("valid={:?} invalid={}", values, invalid.len());
    // 预测：valid=[12, 34] invalid=1
}
