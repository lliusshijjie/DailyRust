// 第 04 课示例：复合类型——元组、数组、切片
//
// 结构：① 原理验证 —— 元组/数组是内联值、切片是胖指针、越界索引真的会 panic
//       ② 语法演示 —— 元组解构与字段、数组初始化与迭代、切片索引/切分/窗口、切片模式
//       ③ 应用实战 —— &[T] 参数、元组多返回值、二分/排序、二维数组
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 04_tuples_arrays_slices.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::hint::black_box;
use std::mem::size_of;

// 切片模式：按长度与首尾元素分类，编译器检查穷尽性
fn describe(slice: &[i32]) -> &'static str {
    match slice {
        [] => "empty",
        [_] => "single",
        [_, _] => "pair",
        [first, .., last] if first == last => "symmetric-many",
        _ => "many",
    }
}

fn min_max(values: &[i32]) -> Option<(i32, i32)> {
    let (&first, rest) = values.split_first()?;
    let mut min = first;
    let mut max = first;
    for &value in rest {
        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
    }
    Some((min, max))
}

// 用切片接收参数：数组、Vec、子切片都能直接传
fn stats(values: &[i32]) -> (i32, i32, i32) {
    let mut min = i32::MAX;
    let mut max = i32::MIN;
    let mut sum = 0;
    for &value in values {
        min = min.min(value);
        max = max.max(value);
        sum += value;
    }
    (min, max, sum)
}

