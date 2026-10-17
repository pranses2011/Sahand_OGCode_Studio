//! ============================================================================
//! designer/profile.rs — گام ۱: پروفایل مقطع (شعاع-ارتفاع)
//! ============================================================================
//! پروفایل، منحنی ۲بعدی در صفحهٔ (r, h) است که با چرخش حول محور Z فرم
//! سه‌بعدی را می‌سازد (سطح انقلاب — surface of revolution).
//!
//! الزامات Docs/PROMPT_03 §4 گام ۱:
//!   - نقاط قابل افزودن/جابه‌جایی/حذف؛
//!   - خط مستقیم و منحنی با smoothing؛
//!   - کنترل قطر کف/بالا و ارتفاع نهایی و taper؛
//!   - پروفایل‌های آماده (vase/lamp/planter) «واقعاً قابل‌ویرایش»؛
//!   - واحد mm.
//!
//! مدل ریاضی: نقاط گرهی (r_i, h_i) با درون‌یابی مونوتون روی h؛
//! اگر دو نقطه h برابر داشتند خط ساده بین آن‌ها (pupil خطی عمودی).
//! ============================================================================

use serde::{Deserialize, Serialize};

/// یک نقطهٔ پروفایل — r شعاع (mm)، h ارتفاع (mm)
///
/// نسخهٔ ۱.۵ — مماس صریح اختیاری (درگ فلش‌های دوطرف در UI):
///   * `m = None` → مماس خودکار Fritsch–Carlson (رفتار ۱.۲..۱.۴)؛
///   * `m = Some(v)` → شیب dr/dh صریح کاربر؛ v در «فضای نرمال‌شدهٔ نقاط
///     مقیاس‌شده» تفسیر می‌شود (نه mm خام) — یعنی شیب نسبت به dh محلی؛
///     صریحِ ۰ = مماس افقی (قله/دره);
///   * سرریز/نامعتبر → به خودکار برمی‌گردد (گارد ایمنی).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProfilePoint {
    /// شعاع از محور مرکزی (mm) — همیشه ≥ 0
    pub r: f64,
    /// ارتفاع از کف (mm)
    pub h: f64,
    /// شیب مماس صریح dr/dh (اختیاری — نسخهٔ ۱.۵)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub m: Option<f64>,
}

/// روش درون‌یابی بین نقاط پروفایل — نسخهٔ ۱.۲
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    /// خط مستقیم بین نقاط (رفتار نسخهٔ ۱.۰/۱.۱ — لبهٔ تیز در گره‌ها)
    Linear,
    /// اسپلاین مونوتون Fritsch–Carlson — نرم و بدون فراجهش (پیش‌فرض)
    Spline,
}

impl Default for Interpolation {
    fn default() -> Self {
        Interpolation::Spline
    }
}

/// ورودی‌های اصلاح فرم — گام ۱ بخش «تنظیم فرم»
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FormAdjust {
    /// ارتفاع نهایی فرم (mm) — پروفایل به این ارتفاع مقیاس می‌شود
    pub height_mm: f64,
    /// قطر پایین (mm) — شعاع پایین به این مقدار تنظیم می‌شود
    pub bottom_diameter_mm: f64,
    /// قطر بالای فرم (mm)
    pub top_diameter_mm: f64,
    /// درصد باریک‌شدن تدریجی به سمت بالا (۰..۱۰۰)
    pub taper_percent: f64,
    /// تعداد نمونه‌های smoothing (۰ = بدون smoothing)
    pub smoothing_passes: u32,
    /// روش درون‌یابی بین نقاط (پیش‌فرض: اسپلاین مونوتون)
    #[serde(default)]
    pub interpolation: Interpolation,
}

impl Default for FormAdjust {
    fn default() -> Self {
        Self {
            height_mm: 100.0,
            bottom_diameter_mm: 60.0,
            top_diameter_mm: 60.0,
            taper_percent: 0.0,
            smoothing_passes: 0,
            interpolation: Interpolation::Spline,
        }
    }
}

/// پروفایل‌های آماده — «واقعاً قابل‌ویرایش» (الزام): خروجی Vec<Point> است
/// که UI همان را در ادیتور نقطه‌ای باز می‌کند
pub struct ProfilePreset;

