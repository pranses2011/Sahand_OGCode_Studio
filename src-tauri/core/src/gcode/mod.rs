//! ============================================================================
//! gcode/mod.rs — ماژول پردازش G-code
//! ============================================================================
//! ساختار دولایه طبق الزام Docs/PROMPT_02 §4:
//!   ۱) lexer.rs    : شکستن خط خام به واژه‌ها (بدون تفسیر)؛
//!   ۲) interpreter.rs : اجرای فرمان‌ها با وضعیت modal و تولید Toolpath IR.
//! doc.rs سند خام (متن، پایان‌خط، BOM) را مدیریت و این دو را کنار هم می‌خواند.
//! ============================================================================

pub mod interpreter;
pub mod lexer;

// re-export های پرکاربرد برای راحتی import در بقیهٔ کد
pub use interpreter::{
    interpret, CommandExecution, InterpretOptions, Interpretation, InterpretedLine, LineType,
    ParamRow, StateSnapshot,
};
pub use lexer::{lex_line, LexedLine, LexedWord, LineKind};
