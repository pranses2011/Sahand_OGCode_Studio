//! ============================================================================
//! gcode-mail — ارسال ایمیل SMTP مستقیم (نسخهٔ ۱٫۷ — رفع درخواست کاربر ۱۰)
//! ============================================================================
//! مشکل نسخهٔ ۱٫۶:
//!   فرم بازخورد از سرویس formsubmit.co استفاده می‌کرد؛ آن سرویس
//!   «پاسخ موفق» برمی‌گرداند اما تا زمانی که مالک ایمیل روی لینک
//!   «فعال‌سازی» یک‌باره کلیک نکرده باشد، هیچ ایمیلی واقعاً تحویل
//!   داده نمی‌شود ⇒ برنامه می‌گفت «ارسال شد» ولی به صندوق نمی‌آمد.
//!
//! راه‌حل قطعی این نسخه:
//!   ارسال مستقیم SMTP از خود برنامه (بدون سرویس واسط):
//!     * سازنده یک‌بار تنظیمات SMTP (مثلاً Gmail + App Password) را در
//!       «دربارهٔ برنامه ← تنظیمات ایمیل» وارد می‌کند؛
//!     * همهٔ بازخوردها/گزارش‌های خطا مستقیم به صندوق او می‌رسد؛
//!     * formsubmit به‌عنوان پشتیبان (وقتی SMTP تنظیم نشده) می‌ماند
//!       و وضعیت هر کانال صادقانه گزارش می‌شود.
//!
//! معماری:
//!   * SmtpConfig سریال‌شونده با اعتبارسنجی و ذخیره/بارگذاری در مسیر
//!     دادهٔ برنامه (توابع خالص — پوسته فقط مسیر app_data می‌دهد)؛
//!   * send_feedback_email: ساخت پیام HTML فارسی (RTL) و ارسال با
//!     TLS (پورت ۴۶۵ ضمنی / ۵۸۷ STARTTLS) — بدون I/O اضافه.
//!
//! امنیت:
//!   * رمز SMTP فقط در فایل محلی app_data/smtp.json می‌ماند (مثل هر
//!     نرم‌افزار ایمیل دسکتاپ)؛ هرگز به فایل خروجی/لاگ کپی نمی‌شود؛
//!   * پشتیبان formsubmit فقط متن پاک‌شدهٔ محدود را می‌فرستد.
//! ============================================================================

use serde::{Deserialize, Serialize};
use std::path::Path;

/// تنظیمات SMTP — یک‌بار توسط سازنده در UI وارد و ذخیره می‌شود
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// میزبان SMTP — مثلاً smtp.gmail.com
    pub host: String,
    /// پورت — ۴۶۵ (TLS ضمنی) یا ۵۸۷ (STARTTLS)
    pub port: u16,
    /// نام کاربری — معمولاً آدرس کامل ایمیل فرستنده
    pub username: String,
    /// رمز/App Password — فقط محلی ذخیره می‌شود
    pub password: String,
    /// نام نمایشی فرستنده
    #[serde(default = "default_from_name")]
    pub from_name: String,
    /// گیرنده — پیش‌فرض ایمیل سازنده (سعید نجفی)
    #[serde(default = "default_to_email")]
    pub to_email: String,
}

fn default_from_name() -> String {
    "Sahand GCode Studio".into()
}

fn default_to_email() -> String {
    "Pranses2011@GMail.com".into()
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "smtp.gmail.com".into(),
            port: 465,
            username: String::new(),
            password: String::new(),
            from_name: default_from_name(),
            to_email: default_to_email(),
        }
    }
}

impl SmtpConfig {
    /// آیا تنظیمات قابل‌استفاده است؟ (میزبان و کاربر و رمز پر شده باشند)
    pub fn is_ready(&self) -> bool {
        !self.host.trim().is_empty()
            && !self.username.trim().is_empty()
            && !self.password.is_empty()
            && self.port > 0
    }

