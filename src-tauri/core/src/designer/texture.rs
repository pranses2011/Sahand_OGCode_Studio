//! ============================================================================
//! designer/texture.rs — گام ۳: بافت‌های مسیر نازل (فقط در G-code)
//! ============================================================================
//! تمایز حیاتی با pattern (تأیید از پژوهش مرجع Docs/REFERENCE_AUDIT.md):
//!   «بافت مسیر نازل را تغییر می‌دهد و در STL ذخیره نمی‌شود؛
//!    G-code می‌تواند هر دو را حمل کند.»
//!
//! بافت «پس از» ساخت مسیر مارپیچی روی نقاط اعمال می‌شود:
//!   p' = modulate(p, texture)
//! و فقط روی Toolpath IR اثر می‌گذارد — mesh/STL دست‌نخورده می‌ماند.
//!
//! الزام: «بافت را روی مسیر مارپیچی اعمال کن، نه فقط روی shader» —
//! همهٔ بافت‌های این ماژول مختصات واقعی نقاط مسیر را تعدیل می‌کنند.
//! ============================================================================

use serde::{Deserialize, Serialize};

/// انواع بافت مسیر
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureKind {
    /// موجک ریپل/شیار عرضی: جابه‌جایی شعاعی دوره‌ای روی طول مسیر
    Ripple,
    /// تور بافته mesh/weave: الگوی بالا-پایین متناوب بین خطوط مجاور
    Weave,
    /// بخیه‌ای stitch: پرش‌های گسستهٔ شعاعی مثل دوخت
    Stitch,
    /// شیار مارپیچی: کانال پیوسته که دور دیواره می‌پیچد (زاویه + طول مسیر)
    SpiralGroove,
    /// چکشی: فرورفتگی‌های شبه‌تصادفی اما قطعی (hash کمی زاویه/ارتفاع)
    Hammered,
    /// سبدی: شبکهٔ متقاطع سینوسی دو جهته (طول مسیر × زاویه)
    Basket,
    /// رگهٔ چوب (wood grain): نویز مقداری نرم قطعی + رگه‌های ظریف زاویه‌ای
    WoodGrain,
    /// چرم (leather): نویز دوجهتی ریز دو-اکتاوی (بافت پوست/چرم)
    Leather,
    /// آج رزوه (knurl): شیارهای متقاطع V شکل مثل دستگیرهٔ پیچ
    Knurl,
    /// موج دریا (ocean): ترکیب دو سینوس با فرکانس نامتناسب (بی‌تکراری)
    Ocean,
    /// حباب (bubbles): فرورفتگی‌های گرد پراکنده قطعی (کف حباب‌دار)
    Bubbles,
    /// تخته‌ای (rows): شیارهای افقی قوسی مثل ردیف تخته‌ها
    Rows,
    // ---------- نسخهٔ ۱٫۷ (درخواست کاربر ۶: بافت‌های بیشتر) ----------
    /// کربنی: بافت تویل ریز متقاطع (پنل کربن)
    Carbon,
    /// ست‌فقرات ماهی (herringbone): ستون‌های زیگزاگ متناوب جهت
    Herringbone,
    /// سنگ‌فرش: سنگ‌های گرد درهم (کف سنگی)
    Cobblestone,
    /// کراکله: شبکهٔ ترک‌های ریز (سرامیک ترک‌خورده)
    Crackle,
    /// فلس ماهی: کاسه‌های هم‌پوشان با آفست ردیفی
    FishScale,
    /// شنی: نویز ریز دو-اکتاوی ملایم (سطح شن)
    Sand,
    /// شیار عمودی ریز: رگه‌های عمودی ظریف (چین‌های پارچه)
    Ridges,
    /// منافذ اسفنجی: گودی‌های ریز پراکنده (اسفنج/متخلخل)
    Pores,
}

impl TextureKind {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            TextureKind::Ripple => "موجک (ریپل)",
            TextureKind::Weave => "تور بافته",
            TextureKind::Stitch => "بخیه‌ای",
            TextureKind::SpiralGroove => "شیار مارپیچی",
            TextureKind::Hammered => "چکشی",
            TextureKind::Basket => "سبدی",
            TextureKind::WoodGrain => "رگهٔ چوب",
            TextureKind::Leather => "چرم",
            TextureKind::Knurl => "آج رزوه",
            TextureKind::Ocean => "موج دریا",
            TextureKind::Bubbles => "حباب",
            TextureKind::Rows => "تخته‌ای",
            TextureKind::Carbon => "کربنی (تویل)",
            TextureKind::Herringbone => "ست‌فقرات ماهی",
            TextureKind::Cobblestone => "سنگ‌فرش",
            TextureKind::Crackle => "کراکله (ترک‌خورده)",
            TextureKind::FishScale => "فلس ماهی",
            TextureKind::Sand => "شنی",
            TextureKind::Ridges => "شیار عمودی ریز",
            TextureKind::Pores => "منافذ اسفنجی",
        }
    }
}

/// پارامترهای یک بافت فعال
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureParams {
    pub kind: TextureKind,
    /// دامنهٔ جابه‌جایی شعاعی (mm)
    pub amplitude_mm: f64,
    /// گام/تراکم الگو (mm طول مسیر در هر دوره)
    pub period_mm: f64,
    /// جهت: true = شعاعی (r)، false = عمودی (z)
    pub radial: bool,
    /// فاز اولیه (درجه)
    pub phase_deg: f64,
    /// محوشدن تدریجی از پایین (۰..۱ از ارتفاع)
    pub fade_in_percent: f64,
    /// محوشدن تدریجی به بالا (۰..۱ از ارتفاع)
    pub fade_out_percent: f64,
    /// مرکز قوس اسپات زاویه‌ای (درجه) — نسخهٔ ۱.۶ (درخواست کاربر ۵):
    /// بافت فقط روی همین قوس از محیط فرم اعمال می‌شود.
    #[serde(default)]
    pub spot_center_deg: f64,
    /// گسترهٔ قوس اسپات (درجه؛ ۳۶۰ = کل دور فرم = رفتار قدیمی).
    #[serde(default = "default_spot_span")]
    pub spot_span_deg: f64,
}