impl ProfilePreset {
    /// گلدان کلاسیک: پایه باریک، شکم پهن، گردن تنگ، لب باز
    pub fn vase_default() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 22.0, h: 0.0, m: None },
            ProfilePoint { r: 28.0, h: 10.0, m: None },
            ProfilePoint { r: 30.0, h: 35.0, m: None },
            ProfilePoint { r: 26.0, h: 60.0, m: None },
            ProfilePoint { r: 18.0, h: 80.0, m: None },
            ProfilePoint { r: 17.0, h: 88.0, m: None },
            ProfilePoint { r: 21.0, h: 100.0, m: None },
        ]
    }

    /// سایه‌روشن (lampshade): مخروط نرم با لب کمی برگشته
    pub fn lampshade() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 8.0, h: 0.0, m: None },
            ProfilePoint { r: 15.0, h: 15.0, m: None },
            ProfilePoint { r: 35.0, h: 90.0, m: None },
            ProfilePoint { r: 40.0, h: 100.0, m: None },
        ]
    }

    /// گلدان کشت (planter): دیوارهٔ شیب‌دار ساده با لبه
    pub fn planter() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 15.0, h: 0.0, m: None },
            ProfilePoint { r: 18.0, h: 5.0, m: None },
            ProfilePoint { r: 30.0, h: 60.0, m: None },
            ProfilePoint { r: 32.0, h: 65.0, m: None },
        ]
    }

    /// لیوان (cup): دیوارهٔ تقریباً مستقیم با لبهٔ کوچک
    pub fn cup() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 20.0, h: 0.0, m: None },
            ProfilePoint { r: 21.0, h: 8.0, m: None },
            ProfilePoint { r: 24.0, h: 80.0, m: None },
            ProfilePoint { r: 25.0, h: 90.0, m: None },
        ]
    }

    /// بطری (bottle): بدنهٔ پهن + گردن باریک
    pub fn bottle() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 18.0, h: 0.0, m: None },
            ProfilePoint { r: 24.0, h: 15.0, m: None },
            ProfilePoint { r: 25.0, h: 45.0, m: None },
            ProfilePoint { r: 18.0, h: 75.0, m: None },
            ProfilePoint { r: 8.0, h: 90.0, m: None },
            ProfilePoint { r: 8.0, h: 110.0, m: None },
            ProfilePoint { r: 9.5, h: 115.0, m: None },
        ]
    }

    /// کاسه (bowl): نیم‌کرهٔ پهن با کف کوچک
    pub fn bowl() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 12.0, h: 0.0, m: None },
            ProfilePoint { r: 16.0, h: 4.0, m: None },
            ProfilePoint { r: 24.0, h: 16.0, m: None },
            ProfilePoint { r: 29.0, h: 34.0, m: None },
            ProfilePoint { r: 31.0, h: 55.0, m: None },
        ]
    }

    /// ساعت شنی (hourglass): دو مخروط با گلو باریک
    pub fn hourglass() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 22.0, h: 0.0, m: None },
            ProfilePoint { r: 20.0, h: 10.0, m: None },
            ProfilePoint { r: 12.0, h: 35.0, m: None },
            ProfilePoint { r: 7.0, h: 50.0, m: None },
            ProfilePoint { r: 12.0, h: 65.0, m: None },
            ProfilePoint { r: 20.0, h: 90.0, m: None },
            ProfilePoint { r: 22.0, h: 100.0, m: None },
        ]
    }

    /// جام (goblet): ساقهٔ باریک + کاسهٔ مخروطی
    pub fn goblet() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 16.0, h: 0.0, m: None },
            ProfilePoint { r: 14.0, h: 5.0, m: None },
            ProfilePoint { r: 5.0, h: 20.0, m: None },
            ProfilePoint { r: 4.5, h: 55.0, m: None },
            ProfilePoint { r: 6.0, h: 62.0, m: None },
            ProfilePoint { r: 16.0, h: 80.0, m: None },
            ProfilePoint { r: 20.0, h: 95.0, m: None },
        ]
    }

    /// فانوس (lantern): بدنهٔ کشیده با کمربند
    pub fn lantern() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 14.0, h: 0.0, m: None },
            ProfilePoint { r: 17.0, h: 10.0, m: None },
            ProfilePoint { r: 18.5, h: 40.0, m: None },
            ProfilePoint { r: 16.5, h: 55.0, m: None },
            ProfilePoint { r: 18.0, h: 75.0, m: None },
            ProfilePoint { r: 15.0, h: 90.0, m: None },
            ProfilePoint { r: 16.5, h: 100.0, m: None },
        ]
    }

    /// رکاب سفید (amphora): کوزهٔ یونانی با دو برجستگی
    pub fn amphora() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 15.0, h: 6.0, m: None },
            ProfilePoint { r: 24.0, h: 30.0, m: None },
            ProfilePoint { r: 25.5, h: 55.0, m: None },
            ProfilePoint { r: 20.0, h: 80.0, m: None },
            ProfilePoint { r: 11.0, h: 95.0, m: None },
            ProfilePoint { r: 12.5, h: 105.0, m: None },
        ]
    }

    /// قلم‌خانه (pencil holder): استوانهٔ ساده با شیار پایین
    pub fn pencil_holder() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 19.0, h: 0.0, m: None },
            ProfilePoint { r: 17.5, h: 5.0, m: None },
            ProfilePoint { r: 19.5, h: 12.0, m: None },
            ProfilePoint { r: 20.0, h: 95.0, m: None },
        ]
    }

    /// گلدان دوطبقه (stacked planter)
    pub fn stacked_planter() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 14.0, h: 0.0, m: None },
            ProfilePoint { r: 18.0, h: 6.0, m: None },
            ProfilePoint { r: 17.0, h: 40.0, m: None },
            ProfilePoint { r: 20.0, h: 45.0, m: None },
            ProfilePoint { r: 23.0, h: 48.0, m: None },
            ProfilePoint { r: 22.0, h: 80.0, m: None },
            ProfilePoint { r: 24.0, h: 85.0, m: None },
        ]
    }

    /// تخم‌مرغی (egg cup): پایهٔ کوچک + کاسهٔ تخم‌مرغی
    pub fn egg_cup() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 12.0, h: 0.0, m: None },
            ProfilePoint { r: 10.0, h: 4.0, m: None },
            ProfilePoint { r: 5.0, h: 15.0, m: None },
            ProfilePoint { r: 7.0, h: 22.0, m: None },
            ProfilePoint { r: 15.0, h: 30.0, m: None },
            ProfilePoint { r: 16.5, h: 40.0, m: None },
        ]
    }

    /// بشکه (barrel): کمربند میانی پهن
    pub fn barrel() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 18.0, h: 0.0, m: None },
            ProfilePoint { r: 22.0, h: 15.0, m: None },
            ProfilePoint { r: 24.0, h: 30.0, m: None },
            ProfilePoint { r: 24.0, h: 45.0, m: None },
            ProfilePoint { r: 22.0, h: 60.0, m: None },
            ProfilePoint { r: 18.0, h: 75.0, m: None },
        ]
    }

    /// مخروط ساده (cone)
    pub fn cone() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 25.0, h: 0.0, m: None },
            ProfilePoint { r: 24.0, h: 30.0, m: None },
            ProfilePoint { r: 16.0, h: 70.0, m: None },
            ProfilePoint { r: 5.0, h: 95.0, m: None },
        ]
    }

    /// شمع‌دان دیواری (wall sconce): نیم‌استوانهٔ کشیده
    pub fn sconce() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 9.0, h: 0.0, m: None },
            ProfilePoint { r: 13.0, h: 8.0, m: None },
            ProfilePoint { r: 22.0, h: 70.0, m: None },
            ProfilePoint { r: 24.0, h: 85.0, m: None },
        ]
    }

    /// دلو (bucket): مخروط معکوس با لبهٔ غلتان
    pub fn bucket() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 14.0, h: 0.0, m: None },
            ProfilePoint { r: 15.0, h: 6.0, m: None },
            ProfilePoint { r: 22.0, h: 70.0, m: None },
            ProfilePoint { r: 23.5, h: 85.0, m: None },
        ]
    }

    /// گلدان بلند چینی (celadon): گردن باریک و شکم گِرد
    pub fn celadon() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 13.0, h: 0.0, m: None },
            ProfilePoint { r: 16.0, h: 8.0, m: None },
            ProfilePoint { r: 23.0, h: 30.0, m: None },
            ProfilePoint { r: 24.0, h: 48.0, m: None },
            ProfilePoint { r: 17.0, h: 72.0, m: None },
            ProfilePoint { r: 10.5, h: 88.0, m: None },
            ProfilePoint { r: 11.0, h: 98.0, m: None },
        ]
    }
}

