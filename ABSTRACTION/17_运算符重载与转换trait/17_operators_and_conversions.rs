// 第 17 课示例：运算符重载与转换 trait
//
// 结构：① 原理验证 —— Add/AddAssign/Neg/Index/Deref 的展开与约束
//       ② 语法演示 —— From/Into、TryFrom、FromStr、AsRef、Ord 排序
//       ③ 应用实战 —— Vector2 运算符、Celsius/Fahrenheit 转换、Score 排序
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 17_operators_and_conversions.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::fmt;
use std::ops::{Add, AddAssign, Deref, Index, Mul, Neg};
use std::str::FromStr;

// 算术运算符：Add 的 Rhs 可以不同，Output 也可以不同
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

// Point + i32：Rhs 是具体类型，不是 Self
impl Add<i32> for Point {
    type Output = Point;

    fn add(self, value: i32) -> Point {
        Point {
            x: self.x + value,
            y: self.y + value,
        }
    }
}

impl Neg for Point {
    type Output = Point;

    fn neg(self) -> Point {
        Point {
            x: -self.x,
            y: -self.y,
        }
    }
}

// += 是独立的 trait，Add 不会自动带来 AddAssign
impl AddAssign for Point {
    fn add_assign(&mut self, other: Point) {
        self.x += other.x;
        self.y += other.y;
    }
}

// Index 必须返回引用；可以用 usize 作为索引类型
impl Index<usize> for Point {
    type Output = i32;

    fn index(&self, index: usize) -> &i32 {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("index out of range"),
        }
    }
}

// Deref：让 Wrapper 自动获得 str 的方法，并支持 &Wrapper -> &str
struct Wrapper(String);

impl Deref for Wrapper {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

// From：不可失败的转换；实现了 From，就自动得到 Into
struct Celsius(f64);
struct Fahrenheit(f64);

impl From<Celsius> for Fahrenheit {
    fn from(value: Celsius) -> Self {
        Self(value.0 * 9.0 / 5.0 + 32.0)
    }
}

// TryFrom：可能失败的转换，必须指定 Error
struct Even(i32);

impl TryFrom<i32> for Even {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value % 2 == 0 {
            Ok(Self(value))
        } else {
            Err("odd number")
        }
    }
}

// FromStr：parse() 背后的 trait
#[derive(Debug)]
struct Version {
    major: u32,
    minor: u32,
}

impl FromStr for Version {
    type Err = &'static str;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (major, minor) = text.split_once('.').ok_or("missing dot")?;
        Ok(Self {
            major: major.parse().map_err(|_| "bad major")?,
            minor: minor.parse().map_err(|_| "bad minor")?,
        })
    }
}

// AsRef：便宜地借用为 &str，泛型 API 常用
fn text_len<T: AsRef<str>>(value: T) -> usize {
    value.as_ref().len()
}

// Ord：全序，让 sort()/BTreeMap 可用
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    points: u32,
}

// 应用：二维向量，重载 + 与 *，提供 dot/norm
#[derive(Debug, Clone, Copy)]
struct Vector2 {
    x: f64,
    y: f64,
}

impl Add for Vector2 {
    type Output = Vector2;

    fn add(self, other: Vector2) -> Vector2 {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Mul<f64> for Vector2 {
    type Output = Vector2;

    fn mul(self, scalar: f64) -> Vector2 {
        Vector2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Vector2 {
    fn dot(self, other: Vector2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn norm(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.1}, {:.1})", self.x, self.y)
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 Point + Point 与 Point + i32 走不同的 Rhs impl
    let left = Point { x: 1, y: 2 };
    let right = Point { x: 3, y: 4 };
    println!("add_points={:?}", left + right);
    // 预测：add_points=Point { x: 4, y: 6 }

    println!("add_scalar={:?}", Point { x: 1, y: 2 } + 10);
    // 预测：add_scalar=Point { x: 11, y: 12 }

    // 1.2 Neg 与 AddAssign 是两个独立 trait
    println!("negated={:?}", -Point { x: 1, y: 2 });
    // 预测：negated=Point { x: -1, y: -2 }

    let mut assigned = Point { x: 5, y: 10 };
    assigned += Point { x: 5, y: 10 };
    println!("assigned={:?}", assigned);
    // 预测：assigned=Point { x: 10, y: 20 }

    // 1.3 Index 返回引用；Deref 让 Wrapper 获得 str 的方法
    println!("indexed={}", Point { x: 7, y: 8 }[0]);
    // 预测：indexed=7

    let wrapper = Wrapper(String::from("hello"));
    println!("deref_len={} starts_with_h={}", wrapper.len(), wrapper.starts_with('h'));
    // 预测：deref_len=5 starts_with_h=true

    println!("===== 2. SYNTAX =====");

    // 2.1 From + Into：实现 From 就自动得到 Into
    let fahrenheit: Fahrenheit = Celsius(100.0).into();
    println!("from={:.1}", fahrenheit.0);
    // 预测：from=212.0

    // 2.2 TryFrom：失败时返回 Err
    let even: Even = 4i32.try_into().unwrap();
    println!("try_ok={} try_err={}", even.0, Even::try_from(3).is_err());
    // 预测：try_ok=4 try_err=true

    // 2.3 FromStr：parse() 的底层 trait
    let version: Version = "1.2".parse().unwrap();
    println!("parsed={}.{}", version.major, version.minor);
    // 预测：parsed=1.2

    // 2.4 AsRef：&str 与 String 都能传
    println!("asref={} {}", text_len("abc"), text_len(String::from("hello")));
    // 预测：asref=3 5

    // 2.5 Ord：derive 后可以直接 sort
    let mut scores = vec![
        Score { points: 3 },
        Score { points: 10 },
        Score { points: 7 },
    ];
    scores.sort();
    println!(
        "sorted={:?}",
        scores.iter().map(|score| score.points).collect::<Vec<_>>()
    );
    // 预测：sorted=[3, 7, 10]

    println!("===== 3. APPLICATION =====");

    // 3.1 Vector2：+ 与 * 重载，配合 dot/norm
    let a = Vector2 { x: 1.0, y: 2.0 };
    let b = Vector2 { x: 3.0, y: 4.0 };
    let sum = a + b;
    let scaled = a * 2.0;
    println!("vector_sum={} dot={:.1}", sum, a.dot(b));
    // 预测：vector_sum=(4.0, 6.0) dot=11.0

    println!("scaled={} norm={:.2}", scaled, a.norm());
    // 预测：scaled=(2.0, 4.0) norm=2.24
}
