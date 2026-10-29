// 第 08 课示例：结构体、方法与关联函数
//
// 结构：① 原理验证 —— 字段重排与 repr(C)、ZST 与元组结构体的布局
//       ② 语法演示 —— 命名/元组/单元结构体、方法与关联函数、关联常量、更新语法、Default、builder
//       ③ 应用实战 —— 领域模型 Account、newtype 智能构造器、泛型 Stack
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 08_structs_and_methods.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::mem::{align_of, size_of};

// 默认 repr(Rust) 允许编译器重排字段以减少 padding
struct Mixed {
    small: u8,
    big: u32,
    another: u8,
}

// repr(C) 保证字段按声明顺序排列，用于 FFI/按字节布局
#[repr(C)]
struct CMixed {
    small: u8,
    big: u32,
    another: u8,
}

struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    // 关联常量：挂在类型上，不占实例空间
    const SIDES: u32 = 4;

    // 关联函数：没有 self，用 Rectangle::new 调用
    fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn square(size: u32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }

    // &self：只读，不拿走所有权
    fn area(&self) -> u32 {
        self.width * self.height
    }

    // &mut self：就地修改
    fn scale(&mut self, factor: u32) {
        self.width *= factor;
        self.height *= factor;
    }

    // self：消费，调用后原值不可用
    fn into_dimensions(self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// 元组结构体：适合 newtype，把单位/语义编进类型
struct Meters(f64);
struct Seconds(f64);

fn speed(distance: Meters, time: Seconds) -> f64 {
    distance.0 / time.0
}

// 单元结构体：零大小，常用作标记类型
struct Marker;

impl Marker {
    fn name(&self) -> &'static str {
        "marker"
    }
}

struct Config {
    host: String,
    port: u16,
    retries: u8,
}

struct Options {
    verbose: bool,
    retries: u8,
    label: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbose: false,
            retries: 3,
            label: String::from("default"),
        }
    }
}

// 消费型 builder：每个方法拿走 self，返回修改后的新值
struct Request {
    url: String,
    timeout: u64,
}

impl Request {
    fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            timeout: 10,
        }
    }

    fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }
}

// 领域模型：方法直接表达业务规则
struct Account {
    owner: String,
    balance: i64,
}

impl Account {
    fn new(owner: &str, initial: i64) -> Self {
        Self {
            owner: owner.to_string(),
            balance: initial,
        }
    }

    fn owner(&self) -> &str {
        &self.owner
    }

    fn deposit(&mut self, amount: i64) -> Result<i64, String> {
        if amount <= 0 {
            return Err(String::from("amount must be positive"));
        }
        self.balance = self
            .balance
            .checked_add(amount)
            .ok_or_else(|| String::from("balance overflow"))?;
        Ok(self.balance)
    }

    fn withdraw(&mut self, amount: i64) -> bool {
        if amount > 0 && self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }

    fn balance(&self) -> i64 {
        self.balance
    }
}

// newtype + 智能构造器：把不变量放在构造函数里
struct Celsius(f64);

impl Celsius {
    fn new(value: f64) -> Option<Self> {
        if value >= -273.15 {
            Some(Self(value))
        } else {
            None
        }
    }

    fn value(&self) -> f64 {
        self.0
    }
}

