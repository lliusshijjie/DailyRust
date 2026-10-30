// 第 09 课示例：枚举与模式匹配
//
// 结构：① 原理验证 —— 带标签联合体的布局、判别值、niche 优化
//       ② 语法演示 —— match 穷尽性、守卫、or/范围/@、if let/while let/let-else、绑定模式
//       ③ 应用实战 —— 表达式求值、状态机、命令解析
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 09_enums_and_matching.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::mem::size_of;
use std::num::NonZeroU32;

// 枚举 = 带标签的联合体：同一时刻只有一个变体是"活的"
enum Shape {
    Point,
    Circle(f64),
    Rect { width: f64, height: f64 },
}

impl Shape {
    fn name(&self) -> &'static str {
        match self {
            Shape::Point => "point",
            Shape::Circle(_) => "circle",
            Shape::Rect { .. } => "rect",
        }
    }

    fn area(&self) -> f64 {
        match self {
            Shape::Point => 0.0,
            Shape::Circle(radius) => std::f64::consts::PI * *radius * *radius,
            Shape::Rect { width, height } => *width * *height,
        }
    }
}

// 无数据的枚举可以指定判别值，也能 as 转整数
enum Level {
    Low = 1,
    Mid = 5,
    High = 10,
}

fn classify(level: Level) -> &'static str {
    match level {
        Level::Low => "low",
        Level::Mid => "mid",
        Level::High => "high",
    }
}

fn describe_number(value: i32) -> String {
    match value {
        0 => String::from("zero"),
        n @ 1..=5 => format!("small({n})"),
        n if n < 0 => format!("negative({n})"),
        n if n % 2 == 0 => format!("even({n})"),
        _ => String::from("other"),
    }
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

// 变体可以带不同类型的数据：单元、结构体、元组
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(u8, u8, u8),
}

fn summarize(message: &Message) -> String {
    match message {
        Message::Quit => String::from("quit"),
        Message::Move { x, y } => format!("move {x},{y}"),
        Message::Write(text) => format!("write {}", text.len()),
        Message::ChangeColor(r, g, b) => format!("color {r},{g},{b}"),
    }
}

// let-else：可失败的解构，失败分支必须发散
fn first_even(values: &[i32]) -> i32 {
    let Some(&value) = values.iter().find(|v| **v % 2 == 0) else {
        return -1;
    };
    value
}

// 递归枚举必须经过 Box 间接层，否则类型大小无限
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn eval(expr: &Expr) -> i64 {
    match expr {
        Expr::Num(value) => *value,
        Expr::Add(left, right) => eval(left) + eval(right),
        Expr::Mul(left, right) => eval(left) * eval(right),
    }
}

// 枚举 + 方法：把状态迁移写成 match self
enum State {
    Idle,
    Running(u32),
    Done,
}

impl State {
    fn next(self) -> State {
        match self {
            State::Idle => State::Running(0),
            State::Running(n) if n < 2 => State::Running(n + 1),
            State::Running(_) => State::Done,
            State::Done => State::Done,
        }
    }

    fn label(&self) -> String {
        match self {
            State::Idle => String::from("idle"),
            State::Running(n) => format!("running({n})"),
            State::Done => String::from("done"),
        }
    }
}

enum Command {
    Quit,
    Echo(String),
    Move { x: i32, y: i32 },
}

