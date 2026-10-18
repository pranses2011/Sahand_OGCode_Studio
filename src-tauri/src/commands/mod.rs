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
    designer_export_stl as core_designer_export_stl, designer_preview_data as core_designer_preview_data,
    designer_random_profile as core_designer_random_profile, designer_viewport as core_designer_viewport,
    delete_printer_profile as core_delete_printer_profile,
    explain_line as core_explain_line, list_printer_profiles as core_list_printer_profiles,
    load_project as core_load_project, parse_gcode as core_parse_gcode,
    printer_presets as core_printer_presets, read_text_file as core_read_text_file,
    save_as as core_save_as, save_printer_profile as core_save_printer_profile,
    save_project as core_save_project, validate_gcode as core_validate_gcode,
    write_text_file as core_write_text_file, DesignerOutput, FastViewport, LineDetail, ParseResult,
    PreviewData, SaveAsRequest, SaveAsResult, StlExportResult,
};
use gcode_core::designer::PrinterProfile;

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

/// پیلود سریع پیش‌نمایش برای نمایشگر 3D — نسخهٔ ۱.۴ (کارایی)
#[tauri::command]
pub fn designer_viewport(project: DesignerProject) -> FastViewport {
    core_designer_viewport(project)
}

/// تولید متن G-code از پروژهٔ طراح — quality: نسخهٔ ۱٫۶ (دیالوگ کیفیت)
/// None|۱ = استاندارد؛ ۰٫۵ = زبر؛ ۲ = ریز؛ ۳ = بسیار ریز
#[tauri::command]
pub fn designer_export_gcode(
    project: DesignerProject,
    quality: Option<f64>,
) -> Result<String, String> {
    core_designer_export_gcode(project, quality)
}

/// تولید فایل STL باینری از پروژهٔ طراح در مسیر انتخابی —
/// quality: None|۱=استاندارد، ۰٫۵=سریع، ۲=ریز، ۳=بسیار ریز (نسخهٔ ۱.۵)
#[tauri::command]
pub fn designer_export_stl(
    project: DesignerProject,
    path: String,
    quality: Option<f64>,
) -> Result<StlExportResult, String> {
    core_designer_export_stl(project, path, quality)
}

/// دادهٔ پیش‌نمایش الگو/بافت برای کارت‌های تصویری UI (از ریاضی واقعی هسته)
#[tauri::command]
pub fn designer_preview_data() -> PreviewData {
    core_designer_preview_data()
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

// ============================================================================
// دستورهای نسخهٔ ۱.۵
// ============================================================================

/// پروفایل تصادفی چاپ‌پسند — دکمهٔ 🎲 در گام ۱ (seed = زمان UI)
#[tauri::command]
pub fn designer_random_profile(seed: u64) -> Vec<gcode_core::designer::ProfilePoint> {
    core_designer_random_profile(seed)
}

/// نوشتن فایل متنی — ذخیرهٔ مستقیم G-code از طراح بدون گذر از ویرایشگر
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<usize, String> {
    core_write_text_file(path, content)
}

// ============================================================================
// دستور پریست‌های پرینتر (گام ۵)
// ============================================================================

/// فهرست ۲۰ پریست پرینتر واقعی (وضعیت community — داده از صفحات رسمی)
#[tauri::command]
pub fn printer_presets() -> Vec<PrinterProfile> {
    core_printer_presets()
}

// ============================================================================
// دستورهای نسخهٔ ۱٫۶ — پروفایل شخصی پرینتر (درخواست کاربر ۴)
// ============================================================================

/// ذخیرهٔ پروفایل شخصی در app_data/printer_profiles — نام از پروفایل
#[tauri::command]
pub fn save_printer_profile(
    app: tauri::AppHandle,
    profile: PrinterProfile,
) -> Result<String, String> {
    let dir = profiles_base_dir(&app)?;
    core_save_printer_profile(dir, profile)
}

/// فهرست پروفایل‌های شخصی ذخیره‌شده
#[tauri::command]
pub fn list_printer_profiles(app: tauri::AppHandle) -> Vec<PrinterProfile> {
    match profiles_base_dir(&app) {
        Ok(dir) => core_list_printer_profiles(dir),
        Err(_) => Vec::new(),
    }
}

/// حذف پروفایل شخصی با نام
#[tauri::command]
pub fn delete_printer_profile(
    app: tauri::AppHandle,
    name: String,
) -> Result<(), String> {
    let dir = profiles_base_dir(&app)?;
    core_delete_printer_profile(dir, name)
}

/// مسیر پایهٔ دادهٔ برنامه (app_data) — سازگار با هر پلتفرم
fn profiles_base_dir(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("دسترسی به مسیر دادهٔ برنامه ناموفق بود: {e}"))
}

