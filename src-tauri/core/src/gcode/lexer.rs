//! ============================================================================
//! gcode/lexer.rs — لغزندهٔ G-code: شکستن خط خام به واژه‌ها
//! ============================================================================
//! وظیفهٔ این ماژول فقط «تفکیک» است نه «تفسیر»:
//!   متن خام خط  →  واژه‌های مرتب (فرمان/پارامتر) + کامنت + N + checksum
//!
//! چرا regex واحد استفاده نمی‌کنیم؟ (الزام Docs/PROMPT_02 §4)
//! چون این شکل‌های رایج باید همه درست خوانده شوند:
//!   G1 X10.5 Y-2 E0.42 F1200     ← فاصله‌دار
//!   G1X10.5Y-2E0.42F1200          ← فشرده بدون هیچ فاصله
//!   N120 G1 X10 Y20 E1.25*71      ← شمارهٔ خط + checksum
//! لغزندهٔ کاراکتری ما هر سه را پشتیبانی می‌کند و به فاصله حساس نیست.
//!
//! قواعد پیاده‌سازی‌شده:
//!   - کامنت «;...» تا پایان خط؛ کامنت درون‌خطی «(......)»
//!   - N همیشه «شمارهٔ خط» است، هرگز محور حرکتی
//!   - * آخرِ خط checksum است؛ اعتبار با XOR بایت‌های پیش از * سنجیده می‌شود
//!   - payload متنی M117 هرگز به پارامتر حرکتی تبدیل نمی‌شود
//!   - اعداد NaN/Infinity رد می‌شوند تا به هندسه نشت نکنند
//!   - متن خام خط بایت‌به‌بایت حفظ می‌شود؛ لغزنده چیزی «اصلاح» نمی‌کند
//! ============================================================================

use serde::{Deserialize, Serialize};

/// یک واژهٔ G-code: حرف + مقدار عددی یا متنی
/// مثال‌ها:  G1  →  letter='G', number=Some(1.0)
///           X10.5 → letter='X', number=Some(10.5)
///           M117 → letter='M', number=Some(117.0)
///           «Hello» (payload پیام M117) → text=Some(...)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexedWord {
    /// حرف واژه (همیشه بزرگ؛ حروف کوچک نرمال می‌شوند)
    pub letter: char,
    /// مقدار عددی واژه؛ None یعنی بدون مقدار (مثل X در «G28 X» به‌معنی هم‌کردن فقط محور X)
    pub number: Option<f64>,
    /// payload متنی (فقط برای فرمان‌های پیام‌دار مثل M117)
    pub text: Option<String>,
    /// نمای خام واژه برای نمایش صادقانه در inspector حتی وقتی عدد نامعتبر است
    pub raw: String,
}

/// نوع کلی خط پس از لغزنده
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineKind {
    /// خط خالی یا فقط فاصله
    Empty,
    /// فقط کامنت یا نشانگر برنامه (مثل %)
    CommentOrMeta,
    /// دارای واژهٔ فرمان (G/M/T و …)
    Command,
    /// واژهٔ بدشکل دارد (عدد نامعتبر مثل XNaN)
    Malformed,
}

/// نتیجهٔ لغزنده برای یک خط
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexedLine {
    /// متن خام خط «بدون» پایان‌خط — هرگز تغییر نمی‌کند
    pub raw: String,
    /// ایندکس صفرمبنیای خط در سند (نگاشت به segmentها با همین است)
    pub line_index: usize,
    /// شمارهٔ نمایشی خط برای ادیتور (۱مبنا)
    pub file_line_no: usize,
    /// شمارهٔ N اگر خط داشته باشد (الزام: N شمارهٔ خط است نه محور)
    pub n_word: Option<u32>,
    /// مقدار خام checksum بعد از * (اگر موجود)
    pub checksum_raw: Option<String>,
    /// نتیجهٔ اعتبارسنجی XOR: Some(true/false) وقتی checksum وجود دارد
    pub checksum_valid: Option<bool>,
    /// واژه‌ها به ترتیب ظهور در خط
    pub words: Vec<LexedWord>,
    /// متن کامنت (بدون علامت‌ها)
    pub comment: Option<String>,
    /// نوع کلی خط
    pub kind: LineKind,
    /// فهرست واژه‌هایی که عددشان قابل تفسیر نبود (مثلاً XNaN)
    pub malformed_words: Vec<String>,
}

