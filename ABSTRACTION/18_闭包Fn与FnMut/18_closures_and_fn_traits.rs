// 第 18 课示例：闭包——Fn / FnMut / FnOnce
//
// 结构：① 原理验证 —— Fn/FnMut/FnOnce 的能力层级、函数指针、Copy 闭包、不相交捕获
//       ② 语法演示 —— 返回 FnMut 闭包、Box<dyn Fn>、泛型 Handler、迭代器适配器
//       ③ 应用实战 —— 中间件管线、事件处理器列表、ID 生成器
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 18_closures_and_fn_traits.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

// 只读闭包：Fn 约束；同时也能当 FnMut/FnOnce 用
fn call_fn<F: Fn() -> i32>(function: F) -> i32 {
    function()
}

// 可多次调用的可变闭包：FnMut 约束
fn call_fn_once<F: FnOnce() -> String>(function: F) -> String {
    function()
}

fn add_one(value: i32) -> i32 {
    value + 1
}

// 返回闭包：必须 move 把局部状态带进闭包
fn make_counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

// 泛型结构体存储闭包：F 由调用方决定
struct Handler<F: Fn(i32) -> i32> {
    function: F,
}

impl<F: Fn(i32) -> i32> Handler<F> {
    fn run(&self, value: i32) -> i32 {
        (self.function)(value)
    }
}

struct Pair {
    left: String,
    right: String,
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 Fn：只读捕获，可以反复调用
    let base = 40;
    let read = || base + 2;
    println!("fn_trait={}", call_fn(read));
    // 预测：fn_trait=42

    // 1.2 FnMut：捕获可变引用，调用时需要可变闭包
    let mut count = 0;
    let mut incrementing = || {
        count += 1;
        count
    };
    println!("fn_mut={},{}", incrementing(), incrementing());
    // 预测：fn_mut=1,2

    // 1.3 FnOnce：把捕获值的所有权交出去，只能调用一次
    let owned = String::from("owned");
    let consume = move || owned;
    println!("fn_once={}", call_fn_once(consume));
    // 预测：fn_once=owned

    // 1.4 函数指针：无捕获闭包可以强制转成 fn 指针
    let pointer: fn(i32) -> i32 = add_one;
    let closure_pointer: fn(i32) -> i32 = |value| value + 1;
    println!("fn_pointer={}", pointer(2) + closure_pointer(1));
    // 预测：fn_pointer=5

    // 1.5 捕获 Copy 值的 move 闭包本身也是 Copy
    let captured = 10;
    let first = move || captured;
    let second = first;
    println!("copy_closure={} {}", first(), second());
    // 预测：copy_closure=10 10

    // 1.6 不相交捕获（Rust 2021+）：闭包只借用用到的字段
    let mut pair = Pair {
        left: String::from("abc"),
        right: String::from("x"),
    };
    let left_len = || pair.left.len();
    pair.right.push('!');
    println!("disjoint=len{} right{}", left_len(), pair.right);
    // 预测：disjoint=len3 rightx!

    println!("===== 2. SYNTAX =====");

    // 2.1 返回闭包：impl FnMut + move
    let mut next = make_counter();
    println!("counter={},{},{}", next(), next(), next());
    // 预测：counter=1,2,3

    // 2.2 Box<dyn Fn>：把不同闭包装进同一个容器
    let ops: Vec<Box<dyn Fn(i32) -> i32>> = vec![
        Box::new(|value| value + 1),
        Box::new(|value| value * 2),
    ];
    let results: Vec<i32> = ops.iter().map(|op| op(5)).collect();
    println!("boxed={:?}", results);
    // 预测：boxed=[6, 10]

    // 2.3 泛型结构体存储闭包
    let handler = Handler {
        function: |value| value * 3,
    };
    println!("handler={}", handler.run(5));
    // 预测：handler=15

    // 2.4 闭包作为迭代器适配器
    let doubled: Vec<i32> = (1..=3).map(|value| value * 2).collect();
    println!("adapters={:?}", doubled);
    // 预测：adapters=[2, 4, 6]

    println!("===== 3. APPLICATION =====");

    // 3.1 中间件管线：每个 Stage 是 Fn(i32) -> i32
    let pipeline: Vec<Box<dyn Fn(i32) -> i32>> = vec![
        Box::new(|value| value + 1),
        Box::new(|value| value * 3),
        Box::new(|value| value - 2),
    ];
    let result = pipeline.iter().fold(5, |value, step| step(value));
    println!("pipeline={}", result);
    // 预测：pipeline=16

    // 3.2 事件处理器列表：借用事件字符串，返回处理结果
    let handlers: Vec<Box<dyn Fn(&str) -> String>> = vec![
        Box::new(|event| format!("log:{}", event)),
        Box::new(|event| format!("metric:{}", event.len())),
    ];
    let outputs: Vec<String> = handlers.iter().map(|handler| handler("click")).collect();
    println!("handlers={}", outputs.join(" | "));
    // 预测：handlers=log:click | metric:5

    // 3.3 ID 生成器：FnMut 闭包持有自增状态
    let mut next_id = make_counter();
    let ids: Vec<u32> = (0..3).map(|_| next_id()).collect();
    println!("ids={:?}", ids);
    // 预测：ids=[1, 2, 3]
}
