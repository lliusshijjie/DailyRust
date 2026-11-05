// 第 15 课示例：关联类型、const 泛型与 GAT
//
// 结构：① 原理验证 —— 关联类型 C::Item、泛型参数的多次实现、const 泛型大小
//       ② 语法演示 —— 关联类型约束、const 泛型 Buffer、GAT 借用字符串、GAT 迭代器
//       ③ 应用实战 —— const 泛型矩阵、GAT 行视图
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 15_associated_types_const_generics.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::any::type_name;
use std::mem::size_of;

// 关联类型：由实现者决定 Item 是什么
trait Collection {
    type Item;

    fn first(&self) -> Option<&Self::Item>;
    fn len(&self) -> usize;
}

impl<T> Collection for Vec<T> {
    type Item = T;

    fn first(&self) -> Option<&Self::Item> {
        self.as_slice().first()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}

// 关联类型出现在泛型约束里
fn first_item<C: Collection>(collection: &C) -> Option<&C::Item> {
    collection.first()
}

// 对比：泛型参数允许同一类型实现多次（不同 T）
trait Convert<T> {
    fn convert(&self, value: T) -> T;
}

struct Meters;

impl Convert<f64> for Meters {
    fn convert(&self, value: f64) -> f64 {
        value * 3.280_84
    }
}

impl Convert<u32> for Meters {
    fn convert(&self, value: u32) -> u32 {
        value * 3
    }
}

// const 泛型：N 进入类型，数组长度由 N 决定
struct Buffer<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self { data: [0; N] }
    }

    fn set(&mut self, index: usize, value: u8) {
        self.data[index] = value;
    }

    fn sum(&self) -> u32 {
        self.data.iter().map(|value| u32::from(*value)).sum()
    }

    fn capacity(&self) -> usize {
        N
    }
}

// const 泛型函数：一次编写，按 N 单态化
fn sum_array<const N: usize>(values: [i32; N]) -> i32 {
    values.iter().sum()
}

// GAT：借用型关联类型，生命周期由方法调用决定
trait Lender {
    type Item<'a>
    where
        Self: 'a;

    fn lend(&self) -> Self::Item<'_>;
}

struct Words {
    data: Vec<String>,
}

impl Lender for Words {
    type Item<'a> = &'a str
    where
        Self: 'a;

    fn lend(&self) -> Self::Item<'_> {
        self.data[0].as_str()
    }
}

// GAT 带 trait 约束：迭代器元素借用 self
struct Numbers(Vec<i32>);

trait Iterable {
    type Iter<'a>: Iterator<Item = &'a i32>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_>;
}

impl Iterable for Numbers {
    type Iter<'a> = std::slice::Iter<'a, i32>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}

// const 泛型矩阵：行列数进入类型
struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn zeros() -> Self {
        Self { data: [[0.0; C]; R] }
    }

    fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row][col] = value;
    }

    fn rows(&self) -> usize {
        R
    }

    fn cols(&self) -> usize {
        C
    }

    fn sum(&self) -> f64 {
        self.data.iter().flatten().sum()
    }
}

// GAT 应用：零拷贝行视图
struct Grid {
    rows: Vec<Vec<i32>>,
}

trait Table {
    type Row<'a>: Iterator<Item = &'a i32>
    where
        Self: 'a;

    fn row(&self, index: usize) -> Self::Row<'_>;
}

impl Table for Grid {
    type Row<'a> = std::slice::Iter<'a, i32>
    where
        Self: 'a;

    fn row(&self, index: usize) -> Self::Row<'_> {
        self.rows[index].iter()
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 关联类型：<T as Trait>::Item 可以参与类型名/约束
    println!("item_type={}", type_name::<<Vec<i32> as Collection>::Item>());
    // 预测：item_type=i32

    let values = vec![10, 20, 30];
    println!(
        "first={:?} len={}",
        first_item(&values),
        Collection::len(&values)
    );
    // 预测：first=Some(10) len=3

    // 1.2 泛型参数允许同一类型实现多次：Convert<f64> 与 Convert<u32>
    println!(
        "convert_f64={:.2} convert_u32={}",
        <Meters as Convert<f64>>::convert(&Meters, 1.0),
        <Meters as Convert<u32>>::convert(&Meters, 2)
    );
    // 预测：convert_f64=3.28 convert_u32=6

    // 1.3 const 泛型：不同 N 是不同类型，大小随之变化
    println!(
        "buffer_sizes={} {}",
        size_of::<Buffer<4>>(),
        size_of::<Buffer<16>>()
    );
    // 预测：buffer_sizes=4 16

    // 1.4 const 泛型函数：同一个函数服务不同长度的数组
    println!("array_sum={}", sum_array([1, 2, 3, 4, 5]));
    // 预测：array_sum=15

    println!("===== 2. SYNTAX =====");

    // 2.1 关联类型作为泛型约束
    let numbers = vec![1, 2, 3];
    println!("assoc_first={:?}", first_item(&numbers));
    // 预测：assoc_first=Some(1)

    // 2.2 const 泛型结构体：N 可用于数组长度，也可当容量常量
    let mut buffer = Buffer::<4>::new();
    buffer.set(0, 7);
    buffer.set(3, 5);
    println!(
        "buffer_capacity={} buffer_sum={}",
        buffer.capacity(),
        buffer.sum()
    );
    // 预测：buffer_capacity=4 buffer_sum=12

    // 2.3 GAT：关联类型带生命周期参数，返回借用 self 的 &str
    let words = Words {
        data: vec![String::from("hello")],
    };
    println!("gat_word={}", words.lend());
    // 预测：gat_word=hello

    // 2.4 GAT + trait 约束：返回借用 self 的迭代器
    let source = Numbers(vec![1, 2, 3]);
    println!("gat_iter={:?}", source.iter().collect::<Vec<_>>());
    // 预测：gat_iter=[1, 2, 3]

    println!("===== 3. APPLICATION =====");

    // 3.1 const 泛型矩阵：行列数编译期确定
    let mut matrix = Matrix::<2, 3>::zeros();
    matrix.set(0, 0, 1.5);
    matrix.set(1, 2, 5.0);
    println!(
        "matrix={}x{} sum={:.2}",
        matrix.rows(),
        matrix.cols(),
        matrix.sum()
    );
    // 预测：matrix=2x3 sum=6.50

    // 3.2 GAT 行视图：每行返回借用的迭代器，不复制数据
    let grid = Grid {
        rows: vec![vec![1, 2], vec![4, 8], vec![10, 20]],
    };
    let row_sums: Vec<i32> = (0..3).map(|index| grid.row(index).sum()).collect();
    println!("row_sums={:?}", row_sums);
    // 预测：row_sums=[3, 12, 30]
}
