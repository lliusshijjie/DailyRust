// 第 35 课示例：原子类型与内存序
//
// 结构：① 原理验证 —— AtomicUsize 计数、AtomicBool 标志、compare_exchange、fetch_update
//       ② 语法演示 —— fetch_max/fetch_min、Release store / Acquire load
//       ③ 应用实战 —— 多维指标（Stats）与自旋锁（SpinLock）
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 35_atomics_and_ordering.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

struct Stats {
    requests: AtomicUsize,
    errors: AtomicUsize,
}

// 用 Acquire/Release 实现的最小自旋锁（仅用于演示内存序）
struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    fn new(data: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn lock(&self) -> SpinGuard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        SpinGuard { lock: self }
    }
}

struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: 持有 guard 期间独占数据
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: 持有 guard 期间独占数据
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 AtomicUsize：多线程计数，Relaxed 只保证原子性
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("counter={}", counter.load(Ordering::SeqCst));
    // 预测：counter=2000

    // 1.2 AtomicBool：Release store + Acquire load + swap
    let flag = AtomicBool::new(false);
    flag.store(true, Ordering::Release);
    let flag_value = flag.load(Ordering::Acquire);
    let swapped = flag.swap(false, Ordering::AcqRel);
    println!("flag={} swapped={}", flag_value, swapped);
    // 预测：flag=true swapped=true

    // 1.3 compare_exchange：成功返回 Ok(旧值)，失败返回 Err(当前值)
    let value = AtomicUsize::new(0);
    let cas_ok = value.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
    let cas_fail = value.compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
    println!("cas_ok={:?} cas_fail={:?}", cas_ok, cas_fail);
    // 预测：cas_ok=Ok(0) cas_fail=Err(1)

    // 1.4 fetch_update：读-改-写原子化，闭包返回 None 表示放弃
    let value = AtomicUsize::new(5);
    let previous = value.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        Some(current + 1)
    });
    println!(
        "fetch_update={:?} now={}",
        previous,
        value.load(Ordering::SeqCst)
    );
    // 预测：fetch_update=Ok(5) now=6

    println!("===== 2. SYNTAX =====");

    // 2.1 fetch_max / fetch_min
    let maximum = AtomicUsize::new(3);
    let old_max = maximum.fetch_max(7, Ordering::SeqCst);
    println!(
        "fetch_max={} now={}",
        old_max,
        maximum.load(Ordering::SeqCst)
    );
    // 预测：fetch_max=3 now=7

    let minimum = AtomicUsize::new(9);
    let old_min = minimum.fetch_min(2, Ordering::SeqCst);
    println!(
        "fetch_min={} now={}",
        old_min,
        minimum.load(Ordering::SeqCst)
    );
    // 预测：fetch_min=9 now=2

    // 2.2 Release/Acquire：写之前的所有操作对读到该值的线程可见
    let data = AtomicUsize::new(0);
    data.store(42, Ordering::Release);
    println!(
        "release_store={} acquire_load={}",
        42,
        data.load(Ordering::Acquire)
    );
    // 预测：release_store=42 acquire_load=42

    println!("===== 3. APPLICATION =====");

    // 3.1 多个独立指标：每个计数器各自原子，但不保证彼此一致
    let stats = Arc::new(Stats {
        requests: AtomicUsize::new(0),
        errors: AtomicUsize::new(0),
    });
    let mut handles = Vec::new();
    for _ in 0..4 {
        let stats = Arc::clone(&stats);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                stats.requests.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    let errors = Arc::clone(&stats);
    handles.push(thread::spawn(move || {
        errors.errors.fetch_add(1000, Ordering::Relaxed);
    }));
    for handle in handles {
        handle.join().unwrap();
    }
    println!(
        "stats=requests {} errors {}",
        stats.requests.load(Ordering::SeqCst),
        stats.errors.load(Ordering::SeqCst)
    );
    // 预测：stats=requests 4000 errors 1000

    // 3.2 自旋锁：Acquire 获取、Release 释放，保护 UnsafeCell 中的数据
    let lock = Arc::new(SpinLock::new(0usize));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let lock = Arc::clone(&lock);
        handles.push(thread::spawn(move || {
            for _ in 0..500 {
                let mut guard = lock.lock();
                *guard += 1;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("spinlock={}", *lock.lock());
    // 预测：spinlock=2000
}