impl LexedLine {
    /// آیا خط فرمان واقعی دارد؟ (برای پرش سریع خطوط خالی/کامنت)
    pub fn is_command(&self) -> bool {
        self.kind == LineKind::Command
    }

    /// یافتن اولین فرمان G با شمارهٔ مشخص — کاربرد Tests و inspector
    pub fn find_g(&self, code: u32) -> bool {
        self.words
            .iter()
            .any(|w| w.letter == 'G' && w.number.map(|n| (n as u64) as u32 == code).unwrap_or(false))
    }
}

/// لغزندهٔ یک خط G-code
/// ورودی باید «بدون پایان‌خط» باشد (doc.rs پایان‌خط را جدا مدیریت می‌کند).
pub fn lex_line(raw_with_maybe_cr: &str, line_index: usize) -> LexedLine {
    // حذف \r باقی‌مانده از CRLF اگر لایهٔ سند هنوز جدا نکرده باشد
    let raw = raw_with_maybe_cr.trim_end_matches('\r');
    let file_line_no = line_index + 1;

    let mut words: Vec<LexedWord> = Vec::new();
    let mut comment: Option<String> = None;
    let mut n_word: Option<u32> = None;
    let mut checksum_raw: Option<String> = None;
    let mut checksum_valid: Option<bool> = None;
    let mut malformed_words: Vec<String> = Vec::new();

    // بایت‌های خط برای پیمایش سریع؛ G-code استاندارد ASCII/UTF-8 است و
    // حروف فرمان همیشه تک‌بایت ASCII هستند. کامنت‌های فارسی هم فقط «متن»اند.
    let bytes = raw.as_bytes();
    let mut pos: usize = 0;
    let len = bytes.len();

    while pos < len {
        let b = bytes[pos];
        // ---- پرش از فاصله‌ها (فشرده/فاصله‌دار بی‌تفاوت است) ----
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        // ---- کامنت ; تا پایان خط ----
        if b == b';' {
            comment = Some(raw[pos + 1..].trim().to_string());
            break; // بعد از کامنت دیگر واژه‌ای نیست
        }
        // ---- کامنت درون‌خطی (...) تا پرانتز بسته ----
        if b == b'(' {
            let close = raw[pos..].find(')').map(|i| pos + i).unwrap_or(len);
            let inner = raw[pos + 1..close.min(len)].trim().to_string();
            // اگر قبلاً کامنت ; نداشته باشیم، متن پرانتز را به کامنت الصاق می‌کنیم؛
            // چند کامنت در یک خط با « | » از هم جدا می‌شوند تا اطلاعاتی از بین نرود.
            comment = Some(match comment {
                Some(prev) => format!("{prev} | {inner}"),
                None => inner,
            });
            pos = (close + 1).min(len);
            continue;
        }
        // ---- checksum: هر چیزی بعد از * تا پایان خط ----
        if b == b'*' {
            checksum_raw = Some(raw[pos + 1..].trim().to_string());
            // اعتبارسنجی XOR بایت‌های پیش از * (روش استاندارد RepRap)
            checksum_valid = Some(validate_checksum(raw, pos));
            break; // checksum همیشه آخر خط است
        }
        // ---- واژه: حرف بزرگ/کوچک + مقدار ----
        if b.is_ascii_alphabetic() {
            let letter = (b as char).to_ascii_uppercase();
            pos += 1;
            // جمع‌کردن کاراکترهای مقدار: رقم، نقطه، علامت
            let val_start = pos;
            while pos < len
                && (bytes[pos].is_ascii_digit()
                    || bytes[pos] == b'.'
                    || bytes[pos] == b'+'
                    || bytes[pos] == b'-')
            {
                pos += 1;
            }
            let val_str = &raw[val_start..pos];

            // ---------- حالت خاص: N شمارهٔ خط ----------
            if letter == 'N' && n_word.is_none() && !val_str.is_empty() {
                match val_str.parse::<u32>() {
                    Ok(n) => {
                        n_word = Some(n);
                        words.push(LexedWord {
                            letter: 'N',
                            number: Some(n as f64),
                            text: None,
                            raw: format!("N{val_str}"),
                        });
                        continue;
                    }
                    Err(_) => {
                        // N غیرعددی؛ به‌عنوان واژهٔ بدشکل ثبت می‌شود نه محور
                        malformed_words.push(format!("N{val_str}"));
                        continue;
                    }
                }
            }

            // ---------- حالت خاص: payload متنی M117 ----------
            // بعد از M117 هرچه می‌آید «پیام» است؛ واژه‌های بعدی هرگز
            // پارامتر حرکتی تفسیر نمی‌شوند (الزام fixture ۱)
            if letter == 'M' && val_str == "117" {
                // ادامهٔ خط تا اولین «;» یا «*» پیام است
                let rest_start = pos;
                let mut rest_end = len;
                for &delim in &[b';', b'*'] {
                    if let Some(i) = raw[rest_start..].find(delim as char) {
                        let abs = rest_start + i;
                        if abs < rest_end {
                            rest_end = abs;
                        }
                    }
                }
                let message = raw[rest_start..rest_end].trim();
                words.push(LexedWord {
                    letter: 'M',
                    number: Some(117.0),
                    text: if message.is_empty() { None } else { Some(message.to_string()) },
                    raw: format!("M117 {message}").trim().to_string(),
                });
                // ادامهٔ پیمایش از انتهای پیام (کامنت/checksum در حلقهٔ اصلی پردازش می‌شود)
                pos = rest_end;
                continue;
            }

            // ---------- مقدار بدشکل الفبایی-عددی (مثل XNaN) ----------
            // اگر بعد از واژه‌ای بدون مقدارِ عددی، رشتهٔ الفبایی-عددیِ چندکاراکتری
            // بیاید (مثل NaN یا inf)، کل آن بخشی از «مقدار بدشکل» است.
            // حرف تکی بعدی واژهٔ جدید است (مثل G28XY) و عدد معتبر + حرف بعدی هم
            // فرمت فشردهٔ واژهٔ بعدی است (مثل G1X10Y20).
            let next_is_alpha = pos < len && bytes[pos].is_ascii_alphabetic();
            if next_is_alpha && val_str.is_empty() {
                let mut run_end = pos;
                while run_end < len && bytes[run_end].is_ascii_alphanumeric() {
                    run_end += 1;
                }
                if run_end - pos > 1 {
                    pos = run_end;
                    let bad_raw = raw[val_start..pos].to_string();
                    malformed_words.push(format!("{letter}{bad_raw}"));
                    words.push(LexedWord {
                        letter,
                        number: None,
                        text: Some(bad_raw.clone()),
                        raw: format!("{letter}{bad_raw}"),
                    });
                    continue;
                }
                // حرف تکی: واژهٔ جدید در چرخهٔ اصلی پردازش می‌شود
            }

            // ---------- واژهٔ عددی معمولی ----------
            if val_str.is_empty() {
                // حرف بدون مقدار (مثل X در «G28 X») — مجاز؛ interpreter معنا می‌دهد
                words.push(LexedWord {
                    letter,
                    number: None,
                    text: None,
                    raw: letter.to_string(),
                });
            } else {
                match parse_number(val_str) {
                    Ok(Some(num)) => words.push(LexedWord {
                        letter,
                        number: Some(num),
                        text: None,
                        raw: format!("{letter}{val_str}"),
                    }),
                    Ok(None) | Err(_) => {
                        // عدد نامعتبر (XNaN و نظایر) — متن خام حفظ، خطای بعداً
                        malformed_words.push(format!("{letter}{val_str}"));
                        // واژه را با عدد None ثبت می‌کنیم تا inspector نمایشش دهد
                        words.push(LexedWord {
                            letter,
                            number: None,
                            text: Some(val_str.to_string()),
                            raw: format!("{letter}{val_str}"),
                        });
                    }
                }
            }
            continue;
        }
        // کاراکتر ناشناخته (مثل % یا # در سینتکس ماکرو) → رد می‌شود؛
        // علامت‌گذاری آن با interpreter است که firmware-dependent را گزارش کند
        pos += 1;
    }

    // تعیین نوع خط
    let kind = if !malformed_words.is_empty() {
        LineKind::Malformed
    } else if words.is_empty() {
        if comment.is_some() || raw.trim() == "%" {
            LineKind::CommentOrMeta
        } else {
            LineKind::Empty
        }
    } else {
        LineKind::Command
    };

    LexedLine {
        raw: raw.to_string(),
        line_index,
        file_line_no,
        n_word,
        checksum_raw,
        checksum_valid,
        words,
        comment,
        kind,
        malformed_words,
    }
}

