//! ============================================================================
//! lib.rs — پوستهٔ Tauri: بوت‌استرپ، state و ثبت دستورهای IPC
//! ============================================================================
//! وظایف این فایل:
//!   ۱) راه‌اندازی Tauri با پلاگین دیالوگ native (Open/Save)؛
//!   ۲) ثبت ۱۰ دستور دامنه‌ای محدود از src/commands؛
//!   ۳) بدون plugin shell/updater/http — الزام امنیتی PROMPT_04 §2.
//!
//! «این برنامه هیچ اتصال شبکه‌ای ندارد»: فهرست pluginها فقط dialog است
//! و CSP بدون remote source در tauri.conf.json تنظیم شده است.
//! ============================================================================

pub mod commands;

use tauri::Manager;

/// راه‌اندازی برنامهٔ دسکتاپ
pub fn run() {
    tauri::Builder::default()
        // دیالوگ native برای Open/Save — تنها plugin فعال (بدون fs scope گسترده)
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // ---- ویرایشگر G-code ----
            commands::parse_gcode,
            commands::explain_line,
            commands::save_as,
            commands::validate_gcode,
            // ---- طراح مارپیچی ----
            commands::designer_compute,
            commands::designer_export_gcode,
            commands::designer_export_stl,
            commands::save_project,
            commands::load_project,
            // ---- فایل ----
            commands::read_text_file,
        ])
        .setup(|app| {
            // تنظیم عنوان پنجره برای اطمینان از هویت مستقل برنامه
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_title("G-code Studio — استودیوی ویرایش و طراحی G-code");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("خطا در راه‌اندازی G-code Studio");
}