/// پیش‌فرض گسترهٔ اسپات — پوشش کامل (سازگاری پروژه‌های ≤۱.۵)
fn default_spot_span() -> f64 {
    360.0
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            kind: TextureKind::Ripple,
            amplitude_mm: 0.4,
            period_mm: 4.0,
            radial: true,
            phase_deg: 0.0,
            fade_in_percent: 0.0,
            fade_out_percent: 0.0,
            spot_center_deg: 0.0,
            spot_span_deg: default_spot_span(),
        }
    }
}

/// اعتبارسانی بافت
pub fn validate_texture(t: &TextureParams) -> Result<(), String> {
    if !t.amplitude_mm.is_finite() || t.amplitude_mm < 0.0 {
        return Err("دامنهٔ بافت نامعتبر است".into());
    }
    if !t.period_mm.is_finite() || t.period_mm < 0.2 {
        return Err("دورهٔ بافت باید حداقل 0.2 mm باشد (تراکم نامعقول)".into());
    }
    if t.fade_in_percent < 0.0 || t.fade_in_percent > 100.0
        || t.fade_out_percent < 0.0 || t.fade_out_percent > 100.0
    {
        return Err("درصد محوشدگی باید بین ۰ تا ۱۰۰ باشد".into());
    }
    Ok(())
}

/// وزن محوشدگی fade-in/out بر اساس ارتفاع نسبی
/// fade_in: از پایین شروع ضعیف و قوی می‌شود؛ fade_out: بالای فرم ضعیف می‌شود
fn fade_weight(t: &TextureParams, z_norm: f64) -> f64 {
    let z = z_norm.clamp(0.0, 1.0);
    let w_in = if t.fade_in_percent <= 0.0 {
        1.0
    } else {
        (z / (t.fade_in_percent / 100.0)).min(1.0)
    };
    let w_out = if t.fade_out_percent <= 0.0 {
        1.0
    } else {
        ((1.0 - z) / (t.fade_out_percent / 100.0)).min(1.0)
    };
    w_in.min(w_out)
}

/// hash قطعی (FxHash-مانند ساده) برای بافت چکشی — بدون RNG
/// همان ورودی همیشه همان خروجی (قابل‌بازتولید در آزمون)
fn deterministic_hash(a: i64, b: i64) -> i32 {
    // ثابت‌های Fibonaccci hashing — پراکندگی خوب با عملیات ارزان
    let mut h: u64 = 0x51_7c_c1_b7_27_22_0a_95_u64
        .wrapping_add((a as u64).wrapping_mul(0x9E3779B97F4A7C15))
        .wrapping_add((b as u64).wrapping_mul(0xC2B2AE3D27D4EB4F));
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    // نگاشت به i32 با حذف بیت علامت
    ((h >> 32) as i32) & i32::MAX
}

/// نویز مقداری ۱بعدی نرم و قطعی — برای رگهٔ چوب
/// سلول‌های صحیح با درون‌یابی smoothstep؛ همان ورودی → همان خروجی
fn value_noise_1d(x: f64, period: f64) -> f64 {
    let scale = (period * 0.5).max(0.5);
    let x = x / scale;
    let i = x.floor() as i64;
    let f = x - i as f64;
    // smoothstep
    let t = f * f * (3.0 - 2.0 * f);
    let a = deterministic_hash(i, 0) as f64 / i32::MAX as f64;
    let b = deterministic_hash(i + 1, 0) as f64 / i32::MAX as f64;
    a + (b - a) * t // ∈ [0, 1]
}

/// نویز مقداری ۲بعدی نرم و قطعی — برای چرم و حباب
fn value_noise_2d(x: f64, y: f64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let fx = x - xi as f64;
    let fy = y - yi as f64;
    let tx = fx * fx * (3.0 - 2.0 * fx);
    let ty = fy * fy * (3.0 - 2.0 * fy);
    let a = deterministic_hash(xi, yi) as f64 / i32::MAX as f64;
    let b = deterministic_hash(xi + 1, yi) as f64 / i32::MAX as f64;
    let c = deterministic_hash(xi, yi + 1) as f64 / i32::MAX as f64;
    let d = deterministic_hash(xi + 1, yi + 1) as f64 / i32::MAX as f64;
    // درون‌یابی دوطرفه
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty // ∈ [0, 1]
}

/// اعمال بافت روی یک نقطهٔ مسیر
/// ورودی/خروجی: (x, y, z) مختصات نقطه؛ s طول پیموده‌شدهٔ مسیر تا این نقطه (mm)؛
/// r_base شعاع پایه (برای جهت شعاعی)؛ z_norm ارتفاع نسبی ۰..۱؛
/// total_height برای fallback جهت عمودی.
///
/// خروجی: نقطهٔ تعدیل‌شده + پرچم برخورد (شعاع منفی)
pub fn apply_texture(
    point: [f64; 3],
    s: f64,
    r_base: f64,
    z_norm: f64,
    textures: &[TextureParams],
) -> ([f64; 3], bool) {
    let (p, c, _) = apply_texture_ex(point, s, r_base, z_norm, textures);
    (p, c)
}