/// تبدیل رشتهٔ عددی G-code به f64 با ردّ فعال NaN/Infinity
/// خروجی Ok(None) یعنی «رشتهٔ عددی نیست ولی بدشکل هم نیست» (مثل رشتهٔ خالی)
/// خروجی Err یعنی «شکل عددی دارد ولی مقدارش نامعتبر است» (NaN/inf)
fn parse_number(s: &str) -> Result<Option<f64>, ()> {
    if s.is_empty() {
        return Ok(None);
    }
    // ردّ صریح شناسه‌های خاص قبل از parse (f64 در Rust «NaN» را می‌پذیرد!)
    let lower = s.to_ascii_lowercase();
    if lower.contains("nan") || lower.contains("inf") {
        return Err(());
    }
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(Some(v)),
        Ok(_) => Err(()), // عدد پردازش‌شد ولی NaN/inf شد → رد
        Err(_) => Err(()), // اصلاً عدد نیست → واژهٔ بدشکل
    }
}

/// اعتبارسنجی checksum به روش استاندارد RepRap:
/// XOR همهٔ بایت‌های خط «قبل از *» باید برابر عدد checksum باشد.
/// ورودی: متن کامل خط و موقعیت کاراکتر *
fn validate_checksum(raw: &str, star_pos: usize) -> bool {
    let given = raw[star_pos + 1..].trim();
    // checksum ممکن است ده‌دهی یا مبنای ۱۶ نوشته شود؛ هر دو را می‌پذیریم
    let given_val = if let Ok(d) = given.parse::<u32>() {
        Some(d)
    } else if let Ok(h) = u32::from_str_radix(given.trim_start_matches("0x"), 16) {
        Some(h)
    } else {
        None
    };
    let Some(given_val) = given_val else {
        return false; // قالب checksum اصلاً عددی نیست
    };
    // XOR بایت‌های پیش از *
    let mut xor: u32 = 0;
    for &b in &raw.as_bytes()[..star_pos] {
        xor ^= b as u32;
    }
    xor == given_val
}

