// 综合练习：学习任务管理器（覆盖第 01～06 课）
//
// 编译：rustc --edition 2024 -D warnings study_task_board.rs -o study_task_board.exe
// 演示：.\study_task_board.exe
// 交互：.\study_task_board.exe --interactive
// 测试：rustc --edition 2024 -D warnings --test study_task_board.rs -o study_task_board_tests.exe
//
// 建议阅读顺序：Task / Board -> add -> duplicate -> swap_priorities
//               -> archive_done -> statistics -> handle_command -> run_demo。
// 中文注释解释原理；控制台提示使用 ASCII，与前六课的示例保持一致。

use std::io::{self, Write};
use std::mem;

// [01] const 用于边界和数组长度；static 为程序横幅提供一份静态数据。
const MAX_TASKS: usize = 100; // 当前任务上限；已归档的任务不计入。
const MAX_TITLE_CHARS: usize = 40;
const MAX_MINUTES: u16 = 1_440;
const MAX_PRIORITY: u8 = 5;
const PRIORITY_LEVELS: usize = MAX_PRIORITY as usize;
static BANNER: &str = "Rust Study Task Board | lessons 01-06";

// struct 只用来把相关字段放在一起，语法与第 05、06 课的示例相同。
// Task 没有实现 Copy：它拥有 String，转交整个任务时发生移动。
struct Task {
    id: u32,
    title: String,
    minutes: u16,
    priority: u8,
    done: bool,
}

struct Board {
    tasks: Vec<Task>,
    history: Vec<Task>,
    last_id: u32,
}

impl Board {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            history: Vec::new(),
            last_id: 0,
        }
    }

    // [05] String 按值传入：成功后把标题移入任务，不复制标题的堆缓冲区。
    // 所有验证通过后才修改看板，失败不会消耗 ID 或插入半个任务。
    fn add(&mut self, title: String, minutes: u16, priority: u8) -> Result<u32, &'static str> {
        validate_title(&title)?;
        if self.tasks.len() >= MAX_TASKS {
            return Err("active task limit reached");
        }
        if !(1..=MAX_MINUTES).contains(&minutes) {
            return Err("minutes must be in 1..=1440");
        }
        if !(1..=MAX_PRIORITY).contains(&priority) {
            return Err("priority must be in 1..=5");
        }

        // [03] ID 不能回绕到 0，所以显式使用 checked_add，而非依赖构建模式。
        let id = self.last_id.checked_add(1).ok_or("task ID exhausted")?;
        self.tasks.push(Task {
            id,
            title,
            minutes,
            priority,
            done: false,
        });
        self.last_id = id;
        Ok(id)
    }

    // [06] 查询只借用；返回的引用指向看板内部，不返回局部临时值。
    // 这里的生命周期可以省略，显式生命周期留待后续课程。
    fn find(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    fn find_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == id)
    }

    fn finish(&mut self, id: u32) -> Result<(), &'static str> {
        let task = self.find_mut(id).ok_or("active task not found")?;
        task.done = true;
        Ok(())
    }

    fn rename(&mut self, id: u32, title: String) -> Result<(), &'static str> {
        validate_title(&title)?;
        let task = self.find_mut(id).ok_or("active task not found")?;
        // [05] 新 String 移入字段，原字段的 String 在赋值时被释放。
        task.title = title;
        Ok(())
    }

    fn duplicate(&mut self, id: u32) -> Result<u32, &'static str> {
        let original = self.find(id).ok_or("active task not found")?;
        // [04/05] 元组承接三个值：String 显式 clone，整数隐式 Copy。
        // 复制出的任务需要独立标题，因而这里的 clone 有明确用途。
        let (title, minutes, priority) =
            (original.title.clone(), original.minutes, original.priority);
        // [06] NLL：original 最后一次使用已结束，现在可以可变借用 self。
        // 新任务重新分配 ID，完成状态从 false 开始。
        self.add(title, minutes, priority)
    }

    fn swap_priorities(&mut self, first_id: u32, second_id: u32) -> Result<(), &'static str> {
        if first_id == second_id {
            return Err("choose two different task IDs");
        }
        let first = self.tasks.iter().position(|task| task.id == first_id);
        let second = self.tasks.iter().position(|task| task.id == second_id);
        let (Some(first), Some(second)) = (first, second) else {
            return Err("active task not found");
        };

        // [02/04] if 产生一个元组值；先把索引按大小排列。
        let (low, high) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        // [06] 一次拆成不重叠的两段，安全地同时获得两个可变引用。
        // 不直接写 &mut self.tasks[first] 和 &mut self.tasks[second]。
        let (left, right) = self.tasks.split_at_mut(high);
        mem::swap(&mut left[low].priority, &mut right[0].priority);
        Ok(())
    }

    fn archive_done(&mut self) -> usize {
        // [05] take 取出整个 Vec，同时留下空 Vec，self 仍是完整有效的值。
        let tasks = mem::take(&mut self.tasks);
        let mut archived = 0;
        for task in tasks {
            // 按值迭代：每个 Task 移入一个目标容器，没有 clone。
            if task.done {
                self.history.push(task);
                archived += 1;
            } else {
                self.tasks.push(task);
            }
        }
        archived
    }

    // [05] self 按值传入，消费看板，把两个 Vec 的所有权交给调用方。
    // Board 没有自定义 Drop，因此允许把字段分别移出。
    fn into_parts(self) -> (Vec<Task>, Vec<Task>) {
        (self.tasks, self.history)
    }
}