/// نسخهٔ ۱.۵ — با «شدت» بافت (۰..۱) برای مدولاسیون سرعت مسیر:
/// شدت = بیشینهٔ |آفست اعمالی| / دامنهٔ مؤثر بین همهٔ بافتهای فعال.
/// ۰ = بدون انحراف (سرعت کامل)؛ ۱ = حداکثر انحراف (حداقل ~۶۰٪ سرعت).
/// مصرف‌کننده: spiral.rs (feedrate هر گام) — مثل ویدئوهای مرجع که
/// سرعت نازل روی فرورفتگی‌های بافت کاهش می‌یابد.
pub fn apply_texture_ex(
    point: [f64; 3],
    s: f64,
    r_base: f64,
    z_norm: f64,
    textures: &[TextureParams],
) -> ([f64; 3], bool, f64) {
    let [x, y, z] = point;
    let mut new_r_offset = 0.0; // آفست شعاعی جمع‌شده
    let mut new_z_offset = 0.0; // آفست عمودی جمع‌شده
    let phase = 0.0f64; // فاز هر بافت داخل حلقه محاسبه می‌شود

    for t in textures {
        let w = fade_weight(t, z_norm);
        if w <= 0.0 {
            continue;
        }
        // اسپات زاویه‌ای — نسخهٔ ۱.۶ (درخواست کاربر ۵): وزن پنجرهٔ زاویه‌ای
        // روی θ نقطه؛ بافت فقط روی قوس انتخابی اعمال می‌شود (۳۶۰ = همه‌جا).
        let theta = y.atan2(x);
        let w_theta =
            crate::designer::angular_window_weight(t.spot_center_deg, t.spot_span_deg, theta);
        if w_theta <= 0.0 {
            continue;
        }
        let w = w * w_theta;
        let ph = t.phase_deg.to_radians();
        // فاز طول مسیر: چند دوره تا اینجا
        let cycles = s / t.period_mm;
        let u = cycles * 2.0 * std::f64::consts::PI;
        // زاویهٔ قطبی نقطه (برای بافت‌های وابسته به جهت دور محور)
        let theta = y.atan2(x);
        let v = match t.kind {
            // موجک: سینوسی نرم روی طول مسیر
            TextureKind::Ripple => (u + ph).sin(),
            // تور بافته: موج مربعی نرم‌شده (tanh) — بالا/پایین متناوب
            TextureKind::Weave => {
                // tanh موج سینوسی را به مربعی نرم تبدیل می‌کند
                let s_ = (u + ph).sin();
                (s_ * 3.0).tanh()
            }
            // بخیه‌ای: پله‌های گسسته — sign با هیسترزیس نرم
            TextureKind::Stitch => {
                // sign(sine) با نرمی محدود
                let s_ = (u + ph).sin();
                if s_.abs() < 0.25 { s_ * 4.0 } else { s_.signum() }
            }
            // شیار مارپیچی: کانالی که هم به طول مسیر و هم به زاویه وابسته است
            // — نتیجه: شیار پیوسته مورب دور دیواره (مثل رزوهٔ پیچ)
            TextureKind::SpiralGroove => {
                let w = (u + ph + theta * 3.0).sin();
                // تیز کردن کف شیار با tanh نرم
                (w * 2.0).tanh()
            }
            // چکشی: فرورفتگی‌های شبه‌تصادفی «قطعی» — hash کمی زاویه و ارتفاع؛
            // بدون RNG/seed تصادفی: همان ورودی همیشه همان نتیجه (قابل‌بازتولید)
            TextureKind::Hammered => {
                // کوانتینه‌کردن فضای (زاویه، ارتفاع) به سلول‌های period-اندازه
                let cell_a = (theta / (t.period_mm * 0.05 + 1e-9)).floor() as i64;
                let cell_z = (z_norm * 200.0 / (t.period_mm + 1e-9)).floor() as i64;
                let h = deterministic_hash(cell_a, cell_z);
                // نگاشت hash به [-1, 1]
                (h as f64 / i32::MAX as f64).clamp(-1.0, 1.0)
            }
            // سبدی: شبکهٔ متقاطع — سینوس طول مسیر × سینوس زاویه (تاریک/روشن متناوب)
            TextureKind::Basket => {
                let a = (u + ph).sin();
                let b = (theta * 6.0 + ph).sin();
                a * b
            }
            // رگهٔ چوب: نویز مقداری نرم قطعی روی طول مسیر (رگه‌های کشیده)
            // + رگه‌های ظریف زاویه‌ای — شبیه الیاف چوب پیچ‌خورده
            TextureKind::WoodGrain => {
                let n = value_noise_1d(s + ph.to_degrees(), t.period_mm) * 2.0 - 1.0;
                let fibers = (theta * 2.0 + n * 3.0).sin() * 0.35;
                (n * 0.7 + fibers) / 1.05 // ≈ [−1, 1]
            }
            // چرم: نویز دوجهتی ریز دو-اکتاوی — بافت دانه‌ای پوست با ترک‌های ظریف
            TextureKind::Leather => {
                // مقیاس دانه‌ها از دوره: دورهٔ کوچک → دانه‌های ریزتر
                let fine = (t.period_mm * 1.5).clamp(1.5, 24.0);
                let a = value_noise_2d(theta * fine, z_norm * fine * 2.0) * 2.0 - 1.0;
                let b = value_noise_2d(theta * fine * 2.3 + 7.7, z_norm * fine * 4.6) * 2.0 - 1.0;
                (a * 0.65 + b * 0.35) * 1.3
            }
            // آج رزوه: شیارهای متقاطع V — |sin| در دو جهت مورب (دستگیرهٔ آجدار)
            TextureKind::Knurl => {
                let a = (u + ph + theta * 3.0).sin().abs();
                let b = (u + ph - theta * 3.0).sin().abs();
                a + b - 1.0
            }
            // موج دریا: دو سینوس با نسبت فرکانس غیرگویا (0.37) —
            // ترکیب بی‌تکرار که توهم موج‌های روان می‌دهد
            TextureKind::Ocean => {
                0.6 * (u + ph).sin() + 0.4 * (u * 0.37 + theta * 2.0 + ph).sin()
            }
            // حباب: فرورفتگی‌های گرد پراکنده — سلول‌های (زاویه، ارتفاع) با
            // مرکز hash قطعی؛ فاصلهٔ شعاعی → برجستگی گرد یا گودی
            TextureKind::Bubbles => {
                let cells_theta = ((2.0 * std::f64::consts::PI) / (t.period_mm * 0.35 + 1e-9))
                    .clamp(6.0, 48.0);
                let cells_z = (200.0 / t.period_mm).clamp(4.0, 60.0);
                let gx = (theta + std::f64::consts::PI) * cells_theta / (2.0 * std::f64::consts::PI);
                let gy = z_norm * cells_z;
                let ci = gx.floor() as i64;
                let cj = gy.floor() as i64;
                // مرکز قطعی سلول از hash — نصف‌سلول جابه‌جا
                let ox = deterministic_hash(ci, cj) as f64 / i32::MAX as f64 * 0.6 - 0.3;
                let oy = deterministic_hash(ci + 991, cj - 991) as f64 / i32::MAX as f64 * 0.6 - 0.3;
                let dx = (gx - ci as f64) - 0.5 - ox;
                let dy = (gy - cj as f64) - 0.5 - oy;
                let d = (dx * dx + dy * dy).sqrt();
                // پروفایل گرد: مرکز حباب برجسته، دور از آن گودی
                (1.0 - 2.2 * d).clamp(-1.0, 1.0)
            }
            // تخته‌ای: شیارهای افقی قوسی — |sin| روی ارتفاع واقعی z؛
            // ردیف‌های تخته‌مانند با درز گود
            TextureKind::Rows => {
                let bands = (100.0 / t.period_mm.max(0.5)).clamp(2.0, 80.0);
                (std::f64::consts::PI * z_norm * bands).sin().abs() * 2.0 - 1.0
            }
            // ---------------- نسخهٔ ۱٫۷: ۸ بافت جدید (درخواست ۶) ----------------
            // کربنی: تویل متقاطع ریز — دو موج متراکم با نسبت نامتناسب؛
            // الگوی ۲×۲ تویل کربنی (مورب‌های پلکانی)
            TextureKind::Carbon => {
                let a = (u + ph).sin();
                let b = (u * 1.37 + ph + theta * 2.0).sin();
                (a * b * 2.4).tanh()
            }
            // ست‌فقرات ماهی: ستون‌های زیگزاگ که جهتشان پالا-پایین عوض می‌شود —
            // هر ستون زاویه‌ای نصف‌دور، شیب مثلثی وارونه
            TextureKind::Herringbone => {
                let col = ((theta / std::f64::consts::PI).floor() as i64).rem_euclid(2);
                let tri = (2.0 / std::f64::consts::PI)
                    * (u + ph).sin().asin();
                if col == 0 { tri } else { -tri }
            }
            // سنگ‌فرش: سنگ‌های گرد درهم — سلول‌های (زاویه، ارتفاع) با
            // پروفایل گرد پهن (مثل حباب اما درشت‌تر و پهن‌تر)
            TextureKind::Cobblestone => {
                let cells_theta = ((2.0 * std::f64::consts::PI) / (t.period_mm * 0.6 + 1e-9))
                    .clamp(5.0, 32.0);
                let cells_z = (100.0 / (t.period_mm * 1.4).max(1.0)).clamp(3.0, 40.0);
                let gx = (theta + std::f64::consts::PI) * cells_theta / (2.0 * std::f64::consts::PI);
                let gy = z_norm * cells_z;
                let ci = gx.floor() as i64;
                let cj = gy.floor() as i64;
                let ox = deterministic_hash(ci, cj) as f64 / i32::MAX as f64 * 0.5 - 0.25;
                let oy = deterministic_hash(ci + 733, cj - 733) as f64 / i32::MAX as f64 * 0.5 - 0.25;
                let dx = (gx - ci as f64) - 0.5 - ox;
                let dy = (gy - cj as f64) - 0.5 - oy;
                let d = (dx * dx + dy * dy).sqrt();
                (1.0 - 2.4 * d).clamp(-1.0, 1.0)
            }
            // کراکله: ترک‌های ریز — «لبهٔ» سلول‌ها برجسته می‌شود (شبکهٔ
            // voronoi ساده: حلقه در d≈۰٫۴۲ هر سلول)
            TextureKind::Crackle => {
                let cells_theta = ((2.0 * std::f64::consts::PI) / (t.period_mm * 0.5 + 1e-9))
                    .clamp(8.0, 48.0);
                let cells_z = (100.0 / t.period_mm).clamp(6.0, 60.0);
                let gx = (theta + std::f64::consts::PI) * cells_theta / (2.0 * std::f64::consts::PI);
                let gy = z_norm * cells_z;
                let ci = gx.floor() as i64;
                let cj = gy.floor() as i64;
                let dx = (gx - ci as f64) - 0.5;
                let dy = (gy - cj as f64) - 0.5;
                let d = (dx * dx + dy * dy).sqrt();
                // حلقهٔ ترک دور مرکز سلول — باریک و تیز
                (1.0 - (d - 0.42).abs() * 4.5).clamp(-1.0, 1.0)
            }
            // فلس ماهی: کاسه‌های هم‌پوشان — |sin| زاویه‌ای با آفست نصف‌سلول
            // در هر ردیف ارتفاعی (مثل فلس‌های واقعی که روی هم می‌افتند)
            TextureKind::FishScale => {
                let rows = (100.0 / t.period_mm.max(1.0)).clamp(4.0, 50.0);
                let row = (z_norm * rows).floor();
                let cols = 6.0;
                let x = theta * cols / (2.0 * std::f64::consts::PI)
                    + row as f64 * 0.5
                    + ph;
                let a = (std::f64::consts::PI * x.fract()).sin();
                a * 2.0 - 1.0
            }
            // شنی: نویز دو-اکتاوی ریز و ملایم — سطح شن نرم
            TextureKind::Sand => {
                let n1 = value_noise_2d(theta * 9.0, z_norm * 18.0) * 2.0 - 1.0;
                let n2 = value_noise_2d(theta * 21.0 + 3.3, z_norm * 42.0) * 2.0 - 1.0;
                (n1 * 0.7 + n2 * 0.3) * 0.9
            }
            // شیار عمودی ریز: رگه‌های عمودی ظریف — cos² زاویه‌ای متراکم
            TextureKind::Ridges => {
                let c = (theta * 10.0 + ph).cos();
                c * c * 2.0 - 1.0
            }
            // منافذ اسفنجی: گودی‌های ریز پراکنده — فقط سلول‌های «منتخب» hash
            // گودی دارند (سطح متخلخل)
            TextureKind::Pores => {
                let cells_theta = ((2.0 * std::f64::consts::PI) / (t.period_mm * 0.4 + 1e-9))
                    .clamp(10.0, 64.0);
                let cells_z = (100.0 / t.period_mm).clamp(8.0, 80.0);
                let gx = (theta + std::f64::consts::PI) * cells_theta / (2.0 * std::f64::consts::PI);
                let gy = z_norm * cells_z;
                let ci = gx.floor() as i64;
                let cj = gy.floor() as i64;
                let hfrac = deterministic_hash(ci, cj) as f64 / i32::MAX as f64;
                if hfrac < 0.35 {
                    let dx = (gx - ci as f64) - 0.5;
                    let dy = (gy - cj as f64) - 0.5;
                    let d = (dx * dx + dy * dy).sqrt();
                    // گودی (منفی) فقط در مرکز سلول‌های منتخب
                    -(1.0 - 3.2 * d).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
        };
        let amount = v * t.amplitude_mm * w;
        if t.radial {
            new_r_offset += amount;
        } else {
            new_z_offset += amount;
        }
    }
    let _ = phase;

    // ---------- اعمال آفست شعاعی ----------
    let mut collided = false;
    let mut out = [x, y, z + new_z_offset];
    if new_r_offset != 0.0 && r_base > 1e-9 {
        let new_r = r_base + new_r_offset;
        if new_r < 0.0 {
            collided = true; // هشدار «بافت باعث جداشدن/محدودهٔ ناممکن»
        }
        let clamped_r = new_r.max(0.0);
        // نقطه روی همان زاویه با شعاع جدید
        let theta = y.atan2(x);
        out[0] = clamped_r * theta.cos();
        out[1] = clamped_r * theta.sin();
    }
    // شدت مؤثر — نسبت انحراف کل به دامنهٔ مؤثر (برای مدولاسیون سرعت)
    let amp_sum: f64 = textures.iter().map(|t| t.amplitude_mm).sum::<f64>().max(1e-9);
    let intensity = ((new_r_offset.abs() + new_z_offset.abs()) / amp_sum).clamp(0.0, 1.0);
    (out, collided, intensity)
}

/// بررسی «سلامت مسیر» پس از بافت: فاصلهٔ نقاط مجاور نباید از حدی بیشتر شود
/// (الزام: «اگر texture باعث جداشدن مسیر… هشدار بساز»)
/// خروجی: بیشینهٔ فاصلهٔ بین نقاط متوالی (mm)
pub fn max_gap_after_texture(points: &[[f64; 3]]) -> f64 {
    let mut max_gap: f64 = 0.0;
    for w in points.windows(2) {
        let d = [
            w[1][0] - w[0][0],
            w[1][1] - w[0][1],
            w[1][2] - w[0][2],
        ];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        max_gap = max_gap.max(l);
    }
    max_gap
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_modulates_radius() {
        // موجک باید شعاع نقطه را تغییر دهد
        // s=0 → sin(0)=0؛ s=1 (ربع دوره با period=4) → sin(π/2)=1 → بیشینه
        let p = [10.0, 0.0, 50.0];
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let (p1, _) = apply_texture(p, 0.0, 10.0, 0.5, &[t.clone()]);
        let (p2, _) = apply_texture(p, 1.0, 10.0, 0.5, &[t]);
        let r1 = (p1[0] * p1[0] + p1[1] * p1[1]).sqrt();
        let r2 = (p2[0] * p2[0] + p2[1] * p2[1]).sqrt();
        assert!((r1 - 10.0).abs() < 1e-9);
        assert!((r2 - 11.0).abs() < 1e-9, "r2 = {r2}");
    }

    #[test]
    fn test_weave_alternates() {
        // تور بافته باید مقادیر مثبت و منفی متناوب بدهد
        let t = TextureParams {
            kind: TextureKind::Weave,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        let (pa, _) = apply_texture(p, 1.0, 10.0, 0.5, &[t.clone()]); // ربع اول: مثبت
        let (pb, _) = apply_texture(p, 3.0, 10.0, 0.5, &[t]); // ربع سوم: منفی
        let ra = (pa[0] * pa[0] + pa[1] * pa[1]).sqrt();
        let rb = (pb[0] * pb[0] + pb[1] * pb[1]).sqrt();
        assert!(ra > rb, "ra={ra} rb={rb}");
    }

    #[test]
    fn test_vertical_direction() {
        // radial=false → فقط z تغییر می‌کند، شعاع ثابت
        // s=1 → sin(π/2)=1 → z+1
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            radial: false,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        let (out, _) = apply_texture(p, 1.0, 10.0, 0.5, &[t]);
        assert!((out[0] - 10.0).abs() < 1e-9);
        assert!((out[1] - 0.0).abs() < 1e-9);
        assert!((out[2] - 51.0).abs() < 1e-9);
    }

    #[test]
    fn test_fade_in_out() {
        // fade کامل: ابتدا و انتهای فرم بی‌اثر
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            fade_in_percent: 50.0,
            fade_out_percent: 50.0,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        // z_norm=0 → وزن صفر
        let (out0, _) = apply_texture(p, 1.0, 10.0, 0.0, &[t.clone()]);
        assert_eq!(out0, p);
        // وسط فرم → وزن ۱ و s=1 → sin(π/2)=1
        let (out_mid, _) = apply_texture(p, 1.0, 10.0, 0.5, &[t]);
        assert!(out_mid != p);
    }

    #[test]
    fn test_collision_detection() {
        // بافت بزرگ‌تر از شعاع → برخورد
        // s=3 → sin(3π/2)=-1 → شعاع منفی
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 20.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [5.0, 0.0, 50.0];
        let (_, collided) = apply_texture(p, 3.0, 5.0, 0.5, &[t]);
        assert!(collided);
    }

    #[test]
    fn test_zero_texture_identity() {
        // بدون بافت → نقطه دست‌نخورده
        let p = [10.0, 5.0, 50.0];
        let (out, _) = apply_texture(p, 100.0, 11.2, 0.5, &[]);
        assert_eq!(out, p);
    }

    #[test]
    fn test_validate_texture() {
        let ok = TextureParams::default();
        assert!(validate_texture(&ok).is_ok());
        let mut bad = TextureParams::default();
        bad.period_mm = 0.05; // خیلی تراکم بالا
        assert!(validate_texture(&bad).is_err());
        let mut bad2 = TextureParams::default();
        bad2.amplitude_mm = f64::NAN;
        assert!(validate_texture(&bad2).is_err());
    }

    #[test]
    fn test_max_gap() {
        // فاصلهٔ نقاط متوالی
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 10.0, 0.0]];
        assert!((max_gap_after_texture(&pts) - 10.0).abs() < 1e-9);
    }

    // ================= آزمون‌های بافت‌های جدید =================

    #[test]
    fn test_spiral_groove_depends_on_angle() {
        // شیار مارپیچی باید به زاویهٔ نقطه وابسته باشد (مورب دور دیواره)
        let t = TextureParams {
            kind: TextureKind::SpiralGroove,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        // دو زاویهٔ متفاوت با s یکسان → شعاع متفاوت
        let (p1, _) = apply_texture([10.0, 0.0, 50.0], 1.0, 10.0, 0.5, &[t.clone()]);
        // زاویهٔ 90°: نقطه (0, 10)
        let (p2, _) = apply_texture([0.0, 10.0, 50.0], 1.0, 10.0, 0.5, &[t]);
        let r1 = (p1[0] * p1[0] + p1[1] * p1[1]).sqrt();
        let r2 = (p2[0] * p2[0] + p2[1] * p2[1]).sqrt();
        assert!(
            (r1 - r2).abs() > 0.05,
            "شیار مارپیچی باید به زاویه وابسته باشد: r1={r1} r2={r2}"
        );
    }

    #[test]
    fn test_hammered_deterministic() {
        // چکشی: همان ورودی → همان خروجی (قطعیت، بدون RNG)
        let t = TextureParams {
            kind: TextureKind::Hammered,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [10.0, 3.0, 50.0];
        let (a1, _) = apply_texture(p, 10.0, 10.6, 0.5, &[t.clone()]);
        let (a2, _) = apply_texture(p, 10.0, 10.6, 0.5, &[t.clone()]);
        assert_eq!(a1, a2, "بافت چکشی باید قطعی باشد");
        // سلول‌های مختلف → دست‌کم برخی مقادیر متفاوت (پراکندگی)
        let mut distinct = std::collections::HashSet::new();
        for i in 0..24 {
            let ang = i as f64 * 0.26;
            let (out, _) = apply_texture(
                [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0],
                i as f64,
                10.0,
                0.5,
                &[t.clone()],
            );
            let r = (out[0] * out[0] + out[1] * out[1]).sqrt();
            distinct.insert((r * 1000.0).round() as i64);
        }
        assert!(distinct.len() > 6, "پراکندگی کم: {}", distinct.len());
    }

    #[test]
    fn test_hammered_bounded() {
        // چکشی: جابه‌جایی همیشه ≤ amplitude
        let t = TextureParams {
            kind: TextureKind::Hammered,
            amplitude_mm: 0.8,
            period_mm: 3.0,
            ..Default::default()
        };
        for i in 0..100 {
            let ang = i as f64 * 0.13;
            let r0 = 12.0;
            let (out, _) = apply_texture(
                [r0 * ang.cos(), r0 * ang.sin(), 40.0],
                i as f64 * 0.7,
                r0,
                (i % 10) as f64 / 10.0,
                &[t.clone()],
            );
            let r = (out[0] * out[0] + out[1] * out[1]).sqrt();
            assert!(r <= r0 + 0.8 + 1e-9 && r >= r0 - 0.8 - 1e-9, "r={r}");
        }
    }

    #[test]
    fn test_basket_cross_pattern() {
        // سبدی: حاصل‌ضرب دو سینوس → در برخی نقاط صفر، در برخی غیرصفر
        let t = TextureParams {
            kind: TextureKind::Basket,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        // s=0 → sin(0)=0 → اثر صفر؛ θ=0 هم b=sin(0)=0
        let (p0, _) = apply_texture([10.0, 0.0, 50.0], 0.0, 10.0, 0.5, &[t.clone()]);
        let r0 = (p0[0] * p0[0] + p0[1] * p0[1]).sqrt();
        assert!((r0 - 10.0).abs() < 1e-9, "r0={r0}");
        // s=1 (sin(π/2)=1)، θ=π/12 → sin(6·π/12)=sin(π/2)=1 → اثر کامل +1
        let ang = std::f64::consts::PI / 12.0;
        let (p1, _) = apply_texture(
            [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0],
            1.0,
            10.0,
            0.5,
            &[t],
        );
        let r1 = (p1[0] * p1[0] + p1[1] * p1[1]).sqrt();
        assert!((r1 - 11.0).abs() < 1e-6, "r1={r1}");
    }

    #[test]
    fn test_all_new_textures_affect_some_point() {
        // هر بافت جدید باید در برخی نقاط اثر داشته باشد
        for kind in [
            TextureKind::SpiralGroove,
            TextureKind::Hammered,
            TextureKind::Basket,
            // ---- نسخهٔ ۱٫۷: ۸ بافت جدید (درخواست ۶) ----
            TextureKind::Carbon,
            TextureKind::Herringbone,
            TextureKind::Cobblestone,
            TextureKind::Crackle,
            TextureKind::FishScale,
            TextureKind::Sand,
            TextureKind::Ridges,
            TextureKind::Pores,
        ] {
            let t = TextureParams {
                kind,
                amplitude_mm: 1.0,
                period_mm: 4.0,
                ..Default::default()
            };
            let mut changed = false;
            for i in 0..40 {
                let ang = i as f64 * 0.17;
                let s = i as f64 * 0.9;
                let (out, _) = apply_texture(
                    [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0],
                    s,
                    10.0,
                    0.5,
                    &[t.clone()],
                );
                let r = (out[0] * out[0] + out[1] * out[1]).sqrt();
                if (r - 10.0).abs() > 1e-6 {
                    changed = true;
                    break;
                }
            }
            assert!(changed, "{kind:?} هیچ اثری نداشت");
        }
    }

    // ================= آزمون‌های بافت‌های جدید (۶ عدد) =================

    #[test]
    fn test_wood_grain_smooth_deterministic() {
        // رگهٔ چوب: قطعی و پیوسته — دو نقطه نزدیک باید مقادیر نزدیک بدهند
        // (نویز نرم است نه پرشی مثل چکشی)
        let t = TextureParams {
            kind: TextureKind::WoodGrain,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        let (a1, _) = apply_texture(p, 10.0, 10.0, 0.5, &[t.clone()]);
        let (a2, _) = apply_texture(p, 10.0, 10.0, 0.5, &[t.clone()]);
        assert_eq!(a1, a2, "قطعیت");
        // پیوستگی: اختلاف s کوچک → اختلاف شعاع کوچک
        let (b1, _) = apply_texture(p, 20.0, 10.0, 0.5, &[t.clone()]);
        let (b2, _) = apply_texture(p, 20.5, 10.0, 0.5, &[t.clone()]);
        let r1 = (b1[0] * b1[0] + b1[1] * b1[1]).sqrt();
        let r2 = (b2[0] * b2[0] + b2[1] * b2[1]).sqrt();
        assert!(
            (r1 - r2).abs() < 0.15,
            "نویز چوب باید نرم باشد: {r1} {r2}"
        );
        // اثر دارد
        let (c, _) = apply_texture(p, 60.0, 10.0, 0.5, &[t.clone()]);
        let rc = (c[0] * c[0] + c[1] * c[1]).sqrt();
        assert!((rc - 10.0).abs() > 0.01, "باید اثر داشته باشد: {rc}");
    }

    #[test]
    fn test_leather_fine_and_deterministic() {
        // چرم: دوجهتی، قطعی، دارای پراکندگی
        let t = TextureParams {
            kind: TextureKind::Leather,
            amplitude_mm: 1.0,
            period_mm: 2.0,
            ..Default::default()
        };
        let mut distinct = std::collections::HashSet::new();
        for i in 0..30 {
            let ang = i as f64 * 0.21;
            let (a1, _) = apply_texture(
                [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0],
                i as f64,
                10.0,
                (i % 7) as f64 / 7.0,
                &[t.clone()],
            );
            let (a2, _) = apply_texture(
                [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0],
                i as f64,
                10.0,
                (i % 7) as f64 / 7.0,
                &[t.clone()],
            );
            assert_eq!(a1, a2, "قطعیت چرم");
            let r = (a1[0] * a1[0] + a1[1] * a1[1]).sqrt();
            distinct.insert((r * 1000.0).round() as i64);
        }
        assert!(distinct.len() > 8, "پراکندگی چرم کم: {}", distinct.len());
    }

    #[test]
    fn test_knurl_bounded() {
        // آج رزوه: |sin|+|sin|−1 ∈ [−1, 1]
        let t = TextureParams {
            kind: TextureKind::Knurl,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        for i in 0..200 {
            let ang = i as f64 * 0.047;
            let s = i as f64 * 0.3;
            let (out, _) = apply_texture(
                [10.0 * ang.cos(), 10.0 * ang.sin(), 40.0],
                s,
                10.0,
                (i % 10) as f64 / 10.0,
                &[t.clone()],
            );
            let r = (out[0] * out[0] + out[1] * out[1]).sqrt();
            assert!(r >= 9.0 - 1e-9 && r <= 11.0 + 1e-9, "r={r}");
        }
    }

    #[test]
    fn test_ocean_non_repeating() {
        // موج دریا: فرکانس نامتناسب → در s مختلف مقدار متفاوت؛ همیشه کران‌دار
        let t = TextureParams {
            kind: TextureKind::Ocean,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        let (a, _) = apply_texture(p, 1.0, 10.0, 0.5, &[t.clone()]);
        let (b, _) = apply_texture(p, 3.0, 10.0, 0.5, &[t.clone()]);
        let ra = (a[0] * a[0] + a[1] * a[1]).sqrt();
        let rb = (b[0] * b[0] + b[1] * b[1]).sqrt();
        assert!((ra - rb).abs() > 0.01, "موج باید تغییر کند: {ra} {rb}");
        // کران: |v| ≤ 0.6+0.4 = 1
        for i in 0..100 {
            let (o, _) = apply_texture(p, i as f64 * 0.5, 10.0, 0.5, &[t.clone()]);
            let r = (o[0] * o[0] + o[1] * o[1]).sqrt();
            assert!(r >= 9.0 - 1e-9 && r <= 11.0 + 1e-9);
        }
    }

    #[test]
    fn test_bubbles_round_and_deterministic() {
        // حباب: قطعی؛ برخی نقاط گودی (r < r0) و برخی برجستگی
        let t = TextureParams {
            kind: TextureKind::Bubbles,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let mut dips = 0;
        let mut bumps = 0;
        for i in 0..80 {
            // نمونه‌برداری غیرهم‌ارز تا مراکز حباب‌ها را بپوشاند
            let ang = i as f64 * 0.105;
            let zn = i as f64 * 0.031;
            let p = [10.0 * ang.cos(), 10.0 * ang.sin(), 50.0];
            let (a1, _) = apply_texture(p, i as f64, 10.0, zn, &[t.clone()]);
            let (a2, _) = apply_texture(p, i as f64, 10.0, zn, &[t.clone()]);
            assert_eq!(a1, a2, "قطعیت حباب");
            let r = (a1[0] * a1[0] + a1[1] * a1[1]).sqrt();
            if r < 9.95 {
                dips += 1;
            } else if r > 10.05 {
                bumps += 1;
            }
        }
        assert!(dips > 3, "حباب باید گودی بسازد: dips={dips}");
        assert!(bumps > 3, "حباب باید برجستگی بسازد: bumps={bumps}");
    }

    #[test]
    fn test_rows_horizontal_bands() {
        // تخته‌ای: فقط به ارتفاع وابسته است — همان z_norm، s متفاوت → همان نتیجه
        let t = TextureParams {
            kind: TextureKind::Rows,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let (a, _) = apply_texture([10.0, 0.0, 50.0], 0.0, 10.0, 0.5, &[t.clone()]);
        let (b, _) = apply_texture([10.0, 0.0, 50.0], 500.0, 10.0, 0.5, &[t.clone()]);
        assert_eq!(a, b, "تخته‌ای نباید به s وابسته باشد");
        // اما به z وابسته است — zn=0.5 و zn=0.13 روی فازهای مختلف می‌افتند
        // (bands=25: π·25·0.5=12.5π→sin=1؛ π·25·0.13=3.25π→sin≈0.71)
        let (c, _) = apply_texture([10.0, 0.0, 50.0], 0.0, 10.0, 0.13, &[t.clone()]);
        let ra = (a[0] * a[0] + a[1] * a[1]).sqrt();
        let rc = (c[0] * c[0] + c[1] * c[1]).sqrt();
        assert!((ra - rc).abs() > 0.01, "ردیف‌ها: ra={ra} rc={rc}");
    }

    // ================= آزمون‌های اسپات زاویه‌ای — نسخهٔ ۱.۶ =================

    #[test]
    fn test_spot_limits_texture_angularly() {
        // بافت موجک با اسپات ۹۰° حول ۰ → فقط نقطه‌های همان قوس جابه‌جا می‌شوند
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            spot_center_deg: 0.0,
            spot_span_deg: 90.0,
            ..Default::default()
        };
        // نقطه روی مرکز اسپات: s=1 (بیشینه موج) → شعاع تغییر می‌کند
        let (p_on, _) = apply_texture([10.0, 0.0, 50.0], 1.0, 10.0, 0.5, &[t.clone()]);
        let r_on = (p_on[0] * p_on[0] + p_on[1] * p_on[1]).sqrt();
        assert!((r_on - 11.0).abs() < 1e-9, "داخل اسپات باید بافت کامل باشد: {r_on}");
        // نقطهٔ مقابل (θ=180) → شعاع دست‌نخورده
        let (p_off, _) = apply_texture([-10.0, 0.0, 50.0], 1.0, 10.0, 0.5, &[t.clone()]);
        let r_off = (p_off[0] * p_off[0] + p_off[1] * p_off[1]).sqrt();
        assert!((r_off - 10.0).abs() < 1e-9, "خارج اسپات باید پایه باشد: {r_off}");
    }

    #[test]
    fn test_spot_texture_full_span_legacy() {
        // دهانهٔ کامل = همان رفتار قدیمی (بافت همه‌جا)
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default() // spot_span پیش‌فرض ۳۶۰
        };
        let (p, _) = apply_texture([10.0, 0.0, 50.0], 1.0, 10.0, 0.5, &[t]);
        let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
        assert!((r - 11.0).abs() < 1e-9, "پیش‌فرض کامل: r={r}");
    }
}
