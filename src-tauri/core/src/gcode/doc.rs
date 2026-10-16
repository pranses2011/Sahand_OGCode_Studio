//! ============================================================================
//! gcode/doc.rs — سند خام G-code: متن، خطوط، BOM و پایان‌خط
//! ============================================================================
//! طبق الزام Docs/PROMPT_02 §3 و §8:
//!   - متن خام هر خط «بایت‌به‌بایت» حفظ می‌شود؛ comment، فاصله، خط خالی،
//!     header/footer و ترتیب خطوط هنگام بارگذاری تغییر نمی‌کنند؛
//!   - BOM و نوع پایان‌خط (LF/CRLF) تشخیص و در Save As بازتولید می‌شوند؛
//!   - ویرایش فقط خط(های) تغییرکرده را جایگزین می‌کند؛
//!   - فایل مبدأ هرگز بازنویسی نمی‌شود (Save As با نام جدید).
//! ============================================================================

use serde::{Deserialize, Serialize};

/// نوع پایان‌خط تشخیص‌داده‌شده در فایل
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEnding {
    /// یونیکس — \n
    Lf,
    /// ویندوز — \r\n
    Crlf,
    /// مخلوط یا نامشخص (مثلاً تک‌خط بدون پایان)؛ خروجی LF انتخاب می‌شود
    Mixed,
}

/// سند G-code — نگه‌دارندهٔ متن خام و خطوط تفکیک‌شده
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeDocument {
    /// آیا فایل BOM UTF-8 داشت
    pub has_bom: bool,
    /// نوع پایان‌خط غالب
    pub line_ending: LineEnding,
    /// خطوط «بدون» پایان‌خط؛ متن خام هر خط دست‌نخورده است
    pub lines: Vec<String>,
    /// مسیر فایل مبدأ (اگر از فایل خوانده شده باشد) — فقط برای نمایش
    pub source_path: Option<String>,
    /// نام فایل مبدأ برای نام پیش‌فرض Save As
    pub source_name: Option<String>,
    /// آیا فایل با پایان‌خط تمام می‌شود (برای بازتولید دقیق)
    /// skip در سریال‌سازی: از متن بازسازی می‌شود و پیام IPC را حجیم نمی‌کند
    #[serde(skip)]
    pub trailing_newline: bool,
}

impl GcodeDocument {
    /// ساخت سند از متن خام کامل فایل
    /// BOM تشخیص داده و حذف می‌شود؛ پایان‌خط‌ها تشخیص داده می‌شوند.
    pub fn from_text(raw: &str, source_path: Option<String>, source_name: Option<String>) -> Self {
        // ---------- BOM ----------
        let has_bom = raw.starts_with('\u{feff}');
        let text = if has_bom { &raw[3..] } else { raw };

        // ---------- تشخیص پایان‌خط ----------
        let lf_count = text.matches('\n').count();
        let crlf_count = text.matches("\r\n").count();
        let line_ending = if lf_count == 0 && !text.contains('\r') {
            LineEnding::Lf // تک‌خط بدون پایان یا خالی
        } else if lf_count == crlf_count {
            LineEnding::Crlf
        } else if crlf_count == 0 {
            LineEnding::Lf
        } else {
            LineEnding::Mixed
        };

        // ---------- تفکیک خطوط ----------
        // split_inclusive نیست چون \r جداگانه هم باید پاک شود؛ متن خام خط
        // «بدون پایان‌خط» نگه داشته می‌شود تا خروجی بایت‌دقیق باشد.
        // حالت خاص: متن کامل خالی → بدون هیچ خطی
        let mut lines: Vec<String> = if text.is_empty() {
            Vec::new()
        } else {
            text.split('\n')
                .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
                .collect()
        };
        // اگر متن با پایان‌خط تمام شده، split یک عنصر خالی آخر می‌سازد که
        // خط واقعی نیست و در سریال‌سازی مجدداً ظاهر می‌شود — حذفش می‌کنیم
        // و پایان‌خط انتهایی را جداگانه نگه می‌داریم (trailing_newline)
        let mut trailing_newline = false;
        if let Some(last) = lines.last() {
            if last.is_empty() && !text.is_empty() {
                trailing_newline = true;
                lines.pop();
            }
        }

        let mut doc = Self {
            has_bom,
            line_ending,
            lines,
            source_path,
            source_name,
            trailing_newline,
        };
        doc
    }

    /// سریال‌سازی مجدد به متن کامل با بازتولید BOM و پایان‌خط
    pub fn to_text(&self) -> String {
        let nl = match self.line_ending {
            LineEnding::Crlf => "\r\n",
            _ => "\n", // Mixed و Lf هردو LF خروجی می‌دهند
        };
        let mut out = String::new();
        if self.has_bom {
            out.push('\u{feff}');
        }
        if self.lines.is_empty() {
            return out;
        }
        out.push_str(&self.lines.join(nl));
        if self.trailing_newline {
            out.push_str(nl);
        }
        out
    }

