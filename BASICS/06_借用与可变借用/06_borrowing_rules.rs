// 第 06 课示例：借用与可变借用
//
// 结构：① 原理验证 —— 共享引用可复制、NLL 在最后一次使用处释放、两阶段借用、字段不相交借用
//       ② 语法演示 —— 解引用、自动重借用、split_at_mut、可变迭代、&mut 模式、Deref 强制转换
//       ③ 应用实战 —— &mut [T] 就地修改、结构体方法、切片分段排序、就地反转、借用写入
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 06_borrowing_rules.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::ptr;

struct Pair {
    left: String,
    right: String,
}

struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Self {
        Self { value: 0 }
    }

    // 修改需要独占借用
    fn increment(&mut self) {
        self.value += 1;
    }

    // 只读查询用共享借用
    fn value(&self) -> i32 {
        self.value
    }
}

fn add_one(values: &mut Vec<i32>) {
    values.push(1);
}

fn first_char(text: &str) -> char {
    text.chars().next().unwrap()
}

fn scale(values: &mut [i32], factor: i32) {
    for value in values {
        *value *= factor;
    }
}

fn append_all(target: &mut String, parts: &[&str]) {
    for part in parts {
        target.push_str(part);
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 共享引用 &T 是 Copy：复制之后两个引用都有效，且指向同一地址
    let shared = 42;
    let r1 = &shared;
    let r2 = r1; // &T 是 Copy，r1 仍然可用
    println!(
        "shared_refs_copy={} ptr_eq={}",
        *r1 == *r2,
        ptr::eq(r1, r2)
    );
    // 预测：shared_refs_copy=true ptr_eq=true

    // 1.2 NLL：借用在最后一次使用处结束，不必等到作用域末尾
    let mut values = vec![1, 2, 3];
    let first = &values[0];
    println!("nll_first={}", first);
    values.push(4); // first 的最后一次使用已经过去，这里可以独占借用
    println!("nll_after_push_len={}", values.len());
    // 预测：nll_first=1
    // 预测：nll_after_push_len=4

    // 1.3 两阶段借用：方法接收者的 &mut 先"预留"，参数求值期间仍可共享读
    let mut two_phase = vec![1, 2];
    two_phase.push(two_phase.len());
    println!("two_phase_push={:?}", two_phase);
    // 预测：two_phase_push=[1, 2, 2]

    // 1.4 不相交借用：不同字段可以同时被 &mut 借用
    let mut pair = Pair {
        left: String::from("left"),
        right: String::from("right"),
    };
    let left = &mut pair.left;
    let right = &mut pair.right;
    left.push('!');
    right.push('?');
    println!("disjoint_fields={} {}", left, right);
    // 预测：disjoint_fields=left! right?

    println!("===== 2. SYNTAX =====");

    // 2.1 通过 &mut 解引用修改值
    let mut value = 10;
    let borrowed = &mut value;
    *borrowed += 5;
    println!("deref={}", value);
    // 预测：deref=15

    // 2.2 自动重借用：&mut 不是 Copy，但传给函数时会自动重借，原引用仍可用
    let mut numbers = vec![1];
    let reference = &mut numbers;
    add_one(reference);
    add_one(reference);
    println!("reborrow={:?}", numbers);
    // 预测：reborrow=[1, 1, 1]

    // 2.3 split_at_mut：把一段可变切片拆成两段互不重叠的可变借用
    let mut data = [1, 2, 3, 4];
    let (left_half, right_half) = data.split_at_mut(2);
    left_half[0] = 10;
    right_half[0] = 30;
    println!("split_mut={:?}", data);
    // 预测：split_mut=[10, 2, 30, 4]

    // 2.4 可变迭代：&mut [T] 上的元素可以就地修改
    for element in &mut data {
        *element *= 2;
    }
    println!("iter_mut={:?}", data);
    // 预测：iter_mut=[20, 4, 60, 8]

    // 2.5 模式匹配里的 &mut：直接拿到字段的可变引用
    let mut slot = Some(5);
    if let Some(inner) = &mut slot {
        *inner += 1;
    }
    println!("opt={:?}", slot);
    // 预测：opt=Some(6)

    let mut text_slot = Some(String::from("s"));
    if let Some(text) = text_slot.as_mut() {
        text.push('!');
    }
    println!("as_mut={:?}", text_slot);
    // 预测：as_mut=Some("s!")

    // 2.6 Deref 强制转换：&String 可以当 &str 用，&mut Vec<T> 同理
    let owned = String::from("rust");
    println!("deref_coercion={}", first_char(&owned));
    // 预测：deref_coercion=r

    println!("===== 3. APPLICATION =====");

    // 3.1 就地修改切片：不需要返回值，也不需要克隆
    let mut scaled = [1, 2, 3];
    scale(&mut scaled, 3);
    println!("scaled={:?}", scaled);
    // 预测：scaled=[3, 6, 9]

    // 3.2 &self 查询、&mut self 修改
    let mut counter = Counter::new();
    counter.increment();
    counter.increment();
    println!("counter={}", counter.value());
    // 预测：counter=2

    // 3.3 分段排序：两段可变借用互不重叠，可以同时修改
    let mut values = [3, 1, 4, 1, 5, 9];
    let (first_three, last_three) = values.split_at_mut(3);
    first_three.sort();
    last_three.sort();
    println!("sorted_halves={:?} {:?}", first_three, last_three);
    // 预测：sorted_halves=[1, 3, 4] [1, 5, 9]

    // 3.4 就地反转：&mut [T] 上的标准库算法
    let mut letters = ['a', 'b', 'c'];
    letters.reverse();
    println!("reversed={:?}", letters);
    // 预测：reversed=['c', 'b', 'a']

    // 3.5 借用写入：调用方保留所有权，函数只借用
    let mut output = String::new();
    append_all(&mut output, &["a", "b"]);
    println!("out={}", output);
    // 预测：out=ab
}
