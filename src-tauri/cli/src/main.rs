//! ============================================================================
//! gcode-cli — پل توسعهٔ مرورگر به هستهٔ واقعی Rust
//! ============================================================================
//! چرا این crate وجود دارد؟
//!   فرانت‌اند در حالت dev مرورگر (بدون پوستهٔ Tauri) دسترسی به invoke ندارد.
//!   این CLI همان توابع gcode-core::ipc را از stdin (JSON درخواست) به stdout
//!   (JSON پاسخ) نگاشت می‌کند تا سرور توسعهٔ Vite بتواند محاسبات «واقعی» را
//!   به UI برساند — نه دادهٔ ساختگی نمایشی (قاعدهٔ صداقت).
//!
//! قرارداد ساده و پایدار:
//!   ورودی:  {"cmd": "<نام دستور>", "args": { ... }}
//!   خروجی:  JSON نتیجه، یا {"__error": "<پیام فارسی>"} در خطا
//!
//! این ابزار فقط برای توسعه/توسعهٔ اسکرین‌شات است؛ در بستهٔ نصب دسکتاپ
//! نقشی ندارد و دسترسی شبکه‌ای هم ندارد (stdin/stdout خالص).
//! ============================================================================

use gcode_core::designer::DesignerProject;
use gcode_core::ipc;
use std::io::Read;

fn main() {
    // خواندن کامل درخواست از stdin
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("خواندن ورودی از stdin ناموفق بود");
        std::process::exit(1);
    }
    let req: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            let _ = serde_json::to_writer(
                std::io::stdout(),
                &serde_json::json!({ "__error": format!("درخواست JSON نامعتبر است: {e}") }),
            );
            return;
        }
    };
    let cmd = req.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    let args = req.get("args").cloned().unwrap_or(serde_json::json!({}));

    let out = dispatch(cmd, &args);
    // خروجی همیشه یک JSON واحد است (یا نتیجه یا __error)
    let _ = serde_json::to_writer(std::io::stdout(), &out);
}

/// نگاشت دستورها — فقط نام‌های معتبر، بدون shell/eval (الزام امنیتی)
fn dispatch(cmd: &str, args: &serde_json::Value) -> serde_json::Value {
    match cmd {
        // ---- ویرایشگر ----
        "parse_gcode" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let file_name = args
                .get("fileName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            to_json(ipc::parse_gcode(content, file_name))
        }
        "validate_gcode" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let volume = args.get("buildVolume").and_then(|v| v.as_array()).and_then(|a| {
                let dims: Vec<f64> = a
                    .iter()
                    .filter_map(|d| d.as_f64())
                    .collect();
                if dims.len() == 3 {
                    Some([dims[0], dims[1], dims[2]])
                } else {
                    None
                }
            });
            to_json(ipc::validate_gcode(content, volume))
        }
        "explain_line" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let line_index = args.get("lineIndex").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            match ipc::explain_line(content, line_index) {
                Some(detail) => to_json(detail),
                None => serde_json::Value::Null,
            }
        }
        // ---- طراح ----
        "designer_compute" => {
            // هم شکل invoke پوسته ({project}) و هم شکل مستقیم (خود پروژه) پذیرفته می‌شود
            let v = args.get("project").cloned().unwrap_or_else(|| args.clone());
            let project: DesignerProject = match serde_json::from_value(v) {
                Ok(p) => p,
                Err(e) => return err_json(format!("پروژهٔ نامعتبر: {e}")),
            };
            to_json(ipc::designer_compute(project))
        }
        "designer_viewport" => {
            // پیلود تخت سریع برای نمایشگر 3D — نسخهٔ ۱.۴
            let v = args.get("project").cloned().unwrap_or_else(|| args.clone());
            let project: DesignerProject = match serde_json::from_value(v) {
                Ok(p) => p,
                Err(e) => return err_json(format!("پروژهٔ نامعتبر: {e}")),
            };
            to_json(ipc::designer_viewport(project))
        }
        "designer_export_gcode" => {
            let v = args.get("project").cloned().unwrap_or_else(|| args.clone());
            let project: DesignerProject = match serde_json::from_value(v) {
                Ok(p) => p,
                Err(e) => return err_json(format!("پروژهٔ نامعتبر: {e}")),
            };
            match ipc::designer_export_gcode(project) {
                Ok(s) => serde_json::Value::String(s),
                Err(e) => err_json(e),
            }
        }
        // ---- پریست‌های پرینتر ----
        "printer_presets" => to_json(ipc::printer_presets()),
        // ---- دادهٔ پیش‌نمایش الگو/بافت برای کارت‌های تصویری UI ----
        "designer_preview_data" => to_json(ipc::designer_preview_data()),
        // ---- پروفایل تصادفی چاپ‌پسند — نسخهٔ ۱.۵ ----
        "designer_random_profile" => {
            let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
            to_json(ipc::designer_random_profile(seed))
        }
        // ---- نوشتن فایل متنی (ذخیرهٔ مستقیم از طراح) — نسخهٔ ۱.۵ ----
        "write_text_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            match ipc::write_text_file(path, content) {
                Ok(n) => to_json(n),
                Err(e) => err_json(e),
            }
        }
        _ => err_json(format!("دستور ناشناخته: {cmd}")),
    }
}

/// سریال‌سازی نتیجهٔ موفق — خطای سریال‌سازی هرگز نباید panic بدهد
fn to_json<T: serde::Serialize>(v: T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or_else(|e| err_json(format!("سریال‌سازی نتیجه ناموفق: {e}")))
}

fn err_json(msg: String) -> serde_json::Value {
    serde_json::json!({ "__error": msg })
}