fn average(values: &[i32]) -> f64 {
    let sum: i32 = values.iter().sum();
    sum as f64 / values.len() as f64
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 元组与数组都是"内联值"：元素直接排在自身内存里
    println!(
        "unit={} pair={} triple={} arr_u8_4={} arr_u32_4={} arr_string_0={}",
        size_of::<()>(),
        size_of::<(u8, u8)>(),
        size_of::<(u8, u8, u8)>(),
        size_of::<[u8; 4]>(),
        size_of::<[u32; 4]>(),
        size_of::<[String; 0]>()
    );
    // 预测：unit=0 pair=2 triple=3 arr_u8_4=4 arr_u32_4=16 arr_string_0=0

    println!("arr_1024_inline={}", size_of::<[u8; 1024]>() == 1024);
    // 预测：arr_1024_inline=true

    // 1.2 切片是胖指针：一个数据指针 + 一个长度，恰好是两个机器字
    println!(
        "fat_two_words={}",
        size_of::<&[u8]>() == 2 * size_of::<&[u8; 4]>()
    );
    // 预测：fat_two_words=true

    // 1.3 数组在元素是 Copy 时整体 Copy：赋值是复制，不是移动
    let original = [1, 2, 3];
    let copied = original;
    println!("copy_semantics={} {}", original[0], copied[0]);
    // 预测：copy_semantics=1 1

    // 1.4 索引越界是运行期 bounds check：debug/release 都会 panic
    //     用 catch_unwind 捕获并静音默认 panic 输出，证明这条检查真实存在
    let data = [10, 20, 30];
    let out_of_range = black_box(5usize);
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let panicked = std::panic::catch_unwind(|| data[out_of_range]).is_err();
    std::panic::set_hook(previous_hook);
    println!("oob_index_panicked={}", panicked);
    // 预测：oob_index_panicked=true

    println!("===== 2. SYNTAX =====");

    // 2.1 元组：解构、字段访问、交换
    let point = (3, 5);
    let (x, y) = point;
    println!("point={},{}", x, y);
    // 预测：point=3,5

    let (a, b) = (1, 2);
    let (a, b) = (b, a);
    println!("swapped={},{}", a, b);
    // 预测：swapped=2,1

    let (id, [r, g, b]) = (7, [10, 20, 30]);
    println!("id={} rgb={},{},{}", id, r, g, b);
    // 预测：id=7 rgb=10,20,30

    // 2.2 数组：定长、可重复初始化、按引用可变迭代
    let arr = [1, 2, 3, 4, 5];
    println!("arr={:?} len={}", arr, arr.len());
    // 预测：arr=[1, 2, 3, 4, 5] len=5

    let zeros = [0u8; 4];
    println!("zeros={:?}", zeros);
    // 预测：zeros=[0, 0, 0, 0]

    let mut doubled = arr;
    for value in &mut doubled {
        *value *= 2;
    }
    println!("doubled={:?}", doubled);
    // 预测：doubled=[2, 4, 6, 8, 10]

    // 2.3 切片：区间索引、split_at、chunks/windows 都是零拷贝视图
    let slice = &arr[1..4];
    println!("slice={:?} len={}", slice, slice.len());
    // 预测：slice=[2, 3, 4] len=3

    let (left, right) = arr.split_at(2);
    println!("left={:?} right={:?}", left, right);
    // 预测：left=[1, 2] right=[3, 4, 5]

    println!("chunks={} windows={}", arr.chunks(2).count(), arr.windows(2).count());
    // 预测：chunks=3 windows=4

    // 2.4 切片模式：按长度分支，首尾/剩余部分直接绑定成引用
    println!(
        "describe={} {} {} {}",
        describe(&[]),
        describe(&[1]),
        describe(&[1, 2]),
        describe(&[1, 2, 1])
    );
    // 预测：describe=empty single pair symmetric-many

    println!(
        "first={} last={}",
        arr.first().copied().unwrap_or(0),
        arr.last().copied().unwrap_or(0)
    );
    // 预测：first=1 last=5

    let rest = &arr[1..];
    if let [head, tail @ ..] = rest {
        println!("head={} tail={:?}", head, tail);
        // 预测：head=2 tail=[3, 4, 5]
    }

    // 2.5 非 Copy 元素的数组：用 from_fn 逐个构造；定长转换用 try_into
    let squares: [i32; 5] = std::array::from_fn(|i| (i as i32) * (i as i32));
    println!("squares={:?}", squares);
    // 预测：squares=[0, 1, 4, 9, 16]

    let fixed: [i32; 3] = vec![7, 8, 9].as_slice().try_into().unwrap();
    println!("fixed={:?}", fixed);
    // 预测：fixed=[7, 8, 9]

    // 2.6 元组做多返回值：Option<(min, max)>
    println!("min_max={:?}", min_max(&[4, 1, 9, 3, 8]));
    // 预测：min_max=Some((1, 9))

    println!("===== 3. APPLICATION =====");

    // 3.1 同一个 &[i32] 函数，数组、Vec、子切片都能直接传
    println!("stats={:?}", stats(&[4, 1, 9, 3, 8]));
    // 预测：stats=(1, 9, 25)

    let vec_data = vec![1, 2, 3, 4];
    println!("average={}", average(&vec_data));
    // 预测：average=2.5

    // 3.2 切片自带排序与二分查找
    let sorted = [1, 3, 5, 7, 9];
    println!("binary_search={:?}", sorted.binary_search(&7));
    // 预测：binary_search=Ok(3)

    let mut data = [5, 3, 1, 4, 2];
    data.sort();
    println!("sorted={:?}", data);
    // 预测：sorted=[1, 2, 3, 4, 5]

    // 3.3 windows：滑动窗口统计，相邻元素求和
    let window_sums: Vec<i32> = [1, 2, 3, 4].windows(2).map(|w| w[0] + w[1]).collect();
    println!("window_sums={:?}", window_sums);
    // 预测：window_sums=[3, 5, 7]

    // 3.4 二维数组就是"数组的数组"：嵌套迭代展平
    let mut grid = [[0u8; 3]; 3];
    let mut next = 1u8;
    for row in &mut grid {
        for cell in row {
            *cell = next;
            next += 1;
        }
    }
    let total: u32 = grid.iter().flatten().map(|&v| u32::from(v)).sum();
    println!("grid_total={}", total);
    // 预测：grid_total=45
}
