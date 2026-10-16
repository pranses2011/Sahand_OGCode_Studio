//! ============================================================================
//! designer/pattern.rs — گام ۲: الگوهای هندسی (در STL می‌مانند)
//! ============================================================================
//! طبق الزام Docs/PROMPT_03 §4 گام ۲ و پژوهش مرجع:
//!   «الگو هندسهٔ مدل را تغییر می‌دهد؛ بنابراین اثرش در STL هم باقی می‌ماند.»
//!
//! مدل ریاضی (modifierهای composable طبق سند):
//!     r(z, θ) = r_profile(z) + Σᵢ patternᵢ(z, θ)
//! با clamp/validation برای جلوگیری از شعاع منفی.
//!
//! هر pattern دامنه، تعداد تکرار/فرکانس، فاز و بازهٔ z اعمال دارد.
//! بازه‌ی z با falloff خطی ساده اعمال می‌شود (صادقانه «range ساده»).
//! ============================================================================

use serde::{Deserialize, Serialize};

/// انواع الگوی هندسی — هرکدام فرمول پارامتری مستقل دارند
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternKind {
    /// موج محوری: برجستگی‌های سینوسی دور محور (twist-خور)
    Wave,
    /// گلبرگ/چندلوبی: lobes با فرم cos(k·θ)
    Petal,
    /// پیچش twist: چرخش زاویه‌ای وابسته به z (در واقع offset فاز موج)
    Twist,
    /// شیار عمودی fluted: کانال‌های عمودی مثل ستون کلاسیک
    Flute,
}

impl PatternKind {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            PatternKind::Wave => "موج محوری",
            PatternKind::Petal => "گلبرگ (چندلوبی)",
            PatternKind::Twist => "پیچش",
            PatternKind::Flute => "شیار عمودی",
        }
    }
}

/// پارامترهای یک الگوی فعال
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternParams {
    pub kind: PatternKind,
    /// دامنه/عمق الگو (mm) — به شعاع اضافه می‌شود
    pub amplitude_mm: f64,
    /// تعداد تکرار دور محور (برای Wave/Petal/Flute: تعداد lobes)
    pub count: u32,
    /// فرکانس عمودی برای Wave/Twist (چند دوره در کل ارتفاع)
    pub frequency: f64,
    /// فاز اولیه (درجه)
    pub phase_deg: f64,
    /// شروع بازهٔ اعمال z (mm)
    pub z_start_mm: f64,
    /// پایان بازهٔ اعمال (None = تا بالای فرم)
    pub z_end_mm: Option<f64>,
}

impl Default for PatternParams {
    fn default() -> Self {
        Self {
            kind: PatternKind::Wave,
            amplitude_mm: 1.5,
            count: 8,
            frequency: 1.0,
            phase_deg: 0.0,
            z_start_mm: 0.0,
            z_end_mm: None,
        }
    }
}

/// اعتبارسانی پارامترها — گارد NaN/منطقی
pub fn validate_pattern(p: &PatternParams, total_height: f64) -> Result<(), String> {
    if !p.amplitude_mm.is_finite() {
        return Err("دامنهٔ الگو باید عدد معتبر باشد".into());
    }
    if !p.frequency.is_finite() || p.frequency < 0.0 {
        return Err("فرکانس الگو نامعتبر است".into());
    }
    if p.z_start_mm < 0.0 || p.z_start_mm >= total_height {
        return Err("شروع بازهٔ الگو خارج از فرم است".into());
    }
    if let Some(z_end) = p.z_end_mm {
        if z_end <= p.z_start_mm {
            return Err("پایان بازهٔ الگو باید بعد از شروع آن باشد".into());
        }
    }
    Ok(())
}

/// وزن بازهٔ z با falloff خطی ساده:
/// داخل بازه ۱.۰؛ خارج بازه ۰.۰؛ لبه‌ها با گذار نرم ۱۰٪ ارتفاع فرم
/// (صادقانه: «یک range ساده با falloff» — نه effector پیچیده)
fn range_weight(p: &PatternParams, z: f64, total_height: f64) -> f64 {
    let z_end = p.z_end_mm.unwrap_or(total_height);
    if z <= p.z_start_mm || z >= z_end {
        // داخل گذار لبه؟
        let fade = (total_height * 0.05).max(1.0);
        let d_start = p.z_start_mm - z; // مثبت وقتی زیر شروع هستیم
        let d_end = z - z_end; // مثبت وقتی بالای پایان هستیم
        let d = d_start.max(d_end);
        if d >= fade {
            return 0.0;
        }
        return 1.0 - d / fade;
    }
    // داخل بازه — گذار لبهٔ داخلی
    let fade = (total_height * 0.05).max(1.0);
    let from_start = (z - p.z_start_mm).min(fade);
    let to_end = (z_end - z).min(fade);
    (from_start.min(to_end) / fade).clamp(0.0, 1.0)
}

