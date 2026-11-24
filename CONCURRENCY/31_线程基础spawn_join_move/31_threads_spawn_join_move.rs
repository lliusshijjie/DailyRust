// 第 31 课示例：线程基础——spawn、join 与 move
//
// 结构：① 原理验证 —— spawn 返回值、join 捕获 panic、available_parallelism
//       ② 语法演示 —— Builder 命名线程、move 转移所有权、scope 借用栈数据
//       ③ 应用实战 —— 并行求和、分块词频、worker panic 隔离
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 31_threads_spawn_join_move.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::panic;
use std::thread;

// 静音 hook 后运行一个会 panic 的线程，避免 stderr 干扰示例输出
fn run_thread_silently<F, T>(function: F) -> thread::Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let handle = thread::spawn(function);
    let result = handle.join();
    panic::set_hook(previous);
    result
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 spawn 返回 JoinHandle；join 等待线程并把返回值带回来
    let handle = thread::spawn(|| 21 * 2);
    println!("spawn_value={}", handle.join().unwrap());
    // 预测：spawn_value=42

    // 1.2 worker panic 不会杀掉进程，join 返回 Err
    println!(
        "panic_join_err={}",
        run_thread_silently(|| -> () { panic!("worker") }).is_err()
    );
    // 预测：panic_join_err=true

    // 1.3 并行度查询（只输出是否可用，避免依赖机器核数）
    let parallelism_ok = thread::available_parallelism()
        .map(|value| value.get() >= 1)
        .unwrap_or(false);
    println!("parallelism_ok={}", parallelism_ok);
    // 预测：parallelism_ok=true

    println!("===== 2. SYNTAX =====");

    // 2.1 Builder 可以给线程命名，便于日志/调试
    let handle = thread::Builder::new()
        .name("worker-1".into())
        .spawn(|| (thread::current().name().unwrap_or("?").to_string(), 3 + 4))
        .expect("spawn named thread");
    println!("named={:?}", handle.join());
    // 预测：named=Ok(("worker-1", 7))

    // 2.2 move 把 String 的所有权交给线程
    let text = String::from("hello");
    let handle = thread::spawn(move || text.len());
    println!("moved_len={}", handle.join().unwrap());
    // 预测：moved_len=5

    // 2.3 thread::scope 允许借用栈上的数据，无需 Arc / 'static
    let values = vec![1, 2, 3];
    let sum = thread::scope(|scope| {
        let first = scope.spawn(|| values[0] + values[1]);
        let second = scope.spawn(|| values[2]);
        first.join().unwrap() + second.join().unwrap()
    });
    println!("scope_sum={}", sum);
    // 预测：scope_sum=6

    println!("===== 3. APPLICATION =====");

    // 3.1 并行求和：把切片分给两个 scoped 线程
    let data = vec![1, 2, 3, 4, 5];
    let (left, right) = data.split_at(data.len() / 2);
    let total = thread::scope(|scope| {
        let left_sum = scope.spawn(|| left.iter().sum::<i32>());
        let right_sum = scope.spawn(|| right.iter().sum::<i32>());
        left_sum.join().unwrap() + right_sum.join().unwrap()
    });
    println!("parallel_sum={}", total);
    // 预测：parallel_sum=15

    // 3.2 分块词频：每个线程统计一部分行
    let lines = ["alpha beta", "gamma", "delta epsilon zeta"];
    let (head, tail) = lines.split_at(1);
    let words = thread::scope(|scope| {
        let first = scope.spawn(|| {
            head.iter()
                .map(|line| line.split_whitespace().count())
                .sum::<usize>()
        });
        let second = scope.spawn(|| {
            tail.iter()
                .map(|line| line.split_whitespace().count())
                .sum::<usize>()
        });
        first.join().unwrap() + second.join().unwrap()
    });
    println!("word_count={}", words);
    // 预测：word_count=6

    // 3.3 一个 worker panic，其他 worker 的结果仍然可用
    let good = thread::spawn(|| 1 + 2);
    let bad = run_thread_silently(|| -> () { panic!("boom") });
    println!("partial={:?} panic={}", good.join(), bad.is_err());
    // 预测：partial=Ok(3) panic=true
}
