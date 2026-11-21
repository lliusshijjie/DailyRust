// 第 29 课示例：Cargo 工程化——features、workspace 与 MSRV
//
// 结构：① 原理验证 —— cfg!(feature) 开关、feature-gated 模块、默认后端
//       ② 语法演示 —— #[cfg(feature)] / #[cfg(not(feature))] 二选一、cfg! 运行时分支
//       ③ 应用实战 —— 默认 text 编码器与 json 编码器的切换、MSRV 常量
//
// 编译运行（edition 2024，需要 rustc 1.85+；默认不启用 json feature）：
//   rustc --edition 2024 -D warnings 29_cargo_features.rs -o run.exe && .\run.exe
// 启用 json feature（真实 Cargo 工程：cargo run --features json；rustc 直编时
// PowerShell 会吞掉 --cfg 里的引号，响应文件写法见课件「30.4 用 rustc 验证 feature」）。
// 先预测每行输出，再运行对照！
//
// 说明：直接用 rustc 编译时没有 Cargo.toml 的 feature 元数据，所以用
// allow(unexpected_cfgs) 关闭 check-cfg 警告；真实 Cargo 工程不需要这一行。

#![allow(unexpected_cfgs)]

// 对应 Cargo.toml:
//   [features]
//   default = ["text"]
//   text = []
//   json = []
#[cfg(feature = "json")]
const BACKEND: &str = "json";
#[cfg(not(feature = "json"))]
const BACKEND: &str = "text";

// feature-gated 模块：对应可选依赖/可选实现
#[cfg(feature = "json")]
mod support {
    pub const LABEL: &str = "json";
}

#[cfg(not(feature = "json"))]
mod support {
    pub const LABEL: &str = "text";
}

// 两套编码实现，编译期二选一
#[cfg(feature = "json")]
fn encode(name: &str, age: u32) -> String {
    format!(r#"{{"name":"{name}","age":{age}}}"#)
}

#[cfg(not(feature = "json"))]
fn encode(name: &str, age: u32) -> String {
    format!("name={name};age={age}")
}

struct Payload<'a> {
    name: &'a str,
    age: u32,
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 cargo --features json 会传 --cfg 'feature="json"'
    println!(
        "json={} text={}",
        cfg!(feature = "json"),
        cfg!(not(feature = "json"))
    );
    // 预测（默认）：json=false text=true
    // 预测（--cfg feature="json"）：json=true text=false

    // 1.2 feature-gated 模块：默认后端 vs json 后端
    println!("backend={}", BACKEND);
    // 预测（默认）：backend=text
    // 预测（json）：backend=json

    println!("===== 2. SYNTAX =====");

    // 2.1 cfg! 是运行时 bool，但两个分支都会参与类型检查
    println!("cfg_bool={}", cfg!(feature = "json"));
    // 预测（默认）：cfg_bool=false
    // 预测（json）：cfg_bool=true

    // 2.2 #[cfg] 在编译期删除不满足条件的代码
    println!("selected={}", support::LABEL);
    // 预测（默认）：selected=text
    // 预测（json）：selected=json

    // 2.3 同一个函数名，两套实现按 feature 二选一
    println!("encode={}", encode("alice", 30));
    // 预测（默认）：encode=name=alice;age=30
    // 预测（json）：encode={"name":"alice","age":30}

    println!("===== 3. APPLICATION =====");

    // 3.1 业务代码不关心后端，只调用统一接口
    let payload = Payload {
        name: "alice",
        age: 30,
    };
    println!("payload={},{}", payload.name, payload.age);
    // 预测：payload=alice,30

    println!("encoded={}", encode(payload.name, payload.age));
    // 预测（默认）：encoded=name=alice;age=30
    // 预测（json）：encoded={"name":"alice","age":30}

    println!("format={}", BACKEND);
    // 预测（默认）：format=text
    // 预测（json）：format=json

    // 3.2 MSRV 与 Cargo.toml 的 rust-version 保持一致（edition 2024 需要 1.85+）
    println!("msrv=1.85");
    // 预测：msrv=1.85
}