/// مقدار یک الگو در (z, θ) — خروجی به mm (به شعاع اضافه می‌شود)
/// θ رادیان، z ارتفاع mm، total_height ارتفاع کل فرم برای نرمال‌سازی
fn pattern_value(p: &PatternParams, z: f64, theta: f64, total_height: f64) -> f64 {
    let w = range_weight(p, z, total_height);
    if w <= 0.0 {
        return 0.0;
    }
    let phase = p.phase_deg.to_radians();
    let zn = (z / total_height.max(1e-9)).clamp(0.0, 1.0); // z نرمال‌شده
    let k = p.count.max(1) as f64;
    let v = match p.kind {
        // موج محوری: برجستگی سینوسی که با ارتفاع هم موج دارد
        PatternKind::Wave => {
            (k * theta + phase + 2.0 * std::f64::consts::PI * p.frequency * zn).sin()
        }
        // گلبرگ: |cos|^n شکل گلبرگ می‌سازد (لبه‌های تیز بین lobes)
        PatternKind::Petal => {
            let c = (k * theta + phase).cos();
            // توان ۲ گلبرگ‌های نرم‌تر می‌سازد
            c * c
        }
        // پیچش: offset شعاعی که با ارتفاع می‌چرخد (توهم تاب خوردن)
        PatternKind::Twist => {
            let twisted = k * theta + phase + 2.0 * std::f64::consts::PI * p.frequency * zn;
            // موج سینوسی با چرخش تدریجی
            twisted.sin() * 0.5 + twisted.cos() * 0.5
        }
        // شیار عمودی: کانال‌های ثابت در θ (مثل ستون)
        PatternKind::Flute => {
            // موج با پلهٔ نرم: کانال‌های گود
            let c = (k * theta + phase).cos();
            // نگاشت به بازهٔ [-1, 1] با گودی بیشتر
            c * c * 2.0 - 1.0
        }
    };
    v * p.amplitude_mm * w
}