    /// نام پیش‌فرض Save As طبق الزام: originalname-edited.gcode
    /// اگر نام مبدأ مشخص نیست، نام عمومی
    pub fn default_save_as_name(&self) -> String {
        match &self.source_name {
            Some(name) => {
                // پسوند را جدا و پسوند استاندارد را جایگزین می‌کنیم
                let stem = name
                    .rsplit_once('.')
                    .map(|(s, _)| s.to_string())
                    .unwrap_or_else(|| name.clone());
                format!("{stem}-edited.gcode")
            }
            None => "untitled-edited.gcode".to_string(),
        }
    }

    /// جایگزینی متن یک خط (برای ویرایش) — بقیهٔ خطوط دست‌نخورده می‌مانند
    pub fn replace_line(&mut self, index: usize, new_text: String) {
        if index < self.lines.len() {
            self.lines[index] = new_text;
        }
    }

    /// شمارش خطوط برای نمایش
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

// ============================================================================
// آزمون‌های واحد — شامل موارد سند (CRLF/BOM/حفظ متن)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lf_roundtrip_exact() {
        // متن LF باید بایت‌به‌بایت بازتولید شود
        let raw = "; header\nG21\nG90\n\nG1 X10 Y10 E0.5 F1200\n; footer\n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert_eq!(doc.line_ending, LineEnding::Lf);
        assert_eq!(doc.to_text(), raw);
    }

    #[test]
    fn test_crlf_roundtrip_exact() {
        // متن CRLF باید بایت‌به‌بایت بازتولید شود
        let raw = "; header\r\nG21\r\nG1 X10\r\n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert_eq!(doc.line_ending, LineEnding::Crlf);
        assert_eq!(doc.to_text(), raw);
    }

    #[test]
    fn test_bom_detected_and_preserved() {
        // BOM تشخیص و در خروجی بازتولید می‌شود
        let raw = "\u{feff}G21\nG90\n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert!(doc.has_bom);
        assert_eq!(doc.to_text(), raw);
        // خط اول BOM نداشته باشد (متن تمیز برای parser)
        assert_eq!(doc.lines[0], "G21");
    }

    #[test]
    fn test_mixed_line_endings() {
        // مخلوط LF/CRLF → Mixed و خروجی LF
        let raw = "G21\r\nG90\nG1 X10\n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert_eq!(doc.line_ending, LineEnding::Mixed);
        assert_eq!(doc.to_text(), "G21\nG90\nG1 X10\n");
    }

    #[test]
    fn test_empty_and_blank_lines_preserved() {
        // خطوط خالی و فقط‌فاصله دست‌نخورده می‌مانند (الزام §3)
        let raw = "G21\n\n   \n; comment\nG90\n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert_eq!(doc.lines, vec!["G21", "", "   ", "; comment", "G90"]);
        assert_eq!(doc.to_text(), raw);
    }

    #[test]
    fn test_single_line_no_newline() {
        let doc = GcodeDocument::from_text("G21", None, None);
        assert_eq!(doc.lines, vec!["G21"]);
        assert_eq!(doc.to_text(), "G21");
    }

    #[test]
    fn test_empty_file() {
        let doc = GcodeDocument::from_text("", None, None);
        assert!(doc.lines.is_empty());
        assert_eq!(doc.to_text(), "");
    }

    #[test]
    fn test_replace_line_only_target() {
        // ویرایش فقط خط هدف را عوض می‌کند (الزام Save As §8)
        let raw = "; keep me\nG1 X10 Y10 E0.5\n; keep too\n";
        let mut doc = GcodeDocument::from_text(raw, None, None);
        doc.replace_line(1, "G1 X20 Y20 E0.5".to_string());
        assert_eq!(doc.lines[0], "; keep me");
        assert_eq!(doc.lines[1], "G1 X20 Y20 E0.5");
        assert_eq!(doc.lines[2], "; keep too");
        // بقیهٔ متن دقیق بازتولید می‌شود
        assert_eq!(doc.to_text(), "; keep me\nG1 X20 Y20 E0.5\n; keep too\n");
    }

    #[test]
    fn test_default_save_as_name() {
        // نام پیش‌فرض Save As طبق الزام
        let doc = GcodeDocument::from_text("G21\n", Some("C:/x/y.gcode".into()), Some("y.gcode".into()));
        assert_eq!(doc.default_save_as_name(), "y-edited.gcode");
        // بدون نام مبدأ
        let doc2 = GcodeDocument::from_text("G21\n", None, None);
        assert_eq!(doc2.default_save_as_name(), "untitled-edited.gcode");
    }

    #[test]
    fn test_comment_spaces_untouched() {
        // فاصله‌ها و کامنت‌ها دقیق حفظ می‌شوند
        let raw = "G1   X10    Y-2   ;  spaced comment  \n";
        let doc = GcodeDocument::from_text(raw, None, None);
        assert_eq!(doc.lines[0], "G1   X10    Y-2   ;  spaced comment  ");
        assert_eq!(doc.to_text(), raw);
    }
}
