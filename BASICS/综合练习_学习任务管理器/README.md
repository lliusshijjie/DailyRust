# 综合练习：学习任务管理器

把第 01～06 课串成一个可以操作的小程序。核心代码在 `study_task_board.rs`，无第三方依赖，可以独立用 `rustc` 编译；测试也放在同一个文件里。

## 设计与范围

程序用 `Board` 拥有两份 `Vec<Task>`：当前任务和归档历史。每个 `Task` 拥有自己的标题 `String`，并记录 ID、预计分钟数、优先级和完成状态。

- 添加、复制、改名、标记完成；对未完成任务批量提高优先级，或交换两个任务的优先级。
- 统计当前任务数量、完成数量、预计总分钟数、平均分钟数，以及 1～5 级优先级分布。
- 把完成任务移动到历史列表；输出当前任务报告及内容校验值。
- 默认运行固定演示，也可进入交互模式。数据只保存在内存中，退出即释放；报告打印到终端，不会自动写文件。

`struct` / `impl` 只承担第 05、06 课示例中已经出现的数据组织和方法调用；暂不扩展生命周期标注、泛型设计、trait 实现、并发或外部库。

## 编译与运行

工具链要求：Rust 1.85+，edition 2024。在本目录打开 PowerShell：

```powershell
rustc --edition 2024 -D warnings .\study_task_board.rs -o .\study_task_board.exe
.\study_task_board.exe
.\study_task_board.exe --interactive
```

默认演示按固定顺序执行 16 条命令。它会刻意执行一次非法优先级输入，显示 `error: priority must be in 1..=5`，然后继续运行，这是预期结果。

演示结束时应有 3 个当前任务、1 个历史任务；当前任务共 80 分钟，平均 26.67 分钟，优先级分布为 `[0, 1, 0, 0, 2]`。

交互模式可输入：

```text
add|所有权练习|25|3
add|切片练习|40|2
add|借用复习|30|5
list
duplicate|1
rename|4|所有权补充练习
done|2
boost|1
swap|1|3
stats
report
archive
history
quit
```

字段用 `|` 分隔，两侧空白会被去掉，标题中间的空格会保留；标题不能包含 `|` 或控制字符。空行和以 `#` 开头的行被忽略。输入 `help` 查看完整命令。

边界约定：

- 当前任务最多 100 个；归档后释放当前任务名额，历史没有额外数量限制。
- 标题为 1～40 个 Unicode 标量值，不按 UTF-8 字节长度或视觉字形数计数。
- 预计分钟数为 `1..=1440`，优先级为 `1..=5`，提高量为 `0..=255`。
- ID 从 1 开始递增，归档后不复用；ID 溢出会报错，失败的添加不会消耗 ID。
- `list` / `stats` / `report` 面向当前任务，其中包括尚未归档的完成任务；`history` 查看归档任务。
- `boost` 跳过已完成任务；`swap` 可交换任意两个当前任务的优先级。
- 重复完成同一个当前任务没有额外效果；复制任务会使用新 ID，并重置为未完成。
- 报告校验值用于比较内容，采用非密码学的 FNV-1a 算法。

## 六课知识如何落到功能

| 课程 | 对应位置 | 需要观察的行为 |
|---|---|---|
| 01 变量绑定与可变性 | 常量定义、`handle_command`、`run_demo` | `const` 与 `static`；遮蔽可以换类型；没有 `mut` 的绑定也能持有 `&mut Board` |
| 02 表达式与控制流 | `statistics`、`swap_priorities`、`run_interactive` | `if` 直接生成值；`let-else`、`return`、`?` 早退；`for`、`loop`、`break`、`continue` |
| 03 标量类型与整数溢出 | `add`、`boost_pending`、`checksum`、统计输出 | ID 用 `checked_add`；优先级用 `saturating_add`；校验值用 `wrapping_mul`；`bool`、`char`、整数、浮点各司其职 |
| 04 元组数组与切片 | `statistics`、演示命令数组、命令解析 | 元组多返回值、数组计数与解构、切片模式；查询用 `&[Task]`，修改用 `&mut [Task]` |
| 05 所有权与移动语义 | `add`、`duplicate`、`rename`、`archive_done`、`into_parts` | 标题移动、整数 Copy、标题 Clone、字段替换时析构、`mem::take`、容器间移动、消费 `self` |
| 06 借用与可变借用 | `find` / `find_mut`、`duplicate`、`swap_priorities`、`run_demo` | 共享和独占借用、NLL、自动重借用、`split_at_mut`、`&String` 到 `&str` 的强制转换 |

没有为了展示语法而引入引用计数、全局可变状态或 `unsafe`。本程序的运算分别需要“报错”“封顶”“回绕”三种策略，`overflowing_*` 没有额外使用场景。

## 建议阅读顺序

1. 看 `Task` 和 `Board`，画出 `Board -> Vec<Task> -> String` 的拥有关系。
2. 看 `add`：为什么标题用 `String`，验证标题却用 `&str`？添加失败时传入的 `String` 会怎样？
3. 看 `duplicate`：为什么标题必须 `clone()`，分钟数和优先级直接读取即可？为什么随后可以调用需要 `&mut self` 的 `add`？
4. 看 `boost_pending` 与 `statistics`：为什么一个借用可变切片，一个借用共享切片？尝试把 `Vec` 换成数组或子切片传入。
5. 看 `swap_priorities`：为什么要先找到索引，再用 `split_at_mut`？传入的两个 ID 顺序相反时会怎样？
6. 看 `archive_done`：`take` 后原字段是什么？按值遍历时，每个任务的所有权流向哪里？
7. 最后看命令解析和 `run_demo`，预测每条命令运行后看板中的数据，再执行对照。

例如，复制任务的过程是：

```text
共享借用原任务
    -> clone 标题 String，Copy 分钟数和优先级
    -> 结束对原任务的共享借用
    -> 可变借用看板，分配新 ID
    -> 把新 String 移进新 Task，再把 Task 移进 Vec
```

移动是所有权语义，不意味着一定发生物理内存复制；编译器可以消除实际搬运。`String` 移动不会深拷贝标题堆缓冲区，归档测试也检查了标题缓冲区地址保持不变。

## 验证

```powershell
rustfmt --edition 2024 --check .\study_task_board.rs
rustc --edition 2024 -D warnings --test .\study_task_board.rs -o .\study_task_board_tests.exe
.\study_task_board_tests.exe
rustc --edition 2024 -O -D warnings --test .\study_task_board.rs -o .\study_task_board_tests_release.exe
.\study_task_board_tests_release.exe
```

文件尾部的测试覆盖输入边界、ID 耗尽、任务容量、复制独立性、饱和提高优先级、交换顺序、归档移动、空集合统计、中文命令、非法命令以及校验值。初读时可以先跳过测试区。

## 后续练习

1. 增加 `reopen|id`：把一个当前任务改回未完成，保持 ID 和标题不变。
2. 增加 `preview|n`：仅用借用的子切片显示前 n 个任务，n 大于任务数量时不能 panic。
3. 增加 `restore|id`：把一个归档任务移回当前列表，保持 ID，检查当前任务容量，尽量不使用 `clone()`。
