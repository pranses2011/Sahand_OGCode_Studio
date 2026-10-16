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
}

impl Default for FormAdjust {
    fn default() -> Self {
        Self {
            height_mm: 100.0,
            bottom_diameter_mm: 60.0,
            top_diameter_mm: 60.0,
            taper_percent: 0.0,
            smoothing_passes: 0,
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
pub fn build_radius_fn(
    points: &[ProfilePoint],
    adjust: &FormAdjust,
) -> impl Fn(f64) -> f64 + '_ {
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
        // درون‌یابی خطی بین دو نقطهٔ مجاور
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
        let t = if (h1 - h0).abs() < 1e-12 { 0.0 } else { (z - h0) / (h1 - h0) };
        r0 + (r1 - r0) * t
    }
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
}
