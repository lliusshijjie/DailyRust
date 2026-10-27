// 第 01 课示例：变量绑定、可变性与遮蔽
//
// 结构：① 原理验证 —— mut 只是编译期权限；遮蔽是新建绑定而非赋值；
//                       `let _` 立即丢弃；延迟初始化由数据流分析保证
//       ② 语法演示 —— let 模式、类型标注、mut 位置、const/static、let-else
//       ③ 应用实战 —— 遮蔽式解析流水线、编译期常量、全局配置、Drop 顺序
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 01_bindings_and_mutability.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::mem::size_of;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

// ------------------------------------------------------------------
// ① 原理验证用的观察对象：谁在什么时候被 drop，打印出来一目了然
struct Tag(&'static str);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("  drop {}", self.0);
    }
}

// const 与 static 都必须写全类型，命名用 SCREAMING_SNAKE_CASE
const MAX_POINTS: u32 = 100_000;                // 编译期常量：每个使用点内联，无固定地址
static BANNER: &str = "rust-daily";             // 全局静态：全程序一份，有固定地址
static HITS: AtomicUsize = AtomicUsize::new(0); // 可变全局的正确姿势：原子量
static CONFIG: OnceLock<String> = OnceLock::new(); // 只初始化一次的全局配置

fn add_one(value: i32) -> i32 {
    value + 1
}

// mut 只修饰函数体内的参数绑定，签名里并没有"可变性"这个信息
fn shorten(mut values: Vec<i32>) -> Vec<i32> {
    while values.len() > 1 {
        values.pop();
    }
    values
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 mut 是绑定的权限位，不是类型的一部分，也没有运行时开销
    let fixed: i32 = 7;         // 不可变绑定
    let mut changing: i32 = 7;  // 可变绑定
    changing += 1;
    println!(
        "fixed={} changing={} size={}",
        fixed,
        changing,
        size_of::<i32>()
    );
    // 预测：fixed=7 changing=8 size=4

    //     权限不进入类型系统：同一个函数既能接收 fixed 也能接收 changing
    println!("sum={}", add_one(fixed) + add_one(changing));
    // 预测：sum=17

    // 1.2 遮蔽是新建绑定，类型可以变：&str -> i32
    let shadow = "42";
    let shadow: i32 = shadow.parse().unwrap();
    println!("shadow={}", shadow);
    // 预测：shadow=42

    // 1.3 遮蔽不会提前销毁旧值：旧绑定活到作用域结束，Drop 仍是逆序
    {
        let tag = Tag("first");
        println!("  first tag = {}", tag.0); // 预测：  first tag = first
        let tag = Tag("second"); // 旧 tag 仍活着，只是名字被遮住
        println!("  second tag = {}", tag.0); // 预测：  second tag = second
    }
    // 预测：离开内层作用域时先 drop second、再 drop first

    // 1.4 `let _ =` 在语句结束立即丢弃；`let _name =` 要活到作用域结束
    let _ = Tag("immediate"); // 预测：  drop immediate（就在这一行之后）
    let _held = Tag("held"); // 命名绑定：main 结束才 drop
    println!("  after let _");
    // 预测：  after let _

    // 1.5 延迟初始化：允许先声明后赋值，但所有路径都必须初始化
    let decided: i32;
    if fixed > 0 {
        decided = 1;
    } else {
        decided = -1;
    }
    println!("decided={}", decided);
    // 预测：decided=1

    println!("===== 2. SYNTAX =====");

    // 2.1 模式解构
    let (left, right) = (1, 2);
    let [first, .., last] = [10, 20, 30];
    let (_, only_second) = ("skip", 9);
    println!("{} {} {} {} {}", left, right, first, last, only_second);
    // 预测：1 2 10 30 9

    // 2.2 mut 可以出现在模式里的单个绑定位置上
    let (mut a, b) = (1, 2);
    a += b;
    println!("a={}", a);
    // 预测：a=3

    // 2.3 mut 形参只影响函数体内绑定，调用者无感知
    println!("{:?}", shorten(vec![1, 2, 3]));
    // 预测：[1]

    // 2.4 const 可用于数组长度；static 有固定地址
    const N: usize = 3;
    let zeros = [0u8; N];
    println!(
        "len={} max={} banner={}",
        zeros.len(),
        MAX_POINTS,
        BANNER
    );
    // 预测：len=3 max=100000 banner=rust-daily

    // 2.5 let-else：失败路径提前返回，成功路径保持扁平
    let raw = "12";
    let Some(n) = raw.parse::<i32>().ok() else {
        println!("parse failed");
        return;
    };
    println!("n={}", n);
    // 预测：n=12

    // 2.6 &mut 与 mut 是两件事：&mut T 是可变引用，mut 是绑定权限
    let mut owner = String::from("mut");
    let borrowed: &mut String = &mut owner;
    borrowed.push_str("-ref");
    println!("{}", owner);
    // 预测：mut-ref

    println!("===== 3. APPLICATION =====");

    // 3.1 遮蔽式解析流水线：每一步都换一个新绑定，全程没有 mut
    let input = "  42, 7 ,9 ";
    let input = input.trim();
    let input = input.replace(' ', "");
    let nums: Vec<i32> = input.split(',').map(|s| s.parse().unwrap()).collect();
    println!("{:?}", nums);
    // 预测：[42, 7, 9]

    // 3.2 全局配置：OnceLock 只初始化一次，之后只读共享
    let config = CONFIG.get_or_init(|| String::from("release"));
    println!("config={}", config);
    // 预测：config=release

    // 3.3 可变全局状态：原子量，无需 unsafe 也不需要锁
    for _ in 0..3 {
        HITS.fetch_add(1, Ordering::Relaxed);
    }
    println!("hits={}", HITS.load(Ordering::Relaxed));
    // 预测：hits=3

    // 3.4 最小可变性：能交给迭代器的，就不手写可变累加器
    let mut sum = 0;
    for n in &nums {
        sum += *n;
    }
    let sum_by_iter: i32 = nums.iter().sum();
    println!("sum={} sum_by_iter={}", sum, sum_by_iter);
    // 预测：sum=58 sum_by_iter=58

    // 3.5 `let x = x;` 对非 Copy 类型是移动，对 Copy 类型是拷贝
    let text = String::from("move-me");
    let text = text; // 移动：新绑定接管所有权，没有复制堆上的数据
    println!("text={}", text);
    // 预测：text=move-me

    let number = 7;
    let number_copy = number; // i32 实现 Copy：拷贝一份，两个绑定都可用
    println!("number={} number_copy={}", number, number_copy);
    // 预测：number=7 number_copy=7

    // 3.6 收尾：_held 在 main 结束时才 drop（预测：最后一行输出是 "  drop held"）
}