// ============================================================================
// دستورهای نسخهٔ ۱٫۷ — ایمیل SMTP مستقیم (درخواست کاربر ۱۰)
// ============================================================================
// رفع باگ ۱٫۶: formsubmit «موفق» جواب می‌داد ولی تا فعال‌سازی یک‌بارهٔ
// مالک ایمیل، هیچی تحویل نمی‌شد. حالا برنامه خودش SMTP مستقیم می‌زند؛
// تنظیمات در app_data/smtp.json (crate gcode-mail — بدون وابستگی Tauri).

use gcode_mail::{SmtpConfig, SmtpConfigView};

/// پیکربندی جاری SMTP — نمای بی‌رمز (رمز هرگز به WebView نمی‌رسد)
#[tauri::command]
pub fn smtp_get_config(app: tauri::AppHandle) -> Result<SmtpConfigView, String> {
    let dir = profiles_base_dir(&app)?;
    let cfg: SmtpConfig = gcode_mail::load_config(&dir)?;
    Ok(cfg.masked())
}

/// ذخیرهٔ پیکربندی SMTP — رمز فقط در فایل محلی می‌ماند
#[tauri::command]
pub fn smtp_save_config(
    app: tauri::AppHandle,
    host: String,
    port: u16,
    username: String,
    password: String,
    from_name: Option<String>,
    to_email: Option<String>,
) -> Result<SmtpConfigView, String> {
    let cfg = SmtpConfig {
        host,
        port,
        username,
        password,
        from_name: from_name.unwrap_or_else(|| "Sahand GCode Studio".into()),
        to_email: to_email.unwrap_or_else(|| "Pranses2011@GMail.com".into()),
    };
    gcode_mail::save_config(&profiles_base_dir(&app)?, &cfg)?;
    Ok(cfg.masked())
}

/// حذف کامل پیکربندی SMTP
#[tauri::command]
pub fn smtp_clear_config(app: tauri::AppHandle) -> Result<(), String> {
    gcode_mail::clear_config(&profiles_base_dir(&app)?)
}

/// ارسال ایمیل آزمایشی — دکمهٔ «تست» در تنظیمات ایمیل
#[tauri::command]
pub fn smtp_test(app: tauri::AppHandle) -> Result<gcode_mail::MailSendReport, String> {
    let dir = profiles_base_dir(&app)?;
    let cfg: SmtpConfig = gcode_mail::load_config(&dir)?;
    gcode_mail::send_test_email(&cfg)
}

/// ارسال بازخورد/گزارش خطا با SMTP مستقیم — HTML فارسی RTL
#[tauri::command]
pub fn smtp_send_feedback(
    app: tauri::AppHandle,
    kind: String,
    message: String,
    from_name: Option<String>,
    reply_email: Option<String>,
    context: Option<String>,
    detail: Option<String>,
    app_version: Option<String>,
) -> Result<gcode_mail::MailSendReport, String> {
    let dir = profiles_base_dir(&app)?;
    let cfg: SmtpConfig = gcode_mail::load_config(&dir)?;
    if !cfg.is_ready() {
        return Err("SMTP تنظیم نشده است — از «تنظیمات ایمیل» آن را کامل کنید".into());
    }
    let subject = if kind == "error" {
        format!(
            "گزارش خودکار خطا — Sahand GCode Studio v{}",
            app_version.as_deref().unwrap_or("")
        )
    } else {
        format!(
            "بازخورد کاربر — Sahand GCode Studio v{}",
            app_version.as_deref().unwrap_or("")
        )
    };
    let html = gcode_mail::build_feedback_html(
        &kind,
        from_name.as_deref().unwrap_or("کاربر ناشناس"),
        reply_email.as_deref().unwrap_or("—"),
        context.as_deref().unwrap_or("—"),
        &message,
        detail.as_deref().unwrap_or(""),
        &format!(
            "Sahand GCode Studio v{} · ارسال مستقیم SMTP از {}",
            app_version.as_deref().unwrap_or(""),
            cfg.from_name
        ),
    );
    gcode_mail::send_feedback_email(&cfg, &subject, &html, reply_email.as_deref())
}

// ============================================================================
// دستورهای نسخهٔ ۱٫۷ — متن به گلدان (درخواست کاربر ۱۱)
// ============================================================================

/// محاسبهٔ متن‌به‌گلدان برای پیش‌نمایش 3D
#[tauri::command]
pub fn text2vase_compute(
    params: gcode_core::designer::text2vase::TextVaseParams,
) -> gcode_core::ipc::TextVaseViewport {
    gcode_core::ipc::text2vase_compute(params)
}

/// تولید G-code کامل متن‌به‌گلدان — quality: None|۱ = استاندارد
#[tauri::command]
pub fn text2vase_export_gcode(
    params: gcode_core::designer::text2vase::TextVaseParams,
    quality: Option<f64>,
) -> Result<String, String> {
    gcode_core::ipc::text2vase_export_gcode(params, quality)
}
