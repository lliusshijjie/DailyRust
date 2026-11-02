// 第 11 课示例：模块、可见性与包布局
//
// 结构：① 原理验证 —— 路径解析、私有可见性方向、pub/crate/super 的差异
//       ② 语法演示 —— use、别名、pub use 门面、私有字段封装、super 路径
//       ③ 应用实战 —— 模块化库存 API、批量调用、crate:: 绝对路径
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 11_modules_and_visibility.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

// 根模块的私有函数：当前模块及其后代可见
fn root_secret() -> u32 {
    99
}

mod outer {
    // 子模块可以访问祖先模块的私有项
    pub fn call_root_secret() -> u32 {
        crate::root_secret()
    }
}

mod geometry {
    pub const UNIT_RADIUS: f64 = 1.0;

    pub mod shapes {
        pub struct Circle {
            radius: f64, // 字段私有，外部只能通过方法访问
        }

        impl Circle {
            pub fn new(radius: f64) -> Self {
                Self { radius }
            }

            pub fn radius(&self) -> f64 {
                self.radius
            }

            pub fn area(&self) -> f64 {
                std::f64::consts::PI * self.radius * self.radius
            }
        }

        pub fn unit() -> Circle {
            Circle::new(super::UNIT_RADIUS) // super 指向父模块 geometry
        }
    }
}

mod store {
    pub mod inventory {
        pub struct Item {
            pub name: String,
            quantity: u32, // 私有字段：库存只能通过方法修改
        }

        impl Item {
            pub fn new(name: &str, quantity: u32) -> Self {
                Self {
                    name: name.to_string(),
                    quantity,
                }
            }

            pub fn quantity(&self) -> u32 {
                self.quantity
            }

            pub fn restock(&mut self, amount: u32) -> u32 {
                self.quantity = self.quantity.saturating_add(amount);
                self.quantity
            }
        }

        // 三种可见性：crate 全域 / 仅父模块 / 完全公开
        pub(crate) fn crate_level() -> &'static str {
            "crate"
        }

        pub(super) fn store_level() -> &'static str {
            "store"
        }

        pub fn public_level() -> &'static str {
            "public"
        }
    }

    // store 能看到 inventory 的 pub(super)
    pub fn parent_can_call_child() -> &'static str {
        inventory::store_level()
    }

    fn parent_secret() -> u32 {
        7
    }

    pub mod inner {
        // 子模块能读取父模块的私有项
        pub fn peek_parent_secret() -> u32 {
            super::parent_secret()
        }
    }
}

// 门面模块：只 re-export 公共 API，调用方不需要知道内部路径
mod api {
    pub use crate::geometry::shapes::{unit, Circle};
    pub use crate::store::inventory::Item;
}

mod math {
    pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
        value.max(min).min(max)
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 绝对路径：crate 根 -> geometry -> shapes -> Circle
    let circle = geometry::shapes::Circle::new(2.0);
    println!("circle_area={:.2}", circle.area());
    // 预测：circle_area=12.57

    // 1.2 私有项对后代可见：outer 能调用根模块的 root_secret
    println!("root_secret={}", outer::call_root_secret());
    // 预测：root_secret=99

    // 1.3 子模块能读父模块私有项，但父模块不能读子模块私有项
    println!("child_peek={}", store::inner::peek_parent_secret());
    // 预测：child_peek=7

    // 1.4 pub(crate) 全 crate 可见；pub(super) 只有父模块能用
    println!(
        "visibility={} {} {}",
        store::inventory::public_level(),
        store::inventory::crate_level(),
        store::parent_can_call_child()
    );
    // 预测：visibility=public crate store

    println!("===== 2. SYNTAX =====");

    // 2.1 use 导入 + 通过门面访问：调用方只知道 api 模块
    use api::{unit, Circle};
    let small = unit();
    let big = Circle::new(2.0);
    println!("unit_area={:.2} big_radius={}", small.area(), big.radius());
    // 预测：unit_area=3.14 big_radius=2

    // 2.2 私有字段 + 公开方法：封装库存不变量
    let mut item = api::Item::new("widget", 5);
    println!("item={} qty={}", item.name, item.quantity());
    // 预测：item=widget qty=5

    item.restock(3);
    println!("restocked={}", item.quantity());
    // 预测：restocked=8

    println!("===== 3. APPLICATION =====");

    // 3.1 模块化 API 的批量使用
    let mut items = vec![api::Item::new("a", 2), api::Item::new("b", 3)];
    let total: u32 = items.iter().map(|entry| entry.quantity()).sum();
    println!("total={}", total);
    // 预测：total=5

    for entry in &mut items {
        entry.restock(1);
    }
    let after: u32 = items.iter().map(|entry| entry.quantity()).sum();
    println!("after_restock={}", after);
    // 预测：after_restock=7

    // 3.2 crate:: 绝对路径调用另一个模块
    println!("clamp={}", crate::math::clamp(15, 0, 10));
    // 预测：clamp=10
}
