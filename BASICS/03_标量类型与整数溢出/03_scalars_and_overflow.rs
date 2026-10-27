// 第 03 课示例：标量类型与整数溢出
//
// 结构：① 原理验证 —— 标量布局、补码表示、debug/release 的溢出语义差异
//       ② 语法演示 —— checked/wrapping/saturating/overflowing、as 转换、除零、NaN
//       ③ 应用实战 —— 输入校验、饱和计数器、wrapping 校验和、金额用整数分
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 03_scalars_and_overflow.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::hint::black_box;
use std::mem::size_of;

fn add_item(current: u8, delta: u8) -> Option<u8> {
    current.checked_add(delta)
}

// FNV-1a 风格的 32 位校验和：溢出回绕正是算法定义的一部分，必须用 wrapping_*
fn checksum(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 标量布局由规范固定；usize/isize 与指针同宽，不写死具体数值
    println!(
        "i8={} i32={} i64={} f64={} char={} bool={}",
        size_of::<i8>(),
        size_of::<i32>(),
        size_of::<i64>(),
        size_of::<f64>(),
        size_of::<char>(),
        size_of::<bool>()
    );
    // 预测：i8=1 i32=4 i64=8 f64=8 char=4 bool=1

    println!(
        "usize_is_pointer_sized={}",
        size_of::<usize>() == size_of::<*const ()>()
    );
    // 预测：usize_is_pointer_sized=true

    // 1.2 整数是补码：同一串比特，按有符号/无符号解释会得到不同的值
    let minus_one: i8 = -1;
    println!("(-1i8) as u8 = {}", minus_one as u8);
    // 预测：(-1i8) as u8 = 255

    println!("(255u8) as i8 = {}", 255u8 as i8);
    // 预测：(255u8) as i8 = -1

    // 1.3 原始 + 的溢出语义取决于编译选项：
    //     debug 编译（默认）开启 overflow-checks -> panic；
    //     release 编译（-O）关闭检查 -> 按补码回绕。
    //     这里用 black_box 阻止常量折叠，再用 catch_unwind 捕获并静音默认 panic 输出。
    let left = black_box(255u8);
    let right = black_box(1u8);
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let panicked = std::panic::catch_unwind(|| left + right).is_err();
    std::panic::set_hook(previous_hook);
    println!("debug_overflow_panicked={}", panicked);
    // 预测：debug_overflow_panicked=true（若改成 -O 编译则为 false，结果是 0）

    println!("===== 2. SYNTAX =====");

    // 2.1 四种显式的溢出策略：checked / wrapping / saturating / overflowing
    let max = u8::MAX;
    println!("checked={:?}", max.checked_add(1));
    // 预测：checked=None

    println!("wrapping={}", max.wrapping_add(1));
    // 预测：wrapping=0

    println!("saturating={}", max.saturating_add(1));
    // 预测：saturating=255

    println!("overflowing={:?}", 250u8.overflowing_add(10));
    // 预测：overflowing=(4, true)

    println!("checked_div={:?}", 7u8.checked_div(0));
    // 预测：checked_div=None

    println!("min_div={:?}", i32::MIN.checked_div(-1));
    // 预测：min_div=None

    // 2.2 as 转换：整数按位截断，浮点向零取整、超范围饱和
    println!("300u16 as u8 = {}", 300u16 as u8);
    // 预测：300u16 as u8 = 44

    println!("-1i32 as u8 = {}", -1i32 as u8);
    // 预测：-1i32 as u8 = 255

    println!("3.99f64 as i32 = {}", 3.99f64 as i32);
    // 预测：3.99f64 as i32 = 3

    println!("1e30f64 as i32 = {}", 1e30f64 as i32);
    // 预测：1e30f64 as i32 = 2147483647

    println!("NaN as i32 = {}", f64::NAN as i32);
    // 预测：NaN as i32 = 0

    // 2.3 整数除法向零取整，余数符号跟随被除数
    println!("-7 / 2 = {} , -7 % 2 = {}", -7i32 / 2, -7i32 % 2);
    // 预测：-7 / 2 = -3 , -7 % 2 = -1

    // 2.4 浮点是 IEEE 754：0.1+0.2 不精确，NaN 不等于自己，-0.0 有符号
    println!("0.1+0.2 == 0.3 ? {}", 0.1_f64 + 0.2_f64 == 0.3_f64);
    // 预测：0.1+0.2 == 0.3 ? false

    println!("NaN == NaN ? {}", black_box(f64::NAN) == black_box(f64::NAN));
    // 预测：NaN == NaN ? false

    println!("-0.0 sign_negative={}", (-0.0_f64).is_sign_negative());
    // 预测：-0.0 sign_negative=true

    println!("inf + 1 = {}", f64::INFINITY + 1.0);
    // 预测：inf + 1 = inf

    // 2.5 char 是 4 字节的 Unicode 标量值；bool 与整数之间没有隐式转换
    println!("'A' as u8 = {}", 'A' as u8);
    // 预测：'A' as u8 = 65

    println!("'中' as u32 = {}", '中' as u32);
    // 预测：'中' as u32 = 20013

    println!(
        "valid_char={} surrogate_rejected={}",
        char::from_u32(0x4E2D).is_some(),
        char::from_u32(0xD800).is_none()
    );
    // 预测：valid_char=true surrogate_rejected=true

    println!("true as u8 = {}", true as u8);
    // 预测：true as u8 = 1

    println!("===== 3. APPLICATION =====");

    // 3.1 解析不可信输入：溢出变成 None，由调用方决定怎么处理
    println!("add_item(250, 10)={:?}", add_item(250, 10));
    // 预测：add_item(250, 10)=None
    println!("add_item(3, 4)={:?}", add_item(3, 4));
    // 预测：add_item(3, 4)=Some(7)

    // 3.2 饱和计数器：到顶就停，永远不会回绕
    let mut score = 250u8;
    score = score.saturating_add(10);
    score = score.saturating_add(10);
    println!("score={}", score);
    // 预测：score=255

    // 3.3 wrapping 校验和：回绕是算法语义的一部分，必须显式表达
    println!("checksum={}", checksum(b"rust-daily"));
    // 预测：checksum=664988555

    // 3.4 金额用整数分：避免 0.1 + 0.2 这类浮点误差进入账本
    let price_cents = 1999i64;
    let quantity = 3i64;
    let total = price_cents.checked_mul(quantity).unwrap();
    println!(
        "total_cents={} total_yuan={}.{:02}",
        total,
        total / 100,
        total % 100
    );
    // 预测：total_cents=5997 total_yuan=59.97

    // 3.5 overflowing_*：一次拿到"回绕后的值 + 是否溢出"两个信息
    let (value, wrapped) = u8::MAX.overflowing_add(2);
    println!("value={} wrapped={}", value, wrapped);
    // 预测：value=1 wrapped=true
}
