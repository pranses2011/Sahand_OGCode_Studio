//! ============================================================================
//! lib.rs — پوستهٔ Tauri: بوت‌استرپ، state و ثبت دستورهای IPC
//! ============================================================================
//! وظایف این فایل:
//!   ۱) راه‌اندازی Tauri با پلاگین دیالوگ native (Open/Save) و
//!      پلاگین HTTP با دامنهٔ محدود (فقط ارسال بازخورد/گزارش خطا —
//!      نسخهٔ ۱٫۶، درخواست کاربر ۹)؛
//!   ۲) ثبت دستورهای دامنه‌ای محدود از src/commands؛
//!   ۳) بدون plugin shell/updater/fs — الزام امنیتی PROMPT_04 §2.
//!
//! «این برنامه هیچ اتصال شبکه‌ای ندارد» — به‌روزرسانی ۱٫۶: تنها استثنا
//! ارسال «بازخورد/گزارش خطا» به سرویس formsubmit.co است که کاربر خودش
//! از فرم داخل برنامه آغاز می‌کند (دامنه در tauri.conf.json محدود است).
//! ============================================================================

pub mod commands;

use tauri::Manager;

/// راه‌اندازی برنامهٔ دسکتاپ
pub fn run() {
    tauri::Builder::default()
        // دیالوگ native برای Open/Save
        .plugin(tauri_plugin_dialog::init())
        // HTTP با دامنهٔ محدود — فقط ارسال بازخورد/گزارش خطا (نسخهٔ ۱٫۶).
        // دامنهٔ مجاز در tauri.conf.json (http scope) تعریف شده است.
        .plugin(
            tauri_plugin_http::init(),
        )
        .invoke_handler(tauri::generate_handler![
            // ---- ویرایشگر G-code ----
            commands::parse_gcode,
            commands::explain_line,
            commands::save_as,
            commands::validate_gcode,
            // ---- طراح مارپیچی ----
            commands::designer_compute,
            commands::designer_viewport,
            commands::designer_export_gcode,
            commands::designer_export_stl,
            commands::designer_preview_data,
            commands::save_project,
            commands::load_project,
            // ---- فایل ----
            commands::read_text_file,
            commands::write_text_file,
            // ---- پریست‌های پرینتر (گام ۵) ----
            commands::printer_presets,
            // ---- نسخهٔ ۱٫۵ ----
            commands::designer_random_profile,
            // ---- نسخهٔ ۱٫۶: پروفایل شخصی (درخواست کاربر ۴) ----
            commands::save_printer_profile,
            commands::list_printer_profiles,
            commands::delete_printer_profile,
        ])
        .setup(|app| {
            // عنوان پنجره با شمارهٔ نسخه (درخواست کاربر: «نسخه در عنوان»)
            // نسخه از package_info می‌آید — همان tauri.conf.json؛ منبع واحد.
            if let Some(win) = app.get_webview_window("main") {
                let v = app.package_info().version.clone();
                let _ = win.set_title(&format!(
                    "Sahand GCode Studio v{}.{}.{} — استودیوی ویرایش و طراحی G-code",
                    v.major, v.minor, v.patch
                ));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("خطا در راه‌اندازی Sahand GCode Studio");
}