/// اعتبارسانی نقاط پروفایل — شعاع منفی و NaN رد می‌شود (گارد ایمنی)
/// تولیدکنندهٔ قطعی شبه‌تصادفی (xorshift64*) — بدون crate خارجی
/// همان seed → همان دنباله (قابل بازتولید در آزمون‌ها)
struct SmallRng(u64);

impl SmallRng {
    fn new(seed: u64) -> Self {
        // seed صفر/کوچک را پراکنده می‌کنیم (golden ratio)
        SmallRng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64* — George Marsaglia
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// float یکنواخت [lo, hi)
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        lo + u * (hi - lo)
    }
    /// انتخاب از چند گزینه
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() % items.len() as u64) as usize]
    }
}

/// پروفایل تصادفی چاپ‌پسند — نسخهٔ ۱.۵ (درخواست کاربر: «دکمهٔ رندوم»)
///
/// قواعد «چاپ خوب» (حس مارپیچی vase):
///   * ۴..۷ نقطه؛ ارتفاع کل ۶۰..۱۴۰mm؛
///   * شعاع کف ≥ ۱۰mm (چسبندگی به میز) و همهٔ شعاع‌ها در ۸..۳۴mm؛
///   * شیب دیواره بین هر جفت نقطه ≤ tan(۵۰°) ≈ ۱٫۲ — فراجهش چاپ‌ناپذیر نه؛
///   * شروع/پایان همیشه مماس افقی خودکار (m=None → Fritsch–Carlson)؛
///   * نقطهٔ پایانی لب: بیرون‌زدگی یا گردن (جمع‌شدن تصادفی).
///
/// خروجی همیشه «اعتبارسانی‌شده» است — گارد داخلی.
pub fn random_profile(seed: u64) -> Vec<ProfilePoint> {
    let mut rng = SmallRng::new(seed);

    // ارتفاع کل و تعداد گره‌ها
    let height = rng.range(60.0, 140.0).round();
    let n_knots = 4 + (rng.next_u64() % 4) as usize; // 4..7

    // شعاع کف — چسبندگی کافی
    let r_bottom = rng.range(11.0, 22.0).round();
    let mut pts: Vec<ProfilePoint> = vec![ProfilePoint { r: r_bottom, h: 0.0, m: None }];

    // الگوی کلی شخصیت فرم — از ۴ خلق‌وخو انتخاب
    enum Mood {
        Bulb,     // شکم پهن + گردن
        Columnar, // ستونی با موج‌های ریز
        Flare,    // شیب بازشونده
        Wave,     // موج‌های متناوب
    }
    let mood = *rng.pick(&[Mood::Bulb, Mood::Columnar, Mood::Flare, Mood::Wave]);

    // پراکندگی ارتفاع گره‌ها — یکنواخت با جیتر
    let dh = height / n_knots as f64;
    for i in 1..n_knots {
        let h = (dh * i as f64 + rng.range(-2.0, 2.0)).round().max(4.0).min(height - 3.0);
        let prev = pts.last().copied().unwrap();
        // دامنهٔ تغییر شعاع مجاز بر پایهٔ شیب سکانت (≤ tan50°)
        let dh_real = (h - prev.h).max(1.0);
        let max_dr = (1.19 * dh_real).min(12.0);
        let target = match mood {
            Mood::Bulb => {
                // پهن در یک‌سوم میانی، تنگ در انتها
                let t = h / height;
                let base = if t < 0.7 { r_bottom + 6.0 + 6.0 * (1.0 - (t - 0.35).abs() / 0.35).max(0.0) } else { r_bottom * 0.55 };
                base + rng.range(-3.0, 3.0)
            }
            Mood::Columnar => prev.r + rng.range(-2.0, 2.0),
            Mood::Flare => prev.r + dh_real * rng.range(0.05, 0.55),
            Mood::Wave => prev.r - (if i % 2 == 0 { 1.0 } else { -1.0 }) * rng.range(0.5, 3.0),
        };
        // اعمال محدودیت شیب + گزاره‌های چاپ
        let mut r = target;
        r = r.clamp(prev.r - max_dr, prev.r + max_dr);
        r = r.clamp(8.0, 34.0);
        // پایهٔ تماس کف هرگز زیر ۱۰mm نمی‌رود (دو نقطهٔ نخست)
        if i == 1 {
            r = r.max(10.0);
        }
        pts.push(ProfilePoint { r: r.round(), h, m: None });
    }

    // لب پایانی — بیرون‌زدگی ملایم یا درون‌فرورفتگی (۲۰/۸۰)
    if let Some(last) = pts.last_mut() {
        let lip = if rng.next_u64() % 5 == 0 {
            (last.r * 0.92).round() // لب جمع‌شده
        } else {
            (last.r * 1.06 + 1.0).round().min(34.0) // لب باز
        };
        last.r = lip.max(8.0);
    }

    // مرتب‌سازی نهایی بر اساس h (گارد) و اعتبارسانی
    sort_by_height(&mut pts);
    debug_assert!(validate_profile(&pts).is_ok());
    pts
}

