// 第 16 课示例：trait 对象与动态分发
//
// 结构：① 原理验证 —— 胖指针/Box 大小、vtable 分发、dyn Any 向下转型
//       ② 语法演示 —— Box<dyn Fn>、Box<dyn Iterator>、带生命周期的 trait 对象、trait upcasting
//       ③ 应用实战 —— 插件注册表、Any 配置存储
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 16_trait_objects_and_dispatch.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::any::Any;
use std::collections::HashMap;
use std::mem::size_of;

trait Speak {
    fn speak(&self) -> String;
}

struct Dog;
struct Cat;

struct Circle {
    radius: f64,
}

impl Speak for Dog {
    fn speak(&self) -> String {
        String::from("woof")
    }
}

impl Speak for Cat {
    fn speak(&self) -> String {
        String::from("meow")
    }
}

impl Speak for Circle {
    fn speak(&self) -> String {
        format!("circle {:.2}", self.radius)
    }
}

// supertrait：Animal 的 trait 对象也能调用 Speak 的方法
trait Animal: Speak {
    fn legs(&self) -> u32;
}

struct Bird;

impl Speak for Bird {
    fn speak(&self) -> String {
        String::from("tweet")
    }
}

impl Animal for Bird {
    fn legs(&self) -> u32 {
        2
    }
}

// 带借用数据的实现：trait 对象可以用 '_ 绑定生命周期
struct Borrowed<'a> {
    text: &'a str,
}

impl Speak for Borrowed<'_> {
    fn speak(&self) -> String {
        self.text.to_uppercase()
    }
}

// 插件接口：异构实现通过 Box<dyn Plugin> 统一存放
trait Plugin {
    fn name(&self) -> &'static str;
    fn run(&self, input: i32) -> i32;
}

struct Double;
struct AddOne;

impl Plugin for Double {
    fn name(&self) -> &'static str {
        "double"
    }

    fn run(&self, input: i32) -> i32 {
        input * 2
    }
}

impl Plugin for AddOne {
    fn name(&self) -> &'static str {
        "add_one"
    }

    fn run(&self, input: i32) -> i32 {
        input + 1
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 trait 对象是胖指针：数据指针 + vtable 指针，恰好两个字
    println!(
        "fat_pointer={} box_words={}",
        size_of::<&dyn Speak>() == 2 * size_of::<usize>(),
        size_of::<Box<dyn Speak>>() / size_of::<usize>()
    );
    // 预测：fat_pointer=true box_words=2

    // 1.2 具体类型大小 vs 装箱后的 trait 对象大小
    println!(
        "circle_size={} box_size={}",
        size_of::<Circle>(),
        size_of::<Box<dyn Speak>>()
    );
    // 预测：circle_size=8 box_size=16

    // 1.3 同一容器装不同具体类型，调用走 vtable
    let animals: Vec<Box<dyn Speak>> = vec![
        Box::new(Dog),
        Box::new(Cat),
        Box::new(Circle { radius: 1.0 }),
    ];
    let sounds: Vec<String> = animals.iter().map(|animal| animal.speak()).collect();
    println!("dispatch={}", sounds.join(" | "));
    // 预测：dispatch=woof | meow | circle 1.00

    // 1.4 dyn Any：运行期类型擦除 + 向下转型
    let boxed_int: Box<dyn Any> = Box::new(42i32);
    let boxed_text: Box<dyn Any> = Box::new(String::from("hello"));
    println!(
        "any_i32={:?} any_str={:?}",
        boxed_int.downcast_ref::<i32>(),
        boxed_text.downcast_ref::<String>()
    );
    // 预测：any_i32=Some(42) any_str=Some("hello")

    println!("===== 2. SYNTAX =====");

    // 2.1 Box<dyn Fn>：闭包也能做 trait 对象
    let ops: Vec<Box<dyn Fn(i32) -> i32>> = vec![
        Box::new(|value| value + 1),
        Box::new(|value| value * 2),
    ];
    let results: Vec<i32> = ops.iter().map(|op| op(5)).collect();
    println!("ops={:?}", results);
    // 预测：ops=[6, 10]

    // 2.2 Box<dyn Iterator>：返回类型可命名的动态迭代器
    let iterator: Box<dyn Iterator<Item = u32>> = Box::new(1..=3);
    println!("iter={:?}", iterator.collect::<Vec<_>>());
    // 预测：iter=[1, 2, 3]

    // 2.3 trait 对象 + 生命周期：借用局部数据
    let local = String::from("hello");
    let object: Box<dyn Speak + '_> = Box::new(Borrowed { text: &local });
    println!("borrowed={}", object.speak());
    // 预测：borrowed=HELLO

    // 2.4 trait upcasting：&dyn Animal -> &dyn Speak
    let bird = Bird;
    let animal: &dyn Animal = &bird;
    let supertrait: &dyn Speak = animal;
    println!("upcast={} legs={}", supertrait.speak(), animal.legs());
    // 预测：upcast=tweet legs=2

    println!("===== 3. APPLICATION =====");

    // 3.1 插件注册表：Vec<Box<dyn Plugin>> 统一调用不同实现
    let plugins: Vec<Box<dyn Plugin>> = vec![Box::new(Double), Box::new(AddOne)];
    let report: Vec<String> = plugins
        .iter()
        .map(|plugin| format!("{}:{}", plugin.name(), plugin.run(5)))
        .collect();
    println!("plugins={}", report.join(", "));
    // 预测：plugins=double:10, add_one:6

    // 3.2 Any 配置存储：不同类型装进同一个 HashMap，取用时向下转型
    let mut config: HashMap<&str, Box<dyn Any>> = HashMap::new();
    config.insert("threads", Box::new(4i32));
    config.insert("name", Box::new(String::from("service")));
    let threads = config
        .get("threads")
        .and_then(|value| value.downcast_ref::<i32>())
        .copied();
    let name = config
        .get("name")
        .and_then(|value| value.downcast_ref::<String>())
        .map(|value| value.as_str());
    println!("config={:?} {:?}", threads, name);
    // 预测：config=Some(4) Some("service")
}
