//! ============================================================================
//! diagnostics.rs — زیرساخت خطاها، هشدارها و عدم‌قطعیت‌ها
//! ============================================================================
//! چرا این ماژول اول نوشته شده است؟
//! طبق اسناد الزام (Docs/PROMPT_02 §4 و PROMPT_04 §4) هر لایهٔ برنامه باید
//! بتواند سه چیز را از هم تفکیک کند:
//!   ۱) «خطا» (Error): parser نتوانسته خط را تفسیر کند؛
//!   ۲) «هشدار» (Warning): تفسیر شد ولی نتیجه مشکوک/محدود است؛
//!   ۳) «عدم‌قطعیت» (uncertain): اثر فرمان به firmware/پروفایل وابسته است و
//!      مدل ما ادعای قطعیت ندارد.
//! ساختار Diagnostic عمداً ساده و سریال‌شونده است تا مستقیم از IPC به UI
//! منتقل شود و در جدول مشکلات فایل نمایش یابد.
//! ============================================================================

use serde::{Deserialize, Serialize};

/// شدت پیام تشخیصی؛ ترتیب اهمیت: خطا > هشدار > اطلاعات
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// خطای قطعی: خط قابل تفسیر نیست (مثلاً عدد NaN یا قوس نامعتبر)
    Error,
    /// هشدار: تفسیر انجام شد ولی نتیجه باید با احتیاط دیده شود
    Warning,
    /// اطلاعات: نکتهٔ غیرخطا برای آگاهی کاربر (مثلاً فرمان ناشناخته)
    Info,
}

/// یک پیام تشخیصی کامل با موقعیت خط
/// `line_index` ایندکس صفرمبنیای خط در آرایهٔ خطوط سند است (نه شمارهٔ N گفته‌کد
/// و نه شمارهٔ نمایشی ادیتور؛ UI خودش نگاشت می‌کند) — None یعنی کل فایل.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// کد پایدار برای آزمون خودکار و فیلتر UI (مثلاً "E-NAN-VALUE")
    pub code: String,
    /// پیام فارسیِ قابل‌فهم برای کاربر
    pub message_fa: String,
    /// ایندکس خط مبدأ (صفرمبنیا) یا None برای مشکل کل‌فایل
    pub line_index: Option<usize>,
    /// آیا این پیام ذاتاً «عدم‌قطعیت» است (وابسته به firmware/پروفایل)
    pub uncertain: bool,
}

impl Diagnostic {
    /// ساخت میان‌بر پیام خطا
    pub fn error(code: &str, message_fa: impl Into<String>, line_index: usize) -> Self {
        Self {
            severity: Severity::Error,
            code: code.to_string(),
            message_fa: message_fa.into(),
            line_index: Some(line_index),
            uncertain: false,
        }
    }

    /// ساخت میان‌بر پیام هشدار
    pub fn warning(code: &str, message_fa: impl Into<String>, line_index: usize) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.to_string(),
            message_fa: message_fa.into(),
            line_index: Some(line_index),
            uncertain: false,
        }
    }

    /// ساخت میان‌بر پیام «نیازمند تفسیر firmware» (عدم‌قطعیت)
    /// طبق الزام: فرمان ناشناخته/ماکرو معنی حدسی نمی‌سازد و متن خام حفظ می‌شود
    pub fn firmware(code: &str, message_fa: impl Into<String>, line_index: usize) -> Self {
        Self {
            severity: Severity::Info,
            code: code.to_string(),
            message_fa: message_fa.into(),
            line_index: Some(line_index),
            uncertain: true,
        }
    }

    /// پیام بدون موقعیت خط (مثلاً مشکل BOM یا پایان‌خط ناهمگون)
    pub fn file_level(severity: Severity, code: &str, message_fa: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.to_string(),
            message_fa: message_fa.into(),
            line_index: None,
            uncertain: false,
        }
    }
}

/// آزمون‌های واحد کوچک برای تضمین سازگاری سازنده‌ها
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_constructors() {
        // سازندهٔ خطا باید ایندکس خط را ذخیره کند و uncertain=false بماند
        let d = Diagnostic::error("E-TEST", "خطای آزمایشی", 7);
        assert_eq!(d.line_index, Some(7));
        assert!(!d.uncertain);
        assert_eq!(d.severity, Severity::Error);

        // پیام firmware ذاتاً uncertain است
        let f = Diagnostic::firmware("I-UNK", "فرمان ناشناخته", 3);
        assert!(f.uncertain);

        // پیام سطح فایل بدون ایندکس است
        let g = Diagnostic::file_level(Severity::Warning, "W-BOM", "هشدار BOM");
        assert_eq!(g.line_index, None);
    }
}