fn parse_command(line: &str) -> Option<Command> {
    let mut parts = line.split_whitespace();
    match parts.next()? {
        "quit" => Some(Command::Quit),
        "echo" => Some(Command::Echo(parts.collect::<Vec<_>>().join(" "))),
        "move" => {
            let x = parts.next()?.parse().ok()?;
            let y = parts.next()?.parse().ok()?;
            Some(Command::Move { x, y })
        }
        _ => None,
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 带标签联合体：标签 + 最大负载（Shape 里 Rect 的 16 字节负载）
    println!(
        "shape_size={} level_size={}",
        size_of::<Shape>(),
        size_of::<Level>()
    );
    // 预测：shape_size=24 level_size=1

    // 1.2 无数据枚举的判别值可以直接 as
    println!("level_high={}", Level::High as u8);
    // 预测：level_high=10

    // 1.3 niche 优化：编译器用"无效位模式"表示 None，不额外占字节
    println!(
        "niche_box={} niche_nonzero={} option_u8={} option_bool={}",
        size_of::<Option<Box<u8>>>() == size_of::<Box<u8>>(),
        size_of::<Option<NonZeroU32>>() == size_of::<u32>(),
        size_of::<Option<u8>>(),
        size_of::<Option<bool>>()
    );
    // 预测：niche_box=true niche_nonzero=true option_u8=2 option_bool=1

    println!("===== 2. SYNTAX =====");

    // 2.1 match 必须穷尽，变体解构直接绑定负载
    let shapes = [
        Shape::Point,
        Shape::Circle(1.0),
        Shape::Rect {
            width: 2.0,
            height: 3.0,
        },
    ];
    for shape in &shapes {
        println!("{} area={:.2}", shape.name(), shape.area());
    }
    // 预测：point area=0.00
    // 预测：circle area=3.14
    // 预测：rect area=6.00

    println!(
        "classify={} {} {}",
        classify(Level::Low),
        classify(Level::Mid),
        classify(Level::High)
    );
    // 预测：classify=low mid high

    // 2.2 守卫、范围、@ 绑定：按顺序命中第一个匹配的臂
    println!(
        "numbers={} {} {} {} {}",
        describe_number(0),
        describe_number(3),
        describe_number(-2),
        describe_number(6),
        describe_number(7)
    );
    // 预测：numbers=zero small(3) negative(-2) even(6) other

    // 2.3 不同变体携带不同类型的数据
    let messages = [
        Message::Quit,
        Message::Move { x: 3, y: 4 },
        Message::Write(String::from("hello")),
        Message::ChangeColor(1, 2, 3),
    ];
    for message in &messages {
        println!("{}", summarize(message));
    }
    // 预测：quit
    // 预测：move 3,4
    // 预测：write 5
    // 预测：color 1,2,3

    // 2.4 绑定模式：匹配 &Option<T> 时绑定自动变成引用
    let maybe = Some(String::from("hello"));
    if let Some(text) = &maybe {
        println!("borrowed_len={}", text.len());
    }
    // 预测：borrowed_len=5

    // 2.5 matches!：只关心"是否匹配"，不取负载
    println!(
        "is_quit={} vowel={} vowel_x={}",
        matches!(Message::Quit, Message::Quit),
        is_vowel('e'),
        is_vowel('x')
    );
    // 预测：is_quit=true vowel=true vowel_x=false

    // 2.6 while let：直到解构失败为止
    let mut stack = vec![1, 2, 3];
    let mut sum = 0;
    while let Some(top) = stack.pop() {
        sum += top;
    }
    println!("while_let_sum={}", sum);
    // 预测：while_let_sum=6

    // 2.7 let-else：可失败解构，失败分支 return
    println!(
        "first_even={} none={}",
        first_even(&[3, 5, 8, 9]),
        first_even(&[1, 3])
    );
    // 预测：first_even=8 none=-1

    // 2.8 or-pattern：多个模式共享同一个绑定
    let point = (0, 5);
    match point {
        (0, y) | (y, 0) => println!("axis={y}"),
        _ => println!("free"),
    }
    // 预测：axis=5

    println!("===== 3. APPLICATION =====");

    // 3.1 递归枚举 + Box：表达式树求值
    let expr = Expr::Add(
        Box::new(Expr::Num(2)),
        Box::new(Expr::Mul(
            Box::new(Expr::Num(3)),
            Box::new(Expr::Num(4)),
        )),
    );
    println!("expr={}", eval(&expr));
    // 预测：expr=14

    // 3.2 状态机：match self 决定迁移
    let mut state = State::Idle;
    for _ in 0..5 {
        state = state.next();
        println!("state={}", state.label());
    }
    // 预测：state=running(0)
    // 预测：state=running(1)
    // 预测：state=running(2)
    // 预测：state=done
    // 预测：state=done

    // 3.3 命令解析：split + ? + 枚举建模
    let lines = ["quit", "echo hello world", "move 3 4"];
    let commands: Vec<Command> = lines.iter().filter_map(|line| parse_command(line)).collect();
    for command in &commands {
        match command {
            Command::Quit => println!("command=quit"),
            Command::Echo(text) => println!("command=echo len={}", text.len()),
            Command::Move { x, y } => println!("command=move {x},{y}"),
        }
    }
    // 预测：command=quit
    // 预测：command=echo len=11
    // 预测：command=move 3,4
}
