// 第 32 课示例：消息传递——mpsc 通道
//
// 结构：① 原理验证 —— send/recv、发送者全部 drop 后关闭、try_recv 空、接收端 drop 后 send 失败
//       ② 语法演示 —— 克隆 Sender 多生产者、try_iter 非阻塞、sync_channel 有界背压
//       ③ 应用实战 —— 工作线程平方流水线、事件计数
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 32_mpsc_channels.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread;

enum Event {
    Click,
    Move,
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 send 转移所有权，recv 取回值
    let (sender, receiver) = mpsc::channel();
    sender.send(42).unwrap();
    println!("recv={}", receiver.recv().unwrap());
    // 预测：recv=42

    // 1.2 所有 Sender 都 drop 后，recv 返回 Err（通道关闭）
    let (sender, receiver) = mpsc::channel::<i32>();
    drop(sender);
    println!("closed={}", receiver.recv().is_err());
    // 预测：closed=true

    // 1.3 try_recv 非阻塞：通道为空时返回 Empty
    let (sender, receiver) = mpsc::channel::<i32>();
    let empty = matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    println!("empty={}", empty);
    // 预测：empty=true
    drop(sender);

    // 1.4 接收端 drop 后，send 返回 Err 并带回原值
    let (sender, receiver) = mpsc::channel::<i32>();
    drop(receiver);
    let error = sender.send(1).unwrap_err();
    println!("send_err={}", error.0);
    // 预测：send_err=1

    println!("===== 2. SYNTAX =====");

    // 2.1 克隆 Sender：多个生产者，单个消费者；排序后再打印保证确定性
    let (sender, receiver) = mpsc::channel();
    let sender2 = sender.clone();
    let producer1 = thread::spawn(move || {
        sender.send((1, 10)).unwrap();
        sender.send((1, 20)).unwrap();
    });
    let producer2 = thread::spawn(move || {
        sender2.send((2, 30)).unwrap();
    });
    let mut messages: Vec<(i32, i32)> = receiver.iter().collect();
    messages.sort_unstable();
    producer1.join().unwrap();
    producer2.join().unwrap();
    println!("multi={:?}", messages);
    // 预测：multi=[(1, 10), (1, 20), (2, 30)]

    // 2.2 try_iter：非阻塞地把当前可用的消息全部取出
    let (sender, receiver) = mpsc::channel();
    for value in 1..=3 {
        sender.send(value).unwrap();
    }
    let values: Vec<i32> = receiver.try_iter().collect();
    println!("try_iter={:?}", values);
    // 预测：try_iter=[1, 2, 3]

    // 2.3 sync_channel(1)：有界通道，满了 try_send 返回 Full
    let (sender, _receiver) = mpsc::sync_channel(1);
    sender.send(1).unwrap();
    let full = matches!(sender.try_send(2), Err(mpsc::TrySendError::Full(_)));
    println!("bounded_full={}", full);
    // 预测：bounded_full=true

    println!("===== 3. APPLICATION =====");

    // 3.1 工作线程：job 通道 -> 平方 -> result 通道
    let (job_sender, job_receiver) = mpsc::channel::<u32>();
    let (result_sender, result_receiver) = mpsc::channel::<u32>();
    let worker = thread::spawn(move || {
        for job in job_receiver {
            result_sender.send(job * job).unwrap();
        }
    });

    for job in 1..=4 {
        job_sender.send(job).unwrap();
    }
    drop(job_sender);

    let mut squares: Vec<u32> = result_receiver.iter().collect();
    worker.join().unwrap();
    squares.sort_unstable();
    println!("squares={:?}", squares);
    // 预测：squares=[1, 4, 9, 16]
    println!("sum={}", squares.iter().sum::<u32>());
    // 预测：sum=30

    // 3.2 事件计数：多个生产者，BTreeMap 保证输出顺序
    let (sender, receiver) = mpsc::channel();
    let sender2 = sender.clone();
    let producer1 = thread::spawn(move || {
        sender.send(Event::Click).unwrap();
        sender.send(Event::Move).unwrap();
    });
    let producer2 = thread::spawn(move || {
        sender2.send(Event::Click).unwrap();
    });

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for event in receiver {
        let key = match event {
            Event::Click => "click",
            Event::Move => "move",
        };
        *counts.entry(key).or_insert(0) += 1;
    }
    producer1.join().unwrap();
    producer2.join().unwrap();
    println!("events={:?}", counts);
    // 预测：events={"click": 2, "move": 1}
}