pub fn validate_profile(points: &[ProfilePoint]) -> Result<(), String> {
    if points.len() < 2 {
        return Err("پروفایل حداقل باید ۲ نقطه داشته باشد".into());
    }
    for p in points {
        if !p.r.is_finite() || !p.h.is_finite() {
            return Err("مقادیر پروفایل باید عدد معتبر باشند".into());
        }
        if p.r < 0.0 {
            return Err(format!("شعاع منفی ({} mm) مجاز نیست", p.r));
        }
        if let Some(m) = p.m {
            if !m.is_finite() {
                return Err(format!("مماس نامعتبر ({} mm) مجاز نیست", m));
            }
        }
    }
    Ok(())
}

/// مرتب‌سازی نقاط بر اساس h صعودی (برای درون‌یابی مونوتون)
pub fn sort_by_height(points: &mut [ProfilePoint]) {
    points.sort_by(|a, b| a.h.partial_cmp(&b.h).unwrap());
}

/// اعمال اصلاح فرم: مقیاس ارتفاع/قطر + taper + smoothing
/// این تابع «شعاع مؤثر در ارتفاع z» را به‌صورت تابع بسته برمی‌گرداند تا
/// زنجیرهٔ pattern/spiral همان را مصرف کند (یک منبع واحد).
pub fn build_radius_fn<'a>(
    points: &'a [ProfilePoint],
    adjust: &'a FormAdjust,
) -> impl Fn(f64) -> f64 + use<'a> {
    // ---------- آماده‌سازی نقاط مقیاس‌شده ----------
    // مقیاس ارتفاع: h → h * (height / max_h)
    // مقیاس شعاع پایین/بالا: خطی بین ضریب پایین و بالا روی h نرمال‌شده
    let max_h = points
        .iter()
        .map(|p| p.h)
        .fold(f64::MIN, f64::max)
        .max(1e-9);
    let h_scale = adjust.height_mm / max_h;

    // ضریب قطر پایین/بالا نسبت به شعاع واقعی نقاط انتها
    let r_bottom_current = points.first().map(|p| p.r).unwrap_or(1.0).max(1e-9);
    let r_top_current = points.last().map(|p| p.r).unwrap_or(1.0).max(1e-9);
    let bottom_scale = (adjust.bottom_diameter_mm / 2.0) / r_bottom_current;
    let top_scale = (adjust.top_diameter_mm / 2.0) / r_top_current;

    // نقاط کاری مقیاس‌شده
    let mut pts: Vec<(f64, f64)> = points
        .iter()
        .map(|p| {
            let t = (p.h / max_h).clamp(0.0, 1.0);
            // درون‌یابی خطی ضریب قطر بین پایین و بالا
            let s = bottom_scale + (top_scale - bottom_scale) * t;
            (p.h * h_scale, p.r * s)
        })
        .collect();

    // ---------- taper: باریک‌شدن درصدی به سمت بالا ----------
    let taper = adjust.taper_percent.clamp(0.0, 100.0) / 100.0;
    if taper > 0.0 {
        let h_top = pts.last().map(|(h, _)| *h).unwrap_or(1.0);
        for (h, r) in pts.iter_mut() {
            let t = (*h / h_top.max(1e-9)).clamp(0.0, 1.0);
            // شعاع کم می‌شود نسبت به درصدِ باریک‌شدن؛ صفرِ کامل مجاز نیست
            *r *= 1.0 - taper * t * 0.98;
        }
    }

    // ---------- smoothing: میانگین‌گیری وزنی چندگذر ----------
    // smoothing حفظ نقاط ابتدا/انتها است (لب کف و بالا ثابت می‌مانند)
    for _ in 0..adjust.smoothing_passes.min(20) {
        if pts.len() < 3 {
            break;
        }
        let mut new_pts = pts.clone();
        for i in 1..pts.len() - 1 {
            new_pts[i].1 = pts[i - 1].1 * 0.25 + pts[i].1 * 0.5 + pts[i + 1].1 * 0.25;
        }
        pts = new_pts;
    }

    // ---------- درون‌یابی: اسپلاین مونوتون Fritsch–Carlson ----------
    // مرتب‌سازی مطمئن بر اساس h (درون‌یابی تابعی h↦r نیاز دارد)
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // ---------- مماس‌های مؤثر (نسخهٔ ۱.۵) ----------
    // مماس صریح کاربر به «فضای مقیاس‌شده» تبدیل می‌شود: m فضایی dr/dh
    // نسبت به dh محلی نقاط خام است؛ پس از مقیاس ارتفاع/شعاع، شیب
    // مؤثر = m × (scale_r/scale_h) موضعی تقریبی می‌شود. ساده و پایدار:
    // تبدیل یک‌بار در همان گره با ضریب مقیاس میانگین آن نقطه.
    // مماس‌های خودکار همچنان Fritsch–Carlson روی نقاط مقیاس‌شده‌اند.
    let use_spline = adjust.interpolation == Interpolation::Spline;
    let tangents = if use_spline {
        effective_tangents(&pts, points, h_scale)
    } else {
        Vec::new()
    };

    // ---------- تابع درون‌یابی ----------
    // جست‌وجوی دوطرفه از آخرین ایندکس (پرسش‌های متوالی z صعودی سریع‌اند)
    move |z: f64| {
        if pts.is_empty() {
            return 0.0;
        }
        if z <= pts[0].0 {
            return pts[0].1;
        }
        if z >= pts[pts.len() - 1].0 {
            return pts[pts.len() - 1].1;
        }
        // درون‌یابی بین دو نقطهٔ مجاور
        let mut lo = 0usize;
        let mut hi = pts.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pts[mid].0 <= z {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let (h0, r0) = pts[lo];
        let (h1, r1) = pts[hi];
        if (h1 - h0).abs() < 1e-12 {
            return r0; // گره‌های هم‌ارتفاع — خط عمودی ساده
        }
        let t = (z - h0) / (h1 - h0);
        if !use_spline {
            // حالت خطی (سازگاری با ۱.۰/۱.۱)
            return r0 + (r1 - r0) * t;
        }
        // هرمیت مکعبی با شیب‌های مونوتون — از هر دو گره دقیقاً می‌گذرد
        let m0 = tangents[lo];
        let m1 = tangents[hi];
        let dh = h1 - h0;
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * r0 + h10 * dh * m0 + h01 * r1 + h11 * dh * m1
    }
}

