// 第 21 课示例：智能指针——Box、Rc、RefCell
//
// 结构：① 原理验证 —— 指针大小、Box 堆分配、Rc 计数、RefCell 运行期借用检查
//       ② 语法演示 —— 递归 Box 列表、Cell、try_borrow、Rc::try_unwrap、Weak
//       ③ 应用实战 —— Rc<RefCell> 共享待办、Weak 父子树
//
// 编译运行（edition 2024，需要 rustc 1.85+）：
//   rustc --edition 2024 -D warnings 21_smart_pointers.rs -o run.exe && .\run.exe
// 先预测每行输出，再运行对照！

use std::cell::{Cell, RefCell};
use std::mem::size_of;
use std::rc::{Rc, Weak};

// Box 递归类型：用间接层打断无限大小
enum List {
    Cons(i32, Box<List>),
    Nil,
}

fn sum_list(list: &List) -> i32 {
    match list {
        List::Cons(value, next) => value + sum_list(next),
        List::Nil => 0,
    }
}

// Rc + RefCell 的典型组合：共享可变数据
struct Node {
    name: &'static str,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,
}

impl Node {
    fn new(name: &'static str) -> Rc<Self> {
        Rc::new(Self {
            name,
            children: RefCell::new(Vec::new()),
            parent: RefCell::new(Weak::new()),
        })
    }

    fn add_child(parent: &Rc<Node>, child: &Rc<Node>) {
        child.parent.replace(Rc::downgrade(parent));
        parent.children.borrow_mut().push(Rc::clone(child));
    }
}

fn main() {
    println!("===== 1. PRINCIPLES =====");

    // 1.1 Box/Rc 都只是一个机器字大小的指针
    println!(
        "pointer_sizes={}",
        size_of::<Box<i32>>() == size_of::<usize>()
            && size_of::<Rc<i32>>() == size_of::<usize>()
    );
    // 预测：pointer_sizes=true

    // 1.2 Box：独占堆内存，DerefMut 可以直接改堆上的值
    let mut boxed = Box::new(5);
    *boxed += 1;
    println!("box_value={}", *boxed);
    // 预测：box_value=6

    // 1.3 Rc：clone 只是引用计数 +1，ptr_eq 说明共享同一份数据
    let first = Rc::new(String::from("shared"));
    let second = Rc::clone(&first);
    println!(
        "rc_strong={} ptr_eq={}",
        Rc::strong_count(&first),
        Rc::ptr_eq(&first, &second)
    );
    // 预测：rc_strong=2 ptr_eq=true

    drop(second);
    println!("rc_after_drop={}", Rc::strong_count(&first));
    // 预测：rc_after_drop=1

    // 1.4 RefCell：运行期借用检查；已有共享借用时 borrow_mut 失败
    let holder = RefCell::new(1);
    {
        let mut value = holder.borrow_mut();
        *value += 1;
    }
    let borrowed = holder.borrow();
    let conflict = holder.try_borrow_mut().is_err();
    println!("refcell={} try_mut_err={}", *borrowed, conflict);
    // 预测：refcell=2 try_mut_err=true

    // 1.5 Rc<RefCell<T>>：多个所有者共享可变数据
    let shared = Rc::new(RefCell::new(Vec::new()));
    let clone = Rc::clone(&shared);
    clone.borrow_mut().push(1);
    shared.borrow_mut().push(2);
    println!(
        "shared={:?} owners={}",
        shared.borrow(),
        Rc::strong_count(&shared)
    );
    // 预测：shared=[1, 2] owners=2

    println!("===== 2. SYNTAX =====");

    // 2.1 Box 递归列表
    let list = List::Cons(
        1,
        Box::new(List::Cons(
            2,
            Box::new(List::Cons(3, Box::new(List::Nil))),
        )),
    );
    println!("list_sum={}", sum_list(&list));
    // 预测：list_sum=6

    // 2.2 Cell：Copy 值的 get/set，不需要借用守卫
    let cell = Cell::new(10);
    cell.set(cell.get() + 1);
    println!("cell={}", cell.get());
    // 预测：cell=11

    // 2.3 RefCell 的 try_borrow / try_borrow_mut
    let counter = RefCell::new(5);
    let _first_borrow = counter.borrow();
    let second_ok = counter.try_borrow().is_ok();
    let blocked = counter.try_borrow_mut().is_err();
    println!("try_borrow_ok={} try_mut_err={}", second_ok, blocked);
    // 预测：try_borrow_ok=true try_mut_err=true

    // 2.4 Rc::try_unwrap：最后一个强引用才能取回所有权
    let solo = Rc::new(String::from("solo"));
    let unwrapped = Rc::try_unwrap(solo).map(|value| value.len());
    println!("try_unwrap={:?}", unwrapped);
    // 预测：try_unwrap=Ok(4)

    // 2.5 Weak：不增加强引用计数；强引用释放后 upgrade 返回 None
    let strong = Rc::new(1);
    let weak = Rc::downgrade(&strong);
    let upgraded = weak.upgrade().map(|value| *value);
    drop(strong);
    let after = weak.upgrade();
    println!("weak_upgrade={:?} weak_after={:?}", upgraded, after);
    // 预测：weak_upgrade=Some(1) weak_after=None

    println!("===== 3. APPLICATION =====");

    // 3.1 共享可变待办：两个句柄操作同一个 Vec
    let todo = Rc::new(RefCell::new(Vec::new()));
    let handle = Rc::clone(&todo);
    handle.borrow_mut().push(String::from("write lesson"));
    todo.borrow_mut().push(String::from("push repo"));
    println!(
        "todo={:?} owners={}",
        todo.borrow(),
        Rc::strong_count(&todo)
    );
    // 预测：todo=["write lesson", "push repo"] owners=2

    // 3.2 父子树：父持强引用，子持 Weak 反向引用，避免循环泄漏
    let root = Node::new("root");
    let child = Node::new("child");
    Node::add_child(&root, &child);
    let parent_name = child.parent.borrow().upgrade().map(|node| node.name);
    println!(
        "tree_children={} tree_parent={:?} weak_count={}",
        root.children.borrow().len(),
        parent_name,
        Rc::weak_count(&root)
    );
    // 预测：tree_children=1 tree_parent=Some("root") weak_count=1
}