    /// اعتبارسنجی — پیام خطای فارسی برای UI
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty() {
            return Err("میزبان SMTP خالی است (مثلاً smtp.gmail.com)".into());
        }
        if self.port == 0 {
            return Err("پورت SMTP نامعتبر است (۴۶۵ یا ۵۸۷)".into());
        }
        if !self.username.contains('@') {
            return Err("نام کاربری SMTP باید آدرس ایمیل کامل باشد".into());
        }
        if self.password.len() < 4 {
            return Err("رمز SMTP خیلی کوتاه است — برای Gmail از App Password ۱۶ رقمی استفاده کنید".into());
        }
        if !self.to_email.contains('@') {
            return Err("ایمیل گیرنده نامعتبر است".into());
        }
        Ok(())
    }

    /// نسخهٔ بی‌خطر برای نمایش در UI — رمز حذف می‌شود
    pub fn masked(&self) -> SmtpConfigView {
        SmtpConfigView {
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            from_name: self.from_name.clone(),
            to_email: self.to_email.clone(),
            has_password: !self.password.is_empty(),
            ready: self.is_ready(),
        }
    }
}

/// نمای بی‌رمز برای بازگرداندن به UI (رمز هرگز به WebView نمی‌رسد)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfigView {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub from_name: String,
    pub to_email: String,
    pub has_password: bool,
    pub ready: bool,
}

// ============================================================================
// ذخیره/بارگذاری پیکربندی — توابع خالص روی مسیر (پوسته مسیر app_data می‌دهد)
// ============================================================================

const CONFIG_FILE: &str = "smtp.json";

/// مسیر فایل پیکربندی در مسیر پایه
pub fn config_path(base_dir: &str) -> std::path::PathBuf {
    Path::new(base_dir).join(CONFIG_FILE)
}

/// ذخیرهٔ پیکربندی — JSON با replace قدیمی
pub fn save_config(base_dir: &str, cfg: &SmtpConfig) -> Result<(), String> {
    cfg.validate()?;
    let dir = Path::new(base_dir);
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("ساخت پوشهٔ تنظیمات ناموفق بود: {e}"))?;
    let path = config_path(base_dir);
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("سریال‌سازی تنظیمات ناموفق بود: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("نوشتن فایل تنظیمات ناموفق بود: {e}"))?;
    Ok(())
}

/// بارگذاری پیکربندی — نبود فایل = پیش‌فرض خالی (نه خطا)
pub fn load_config(base_dir: &str) -> Result<SmtpConfig, String> {
    let path = config_path(base_dir);
    if !path.exists() {
        return Ok(SmtpConfig::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("خواندن فایل تنظیمات ناموفق بود: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("فایل تنظیمات خراب است: {e}"))
}

/// حذف کامل پیکربندی
pub fn clear_config(base_dir: &str) -> Result<(), String> {
    let path = config_path(base_dir);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("حذف تنظیمات ناموفق بود: {e}"))?;
    }
    Ok(())
}

// ============================================================================
// ساخت پیام HTML فارسی (RTL) — یکسان برای بازخورد و گزارش خطا
// ============================================================================

/// فیلد یک ردیف جدول HTML
fn td(label: &str, value: &str) -> String {
    format!(
        "<tr><td style=\"padding:6px 10px;background:#f1f3f6;border:1px solid #dde;font-family:Tahoma,sans-serif\">{label}</td><td style=\"padding:6px 10px;border:1px solid #dde;font-family:Tahoma,sans-serif\">{value}</td></tr>"
    )
}

/// فرار کاراکترهای HTML در متن کاربر
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// ساخت HTML ایمیل فارسی (RTL) از فیلدهای بازخورد
pub fn build_feedback_html(kind: &str, from: &str, reply: &str, context: &str, message: &str, detail: &str, env: &str) -> String {
    let title = if kind == "error" {
        "🛠 گزارش خودکار خطا"
    } else {
        "💬 بازخورد کاربر"
    };
    let detail_row = if detail.trim().is_empty() {
        String::new()
    } else {
        format!(
            "{}",
            td("جزئیات فنی", &format!("<pre style=\"white-space:pre-wrap;margin:0;font-size:12px\">{}</pre>", esc(detail)))
        )
    };
    format!(
        "<div dir=\"rtl\" style=\"font-family:Tahoma,Vazirmatn,sans-serif;max-width:640px;margin:auto\">\
<h2 style=\"color:#b3541e\">{title} — Sahand GCode Studio</h2>\
<table style=\"border-collapse:collapse\">\
{}{}{}{}{}\
</table>\
<h3 style=\"margin-bottom:4px\">پیام</h3>\
<div style=\"background:#faf7f2;border:1px solid #eeddc8;border-radius:8px;padding:12px 14px;white-space:pre-wrap\">{}</div>\
<p style=\"color:#888;font-size:12px;margin-top:16px\">{}</p>\
</div>",
        td("نوع", if kind == "error" { "گزارش خطای خودکار" } else { "بازخورد کاربر" }),
        td("از", &esc(from)),
        td("ایمیل پاسخ", &esc(reply)),
        td("بخش برنامه", &esc(context)),
        detail_row,
        esc(message),
        esc(env),
    )
}

// ============================================================================
// ارسال SMTP — TLS ضمنی (۴۶۵) یا STARTTLS (۵۸۷)
// ============================================================================

/// نتیجهٔ ارسال — برای گزارش صادقانه به UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailSendReport {
    pub ok: bool,
    /// کانالی که واقعاً استفاده شد: smtp
    pub channel: String,
    /// پیام فارسی وضعیت
    pub message: String,
}

