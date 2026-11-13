// 第 22 课示例：Option 与组合子
//
// 结构：① 原理验证 —— niche 布局、Some/None、Default/From、? 传播
//       ② 语法演示 —— map/and_then/or_else/filter/map_or、take、as_deref、zip/flatten/transpose、迭代
//       ③ 应用实战 —— 配置链、查找、安全解析、get_or_insert_with 缓存
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 22_option_and_combinators.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::BTreeMap;
use std::mem::size_of;
use std::num::NonZeroU32;

// ? 在返回 Option 的函数里传播 None
fn parse_positive(text: &str) -> Option<i32> {
    let value: i32 = text.parse().ok()?;
    (value > 0).then_some(value)
}

fn find_user<'a>(users: &'a [&'a str], name: &str) -> Option<&'a str> {
    users.iter().copied().find(|user| *user == name)
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 niche 优化：有无效位模式的 T 可以让 Option 不额外占空间
    println!(
        "niche={} option_bool={}",
        size_of::<Option<&u8>>() == size_of::<&u8>()
            && size_of::<Option<NonZeroU32>>() == size_of::<u32>(),
        size_of::<Option<bool>>()
    );
    // 预测：niche=true option_bool=1

    // 1.2 Some / None 是显式的缺失值建模
    let some = Some(42);
    let none: Option<i32> = None;
    println!("some={:?} none_none={}", some, none.is_none());
    // 预测：some=Some(42) none_none=true

    // 1.3 Default 是 None；From<T> 是 Some
    let defaulted: Option<i32> = Default::default();
    let from_value = Option::from(7);
    println!("default_none={} from_some={:?}", defaulted.is_none(), from_value);
    // 预测：default_none=true from_some=Some(7)

    // 1.4 ? 在 Option 函数里传播 None
    println!(
        "question={:?} bad={:?}",
        parse_positive("8"),
        parse_positive("-8")
    );
    // 预测：question=Some(8) bad=None

    println!("===== 2. SYNTAX =====");

    // 2.1 map 与 and_then：普通变换 vs 返回 Option 的变换
    let value = Some(5);
    println!(
        "map={:?} and_then={:?}",
        value.map(|number| number * 2),
        Some("42").and_then(|text| text.parse::<i32>().ok())
    );
    // 预测：map=Some(10) and_then=Some(42)

    // 2.2 or_else 与 filter
    let fallback: Option<i32> = None.or_else(|| Some(0));
    let filtered = value.filter(|number| *number > 10);
    println!("or_else={:?} filter_none={}", fallback, filtered.is_none());
    // 预测：or_else=Some(0) filter_none=true

    // 2.3 map_or 与 map_or_else：后者惰性求默认值
    println!(
        "map_or={} map_or_else={}",
        value.map_or(0, |number| number * 3),
        value.map_or_else(|| 0, |number| number + 1)
    );
    // 预测：map_or=15 map_or_else=6

    // 2.4 take：从 &mut Option 里取出值，留下 None
    let mut slot = Some(5);
    let taken = slot.take();
    println!("take={:?} left={:?}", taken, slot);
    // 预测：take=Some(5) left=None

    // 2.5 as_deref：Option<String> -> Option<&str>
    let owned = Some(String::from("hello"));
    let borrowed: Option<&str> = owned.as_deref();
    println!("deref={:?}", borrowed);
    // 预测：deref=Some("hello")

    // 2.6 zip / flatten / transpose
    let nested: Option<Option<i32>> = Some(Some(3));
    println!(
        "zip={:?} flatten={:?}",
        Some(1).zip(Some('a')),
        nested.flatten()
    );
    // 预测：zip=Some((1, 'a')) flatten=Some(3)

    let transposed = Some(Ok::<i32, &str>(1)).transpose();
    println!("transpose={:?}", transposed);
    // 预测：transpose=Ok(Some(1))

    // 2.7 Option 也是 0/1 个元素的迭代器
    println!(
        "iter_sum={} is_some_and={}",
        Some(1).into_iter().sum::<i32>(),
        Some(4).is_some_and(|number| number % 2 == 0)
    );
    // 预测：iter_sum=1 is_some_and=true

    println!("===== 3. APPLICATION =====");

    // 3.1 配置链：查表 -> 解析 -> 默认值
    let config: BTreeMap<&str, &str> =
        BTreeMap::from([("port", "8080"), ("host", "localhost")]);
    let port: Option<u16> = config.get("port").and_then(|raw| raw.parse().ok());
    let host = config.get("host").copied();
    println!("config={:?} host={:?}", port, host);
    // 预测：config=Some(8080) host=Some("localhost")

    // 3.2 查找：找不到就是 None，不抛异常
    println!(
        "user={:?} missing={:?}",
        find_user(&["alice", "bob"], "bob"),
        find_user(&["alice"], "carol")
    );
    // 预测：user=Some("bob") missing=None

    // 3.3 安全解析：非法项跳过，合法项求和
    let total: i32 = ["10", "x", "30"]
        .iter()
        .filter_map(|text| text.parse::<i32>().ok())
        .sum();
    println!("sum={}", total);
    // 预测：sum=40

    // 3.4 get_or_insert_with：惰性初始化缓存
    let mut cache: Option<String> = None;
    let cached = cache.get_or_insert_with(|| String::from("computed"));
    println!("cache={}", cached);
    // 预测：cache=computed
}