fn validate_title(title: &str) -> Result<(), &'static str> {
    // [03] char 是 Unicode 标量值；chars().count() 与 UTF-8 字节长度不同。
    // 这里按 Unicode 标量值限制长度，不按视觉上的字形簇计数。
    if title.trim().is_empty() {
        return Err("title must not be empty");
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err("title must contain at most 40 Unicode scalar values");
    }
    // 标题会用于终端和逐行报告，拒绝控制字符及命令分隔符。
    if title.contains('|') || title.chars().any(char::is_control) {
        return Err("title must not contain pipes or control characters");
    }
    Ok(())
}

// [06] 参数是可变切片，既可处理 Vec，也可处理数组和子切片。
fn boost_pending(tasks: &mut [Task], delta: u8) -> usize {
    let mut changed = 0;
    for task in tasks {
        if task.done {
            continue;
        }
        let previous = task.priority; // u8 是 Copy，读取后原字段仍可使用。
        // [03] 优先级需要封顶：先避免 u8 溢出，再限制在业务上限 5。
        task.priority = task.priority.saturating_add(delta).min(MAX_PRIORITY);
        if task.priority != previous {
            changed += 1;
        }
    }
    changed
}

// [04] 元组做多返回值；定长数组记录 1～5 级任务各有几个。
fn statistics(tasks: &[Task]) -> (usize, u64, f64, [usize; PRIORITY_LEVELS]) {
    let mut done = 0;
    let mut total_minutes = 0u64;
    let mut priorities = [0usize; PRIORITY_LEVELS];
    for task in tasks {
        if task.done {
            done += 1;
        }
        // u16 先无损扩展为 u64；当前任务最多 100 个，不会累计溢出。
        total_minutes += u64::from(task.minutes);
        priorities[usize::from(task.priority - 1)] += 1;
    }
    // [02/03] if 是表达式；空切片单独处理，避免浮点除零得到 NaN。
    let average = if tasks.is_empty() {
        0.0
    } else {
        total_minutes as f64 / tasks.len() as f64
    };
    (done, total_minutes, average, priorities)
}

fn print_tasks(label: &str, tasks: &[Task]) {
    println!("{label}: {} task(s)", tasks.len());
    if tasks.is_empty() {
        println!("  (empty)");
        return;
    }
    for task in tasks {
        let marker: char = if task.done { 'x' } else { ' ' };
        println!(
            "  [{marker}] #{} P{} {:>4} min | {}",
            task.id, task.priority, task.minutes, task.title
        );
    }
}

// [06] 借用调用方的 String 写入报告，调用方保留所有权。
fn append_report(output: &mut String, tasks: &[Task]) {
    for task in tasks {
        output.push_str(&format!(
            "{}|{}|{}|{}|{}\n",
            task.id, task.title, task.minutes, task.priority, task.done
        ));
    }
}

