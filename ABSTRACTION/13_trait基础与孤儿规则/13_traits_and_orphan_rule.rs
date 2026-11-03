// 第 13 课示例：trait 基础与孤儿规则
//
// 结构：① 原理验证 —— 默认方法、覆盖、supertrait、blanket impl、关联常量、trait 对象
//       ② 语法演示 —— 泛型约束、impl Trait 参数、dyn、UFCS、本地 trait 扩展外部类型
//       ③ 应用实战 —— 异构 Summary 摘要、扩展 trait、Money 的 Display
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 13_traits_and_orphan_rule.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::fmt;

// trait 是能力契约：必须实现 name/area，describe 有默认实现
trait Shape {
    fn name(&self) -> &'static str;
    fn area(&self) -> f64;

    // 默认方法可以调用必须方法
    fn describe(&self) -> String {
        format!("{} with area {:.2}", self.name(), self.area())
    }
}

struct Circle {
    radius: f64,
}

struct Rect {
    width: f64,
    height: f64,
}

struct Square {
    side: f64,
}

impl Shape for Circle {
    fn name(&self) -> &'static str {
        "circle"
    }

    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

impl Shape for Rect {
    fn name(&self) -> &'static str {
        "rect"
    }

    fn area(&self) -> f64 {
        self.width * self.height
    }
}

impl Shape for Square {
    fn name(&self) -> &'static str {
        "square"
    }

    fn area(&self) -> f64 {
        self.side * self.side
    }

    // 覆盖默认方法
    fn describe(&self) -> String {
        format!("square: {:.2}", self.area())
    }
}

// 关联常量放在单独 trait：带关联常量的 trait 不能做 trait 对象
trait Sides {
    const COUNT: u32;
}

impl Sides for Circle {
    const COUNT: u32 = 0;
}

impl Sides for Rect {
    const COUNT: u32 = 4;
}

// 本地泛型类型实现本地 trait：允许
struct Wrapper<T> {
    inner: T,
}

impl<T: Shape> Shape for Wrapper<T> {
    fn name(&self) -> &'static str {
        "wrapper"
    }

    fn area(&self) -> f64 {
        self.inner.area()
    }
}

// supertrait：实现 Named 必须先实现 Display
struct Tag(&'static str);

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

trait Named: fmt::Display {
    fn name(&self) -> &'static str;

    fn named(&self) -> String {
        format!("{} ({})", self.name(), self)
    }
}

impl Named for Tag {
    fn name(&self) -> &'static str {
        "tag"
    }
}

// blanket impl：本地 trait 可以为"所有实现了 Display 的类型"统一实现
trait Printable {
    fn printed(&self) -> String;
}

impl<T: fmt::Display> Printable for T {
    fn printed(&self) -> String {
        self.to_string()
    }
}

// 本地类型实现外部 trait（Display）：孤儿规则允许
struct Money(i64);

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}

// 本地 trait 扩展外部类型（str）：孤儿规则允许
trait StrExt {
    fn shout(&self) -> String;
}

impl StrExt for str {
    fn shout(&self) -> String {
        self.to_uppercase()
    }
}

// 泛型函数：T 必须实现 Shape
fn total_area<T: Shape>(items: &[T]) -> f64 {
    items.iter().map(|item| item.area()).sum()
}

// impl Trait 参数：等价于匿名泛型
fn describe_one(shape: &impl Shape) -> String {
    shape.describe()
}

// 应用：异构集合只能通过 trait 对象装进同一个容器
trait Summary {
    fn summarize(&self) -> String;
}

struct Article {
    title: String,
}

struct Tweet {
    user: String,
    text: String,
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("[article] {}", self.title)
    }
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("[tweet] {}: {}", self.user, self.text)
    }
}

fn summarize_all(items: &[Box<dyn Summary>]) -> Vec<String> {
    items.iter().map(|item| item.summarize()).collect()
}

// 扩展 trait：给外部类型 Vec<i32> 加本地方法
trait Second {
    fn second(&self) -> Option<i32>;
}

impl Second for Vec<i32> {
    fn second(&self) -> Option<i32> {
        self.get(1).copied()
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 默认方法：只实现必须方法就能拿到 describe
    let circle = Circle { radius: 1.0 };
    println!("default={}", circle.describe());
    // 预测：default=circle with area 3.14

    // 1.2 覆盖默认方法
    let square = Square { side: 2.0 };
    println!("override={}", square.describe());
    // 预测：override=square: 4.00

    // 1.3 supertrait：Named 的默认方法能使用 Display 的能力
    println!("supertrait={}", Tag("gold").named());
    // 预测：supertrait=tag (gold)

    // 1.4 blanket impl：i32 因为实现 Display 而自动获得 printed
    let answer = 42i32;
    println!("blanket={}", answer.printed());
    // 预测：blanket=42

    // 1.5 关联常量：用 <Type as Trait>::CONST 访问
    println!(
        "sides={} {}",
        <Circle as Sides>::COUNT,
        <Rect as Sides>::COUNT
    );
    // 预测：sides=0 4

    // 1.6 trait 对象：异构集合，vtable 动态分发
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Rect {
            width: 2.0,
            height: 3.0,
        }),
    ];
    let object_total: f64 = shapes.iter().map(|shape| shape.area()).sum();
    println!("object_total={:.2}", object_total);
    // 预测：object_total=9.14

    println!("===== 2. SYNTAX =====");

    // 2.1 泛型约束：T: Shape
    let circles = [Circle { radius: 1.0 }, Circle { radius: 2.0 }];
    println!("total_area={:.2}", total_area(&circles));
    // 预测：total_area=15.71

    // 2.2 impl Trait 参数
    println!("apit={}", describe_one(&circle));
    // 预测：apit=circle with area 3.14

    // 2.3 &dyn Trait：借用的 trait 对象
    let rect = Rect {
        width: 2.0,
        height: 3.0,
    };
    let dyn_shape: &dyn Shape = &rect;
    println!("dyn={}", dyn_shape.describe());
    // 预测：dyn=rect with area 6.00

    // 2.4 泛型类型实现 trait 后，自身也可以当 Shape 用
    let wrapper = Wrapper {
        inner: Circle { radius: 2.0 },
    };
    println!(
        "wrapper={} area={:.2}",
        Shape::name(&wrapper),
        Shape::area(&wrapper)
    );
    // 预测：wrapper=wrapper area=12.57

    // 2.5 UFCS：显式指定 trait 方法
    println!("ufcs={:.2}", Shape::area(&circle));
    // 预测：ufcs=3.14

    // 2.6 本地类型实现外部 trait（Display）
    println!("display={}", Money(1999));
    // 预测：display=19.99

    // 2.7 本地 trait 扩展外部类型（str）
    println!("ext={}", "hi".shout());
    // 预测：ext=HI

    println!("===== 3. APPLICATION =====");

    // 3.1 异构摘要：Article 和 Tweet 装进同一个 Vec
    let items: Vec<Box<dyn Summary>> = vec![
        Box::new(Article {
            title: String::from("Rust traits"),
        }),
        Box::new(Tweet {
            user: String::from("alice"),
            text: String::from("hello"),
        }),
    ];
    println!("digest={}", summarize_all(&items).join(" | "));
    // 预测：digest=[article] Rust traits | [tweet] alice: hello

    // 3.2 扩展 trait：给 Vec<i32> 加 second 方法
    let numbers = vec![1, 2, 3];
    println!("second={:?}", numbers.second());
    // 预测：second=Some(2)
}
