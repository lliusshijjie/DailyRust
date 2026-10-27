// 第 02 课示例：表达式与控制流
//
// 结构：① 原理验证 —— 块是表达式、if 是表达式、loop 的 break 值、发散类型 !
//       ② 语法演示 —— 尾表达式与分号、if 链、while/for、标签跳转、while let、带值 break
//       ③ 应用实战 —— 用 break 值找目标、重试循环、早退、Collatz
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 02_expressions_and_control_flow.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::mem::size_of_val;

// 早退：前置条件不满足时立刻结束函数，而不是层层缩进
fn checked_even(value: i32) -> i32 {
    if value % 2 != 0 {
        return -1;
    }
    value
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    let mut divisor = 2;
    while divisor * divisor <= n {
        if n % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

// 用 loop 的 break 值直接返回结果，不需要外部的 mut 结果变量
fn find_first_prime(threshold: u32) -> u32 {
    let mut candidate = threshold + 1;
    loop {
        if is_prime(candidate) {
            break candidate;
        }
        candidate += 1;
    }
}

fn collatz_len(mut n: u64) -> u64 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 块是表达式：块的值 = 最后一个不带分号的表达式
    let block_value = {
        let a = 2;
        let b = 3;
        a * b // 尾表达式，没有分号
    };
    println!("block_value={}", block_value);
    // 预测：block_value=6

    //     给尾表达式加上分号，块的值就变成单元类型 ()
    let unit_value = {
        5;
    };
    println!("unit_size={}", size_of_val(&unit_value));
    // 预测：unit_size=0

    // 1.2 if 是表达式：两个分支同型，所以可以直接绑定
    let sign = if block_value > 0 {
        "positive"
    } else {
        "non-positive"
    };
    println!("sign={}", sign);
    // 预测：sign=positive

    // 1.3 loop 是唯一能用 break 带出值的循环
    let mut n = 0;
    let first_square_over_50 = loop {
        n += 1;
        if n * n > 50 {
            break n; // break 的值就是整个 loop 表达式的值
        }
    };
    println!("first_square_over_50={}", first_square_over_50);
    // 预测：first_square_over_50=8

    // 1.4 发散类型 !：panic!/return 能统一到任意类型，所以这种 if 合法
    let status: i32 = if block_value > 0 {
        1
    } else {
        panic!("unreachable branch") // 类型是 !，可强转为 i32
    };
    println!("status={}", status);
    // 预测：status=1

    println!("===== 2. SYNTAX =====");

    // 2.1 if / else if / else：条件必须是 bool，分支必须同型
    let score = 83;
    let level = if score >= 90 {
        "A"
    } else if score >= 80 {
        "B"
    } else {
        "C"
    };
    println!("level={}", level);
    // 预测：level=B

    // 2.2 while：条件循环，表达式类型是 ()
    let mut countdown = 3;
    while countdown > 0 {
        print!("{countdown}");
        countdown -= 1;
    }
    println!(" lift-off");
    // 预测：321 lift-off

    // 2.3 for：遍历任何 IntoIterator；范围是惰性的，rev() 反向
    let total: i32 = (1..=4).sum();
    for step in (1..=3).rev() {
        print!("{step}");
    }
    println!(" total={}", total);
    // 预测：321 total=10

    // 2.4 标签：一次跳出多层循环，不必引入布尔标志
    let mut found = None;
    'rows: for row in 0..3 {
        for col in 0..3 {
            if row == 1 && col == 2 {
                found = Some((row, col));
                break 'rows; // 带标签的 break 跳出外层
            }
        }
    }
    println!("found={:?}", found);
    // 预测：found=Some((1, 2))

    // 2.5 while let：把"直到取不到值"直接写进循环条件
    let mut stack = vec![1, 2, 3];
    let mut popped = Vec::new();
    while let Some(top) = stack.pop() {
        popped.push(top);
    }
    println!("popped={:?}", popped);
    // 预测：popped=[3, 2, 1]

    // 2.6 块表达式收敛临时作用域：中间变量不会泄漏到外层
    let parsed = {
        let raw = "  42 ";
        raw.trim().parse::<i32>().unwrap()
    };
    println!("parsed={}", parsed);
    // 预测：parsed=42

    // 2.7 早退：前置检查失败就 return，主逻辑保持扁平
    println!("checked_even(10)={}", checked_even(10));
    // 预测：checked_even(10)=10
    println!("checked_even(7)={}", checked_even(7));
    // 预测：checked_even(7)=-1

    // 2.8 带标签的块：把"提前退出一个表达式"写成 break 'label value（1.65+）
    let picked = 'search: {
        for i in 1..10 {
            if i * i > 30 {
                break 'search i;
            }
        }
        -1
    };
    println!("picked={}", picked);
    // 预测：picked=6

    println!("===== 3. APPLICATION =====");

    // 3.1 用 break 值替代"外部 mut 结果变量"：找到即返回
    println!("first prime > 100 = {}", find_first_prime(100));
    // 预测：first prime > 100 = 101

    // 3.2 重试循环：continue 表示"这轮不算"，break 带出最终结果
    let mut attempts = 0;
    let outcome = loop {
        attempts += 1;
        if attempts < 3 {
            continue; // 前两次视为失败，继续重试
        }
        break "ok after 3 attempts";
    };
    println!("{} (attempts={})", outcome, attempts);
    // 预测：ok after 3 attempts (attempts=3)

    // 3.3 for + enumerate 找位置，命中即 break
    let values = [4, 8, 15, 16, 23, 42];
    let mut position = None;
    for (index, value) in values.iter().enumerate() {
        if *value > 20 {
            position = Some(index);
            break;
        }
    }
    println!("position={:?}", position);
    // 预测：position=Some(4)

    // 3.4 用 if 表达式生成配置值，避免先声明 mut 再分支赋值
    let verbose = false;
    let workers = if verbose { 8 } else { 2 };
    println!("workers={}", workers);
    // 预测：workers=2

    // 3.5 循环 + 表达式组合：Collatz 序列步数
    println!("collatz(27)={}", collatz_len(27));
    // 预测：collatz(27)=111
}