// [03] 简单 FNV-1a 校验值：乘法回绕是算法定义的一部分。
// 用于比较报告内容，不能用于安全校验或密码学用途。
fn checksum(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn print_help() {
    println!(
        "Commands (separate fields with |):\n\
         add|title|minutes|priority   add a pending task\n\
         list                       list current tasks\n\
         done|id                    mark a task as done\n\
         rename|id|title            replace a title\n\
         duplicate|id               clone a title into a new pending task\n\
         boost|delta                raise pending priorities, capped at 5\n\
         swap|id|id                 exchange two priorities\n\
         stats                      summarize current tasks\n\
         archive                    move completed tasks into history\n\
         history                    list archived tasks\n\
         report                     print current tasks and their checksum\n\
         help                       show commands\n\
         quit                       exit (data is kept in memory only)"
    );
}

// [02] Result 和 ? 仅用于把错误交给外层统一显示，与第 02 课的早退相同。
// 返回 true 继续处理，false 结束；这里只借用看板，不拿走它。
fn handle_command(board: &mut Board, line: &str) -> Result<bool, &'static str> {
    // [01] 遮蔽：同名新绑定由 &str 变为 Vec<&str>，不是给原变量赋值。
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(true);
    }
    let line: Vec<&str> = line.split('|').map(str::trim).collect();
    // [04] 对切片做模式匹配，字段个数不正确会落入最后的错误分支。
    match line.as_slice() {
        ["add", title, minutes, priority] => {
            let minutes = minutes.parse::<u16>().map_err(|_| "invalid minutes")?;
            let priority = priority.parse::<u8>().map_err(|_| "invalid priority")?;
            let id = board.add(title.to_string(), minutes, priority)?;
            println!("added #{id}");
        }
        ["list"] => print_tasks("Current", &board.tasks),
        ["done", id] => {
            let id = id.parse::<u32>().map_err(|_| "invalid task ID")?;
            board.finish(id)?;
            println!("completed #{id}");
        }
        ["rename", id, title] => {
            let id = id.parse::<u32>().map_err(|_| "invalid task ID")?;
            board.rename(id, title.to_string())?;
            println!("renamed #{id}");
        }
        ["duplicate", id] => {
            let id = id.parse::<u32>().map_err(|_| "invalid task ID")?;
            let new_id = board.duplicate(id)?;
            println!("duplicated #{id} as #{new_id}");
        }
        ["boost", delta] => {
            let delta = delta
                .parse::<u8>()
                .map_err(|_| "delta must be in 0..=255")?;
            let changed = boost_pending(&mut board.tasks, delta);
            println!("boosted {changed} pending task(s)");
        }
        ["swap", first, second] => {
            let first = first.parse::<u32>().map_err(|_| "invalid task ID")?;
            let second = second.parse::<u32>().map_err(|_| "invalid task ID")?;
            board.swap_priorities(first, second)?;
            println!("swapped priorities of #{first} and #{second}");
        }
        ["stats"] => {
            let (done, minutes, average, priorities) = statistics(&board.tasks);
            println!(
                "current={} done={done} planned_minutes={minutes} average_minutes={average:.2}",
                board.tasks.len()
            );
            println!("priority_counts_1_to_5={priorities:?}");
        }
        ["archive"] => println!("archived {} task(s)", board.archive_done()),
        ["history"] => print_tasks("History", &board.history),
        ["report"] => {
            let mut report = String::new();
            append_report(&mut report, &board.tasks);
            print!("{report}");
            println!("checksum={:08x}", checksum(report.as_bytes()));
        }
        ["help"] => print_help(),
        ["quit"] => return Ok(false),
        _ => return Err("unknown command or wrong field count; enter help"),
    }
    Ok(true)
}

fn execute_command(board: &mut Board, line: &str) -> bool {
    match handle_command(board, line) {
        Ok(keep_running) => keep_running,
        Err(message) => {
            println!("error: {message}");
            true // 一条命令失败后仍可继续输入。
        }
    }
}