/// ارسال ایمیل بازخورد/گزارش خطا با تنظیمات ذخیره‌شده
///
/// `reply_to` اختیاری — اگر کاربر ایمیل پاسخ داده باشد پاسخ‌ها مستقیم به او می‌رود.
pub fn send_feedback_email(
    cfg: &SmtpConfig,
    subject: &str,
    body_html: &str,
    reply_to: Option<&str>,
) -> Result<MailSendReport, String> {
    cfg.validate()?;
    let report = send_email_inner(cfg, subject, body_html, reply_to)?;
    Ok(report)
}

/// ارسال ایمیل آزمایشی — دکمهٔ «تست» در UI تنظیمات
pub fn send_test_email(cfg: &SmtpConfig) -> Result<MailSendReport, String> {
    let html = "<div dir=\"rtl\" style=\"font-family:Tahoma,sans-serif\">\
<h3 style=\"color:#2e7d32\">✅ تنظیمات ایمیل Sahand GCode Studio درست کار می‌کند</h3>\
<p>این ایمیل آزمایشی است — از این پس همهٔ بازخوردها و گزارش‌های خطای کاربران مستقیماً به همین صندوق می‌رسد.</p></div>";
    let subject = "✅ تست موفق — Sahand GCode Studio";
    send_email_inner(cfg, subject, html, None)
}

fn send_email_inner(
    cfg: &SmtpConfig,
    subject: &str,
    body_html: &str,
    reply_to: Option<&str>,
) -> Result<MailSendReport, String> {
    use lettre::{
        message::{header::ContentType, Mailbox},
        transport::smtp::authentication::Credentials,
        Message, SmtpTransport, Transport,
    };

    // ----- ساخت پیام -----
    let from: Mailbox = format!("{} <{}>", cfg.from_name, cfg.username)
        .parse()
        .map_err(|_| "آدرس فرستنده نامعتبر است".to_string())?;
    let to: Mailbox = cfg
        .to_email
        .parse()
        .map_err(|_| "آدرس گیرنده نامعتبر است".to_string())?;
    let mut builder = Message::builder().from(from).to(to).subject(subject);
    if let Some(rt) = reply_to {
        if let Ok(mb) = rt.parse::<Mailbox>() {
            builder = builder.reply_to(mb);
        }
    }
    let email = builder
        .header(ContentType::TEXT_HTML)
        .body(body_html.to_string())
        .map_err(|e| format!("ساخت پیام ناموفق بود: {e}"))?;

    // ----- اتصال با TLS -----
    let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());
    let builder = if cfg.port == 465 {
        // TLS ضمنی (Gmail پیش‌فرض امن)
        SmtpTransport::relay(&cfg.host)
            .map_err(|e| format!("میزبان SMTP در دسترس نیست: {e}"))?
    } else {
        // STARTTLS — پورت ۵۸۷ و بیشتر سرویس‌ها
        SmtpTransport::starttls_relay(&cfg.host)
            .map_err(|e| format!("میزبان SMTP در دسترس نیست: {e}"))?
    };
    let mailer = builder
        .port(cfg.port)
        .credentials(creds)
        .timeout(Some(std::time::Duration::from_secs(45)))
        .build();

    // ----- ارسال -----
    mailer
        .send(&email)
        .map_err(|e| format!("ارسال SMTP ناموفق بود: {e}"))?;

    Ok(MailSendReport {
        ok: true,
        channel: "smtp".into(),
        message: "ایمیل مستقیماً از سرور شما ارسال و تحویل داده شد".into(),
    })
}