// 泛型结构体：impl 上的泛型参数要重复声明
struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn push(&mut self, item: T) {
        self.items.push(item);
    }

    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 repr(Rust) 会重排字段以减少 padding；repr(C) 保持声明顺序
    let mixed = Mixed {
        small: 1,
        big: 2,
        another: 3,
    };
    let cmixed = CMixed {
        small: 4,
        big: 5,
        another: 6,
    };
    let mixed_sum = u32::from(mixed.small) + mixed.big + u32::from(mixed.another);
    let cmixed_sum = u32::from(cmixed.small) + cmixed.big + u32::from(cmixed.another);
    println!(
        "mixed={} c_mixed={} sums={}",
        size_of::<Mixed>(),
        size_of::<CMixed>(),
        mixed_sum + cmixed_sum
    );
    // 预测：mixed=8 c_mixed=12 sums=21

    // 1.2 单元结构体是零大小类型；元组结构体大小就是字段大小
    println!(
        "unit={} tuple={} align_marker={}",
        size_of::<Marker>(),
        size_of::<Meters>(),
        align_of::<Marker>()
    );
    // 预测：unit=0 tuple=8 align_marker=1

    println!("===== 2. SYNTAX =====");

    // 2.1 关联函数、&self、&mut self、self、关联常量、UFCS
    let mut rect = Rectangle::new(3, 4);
    println!("area={}", rect.area());
    // 预测：area=12

    rect.scale(2);
    println!("scaled={}", rect.area());
    // 预测：scaled=48

    println!(
        "sides={} square_area={}",
        Rectangle::SIDES,
        Rectangle::square(5).area()
    );
    // 预测：sides=4 square_area=25

    println!("ufcs={}", Rectangle::area(&rect));
    // 预测：ufcs=48

    let dimensions = rect.into_dimensions();
    println!("dimensions={:?}", dimensions);
    // 预测：dimensions=(6, 8)

    // 2.2 元组结构体：单位进入类型，参数顺序写错会编译失败
    let distance = Meters(100.0);
    let time = Seconds(9.58);
    let meters = distance.0;
    let speed_value = speed(distance, time);
    println!("meters={:.1} speed={:.2}", meters, speed_value);
    // 预测：meters=100.0 speed=10.44

    // 2.3 单元结构体
    println!("marker={}", Marker.name());
    // 预测：marker=marker

    // 2.4 结构体更新语法：..base 会移动未显式给出的非 Copy 字段
    let base = Config {
        host: String::from("prod"),
        port: 80,
        retries: 3,
    };
    let dev = Config {
        host: String::from("dev"),
        ..base
    };
    println!(
        "dev_host={} dev_port={} dev_retries={} base_port={}",
        dev.host, dev.port, dev.retries, base.port
    );
    // 预测：dev_host=dev dev_port=80 dev_retries=3 base_port=80

    // 2.5 Default + 更新语法：只写关心的字段
    let opts = Options {
        verbose: true,
        ..Options::default()
    };
    println!(
        "opts_verbose={} opts_retries={} opts_label={}",
        opts.verbose, opts.retries, opts.label
    );
    // 预测：opts_verbose=true opts_retries=3 opts_label=default

    // 2.6 消费型 builder：方法链每一步都移动 self
    let request = Request::new("api.example.com").timeout(30);
    println!(
        "request_url={} request_timeout={}",
        request.url, request.timeout
    );
    // 预测：request_url=api.example.com request_timeout=30

    println!("===== 3. APPLICATION =====");

    // 3.1 领域模型：方法把业务规则收进类型
    let mut account = Account::new("alice", 100);
    println!("owner={}", account.owner());
    // 预测：owner=alice
    println!("deposit={:?}", account.deposit(50));
    // 预测：deposit=Ok(150)
    println!(
        "withdraw={} balance={}",
        account.withdraw(30),
        account.balance()
    );
    // 预测：withdraw=true balance=120
    println!("overdraw={}", account.withdraw(1000));
    // 预测：overdraw=false

    // 3.2 newtype + 智能构造器：非法状态构造不出来
    let valid = Celsius::new(36.6).map(|celsius| celsius.value());
    let invalid = Celsius::new(-300.0);
    println!("celsius={:?} invalid={}", valid, invalid.is_none());
    // 预测：celsius=Some(36.6) invalid=true

    // 3.3 泛型结构体：方法在同一份代码里服务所有 T
    let mut stack: Stack<i32> = Stack::new();
    stack.push(1);
    stack.push(2);
    let len = stack.len();
    println!("stack_len={} popped={:?}", len, stack.pop());
    // 预测：stack_len=2 popped=Some(2)
}