fn run_demo() {
    // [04] 命令是定长数组；运行时按值遍历，&str 元素本身是 Copy。
    let commands = [
        "add|Ownership basics|25|3",
        "add|Slice practice|40|2",
        "add|Borrowing review|30|5",
        "list",
        "duplicate|1",
        "rename|4|Ownership extra practice",
        "done|2",
        "boost|255",
        "swap|1|2",
        "stats",
        "report",
        "archive",
        "history",
        "list",
        "add|Invalid priority|10|9", // 预期错误：看板不会被修改。
        "stats",
    ];
    let mut board = Board::new();
    // [01/06] 绑定本身没有 mut，也能通过 &mut Board 修改看板。
    let borrowed = &mut board;
    for command in commands {
        println!("\n> {command}");
        // 自动重借用：每次函数调用都暂借 borrowed，它不是被 Copy 了。
        if !execute_command(borrowed, command) {
            break;
        }
    }
    // borrowed 的最后一次使用已结束，可以按值消费 board。
    let (current, history) = board.into_parts();
    println!(
        "\nOwnership returned: current={} history={}",
        current.len(),
        history.len()
    );
    // 两个 Vec 在函数结束时自动释放，内部 Task 和 String 随之析构。
}

fn run_interactive() -> io::Result<()> {
    let mut board = Board::new();
    print_help();
    loop {
        print!("\nboard> ");
        io::stdout().flush()?;
        let mut line = String::new();
        // [02] EOF 和 quit 都是正常退出路径；I/O 失败由 ? 向上传递。
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        if !execute_command(&mut board, &line) {
            break;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    println!("{BANNER}");
    let args: Vec<String> = std::env::args().skip(1).collect();
    // [04] 区分空参数切片、单参数切片和其他长度。
    match args.as_slice() {
        [] => run_demo(),
        [mode] if mode == "--demo" => run_demo(),
        [mode] if mode == "--interactive" => run_interactive()?,
        [mode] if mode == "--help" => {
            println!("Usage: study_task_board.exe [--demo | --interactive | --help]");
            print_help();
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected --demo, --interactive, --help, or no arguments",
            ));
        }
    }
    Ok(())
}

