// 第 33 课示例：共享状态——Mutex、RwLock 与 Arc
//
// 结构：① 原理验证 —— Arc 引用计数、Arc<Mutex> 并发计数、锁中毒恢复、try_lock
//       ② 语法演示 —— Arc<Mutex<Vec>> 共享日志、RwLock 并发读、Arc::make_mut 写时复制
//       ③ 应用实战 —— 共享缓存聚合、RwLock 配置读写
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 33_mutex_rwlock_arc.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

// 静音 hook 后运行会 panic 的线程（用于制造锁中毒）
fn run_thread_silently<F, T>(function: F) -> thread::Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let handle = thread::spawn(function);
    let result = handle.join();
    std::panic::set_hook(previous);
    result
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 Arc：原子引用计数，clone 只加计数
    let base = Arc::new(1);
    let clone = Arc::clone(&base);
    let strong = Arc::strong_count(&base);
    let same = Arc::ptr_eq(&base, &clone);
    drop(clone);
    println!(
        "arc_strong={} after={} ptr_eq={}",
        strong,
        Arc::strong_count(&base),
        same
    );
    // 预测：arc_strong=2 after=1 ptr_eq=true

    // 1.2 Arc<Mutex<T>>：多线程共享可变状态
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let mut guard = counter.lock().unwrap();
                *guard += 1;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("counter={}", *counter.lock().unwrap());
    // 预测：counter=2000

    // 1.3 锁中毒：持锁线程 panic 后，后续 lock 返回 Err，但数据仍可取回
    let poisoned = Arc::new(Mutex::new(6));
    let clone = Arc::clone(&poisoned);
    let _ = run_thread_silently(move || -> () {
        let mut guard = clone.lock().unwrap();
        *guard += 1;
        panic!("poison the mutex");
    });

    let lock_result = poisoned.lock();
    let is_poisoned = lock_result.is_err();
    let recovered = match lock_result {
        Ok(guard) => *guard,
        Err(error) => *error.into_inner(),
    };
    println!("poisoned={} recovered={}", is_poisoned, recovered);
    // 预测：poisoned=true recovered=7

    // 1.4 try_lock：已经持锁时立即失败，不会阻塞
    let mutex = Mutex::new(1);
    let guard = mutex.lock().unwrap();
    let blocked = mutex.try_lock().is_err();
    println!("try_lock_blocked={}", blocked);
    // 预测：try_lock_blocked=true
    drop(guard);

    println!("===== 2. SYNTAX =====");

    // 2.1 Arc<Mutex<Vec>>：多线程写入同一个日志，排序后再打印
    let log = Arc::new(Mutex::new(Vec::new()));
    let log1 = Arc::clone(&log);
    let log2 = Arc::clone(&log);
    let first = thread::spawn(move || {
        let mut guard = log1.lock().unwrap();
        guard.push(String::from("beta"));
        guard.push(String::from("gamma"));
    });
    let second = thread::spawn(move || {
        log2.lock().unwrap().push(String::from("alpha"));
    });
    first.join().unwrap();
    second.join().unwrap();
    let mut entries = log.lock().unwrap().clone();
    entries.sort();
    println!("shared_log={:?}", entries);
    // 预测：shared_log=["alpha", "beta", "gamma"]

    // 2.2 RwLock：多个读者可以同时持有读锁
    let values = RwLock::new(vec![1, 2, 3]);
    let sums = thread::scope(|scope| {
        let first = scope.spawn(|| values.read().unwrap().iter().sum::<i32>());
        let second = scope.spawn(|| values.read().unwrap().iter().sum::<i32>());
        vec![first.join().unwrap(), second.join().unwrap()]
    });
    println!("rwlock_sums={:?}", sums);
    // 预测：rwlock_sums=[6, 6]

    // 2.3 Arc::make_mut：有共享引用时先克隆，再修改（写时复制）
    let mut first = Arc::new(vec![1]);
    let second = Arc::clone(&first);
    Arc::make_mut(&mut first).push(2);
    println!("cow_a={:?} cow_b={:?}", first, second);
    // 预测：cow_a=[1, 2] cow_b=[1]

    println!("===== 3. APPLICATION =====");

    // 3.1 共享缓存：两个 worker 更新同一个 BTreeMap，最后聚合输出
    let cache = Arc::new(Mutex::new(BTreeMap::new()));
    let cache1 = Arc::clone(&cache);
    let cache2 = Arc::clone(&cache);
    let first = thread::spawn(move || {
        let mut guard = cache1.lock().unwrap();
        *guard.entry("a").or_insert(0) += 1;
        *guard.entry("b").or_insert(0) += 1;
    });
    let second = thread::spawn(move || {
        *cache2.lock().unwrap().entry("a").or_insert(0) += 1;
    });
    first.join().unwrap();
    second.join().unwrap();
    println!("cache={:?}", cache.lock().unwrap());
    // 预测：cache={"a": 2, "b": 1}

    // 3.2 RwLock 配置：读多写少；写锁独占，读锁共享
    let config = RwLock::new(8080u16);
    thread::scope(|scope| {
        scope.spawn(|| {
            *config.write().unwrap() = 9090;
        });
    });
    println!("config={}", *config.read().unwrap());
    // 预测：config=9090
}
