// 第 34 课示例：Send 与 Sync
//
// 结构：① 原理验证 —— 编译期断言、结构体自动推导、PhantomData 选择退出
//       ② 语法演示 —— Arc/Mutex/Cache 的 Send+Sync、闭包 Send、API 约束
//       ③ 应用实战 —— 泛型线程安全缓存
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 34_send_and_sync.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！
//
// 说明：Send/Sync 是编译期自动 trait，示例用 assert_send/assert_sync 做"编译期测试"；
// 反向案例（Rc 不 Send、Cell 不 Sync、MutexGuard 不 Send 等）在课件与探针中展示。

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::mem::size_of;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

// 结构体/枚举的 Send/Sync 由所有字段共同决定
struct Composite {
    value: i32,
    shared: Arc<Mutex<i32>>,
}

// PhantomData<Rc<()>> 让类型"看起来"含有 Rc，从而不实现 Send/Sync
struct LocalOnly {
    value: i32,
    _marker: PhantomData<Rc<()>>,
}

// 泛型缓存：Mutex<T> 让它可跨线程共享（T: Send 时）
struct Cache<T> {
    map: Mutex<BTreeMap<String, T>>,
}

impl<T> Cache<T> {
    fn new() -> Self {
        Cache {
            map: Mutex::new(BTreeMap::new()),
        }
    }

    fn put(&self, key: &str, value: T) {
        self.map.lock().unwrap().insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> Option<T>
    where
        T: Copy,
    {
        self.map.lock().unwrap().get(key).copied()
    }
}

// API 边界：要求闭包 Send + 'static
fn run_worker<F>(worker: F) -> &'static str
where
    F: FnOnce() -> &'static str + Send + 'static,
{
    worker()
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 基础类型同时 Send + Sync
    assert_send::<i32>();
    assert_sync::<i32>();
    println!("primitives=true");
    // 预测：primitives=true

    // 1.2 Arc<Mutex<T>>：Send + Sync（T: Send）
    assert_send::<Arc<Mutex<i32>>>();
    assert_sync::<Arc<Mutex<i32>>>();
    println!("shared=true");
    // 预测：shared=true

    // 1.3 组合类型：所有字段都满足才自动实现
    let shared = Arc::new(Mutex::new(0));
    let composite = Composite {
        value: 1,
        shared: Arc::clone(&shared),
    };
    assert_send::<Composite>();
    assert_sync::<Composite>();
    println!(
        "composite={}",
        composite.value + *composite.shared.lock().unwrap()
    );
    // 预测：composite=1

    // 1.4 PhantomData 零大小，但会影响自动 trait 推导
    let local = LocalOnly {
        value: 7,
        _marker: PhantomData,
    };
    println!("phantom_size={} value={}", size_of::<LocalOnly>(), local.value);
    // 预测：phantom_size=4 value=7

    println!("===== 2. SYNTAX =====");

    // 2.1 Arc<Mutex<Vec<String>>> 也满足 Send + Sync
    assert_send::<Arc<Mutex<Vec<String>>>>();
    assert_sync::<Arc<Mutex<Vec<String>>>>();
    println!("arc_send_sync=true");
    // 预测：arc_send_sync=true

    // 2.2 Mutex<T> 本身：T: Send 时 Send + Sync
    assert_send::<Mutex<u64>>();
    assert_sync::<Mutex<u64>>();
    println!("mutex_send_sync=true");
    // 预测：mutex_send_sync=true

    // 2.3 泛型 Cache<i32>：自动推导出 Send + Sync
    assert_send::<Cache<i32>>();
    assert_sync::<Cache<i32>>();
    println!("cache_send_sync=true");
    // 预测：cache_send_sync=true

    // 2.4 API 的 Send 约束：闭包捕获的类型必须 Send
    println!("bound={}", run_worker(|| "done"));
    // 预测：bound=done

    println!("===== 3. APPLICATION =====");

    // 3.1 泛型线程安全缓存：把 Arc<Cache<i32>> 移到 worker
    let cache = Arc::new(Cache::new());
    let worker_cache = Arc::clone(&cache);
    let worker = thread::spawn(move || {
        worker_cache.put("a", 1);
        worker_cache.put("b", 2);
    });
    worker.join().unwrap();

    let mut snapshot = BTreeMap::new();
    for key in ["a", "b"] {
        if let Some(value) = cache.get(key) {
            snapshot.insert(key, value);
        }
    }
    println!("cache={:?}", snapshot);
    // 预测：cache={"a": 1, "b": 2}
}