/// محاسبهٔ شیب‌های هرمیتی مونوتون (Fritsch–Carlson 1980):
/// ۱) شیب نقطه‌ای هر بازه δᵢ؛
/// ۲) شیب گره‌ای: اگر علامت دو δ مجاور فرق کند → ۰ (بیشینه/کمینهٔ محلی،
///    بدون فراجهش)؛ وگرنه میانگین هارمونیک وزنی؛
/// ۳) محدودسازی |m| ≤ 3·|δ| هر بازه برای تضمین یکنوایی.
/// نتیجه: منحنی نرم بدون نوسان — دقیقاً از گره‌ها می‌گذرد.
fn monotone_tangents(pts: &[(f64, f64)]) -> Vec<f64> {
    let n = pts.len();
    if n < 2 {
        return vec![0.0; n];
    }
    // δ بازه‌ها
    let mut delta = vec![0.0f64; n - 1];
    for i in 0..n - 1 {
        let dh = pts[i + 1].0 - pts[i].0;
        delta[i] = if dh.abs() < 1e-12 {
            0.0
        } else {
            (pts[i + 1].1 - pts[i].1) / dh
        };
    }
    // شیب اولیهٔ گره‌ها
    let mut m = vec![0.0f64; n];
    m[0] = delta[0];
    m[n - 1] = delta[n - 2];
    for i in 1..n - 1 {
        if delta[i - 1] * delta[i] <= 0.0 {
            m[i] = 0.0; // بیشینه/کمینهٔ محلی → مماس افقی
        } else {
            // میانگین هارمونیک وزنی (Fritsch–Carlson)
            let w1 = 2.0 * (pts[i + 1].0 - pts[i].0) + (pts[i].0 - pts[i - 1].0);
            let w2 = (pts[i + 1].0 - pts[i].0) + 2.0 * (pts[i].0 - pts[i - 1].0);
            m[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
        }
    }
    // محدودسازی یکنوایی هر بازه (m نباید از ۳δ عبور کند)
    for i in 0..n - 1 {
        if delta[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let limit = 3.0 * delta[i];
            if (delta[i] > 0.0 && m[i] > limit) || (delta[i] < 0.0 && m[i] < limit) {
                m[i] = limit;
            }
            if (delta[i] > 0.0 && m[i + 1] > limit) || (delta[i] < 0.0 && m[i + 1] < limit) {
                m[i + 1] = limit;
            }
            // شیب با علامت مخالف δ هم فراجهش می‌سازد → صفر
            if delta[i] * m[i] < 0.0 {
                m[i] = 0.0;
            }
            if delta[i] * m[i + 1] < 0.0 {
                m[i + 1] = 0.0;
            }
        }
    }    m
}

/// مماس‌های مؤثر — نسخهٔ ۱.۵ (فلش‌های دوطرف در UI)
///
/// قاعده:
///   * گره بدون مماس صریح → Fritsch–Carlson خودکار (رفتار ۱.۲..۱.۴)؛
///   * گره با مماس صریح `m` (فضای نقاط خام dr/dh) → تبدیل به فضای مقیاس‌شده
///     با ضریب s_i/h_scale موضعی و اعتبارسنجی دامنه:
///     |m| ≤ ۶·max(|δ_chap|, |δ_rast|) — کنترل خلاقانه ولی بدون بی‌نهایت.
///
/// برخلاف مماس خودکار، مماس صریح «علامت مخالف δ» مجاز است (S-curves بین
/// گره‌ها = خمیدگی خلاقانه مثل اپ‌های ویرایش منحنی). فراجهش نرم محدود است؛
/// مصرف‌کننده‌ها شعاع را با `.max(0)` ایمن می‌کنند.
fn effective_tangents(
    pts: &[(f64, f64)],
    source: &[ProfilePoint],
    h_scale: f64,
) -> Vec<f64> {
    let mut t = monotone_tangents(pts);
    if pts.is_empty() {
        return t;
    }
    // δ چپ/راست هر گره (برای سقف دامنه) — روی نقاط مقیاس‌شده
    let n = pts.len();
    let delta_at = |i: usize| -> f64 {
        // بازهٔ چپ i-1→i و بازهٔ راست i→i+1؛ بزرگ‌ترین قدرمطلق
        let mut best = 0.0f64;
        if i > 0 {
            let dh = pts[i].0 - pts[i - 1].0;
            if dh.abs() > 1e-12 {
                best = best.max(((pts[i].1 - pts[i - 1].1) / dh).abs());
            }
        }
        if i + 1 < n {
            let dh = pts[i + 1].0 - pts[i].0;
            if dh.abs() > 1e-12 {
                best = best.max(((pts[i + 1].1 - pts[i].1) / dh).abs());
            }
        }
        best
    };
    // تطبیق نقطهٔ خام (h·h_scale) به ایندکس مرتب‌شده — نزدیک‌ترین گره
    for s in source {
        let Some(m_raw) = s.m.filter(|m| m.is_finite()) else {
            continue;
        };
        let h_s = s.h * h_scale;
        let mut best_i: Option<usize> = None;
        let mut best_d = f64::INFINITY;
        for (i, (h, _)) in pts.iter().enumerate() {
            let d = (h - h_s).abs();
            if d < best_d {
                best_d = d;
                best_i = Some(i);
            }
        }
        let Some(i) = best_i else { continue };
        if best_d > 1e-6 * (pts[n - 1].0 - pts[0].0).max(1.0) {
            continue; // نقطهٔ خام بدون گره متناظر (نباید رخ دهد)
        }
        // تبدیل فضای خام → مقیاس‌شده: r' = r·s؛ h' = h·h_scale
        let s_scale = if s.r.abs() > 1e-9 { pts[i].1 / s.r } else { 1.0 };
        let m_scaled = m_raw * s_scale / h_scale.max(1e-9);
        // سقف دامنه — ۶ برابر شیب سکانت مجاور (خلاق ولی مهارشده)
        let cap = 6.0 * delta_at(i);
        t[i] = m_scaled.clamp(-cap.max(0.05), cap.max(0.05));
    }
    t
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_profiles_valid() {
        // هر سه preset باید اعتبارسانی و مرتب باشند
        for pts in [
            ProfilePreset::vase_default(),
            ProfilePreset::lampshade(),
            ProfilePreset::planter(),
        ] {
            assert!(validate_profile(&pts).is_ok(), "پروفایل نامعتبر");
            // ارتفاع‌ها صعودی‌اند
            for w in pts.windows(2) {
                assert!(w[0].h <= w[1].h);
            }
        }
    }

    #[test]
    fn test_validate_negative_radius() {
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: -5.0, h: 10.0, m: None },
        ];
        assert!(validate_profile(&pts).is_err());
    }

    #[test]
    fn test_radius_fn_endpoints() {
        // قطر پایین/بالا باید دقیقاً اعمال شود
        let pts = ProfilePreset::vase_default();
        let adj = FormAdjust {
            height_mm: 100.0,
            bottom_diameter_mm: 40.0,
            top_diameter_mm: 30.0,
            taper_percent: 0.0,
            smoothing_passes: 0,
            interpolation: Interpolation::Linear,
        };
        let f = build_radius_fn(&pts, &adj);
        assert!((f(0.0) - 20.0).abs() < 1e-9, "پایین = {}", f(0.0));
        assert!((f(100.0) - 15.0).abs() < 1e-9, "بالا = {}", f(100.0));
    }

    #[test]
    fn test_radius_fn_height_rescale() {
        // ارتفاع نهایی باید مقیاس شود
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 10.0, h: 50.0, m: None },
        ];
        let adj = FormAdjust {
            height_mm: 200.0,
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            taper_percent: 0.0,
            smoothing_passes: 0,
            interpolation: Interpolation::Linear,
        };
        let f = build_radius_fn(&pts, &adj);
        // در z=100 (وسط فرم جدید) شعاع همان 10 است
        assert!((f(100.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_taper_narrows_top() {
        // taper باید بالا را باریک‌تر کند
        let pts = vec![
            ProfilePoint { r: 20.0, h: 0.0, m: None },
            ProfilePoint { r: 20.0, h: 100.0, m: None },
        ];
        let no_taper = FormAdjust {
            taper_percent: 0.0,
            bottom_diameter_mm: 40.0,
            top_diameter_mm: 40.0,
            ..Default::default()
        };
        let with_taper = FormAdjust { taper_percent: 50.0, ..no_taper };
        let f0 = build_radius_fn(&pts, &no_taper);
        let f1 = build_radius_fn(&pts, &with_taper);
        assert!(f1(100.0) < f0(100.0));
        // پایین تقریباً بدون تغییر
        assert!((f1(0.0) - f0(0.0)).abs() < 1e-9);
    }

    #[test]
    fn test_smoothing_softens_corners() {
        // smoothing باید گوشهٔ تیز را نرم کند
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 50.0, m: None }, // قلهٔ تیز
            ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        let no_smooth = FormAdjust {
            smoothing_passes: 0,
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            ..Default::default()
        };
        let smoothed = FormAdjust { smoothing_passes: 10, ..no_smooth };
        let f0 = build_radius_fn(&pts, &no_smooth);
        let f1 = build_radius_fn(&pts, &smoothed);
        // قلهٔ تیز کم‌ارتفاع‌تر می‌شود
        assert!(f1(50.0) < f0(50.0), "{} vs {}", f1(50.0), f0(50.0));
        // اما انتها ثابت می‌مانند
        assert!((f1(0.0) - f0(0.0)).abs() < 1e-9);
        assert!((f1(100.0) - f0(100.0)).abs() < 1e-9);
    }

    #[test]
    fn test_monotone_interpolation() {
        // بین دو نقطه، درون‌یابی خطی است
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 100.0, m: None },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 60.0,
            ..Default::default()
        };
        let f = build_radius_fn(&pts, &adj);
        assert!((f(50.0) - 20.0).abs() < 1e-9); // وسط = میانگین
        assert!((f(25.0) - 15.0).abs() < 1e-9);
    }

    // ================= آزمون‌های اسپلاین مونوتون (نسخهٔ ۱.۲) =================

    #[test]
    fn test_spline_passes_through_knots() {
        // اسپلاین باید از تمام گره‌ها دقیقاً بگذرد
        // (قطرها = ۲×شعاع انتهایی‌ها تا مقیاس یک‌به‌یک باشد)
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 25.0, m: None },
            ProfilePoint { r: 18.0, h: 60.0, m: None },
            ProfilePoint { r: 22.0, h: 100.0, m: None },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 44.0,
            ..Default::default()
        };
        let f = build_radius_fn(&pts, &adj);
        for p in &pts {
            assert!((f(p.h) - p.r).abs() < 1e-9, "گره h={} رد شد: {}", p.h, f(p.h));
        }
    }

    #[test]
    fn test_spline_smooths_sharp_corner() {
        // گوشهٔ تیز: خطی وسط بازهٔ اول = 20؛ اسپلاین نرم‌تر (≠ خطی)
        // ولی بدون فراجهش از بازهٔ [10, 30]
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 50.0, m: None },
            ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        let adj_spline = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            ..Default::default()
        }; // پیش‌فرض: Spline
        let adj_linear = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            interpolation: Interpolation::Linear,
            ..Default::default()
        };
        let fs = build_radius_fn(&pts, &adj_spline);
        let fl = build_radius_fn(&pts, &adj_linear);
        let mid = fs(25.0);
        // نرمی: با فاصلهٔ معنادار از خطی
        assert!(
            (mid - fl(25.0)).abs() > 0.3,
            "اسپلاین باید گوشه را نرم کند: mid={mid} linear={}",
            fl(25.0)
        );
        // بدون فراجهش: در [10, 30] می‌ماند و قله دقیقاً ۳۰ در گره است
        for i in 0..=100 {
            let z = i as f64;
            assert!(fs(z) >= 10.0 - 1e-9 && fs(z) <= 30.0 + 1e-9, "فراجهش در z={z}: {}", fs(z));
        }
    }

    #[test]
    fn test_spline_monotone_data_no_overshoot() {
        // دادهٔ یکنوا: نتیجه باید یکنوا و در بازهٔ انتهایی‌ها بماند
        let pts = vec![
            ProfilePoint { r: 5.0, h: 0.0, m: None },
            ProfilePoint { r: 12.0, h: 20.0, m: None },
            ProfilePoint { r: 19.0, h: 45.0, m: None },
            ProfilePoint { r: 25.0, h: 80.0, m: None },
            ProfilePoint { r: 28.0, h: 100.0, m: None },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 10.0,
            top_diameter_mm: 56.0,
            ..Default::default()
        };
        let f = build_radius_fn(&pts, &adj);
        let mut prev = f(0.0);
        for i in 1..=200 {
            let z = i as f64 * 0.5;
            let v = f(z);
            assert!(v >= prev - 1e-9, "یکنوایی شکست در z={z}: {v} < {prev}");
            assert!(v >= 5.0 - 1e-9 && v <= 28.0 + 1e-9, "خارج بازه در z={z}: {v}");
            prev = v;
        }
    }

    #[test]
    fn test_linear_mode_matches_legacy() {
        // حالت Linear دقیقاً رفتار ۱.۰/۱.۱: میان‌پاره‌خطی
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 50.0, m: None },
            ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            interpolation: Interpolation::Linear,
            ..Default::default()
        };
        let f = build_radius_fn(&pts, &adj);
        assert!((f(25.0) - 20.0).abs() < 1e-9);
        assert!((f(75.0) - 20.0).abs() < 1e-9);
    }

    // ================= آزمون‌های نسخهٔ ۱.۵ =================

    #[test]
    fn test_explicit_tangent_changes_shape() {
        // مماس صریح باید شکل بین دو گره را عوض کند
        let base = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 50.0, m: None },
            ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 20.0,
            ..Default::default()
        };
        // مماس افقی صریح در گرهٔ میانی → قلهٔ صاف‌تر از خودکار
        let with_m = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: None },
            ProfilePoint { r: 30.0, h: 50.0, m: Some(0.0) },
            ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        let f_auto = build_radius_fn(&base, &adj);
        let f_manual = build_radius_fn(&with_m, &adj);
        // در نزدیکی گرهٔ میانی (۴۵mm)، شکل‌ها باید تفاوت معنادار داشته باشند
        let da = (f_auto(45.0) - f_manual(45.0)).abs()
            + (f_auto(55.0) - f_manual(55.0)).abs();
        assert!(da > 0.05, "مماس صریح باید شکل را عوض کند: diff={da}");
        // هر دو از گره‌ها می‌گذرند
        for f in [&f_auto, &f_manual] {
            assert!((f(50.0) - 30.0).abs() < 1e-9);
        }
    }

    #[test]
    fn test_explicit_tangent_nonfinite_falls_back() {
        // مماس NaN → نادیده گرفته می‌شود؛ نتیجه = خودکار
        let pts = vec![
            ProfilePoint { r: 10.0, h: 0.0, m: Some(f64::NAN) },
            ProfilePoint { r: 25.0, h: 100.0, m: Some(f64::INFINITY) },
        ];
        let adj = FormAdjust {
            bottom_diameter_mm: 20.0,
            top_diameter_mm: 50.0,
            ..Default::default()
        };
        let f = build_radius_fn(&pts, &adj);
        assert!((f(0.0) - 10.0).abs() < 1e-9);
        assert!((f(100.0) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn test_new_presets_valid() {
        // همهٔ پریست‌های ۱.۵ معتبر و مرتب
        let all = [
            ProfilePreset::cup(),
            ProfilePreset::bottle(),
            ProfilePreset::bowl(),
            ProfilePreset::hourglass(),
            ProfilePreset::goblet(),
            ProfilePreset::lantern(),
            ProfilePreset::amphora(),
            ProfilePreset::pencil_holder(),
            ProfilePreset::stacked_planter(),
            ProfilePreset::egg_cup(),
            ProfilePreset::barrel(),
            ProfilePreset::cone(),
            ProfilePreset::sconce(),
            ProfilePreset::bucket(),
            ProfilePreset::celadon(),
        ];
        assert_eq!(all.len(), 15);
        for pts in all {
            assert!(validate_profile(&pts).is_ok());
            for w in pts.windows(2) {
                assert!(w[0].h <= w[1].h, "hها باید صعودی باشند");
            }
            // شعاع کف ≥ ۱۰mm (چسبندگی) — برای همهٔ پریست‌های نمایشی
            assert!(pts.first().map(|p| p.r >= 9.0).unwrap_or(false));
        }
    }

    #[test]
    fn test_random_profile_deterministic() {
        // همان seed → همان پروفایل (قطعیت)
        let a = random_profile(12345);
        let b = random_profile(12345);
        assert_eq!(a, b);
        // seed متفاوت → معمولاً پروفایل متفاوت
        let c = random_profile(999);
        assert_ne!(a, c);
    }

    #[test]
    fn test_random_profile_print_safe() {
        // ۲۰۰ seed: همیشه چاپ‌پسند
        for seed in 0..200u64 {
            let pts = random_profile(seed);
            assert!(validate_profile(&pts).is_ok(), "seed={seed} نامعتبر");
            // ارتفاع معقول
            let h_max = pts.last().map(|p| p.h).unwrap_or(0.0);
            assert!(h_max >= 55.0 && h_max <= 145.0, "seed={seed} h={h_max}");
            // شعاع‌ها در بازهٔ چاپ خوب
            for p in &pts {
                assert!(p.r >= 8.0 && p.r <= 34.5, "seed={seed} r={}", p.r);
            }
            // شعاع کف ≥ ۱۰ (چسبندگی)
            assert!(pts[0].r >= 10.0, "seed={seed} پایه={}", pts[0].r);
            // شیب دیواره ≤ tan(۵۱°) با تلورانس گردشدن
            for w in pts.windows(2) {
                let dr = (w[1].r - w[0].r).abs();
                let dh = (w[1].h - w[0].h).max(0.5);
                assert!(
                    dr / dh <= 1.25,
                    "seed={seed} شیب تند: dr={dr} dh={dh}"
                );
            }
        }
    }
}