/// محاسبهٔ checksum درست برای یک خط (برای سیاست Save As با بازمحاسبه)
/// متن ورودی نباید خودش * داشته باشد
pub fn compute_checksum(line_without_star: &str) -> u8 {
    let mut xor: u8 = 0;
    for &b in line_without_star.as_bytes() {
        xor ^= b;
    }
    xor
}

// ============================================================================
// آزمون‌های واحد لغزنده — مطابق fixtureهای Docs/TEST_GCODE_FIXTURES.md
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spaced_format() {
        // فرمت فاصله‌دار رایج
        let l = lex_line("G1 X10.5 Y-2 E0.42 F1200", 0);
        assert_eq!(l.kind, LineKind::Command);
        assert_eq!(l.words.len(), 5);
        assert_eq!(l.words[0].letter, 'G');
        assert_eq!(l.words[0].number, Some(1.0));
        assert_eq!(l.words[1].number, Some(10.5));
        assert_eq!(l.words[2].number, Some(-2.0));
        assert_eq!(l.words[3].number, Some(0.42));
        assert_eq!(l.words[4].number, Some(1200.0));
    }

    #[test]
    fn test_compact_format() {
        // فرمت فشردهٔ بدون فاصله — نباید با فرمت فاصله‌دار تفاوت معنایی داشته باشد
        let l = lex_line("G1X10.5Y-2E0.42F1200", 5);
        assert_eq!(l.kind, LineKind::Command);
        assert_eq!(l.words.len(), 5);
        assert_eq!(l.words[1].number, Some(10.5));
        assert_eq!(l.words[2].number, Some(-2.0));
    }

    #[test]
    fn test_n_and_checksum() {
        // N=شمارهٔ خط (نه محور) + checksum XOR معتبر
        let l = lex_line("N1 G21*27", 0);
        assert_eq!(l.n_word, Some(1));
        assert_eq!(l.checksum_raw.as_deref(), Some("27"));
        assert_eq!(l.checksum_valid, Some(true));
        // N هرگز در واژه‌های محور نیست
        assert!(!l.words.iter().any(|w| w.letter == 'N' && w.number.is_none()));
    }

    #[test]
    fn test_checksum_invalid_detected() {
        // دست‌کاری checksum باید تشخیص داده شود
        let l = lex_line("N1 G21*99", 0);
        assert_eq!(l.checksum_valid, Some(false));
    }

    #[test]
    fn test_comment_not_executed() {
        // کامنت شبیه‌فرمان هرگز فرمان تولید نمی‌کند (fixture ۴)
        let l = lex_line("; G1 X999 Y999 E999 -- this is only a comment", 0);
        assert_eq!(l.kind, LineKind::CommentOrMeta);
        assert!(l.words.is_empty());
        assert!(l.comment.is_some());
    }

    #[test]
    fn test_paren_comment() {
        let l = lex_line("G1 X10 (slow move) Y20", 0);
        assert_eq!(l.words.len(), 3);
        assert_eq!(l.comment.as_deref(), Some("slow move"));
        assert_eq!(l.words[2].number, Some(20.0));
    }

    #[test]
    fn test_m117_payload_is_text() {
        // payload متنی M117 نباید پارامتر شود (fixture ۱ و ۵)
        let l = lex_line("M117 Hello printer", 0);
        assert_eq!(l.words.len(), 1);
        assert_eq!(l.words[0].text.as_deref(), Some("Hello printer"));
        let l2 = lex_line("M117 Unknown values: X999 E999", 0);
        assert_eq!(l2.words.len(), 1);
        assert_eq!(l2.words[0].text.as_deref(), Some("Unknown values: X999 E999"));
    }

    #[test]
    fn test_m117_with_comment() {
        let l = lex_line("M117 Hello printer ; note", 0);
        assert_eq!(l.words[0].text.as_deref(), Some("Hello printer"));
        assert_eq!(l.comment.as_deref(), Some("note"));
    }

    #[test]
    fn test_nan_rejected() {
        // XNaN باید واژهٔ بدشکل شناخته شود نه عدد (fixture ۵)
        let l = lex_line("G1 XNaN Y6 E0.3 F900", 0);
        assert_eq!(l.kind, LineKind::Malformed);
        assert!(l.malformed_words.contains(&"XNaN".to_string()));
        // بقیهٔ واژه‌ها سالم‌اند و متن خام حفظ شده است
        assert_eq!(l.raw, "G1 XNaN Y6 E0.3 F900");
        assert!(l.words.iter().any(|w| w.letter == 'Y' && w.number == Some(6.0)));
    }

    #[test]
    fn test_empty_and_lowercase() {
        assert_eq!(lex_line("", 0).kind, LineKind::Empty);
        assert_eq!(lex_line("   ", 1).kind, LineKind::Empty);
        // حروف کوچک نرمال می‌شوند
        let l = lex_line("g1 x10", 0);
        assert_eq!(l.words[0].letter, 'G');
        assert_eq!(l.words[1].letter, 'X');
    }

    #[test]
    fn test_compute_checksum_roundtrip() {
        // سازندهٔ checksum با اعتبارسنج هم‌خوان باید باشد
        let line = "N1 G21";
        let cs = compute_checksum(line);
        let full = format!("{line}*{}", cs as u32);
        let lexed = lex_line(&full, 0);
        assert_eq!(lexed.checksum_valid, Some(true));
    }

    #[test]
    fn test_fixture_checksums() {
        // هر چهار خط checksum دارِ fixture ۴ معتبرند
        for raw in ["N1 G21*27", "N2 G90*18", "N3 M82*26", "N4 G1 X10 Y10 E0.5 F1200*6"] {
            let l = lex_line(raw, 0);
            assert_eq!(l.checksum_valid, Some(true), "failed for {raw}");
        }
    }

    #[test]
    fn test_word_without_value() {
        // «G28 X» → X بدون مقدار مجاز است (هم‌کردن فقط محور X)
        let l = lex_line("G28 X", 0);
        assert_eq!(l.words.len(), 2);
        assert_eq!(l.words[1].letter, 'X');
        assert!(l.words[1].number.is_none());
        // واژهٔ بدون مقدار بدشکل نیست
        assert_eq!(l.kind, LineKind::Command);
    }
}