/// اعمال همهٔ الگوها روی شعاع پایه — قلب composable بودن
/// خروجی: شعاع نهایی (همیشه ≥ 0 با clamp هشداردهنده)
/// برگرداندن negative_clamped برای تشخیص هشدار «مقطع نامعتبر»
pub fn apply_patterns(
    base_radius: f64,
    z: f64,
    theta: f64,
    total_height: f64,
    patterns: &[PatternParams],
) -> (f64, bool) {
    let mut r = base_radius;
    for p in patterns {
        r += pattern_value(p, z, theta, total_height);
    }
    // clamp شعاع منفی (الزام: «clamp/validation برای شعاع منفی»)
    if r < 0.0 {
        return (0.0, true); // clamped=true → هشدار
    }
    (r, false)
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn base_pattern(kind: PatternKind) -> PatternParams {
        PatternParams { kind, ..Default::default() }
    }

    #[test]
    fn test_pattern_affects_radius() {
        // هر نوع الگو باید شعاع را در برخی زاویه‌ها تغییر دهد
        for kind in [PatternKind::Wave, PatternKind::Petal, PatternKind::Twist, PatternKind::Flute]
        {
            let p = base_pattern(kind);
            let (r, _) = apply_patterns(20.0, 50.0, 0.5, 100.0, &[p]);
            // در برخی θ تغییر می‌کند (تست چند θ)
            let mut changed = false;
            for i in 0..16 {
                let theta = i as f64 * 0.4;
                let (rr, _) = apply_patterns(20.0, 50.0, theta, 100.0, &[base_pattern(kind)]);
                if (rr - 20.0).abs() > 1e-9 {
                    changed = true;
                    break;
                }
            }
            assert!(changed, "{kind:?} هیچ اثری نداشت");
            let _ = r;
        }
    }

    #[test]
    fn test_zero_amplitude_no_effect() {
        // دامنهٔ صفر → بی‌اثر
        let mut p = base_pattern(PatternKind::Petal);
        p.amplitude_mm = 0.0;
        let (r, _) = apply_patterns(20.0, 50.0, 0.5, 100.0, &[p]);
        assert_eq!(r, 20.0);
    }

    #[test]
    fn test_negative_radius_clamped() {
        // شعاع منفی باید clamp شود و هشدار برگردد
        let mut p = base_pattern(PatternKind::Wave);
        p.amplitude_mm = 100.0; // از شعاع پایه بزرگ‌تر
        let (r, clamped) = apply_patterns(5.0, 50.0, 3.14159, 100.0, &[p]);
        assert_eq!(r, 0.0);
        assert!(clamped);
    }

    #[test]
    fn test_z_range_respected() {
        // خارج از بازهٔ z اثر صفر است
        let mut p = base_pattern(PatternKind::Petal);
        p.z_start_mm = 40.0;
        p.z_end_mm = Some(60.0);
        // پایین فرم (z=10) عملاً بدون اثر (خارج بازه + دور از falloff)
        let (r_low, _) = apply_patterns(20.0, 10.0, 0.7, 100.0, &[p.clone()]);
        // داخل بازه اثر دارد
        let (r_mid, _) = apply_patterns(20.0, 50.0, 0.7, 100.0, &[p]);
        assert!((r_low - 20.0).abs() < 0.1, "r_low = {r_low}");
        assert!((r_mid - 20.0).abs() > 0.01, "r_mid = {r_mid}");
    }

    #[test]
    fn test_patterns_composable() {
        // دو الگو با هم → مجموع اثرها (الزام composable)
        let p1 = base_pattern(PatternKind::Wave);
        let p2 = base_pattern(PatternKind::Petal);
        let (r_both, _) = apply_patterns(20.0, 50.0, 0.9, 100.0, &[p1.clone(), p2.clone()]);
        let (r1, _) = apply_patterns(20.0, 50.0, 0.9, 100.0, &[p1]);
        let (r2, _) = apply_patterns(20.0, 50.0, 0.9, 100.0, &[p2]);
        assert!((r_both - r1 - r2 + 20.0).abs() < 1e-9, "مجموع خطی نیست");
    }

    #[test]
    fn test_validate_rejects_bad_range() {
        let mut p = base_pattern(PatternKind::Wave);
        p.z_start_mm = 90.0;
        p.z_end_mm = Some(80.0); // معکوس
        assert!(validate_pattern(&p, 100.0).is_err());
        // شروع خارج از فرم
        let mut p2 = base_pattern(PatternKind::Wave);
        p2.z_start_mm = 150.0;
        assert!(validate_pattern(&p2, 100.0).is_err());
        // سالم
        let p3 = base_pattern(PatternKind::Wave);
        assert!(validate_pattern(&p3, 100.0).is_ok());
    }

    #[test]
    fn test_petal_lobe_count() {
        // تعداد گلبرگ باید با count کنترل شود: بیشینه‌های محلی
        let mut p = base_pattern(PatternKind::Petal);
        p.count = 6;
        p.amplitude_mm = 2.0;
        // نمونه‌برداری ۳۶۰ درجه و شمارش بیشینه‌های محلی
        let n = 720;
        let vals: Vec<f64> = (0..=n)
            .map(|i| {
                let theta = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                let (r, _) = apply_patterns(20.0, 50.0, theta, 100.0, &[p.clone()]);
                r
            })
            .collect();
        let mut peaks = 0;
        for i in 1..n {
            if vals[i] > vals[i - 1] && vals[i] >= vals[i + 1] && (vals[i] - 20.0).abs() > 1e-6 {
                peaks += 1;
            }
        }
        // ۶ گلبرگ در دور کامل (تقریباً — لبه‌های sampling)
        assert!(peaks >= 5 && peaks <= 7, "peaks = {peaks}");
    }

    #[test]
    fn test_flute_is_vertical() {
        // شیار باید مستقل از z باشد (ستون عمودی)
        let p = base_pattern(PatternKind::Flute);
        let (r1, _) = apply_patterns(20.0, 30.0, 0.8, 100.0, &[p.clone()]);
        let (r2, _) = apply_patterns(20.0, 70.0, 0.8, 100.0, &[p]);
        assert!((r1 - r2).abs() < 1e-6, "شیار نباید با z تغییر کند");
    }
}
