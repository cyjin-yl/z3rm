// 迁移追踪类型定义
// 来源: spec §8.1 — z3rm_todo 宏需要 inventory 类型来注册迁移洞
// 用途: #[z3rm_todo] 宏展开时向 inventory 提交一个 Z3rmTodo 条目

/// 迁移洞条目，由 #[z3rm_todo] 宏注册到 inventory。
/// build script 收集所有条目并报告剩余洞数。
pub struct Z3rmTodo {
    /// 类别: removed-crate | broken-ref | stub | disabled-feature
    pub category: &'static str,
    /// 人类可读的描述
    pub description: &'static str,
    /// 源文件路径 (file!())
    pub file: &'static str,
    /// 行号 (line!())
    pub line: u32,
}

inventory::collect!(Z3rmTodo);