// ============================================================================
// آزمون‌ها — بدون شبکه: فقط پیکربندی/اعتبارسنجی/HTML
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let cfg = SmtpConfig {
            host: "smtp.gmail.com".into(),
            port: 465,
            username: "someone@gmail.com".into(),
            password: "abcd efgh ijkl mnop".into(),
            from_name: "Sahand GCode Studio".into(),
            to_email: "Pranses2011@GMail.com".into(),
        };
        let dir = std::env::temp_dir().join("gcode-mail-test-rt");
        let base = dir.to_string_lossy().to_string();
        save_config(&base, &cfg).expect("ذخیره باید موفق باشد");
        let back = load_config(&base).expect("بارگذاری باید موفق باشد");
        assert_eq!(back.host, cfg.host);
        assert_eq!(back.port, cfg.port);
        assert_eq!(back.username, cfg.username);
        assert_eq!(back.password, cfg.password);
        assert_eq!(back.to_email, cfg.to_email);
        let _ = clear_config(&base);
        assert!(load_config(&base).unwrap().password.is_empty(), "بعد از حذف، خالی");
    }

    #[test]
    fn test_config_validation() {
        let mut cfg = SmtpConfig::default();
        assert!(!cfg.is_ready(), "پیش‌فرض بدون کاربر/رمز آماده نیست");
        assert!(cfg.validate().is_err(), "اعتبارسنجی باید خطا بدهد");
        cfg.username = "a@gmail.com".into();
        cfg.password = "1234567890123456".into();
        assert!(cfg.validate().is_ok());
        assert!(cfg.is_ready());
        // پورت صفر
        cfg.port = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_masked_has_no_password() {
        let cfg = SmtpConfig {
            password: "SECRET".into(),
            ..SmtpConfig::default()
        };
        let m = cfg.masked();
        // نمای سریال‌شده نباید رمز یا رشتهٔ محرمانه داشته باشد
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("SECRET"), "رمز نباید در نمای UI باشد");
        assert!(m.has_password);
    }

    #[test]
    fn test_feedback_html_escaping() {
        let html = build_feedback_html(
            "feedback",
            "<x>",
            "a@b.c",
            "فرم",
            "<script>alert(1)</script>",
            "",
            "test env",
        );
        assert!(html.contains("&lt;script&gt;"), "تگ HTML باید فرار کند");
        assert!(!html.contains("<script>alert"), "هیچ تگ اجراشونده‌ای نباید بماند");
        assert!(html.contains("dir=\"rtl\""), "ایمیل فارسی RTL است");
    }

    #[test]
    fn test_old_config_json_loads_with_defaults() {
        // فایل قدیمی بدون from_name/to_email → پیش‌فرض سازنده
        let raw = r#"{"host":"smtp.gmail.com","port":465,"username":"u@gmail.com","password":"1234567890123456"}"#;
        let cfg: SmtpConfig = serde_json::from_str(raw).expect("باید بخواند");
        assert_eq!(cfg.from_name, "Sahand GCode Studio");
        assert_eq!(cfg.to_email, "Pranses2011@GMail.com");
        assert!(cfg.is_ready());
    }
}