// 以下是独立的验证区，初读时可先跳过；正常运行不会执行测试。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_inputs_leave_the_board_unchanged() {
        let mut board = Board::new();
        for (title, minutes, priority) in [
            (" ", 25, 3),
            ("A", 0, 3),
            ("A", MAX_MINUTES + 1, 3),
            ("A", 25, 0),
            ("A", 25, MAX_PRIORITY + 1),
            ("A|B", 25, 3),
            ("A\nB", 25, 3),
        ] {
            assert!(board.add(title.to_string(), minutes, priority).is_err());
        }
        assert!(board.add("学".repeat(MAX_TITLE_CHARS + 1), 25, 3).is_err());
        assert!(board.tasks.is_empty());
        assert_eq!(board.last_id, 0);
        assert_eq!(board.add("学".repeat(MAX_TITLE_CHARS), 25, 3), Ok(1));
    }

    #[test]
    fn capacity_and_id_exhaustion_do_not_wrap_or_partially_insert() {
        let mut board = Board::new();
        for _ in 0..MAX_TASKS {
            board.add("task".to_string(), 1, 1).unwrap();
        }
        assert_eq!(
            board.add("extra".to_string(), 1, 1),
            Err("active task limit reached")
        );
        assert_eq!(board.tasks.len(), MAX_TASKS);
        assert_eq!(board.last_id, MAX_TASKS as u32);

        let mut board = Board::new();
        board.last_id = u32::MAX - 1;
        assert_eq!(board.add("last".to_string(), 1, 1), Ok(u32::MAX));
        assert_eq!(
            board.add("extra".to_string(), 1, 1),
            Err("task ID exhausted")
        );
        assert_eq!(board.tasks.len(), 1);
        assert_eq!(board.last_id, u32::MAX);
    }

    #[test]
    fn duplicated_title_is_independent_and_done_is_reset() {
        let mut board = Board::new();
        let original = board.add("original".to_string(), 25, 3).unwrap();
        board.finish(original).unwrap();
        let duplicate = board.duplicate(original).unwrap();
        board.rename(duplicate, "changed".to_string()).unwrap();
        assert_eq!(board.find(original).unwrap().title, "original");
        assert!(board.find(original).unwrap().done);
        assert_eq!(board.find(duplicate).unwrap().title, "changed");
        assert!(!board.find(duplicate).unwrap().done);
    }

    #[test]
    fn boost_saturates_and_only_changes_the_borrowed_pending_slice() {
        let mut board = Board::new();
        board.add("pending".to_string(), 10, 4).unwrap();
        let done = board.add("done".to_string(), 20, 2).unwrap();
        board.add("outside slice".to_string(), 30, 1).unwrap();
        board.finish(done).unwrap();
        assert_eq!(boost_pending(&mut board.tasks[..2], u8::MAX), 1);
        assert_eq!(board.tasks[0].priority, MAX_PRIORITY);
        assert_eq!(board.tasks[1].priority, 2);
        assert_eq!(board.tasks[2].priority, 1);
        assert_eq!(boost_pending(&mut board.tasks[..2], u8::MAX), 0);
    }

    #[test]
    fn swap_accepts_both_orders_and_rejects_invalid_ids() {
        let mut board = Board::new();
        let first = board.add("first".to_string(), 10, 1).unwrap();
        let second = board.add("second".to_string(), 20, 5).unwrap();
        board.swap_priorities(second, first).unwrap();
        assert_eq!(board.find(first).unwrap().priority, 5);
        assert_eq!(board.find(second).unwrap().priority, 1);
        assert!(board.swap_priorities(first, first).is_err());
        assert!(board.swap_priorities(first, 999).is_err());
        assert_eq!(board.find(first).unwrap().priority, 5);
        board.swap_priorities(first, second).unwrap();
        assert_eq!(board.find(first).unwrap().priority, 1);
    }

    #[test]
    fn archive_preserves_order_and_transfers_owned_strings() {
        let mut board = Board::new();
        let first = board.add("keep first".to_string(), 10, 1).unwrap();
        let second = board.add("archive".to_string(), 20, 2).unwrap();
        let third = board.add("keep third".to_string(), 30, 3).unwrap();
        let title_ptr = board.find(second).unwrap().title.as_ptr();
        board.finish(second).unwrap();
        assert_eq!(board.archive_done(), 1);
        assert_eq!(board.tasks[0].id, first);
        assert_eq!(board.tasks[1].id, third);
        assert_eq!(board.history[0].id, second);
        assert_eq!(board.history[0].title.as_ptr(), title_ptr);
        assert!(board.find(second).is_none());
        assert_eq!(board.archive_done(), 0);
        assert!(board.add("new task".to_string(), 10, 1).unwrap() > third);
        let (tasks, history) = board.into_parts();
        assert_eq!((tasks.len(), history.len()), (3, 1));
    }

    #[test]
    fn statistics_cover_empty_and_mixed_tasks() {
        assert_eq!(statistics(&[]), (0, 0, 0.0, [0; PRIORITY_LEVELS]));
        let mut board = Board::new();
        board.add("first".to_string(), 10, 1).unwrap();
        let second = board.add("second".to_string(), 20, 5).unwrap();
        board.finish(second).unwrap();
        assert_eq!(statistics(&board.tasks), (1, 30, 15.0, [1, 0, 0, 0, 1]));
    }

    #[test]
    fn parser_handles_whitespace_unicode_errors_and_quit() {
        let mut board = Board::new();
        assert_eq!(
            handle_command(&mut board, " add | 借用 练习 | 25 | 3 \r\n"),
            Ok(true)
        );
        assert_eq!(board.tasks[0].title, "借用 练习");
        for command in [
            "add|missing fields",
            "add|x|-1|3",
            "add|x|65536|3",
            "done|abc",
            "done|999",
            "rename|1|",
            "boost|256",
            "list|extra",
            "unknown",
        ] {
            assert!(handle_command(&mut board, command).is_err(), "{command}");
        }
        assert_eq!(board.tasks.len(), 1);
        assert_eq!(board.last_id, 1);
        assert_eq!(handle_command(&mut board, " # comment"), Ok(true));
        assert_eq!(handle_command(&mut board, ""), Ok(true));
        assert_eq!(handle_command(&mut board, "quit"), Ok(false));
    }

    #[test]
    fn report_appends_and_checksum_matches_known_vectors() {
        let mut board = Board::new();
        board.add("borrow".to_string(), 25, 3).unwrap();
        let mut report = "header\n".to_string();
        append_report(&mut report, &board.tasks);
        assert_eq!(report, "header\n1|borrow|25|3|false\n");
        assert_eq!(checksum(b""), 0x811c_9dc5);
        assert_eq!(checksum(b"hello"), 0x4f9f_2cab);
    }
}
