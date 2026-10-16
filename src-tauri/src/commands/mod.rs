//! ============================================================================
//! commands/mod.rs — wrapperهای نازک #[tauri::command] روی منطق gcode-core
//! ============================================================================
//! الزام امنیتی Docs/PROMPT_04 §2:
//!   «IPC فقط commandهای دامنه‌ای و مشخص ارائه کند؛
//!    command اجرای arbitrary shell، eval یا اجرای G-code وجود نداشته باشد.»
//!
//! این فایل فقط ۱۰ wrapper یک‌خطی است؛ تمام منطق در gcode-core::ipc است که
//! با cargo test روی هر پلتفرمی آزموده می‌شود. هیچ shell/eval/شبکه‌ای وجود
//! ندارد و دسترسی فایل فقط از مسیرهای برگشتی دیالوگ native است.
//! ============================================================================

use gcode_core::designer::DesignerProject;
use gcode_core::export::SaveValidation;
use gcode_core::ipc::{
    designer_compute as core_designer_compute, designer_export_gcode as core_designer_export_gcode,
    designer_export_stl as core_designer_export_stl, explain_line as core_explain_line,
    load_project as core_load_project, parse_gcode as core_parse_gcode,
    read_text_file as core_read_text_file, save_as as core_save_as,
    save_project as core_save_project, validate_gcode as core_validate_gcode,
    DesignerOutput, LineDetail, ParseResult, SaveAsRequest, SaveAsResult, StlExportResult,
};

// ============================================================================
// دستورهای ویرایشگر G-code
// ============================================================================

/// تفسیر کامل فایل G-code و ساخت payload فشرده برای UI
#[tauri::command]
pub fn parse_gcode(content: String, file_name: Option<String>) -> ParseResult {
    core_parse_gcode(content, file_name)
}

/// توضیح کامل یک خط برای پنل inspector
#[tauri::command]
pub fn explain_line(content: String, line_index: usize) -> Option<LineDetail> {
    core_explain_line(content, line_index)
}

/// ذخیره با نام جدید — فایل مبدأ هرگز بازنویسی نمی‌شود
#[tauri::command]
pub fn save_as(req: SaveAsRequest) -> Result<SaveAsResult, String> {
    core_save_as(req)
}

/// اعتبارسنجی پیش از ذخیره (syntax/نامعلوم/checksum/bounds)
#[tauri::command]
pub fn validate_gcode(content: String, build_volume: Option<[f64; 3]>) -> SaveValidation {
    core_validate_gcode(content, build_volume)
}

// ============================================================================
// دستورهای طراح مارپیچی
// ============================================================================

/// محاسبهٔ کامل فرم مارپیچی از پروژه
#[tauri::command]
pub fn designer_compute(project: DesignerProject) -> DesignerOutput {
    core_designer_compute(project)
}

/// تولید متن G-code از پروژهٔ طراح
#[tauri::command]
pub fn designer_export_gcode(project: DesignerProject) -> Result<String, String> {
    core_designer_export_gcode(project)
}

/// تولید فایل STL باینری از پروژهٔ طراح در مسیر انتخابی
#[tauri::command]
pub fn designer_export_stl(project: DesignerProject, path: String) -> Result<StlExportResult, String> {
    core_designer_export_stl(project, path)
}

/// ذخیرهٔ پروژهٔ طراح به JSON در مسیر انتخابی
#[tauri::command]
pub fn save_project(project: DesignerProject, path: String) -> Result<usize, String> {
    core_save_project(project, path)
}

/// بارگذاری پروژهٔ طراح با اعتبارسنجی schema ایمن
#[tauri::command]
pub fn load_project(path: String) -> Result<DesignerProject, String> {
    core_load_project(path)
}

// ============================================================================
// دستور فایل
// ============================================================================

/// خواندن فایل متنی انتخابی از دیالوگ native (سقف ۲۵۶MB)
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    core_read_text_file(path)
}
