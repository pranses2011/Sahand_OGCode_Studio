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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProfilePoint {
    /// شعاع از محور مرکزی (mm) — همیشه ≥ 0
    pub r: f64,
    /// ارتفاع از کف (mm)
    pub h: f64,
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
            ProfilePoint { r: 22.0, h: 0.0 },
            ProfilePoint { r: 28.0, h: 10.0 },
            ProfilePoint { r: 30.0, h: 35.0 },
            ProfilePoint { r: 26.0, h: 60.0 },
            ProfilePoint { r: 18.0, h: 80.0 },
            ProfilePoint { r: 17.0, h: 88.0 },
            ProfilePoint { r: 21.0, h: 100.0 },
        ]
    }

    /// سایه‌روشن (lampshade): مخروط نرم با لب کمی برگشته
    pub fn lampshade() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 8.0, h: 0.0 },
            ProfilePoint { r: 15.0, h: 15.0 },
            ProfilePoint { r: 35.0, h: 90.0 },
            ProfilePoint { r: 40.0, h: 100.0 },
        ]
    }

    /// گلدان کشت (planter): دیوارهٔ شیب‌دار ساده با لبه
    pub fn planter() -> Vec<ProfilePoint> {
        vec![
            ProfilePoint { r: 15.0, h: 0.0 },
            ProfilePoint { r: 18.0, h: 5.0 },
            ProfilePoint { r: 30.0, h: 60.0 },
            ProfilePoint { r: 32.0, h: 65.0 },
        ]
    }
}

/// اعتبارسانی نقاط پروفایل — شعاع منفی و NaN رد می‌شود (گارد ایمنی)
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

    // شیب‌های هرمیتی گره‌ها (فقط در حالت Spline محاسبه و مصرف می‌شوند)
    let use_spline = adjust.interpolation == Interpolation::Spline;
    let tangents = if use_spline {
        monotone_tangents(&pts)
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
    }
    m
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: -5.0, h: 10.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 10.0, h: 50.0 },
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
            ProfilePoint { r: 20.0, h: 0.0 },
            ProfilePoint { r: 20.0, h: 100.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 30.0, h: 50.0 }, // قلهٔ تیز
            ProfilePoint { r: 10.0, h: 100.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 30.0, h: 100.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 30.0, h: 25.0 },
            ProfilePoint { r: 18.0, h: 60.0 },
            ProfilePoint { r: 22.0, h: 100.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 30.0, h: 50.0 },
            ProfilePoint { r: 10.0, h: 100.0 },
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
            ProfilePoint { r: 5.0, h: 0.0 },
            ProfilePoint { r: 12.0, h: 20.0 },
            ProfilePoint { r: 19.0, h: 45.0 },
            ProfilePoint { r: 25.0, h: 80.0 },
            ProfilePoint { r: 28.0, h: 100.0 },
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
            ProfilePoint { r: 10.0, h: 0.0 },
            ProfilePoint { r: 30.0, h: 50.0 },
            ProfilePoint { r: 10.0, h: 100.0 },
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
}
