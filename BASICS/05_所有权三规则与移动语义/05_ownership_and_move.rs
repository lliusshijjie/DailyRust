// 第 05 课示例：所有权三规则与移动语义
//
// 结构：① 原理验证 —— 移动只搬头部不复制堆、移动不触发析构、String 是 3 字头部
//       ② 语法演示 —— Copy/Clone、传参与返回、部分移动、重初始化、take/swap、Drop 顺序
//       ③ 应用实战 —— 队列、借用代替移动、Rc 共享、消费型 builder、mem::take
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 05_ownership_and_move.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::mem::{size_of, swap, take};
use std::rc::Rc;

// 观察析构时机：移动不会 drop 源值，最终所有者离开作用域才 drop
struct Tracked(&'static str);

impl Drop for Tracked {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Copy 要求所有字段都 Copy；derive Copy 必须同时 derive Clone
#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Pair {
    left: String,
    right: String,
}

#[derive(Debug)]
struct Request {
    url: String,
    timeout: u64,
}

impl Request {
    fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            timeout: 10,
        }
    }

    // 消费 self 的 builder：拿走所有权，修改后把新值交回来
    fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }
}

fn consume(text: String) -> usize {
    text.len()
}

fn produce() -> String {
    String::from("made")
}

// 借用而不是拿走所有权：调用方之后还能继续用 items
fn total_len(items: &[String]) -> usize {
    items.iter().map(|item| item.len()).sum()
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 String 的所有权只是栈上的 3 字头部（ptr/len/cap），堆缓冲不跟着搬家
    println!(
        "string_three_words={}",
        size_of::<String>() == 3 * size_of::<usize>()
    );
    // 预测：string_three_words=true

    let original = String::from("heap-buffer");
    let heap_ptr = original.as_ptr();
    let moved = original; // 移动：只复制 3 字头部
    println!("same_heap_after_move={}", heap_ptr == moved.as_ptr());
    // 预测：same_heap_after_move=true

    // 1.2 移动不会析构源值：drop 只发生在最终所有者身上
    println!("before move");
    {
        let tracked = Tracked("owner-inner");
        let moved_tracked = tracked; // 移动，不 drop
        println!("moved {}", moved_tracked.0);
    }
    println!("after move");
    // 预测：before move / moved owner-inner / drop owner-inner / after move

    println!("===== 2. SYNTAX =====");

    // 2.1 Copy：按位复制，两个绑定都有效
    let copy_a = 7i32;
    let copy_b = copy_a;
    println!("copy_value={} {}", copy_a, copy_b);
    // 预测：copy_value=7 7

    let point = Point { x: 3, y: 4 };
    let point_copy = point;
    println!(
        "copy_struct={:?} {:?} sum={}",
        point,
        point_copy,
        point.x + point_copy.y
    );
    // 预测：copy_struct=Point { x: 3, y: 4 } Point { x: 3, y: 4 } sum=7

    // 2.2 移动：所有权转交，源绑定之后不可再用（编译期检查 E0382）
    let owned = String::from("hello");
    let moved_string = owned; // 此后使用 owned 会编译失败
    println!("moved_len={}", moved_string.len());
    // 预测：moved_len=5

    // 2.3 传参与返回都是移动；clone 是显式深拷贝，堆缓冲会另分配
    let text = String::from("abcdef");
    let consumed_len = consume(text);
    let made = produce();
    println!("consumed_len={} returned_len={}", consumed_len, made.len());
    // 预测：consumed_len=6 returned_len=4

    let source = String::from("clone-me");
    let duplicate = source.clone();
    println!(
        "clone_same_heap={} equal={}",
        source.as_ptr() == duplicate.as_ptr(),
        source == duplicate
    );
    // 预测：clone_same_heap=false equal=true

    // 2.4 部分移动：从非 Drop 结构体里移出一个字段，其余字段仍可用
    let pair = Pair {
        left: String::from("left"),
        right: String::from("right"),
    };
    let left = pair.left;
    println!("partial_move={} {}", left, pair.right);
    // 预测：partial_move=left right

    // 2.5 移动之后可以重新初始化；条件分支里只要所有路径都完成初始化即可
    let mut state = String::from("first");
    let taken_state = state; // 移走
    state = String::from("second"); // 重新初始化，state 又有效了
    println!("reinit_after_move={} taken={}", state, taken_state);
    // 预测：reinit_after_move=second taken=first

    let mut maybe = String::from("alive");
    if maybe.len() > 3 {
        drop(maybe);
        maybe = String::from("reborn"); // 另一条路径没动过 maybe
    }
    println!("conditional={}", maybe);
    // 预测：conditional=reborn

    // 2.6 从可变借用里"取出"值：Option::take 与 mem::swap
    let mut slot = Some(String::from("payload"));
    let taken_slot = slot.take();
    println!("take={:?} rest={:?}", taken_slot, slot);
    // 预测：take=Some("payload") rest=None

    let mut first = String::from("A");
    let mut second = String::from("B");
    swap(&mut first, &mut second);
    println!("swap={} {}", first, second);
    // 预测：swap=B A

    // 2.7 Rc 的 clone 只增加引用计数，不复制底层数据
    let shared = Rc::new(5);
    let shared2 = Rc::clone(&shared);
    println!("rc_count={} shared={}", Rc::strong_count(&shared), *shared2);
    // 预测：rc_count=2 shared=5

    // 2.8 同一作用域内局部变量按声明逆序析构
    {
        let _first = Tracked("first");
        let _second = Tracked("second");
        println!("drop_order_start");
    }
    // 预测：drop_order_start / drop second / drop first

    println!("===== 3. APPLICATION =====");

    // 3.1 容器：push 把所有权移进去，pop 把所有权交出来
    let mut queue = vec![
        String::from("a"),
        String::from("b"),
        String::from("c"),
    ];
    queue.push(String::from("d"));
    let popped = queue.pop().unwrap();
    println!("queue_len={} popped={}", queue.len(), popped);
    // 预测：queue_len=3 popped=d

    // 3.2 借用代替移动：只读统计不需要拿走所有权
    let items = vec![String::from("rust"), String::from("move")];
    println!("total_len={}", total_len(&items));
    // 预测：total_len=8

    // 3.3 Rc：单线程里的共享所有权，clone 只加计数
    let doc = Rc::new(String::from("shared-doc"));
    let doc2 = Rc::clone(&doc);
    println!(
        "doc_owners={} same={}",
        Rc::strong_count(&doc),
        Rc::ptr_eq(&doc, &doc2)
    );
    // 预测：doc_owners=2 same=true

    // 3.4 消费型 builder：每个方法拿走 self，链式返回新值
    let request = Request::new("api.example.com").timeout(30);
    println!(
        "builder={:?} url_len={} timeout={}",
        request,
        request.url.len(),
        request.timeout
    );
    // 预测：builder=Request { url: "api.example.com", timeout: 30 } url_len=15 timeout=30

    // 3.5 mem::take：取出字段的旧值并留下 Default 空值，结构体本身不被移动
    let mut config = Pair {
        left: String::from("key"),
        right: String::from("value"),
    };
    let value = take(&mut config.right);
    println!(
        "reset={:?} kept={:?} taken={:?}",
        config.right, config.left, value
    );
    // 预测：reset="" kept="key" taken="value"
}
