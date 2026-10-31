// 第 10 课示例：字符串——String、&str 与 UTF-8
//
// 结构：① 原理验证 —— String 是 3 字头部、&str 是 2 字胖指针、UTF-8 字节数 vs 字符数
//       ② 语法演示 —— 构建/拼接、char_indices、切片边界、split/lines/trim、查找替换、parse、大小写
//       ③ 应用实战 —— 词频统计、CSV 求和、回文判断、write! 报告、UTF-8 校验
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 10_strings_and_utf8.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::BTreeMap;
use std::fmt::Write;
use std::mem::{size_of, size_of_val};

// 参数用 &str：String、字符串字面量、子切片都能传
fn shout(text: &str) -> String {
    text.to_uppercase()
}

// Unicode 安全的回文判断：只保留字母数字，大小写折叠后比较字符序列
fn is_palindrome(text: &str) -> bool {
    let cleaned: Vec<char> = text
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    cleaned.iter().eq(cleaned.iter().rev())
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 String 是拥有所有权的堆缓冲（ptr/len/cap 三个字），&str 是胖指针（ptr+len）
    println!(
        "string_three_words={} str_two_words={}",
        size_of::<String>() == 3 * size_of::<usize>(),
        size_of::<&str>() == 2 * size_of::<usize>()
    );
    // 预测：string_three_words=true str_two_words=true

    // 1.2 str 本身是未定长类型：size_of_val 返回"指向的字节数"，不是指针大小
    println!("str_val_size={}", size_of_val("rust"));
    // 预测：str_val_size=4

    // 1.3 len() 数的是 UTF-8 字节，chars().count() 数的是 Unicode 标量值
    println!(
        "utf8=rust:{}:{} zh:{}:{}",
        "rust".len(),
        "rust".chars().count(),
        "中文".len(),
        "中文".chars().count()
    );
    // 预测：utf8=rust:4:4 zh:6:2

    // 1.4 String 的容量至少覆盖长度（可能更大，取决于分配策略）
    let buffer = String::from("hello");
    println!("capacity_ok={}", buffer.capacity() >= buffer.len());
    // 预测：capacity_ok=true

    println!("===== 2. SYNTAX =====");

    // 2.1 构建：push(char) 与 push_str(&str)
    let mut built = String::from("hello");
    built.push(' ');
    built.push_str("world");
    println!("built={} len={}", built, built.len());
    // 预测：built=hello world len=11

    // 2.2 + 只接受 String + &str，并且会移动左边的 String
    let left = String::from("foo");
    let right = String::from("bar");
    let combined = left + right.as_str();
    println!("concat={} right_kept={}", combined, right);
    // 预测：concat=foobar right_kept=bar

    // 2.3 format! 借用参数，不移动任何值
    let name = "Rust";
    let formatted = format!("{name}-{}", 2024);
    println!("formatted={}", formatted);
    // 预测：formatted=Rust-2024

    // 2.4 UTF-8 变长编码：每个字符的起始字节下标与字节长度
    for (index, ch) in "a中b".char_indices() {
        println!("char_index={} utf8_len={}", index, ch.len_utf8());
    }
    // 预测：char_index=0 utf8_len=1
    // 预测：char_index=1 utf8_len=3
    // 预测：char_index=4 utf8_len=1

    // 2.5 字节切片：ASCII 下安全；跨字符边界会 panic，用 get 得到 Option
    let text = String::from("abcdef");
    println!("slice={}", &text[1..4]);
    // 预测：slice=bcd

    let zh = String::from("中文");
    println!(
        "zh_valid={} zh_invalid={}",
        zh.get(0..3).is_some(),
        zh.get(0..2).is_none()
    );
    // 预测：zh_valid=true zh_invalid=true

    // 2.6 拆分：split 返回借用原串的 &str 切片
    let csv = "a,b,c";
    println!("split={:?}", csv.split(',').collect::<Vec<_>>());
    // 预测：split=["a", "b", "c"]

    println!(
        "lines={} trim={}",
        "one\r\ntwo\nthree".lines().count(),
        "  pad  ".trim()
    );
    // 预测：lines=3 trim=pad

    // 2.7 查找与替换：find 返回字节下标，replace 返回新 String
    let sentence = "the cat sat on the mat";
    println!(
        "find={:?} replace={}",
        sentence.find("cat"),
        sentence.replace("cat", "dog")
    );
    // 预测：find=Some(4) replace=the dog sat on the mat

    // 2.8 parse 与 Unicode 大小写
    println!("parsed={}", "42".parse::<i32>().unwrap() + 1);
    // 预测：parsed=43

    println!("upper={}", "Straße".to_uppercase());
    // 预测：upper=STRASSE

    // 2.9 按字符反转（按字节反转会破坏 UTF-8）
    let reversed: String = "abc".chars().rev().collect();
    println!("reversed={}", reversed);
    // 预测：reversed=cba

    // 2.10 &String 自动退化为 &str
    let owned = String::from("hi");
    println!("shout={}", shout(&owned));
    // 预测：shout=HI

    println!("===== 3. APPLICATION =====");

    // 3.1 词频统计：键借用原文的 &str，不发生字符串复制
    let text = "the cat and the dog and the bird";
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    println!("counts={:?}", counts);
    // 预测：counts={"and": 2, "bird": 1, "cat": 1, "dog": 1, "the": 3}

    // 3.2 逐行解析 CSV：lines 处理 \n 与 \r\n，split_once 取两段
    let data = "name,score\nalice,90\nbob,85";
    let mut total = 0;
    for line in data.lines().skip(1) {
        if let Some((_, score)) = line.split_once(',') {
            total += score.parse::<i32>().unwrap_or(0);
        }
    }
    println!("score_total={}", total);
    // 预测：score_total=175

    // 3.3 Unicode 安全的回文：忽略大小写与标点
    println!(
        "palindrome={} {}",
        is_palindrome("Never odd or even"),
        is_palindrome("rust")
    );
    // 预测：palindrome=true false

    // 3.4 用 write! 往已有 String 里追加，避免 format! 反复分配
    let mut report = String::new();
    writeln!(report, "name={}", "daily").unwrap();
    writeln!(report, "lessons={}", 10).unwrap();
    println!(
        "report_lines={} report_len={}",
        report.lines().count(),
        report.len()
    );
    // 预测：report_lines=2 report_len=22

    // 3.5 UTF-8 校验：from_utf8 返回 Result，lossy 用替换字符兜底
    let valid = String::from_utf8(vec![104, 105]);
    let invalid = String::from_utf8(vec![0xFF]);
    let lossy = String::from_utf8_lossy(&[0xFF, b'a']);
    println!(
        "utf8_valid={} invalid_err={} lossy_len={}",
        valid.is_ok(),
        invalid.is_err(),
        lossy.len()
    );
    // 预测：utf8_valid=true invalid_err=true lossy_len=4

    // 3.6 String 与 Vec<u8> 的零拷贝互转
    let bytes = String::from("hi").into_bytes();
    println!("into_bytes={:?}", bytes);
    // 预测：into_bytes=[104, 105]
}
