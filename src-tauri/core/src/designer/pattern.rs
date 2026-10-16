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
    /// لوزی (diamond): شبکهٔ لوزی از برجستگی‌ها — |sin(kθ)|·|sin(2πf·zn)|
    Diamond,
    /// مهری (chevron): زیگزاگ مورب پیچیده دور فرم — موج مثلثی
    Chevron,
    /// حلقهٔ افقی (rings): برجستگی‌های افقی مستقل از θ (مثل دیوارهٔ داسی)
    Rings,
    /// فلس (scale): ردیف‌های مورب فلس‌های هم‌پوشان — |sin| قوسی
    Scale,
    /// تراش‌خورده (facet): مقطع چندضلعی منتظم با عمق کنترل‌شونده
    Facet,
}

impl PatternKind {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            PatternKind::Wave => "موج محوری",
            PatternKind::Petal => "گلبرگ (چندلوبی)",
            PatternKind::Twist => "پیچش",
            PatternKind::Flute => "شیار عمودی",
            PatternKind::Diamond => "لوزی",
            PatternKind::Chevron => "مهری (زیگزاگ)",
            PatternKind::Rings => "حلقهٔ افقی",
            PatternKind::Scale => "فلس",
            PatternKind::Facet => "تراش‌خورده",
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
        // گلبرگ: cos² شکل گلبرگ می‌سازد (لبه‌های تیز بین lobes)
        // ریاضی: cos²(x) دورهٔ π دارد و در هر دوره یک بیشینه؛ پس cos²(kθ/2)
        // در یک دور کامل (θ∈[0,2π)) دقیقاً k گلبرگ می‌سازد
        PatternKind::Petal => {
            let c = (k * theta * 0.5 + phase).cos();
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
        // لوزی: شبکهٔ لوزی — برجستگی جایی که هر دو |sin| بیشینه‌اند؛
        // گودال جایی که هر دو صفرند. قطر لوزی‌ها با count (زاویه‌ای) و
        // frequency (عمودی) کنترل می‌شود.
        PatternKind::Diamond => {
            let a = (k * theta + phase).sin().abs();
            let b = (2.0 * std::f64::consts::PI * p.frequency * zn).sin().abs();
            a + b - 1.0
        }
        // مهری: موج مثلثی مورب — tri(x) = (2/π)·asin(sin(x))
        // زیگزاگ تیز که با ارتفاع جابه‌جا می‌شود (چین‌مانند)
        PatternKind::Chevron => {
            let x = k * theta + phase + 2.0 * std::f64::consts::PI * p.frequency * zn;
            (2.0 / std::f64::consts::PI) * x.sin().asin()
        }
        // حلقهٔ افقی: فقط وابسته به z — برجستگی‌های حلقه‌ای مثل دیوارهٔ داسی.
        // count روی θ بی‌اثر است (صادقانه در UI توضیح داده می‌شود).
        PatternKind::Rings => {
            (2.0 * std::f64::consts::PI * p.frequency * zn + phase).sin()
        }
        // فلس: قوس‌های |sin| مورب — ردیف فلس‌های هم‌پوشان که با ارتفاع
        // جابه‌جا می‌شوند (شبیه زره فلس‌دار)
        PatternKind::Scale => {
            let x = k * theta + phase + 2.0 * std::f64::consts::PI * p.frequency * zn;
            x.sin().abs() * 2.0 - 1.0
        }
        // تراش‌خورده: مقطع k-ضلعی منتظم. منحنی چندضلعی:
        //   r(θ) = R·cos(π/k)/cos(m) که m فاصلهٔ زاویه‌ای از مرکز وجه است.
        // انحراف نرمال‌شده با amplitude (= عمق وجه)؛ گوشه‌ها روی شعاع پایه.
        // با frequency>0 وجه‌ها به‌آرامی با ارتفاع می‌چرخند (چندضلعی مارپیچ).
        PatternKind::Facet => {
            let kk = k.max(3.0); // مثلث حداقل چندضلعی معتبر
            let sector = 2.0 * std::f64::consts::PI / kk;
            // زاویهٔ مؤثر (با چرخش تدریجی بر اساس ارتفاع)
            let th_eff = theta + 2.0 * std::f64::consts::PI * p.frequency * zn;
            let m = ((th_eff + phase).rem_euclid(sector) - sector / 2.0).abs();
            // m ∈ [0, π/k] — فاصله از مرکز وجه؛ گوشه در m=π/k
            let denom = m.min(std::f64::consts::PI / kk).cos().max(1e-9);
            let ratio = (std::f64::consts::PI / kk).cos() / denom; // ∈ (0, 1]
            // انحراف ≤ 0 (وجه‌ها به داخل می‌نشینند) — نرمال‌سازی به عمق کامل
            -((1.0 - ratio) / (1.0 - (std::f64::consts::PI / kk).cos()).max(1e-9))
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
        for kind in [
            PatternKind::Wave, PatternKind::Petal, PatternKind::Twist, PatternKind::Flute,
            PatternKind::Diamond, PatternKind::Chevron, PatternKind::Rings,
            PatternKind::Scale, PatternKind::Facet,
        ]
        {
            let p = base_pattern(kind);
            let (r, _) = apply_patterns(20.0, 50.0, 0.5, 100.0, &[p]);
            // نمونه‌برداری از θ و z (Rings به θ وابسته نیست؛ به z وابسته است)
            let mut changed = false;
            'outer: for zi in 0..8 {
                for ti in 0..16 {
                    let theta = ti as f64 * 0.4;
                    let z = 5.0 + zi as f64 * 11.0;
                    let (rr, _) = apply_patterns(20.0, z, theta, 100.0, &[base_pattern(kind)]);
                    if (rr - 20.0).abs() > 1e-9 {
                        changed = true;
                        break 'outer;
                    }
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
        // آرگومان سینوس: 8θ + 2π·1·0.5 = 8θ + π؛ برای مقدار -1 → 8θ+π = 3π/2 → θ = π/16
        let mut p = base_pattern(PatternKind::Wave);
        p.amplitude_mm = 100.0; // از شعاع پایه بزرگ‌تر
        let theta = std::f64::consts::PI / 16.0;
        let (r, clamped) = apply_patterns(5.0, 50.0, theta, 100.0, &[p]);
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

    #[test]
    fn test_rings_independent_of_theta() {
        // حلقهٔ افقی باید مستقل از θ باشد
        let p = base_pattern(PatternKind::Rings);
        let (r1, _) = apply_patterns(20.0, 50.0, 0.3, 100.0, &[p.clone()]);
        let (r2, _) = apply_patterns(20.0, 50.0, 2.7, 100.0, &[p]);
        assert!((r1 - r2).abs() < 1e-9, "r1={r1} r2={r2}");
        // اما وابسته به z است
        let (r3, _) = apply_patterns(20.0, 25.0, 0.3, 100.0, &[base_pattern(PatternKind::Rings)]);
        assert!((r3 - r1).abs() > 0.1, "حلقه باید با z تغییر کند");
    }

    #[test]
    fn test_diamond_lattice_extremes() {
        // لوزی: هر دو |sin|=1 → بیشینه (+amplitude)؛ هر دو صفر → کمینه (-amplitude)
        let mut p = base_pattern(PatternKind::Diamond);
        p.amplitude_mm = 2.0;
        p.frequency = 2.0;
        // zn=0.25 → sin(2π·2·0.25)=sin(π)=0؛ k=8: θ=π/16 → sin(8·π/16)=sin(π/2)=1
        let (r_peak_side, _) = apply_patterns(20.0, 25.0, std::f64::consts::PI / 16.0, 100.0, &[p.clone()]);
        // a=1, b=0 → v=0 → r=20 (شبکهٔ لوزی: قلهٔ نه‌تایی نه گودال)
        assert!((r_peak_side - 20.0).abs() < 1e-9);
        // zn=0.25, θ=0 → a=0,b=0 → v=-1 → گودال کامل
        let (r_valley, _) = apply_patterns(20.0, 25.0, 0.0, 100.0, &[p.clone()]);
        assert!((r_valley - 18.0).abs() < 1e-9, "r_valley={r_valley}");
        // قلهٔ لوزی: زاویه‌ای بیشینه + عمودی بیشینه → v=+1
        // zn=0.125 → sin(2π·2·0.125)=sin(π/2)=1؛ θ=π/16 → a=1
        let (r_peak, _) = apply_patterns(20.0, 12.5, std::f64::consts::PI / 16.0, 100.0, &[p]);
        assert!((r_peak - 22.0).abs() < 1e-6, "r_peak={r_peak}");
    }

    #[test]
    fn test_chevron_bounded() {
        // موج مثلثی باید در [-1, 1] بماند (زیگزاگ تیز)
        let p = base_pattern(PatternKind::Chevron);
        for i in 0..200 {
            let theta = i as f64 * 0.031;
            let z = (i % 100) as f64;
            let (r, _) = apply_patterns(20.0, z, theta, 100.0, &[p.clone()]);
            assert!(r >= 20.0 - 1.5 - 1e-9 && r <= 20.0 + 1.5 + 1e-9, "r={r}");
        }
    }

    #[test]
    fn test_scale_scallops_are_positive_shifted() {
        // فلس: |sin|·2-1 ∈ [-1, 1]؛ در صفرهای sin گودال کامل
        let mut p = base_pattern(PatternKind::Scale);
        p.amplitude_mm = 1.0;
        p.frequency = 0.0;
        // k=8: θ=π/8 → sin(π)=0 → v=-1
        let (r_v, _) = apply_patterns(20.0, 50.0, std::f64::consts::PI / 8.0, 100.0, &[p.clone()]);
        // θ=π/16 → sin(π/2)=1 → v=+1
        let (r_p, _) = apply_patterns(20.0, 50.0, std::f64::consts::PI / 16.0, 100.0, &[p]);
        assert!((r_v - 19.0).abs() < 1e-9, "r_v={r_v}");
        assert!((r_p - 21.0).abs() < 1e-9, "r_p={r_p}");
    }

    #[test]
    fn test_facet_polygon_corners_and_faces() {
        // تراش‌خورده: گوشه‌های چندضلعی روی شعاع پایه، مرکز وجه به اندازهٔ
        // عمق (amplitude) به داخل می‌نشیند.
        // تقسیم‌بندی زاویه‌ای: مراکز وجه در θ = sector/2 + n·sector
        let mut p = base_pattern(PatternKind::Facet);
        p.amplitude_mm = 2.0;
        p.count = 6; // شش‌ضلعی → sector=60°
        p.frequency = 0.0;
        // θ=0 → m=π/6 → گوشه → r = 20
        let (r_corner, _) = apply_patterns(20.0, 50.0, 0.0, 100.0, &[p.clone()]);
        // θ=30° (π/6) → m=0 → مرکز وجه → r = 20-2 = 18
        let (r_face, _) = apply_patterns(20.0, 50.0, std::f64::consts::PI / 6.0, 100.0, &[p.clone()]);
        assert!((r_corner - 20.0).abs() < 1e-6, "r_corner={r_corner}");
        assert!((r_face - 18.0).abs() < 1e-6, "r_face={r_face}");
        // انحراف همیشه ≤ 0 (وجه‌ها فقط به داخل)
        for i in 0..72 {
            let theta = i as f64 * std::f64::consts::PI / 36.0;
            let (r, _) = apply_patterns(20.0, 50.0, theta, 100.0, &[p.clone()]);
            assert!(r <= 20.0 + 1e-9, "r={r} نباید از پایه بیشتر شود");
        }
    }

    #[test]
    fn test_facet_frequency_rotates() {
        // frequency>0 → وجه‌ها با ارتفاع می‌چرخند: همان θ در دو ارتفاع
        // نتایج متفاوت می‌دهد (z=25 دقیقاً روی گوشهٔ چرخیده می‌افتد)
        let mut p = base_pattern(PatternKind::Facet);
        p.count = 4;
        p.frequency = 1.0;
        let (r1, _) = apply_patterns(20.0, 10.0, 0.0, 100.0, &[p.clone()]);
        let (r2, _) = apply_patterns(20.0, 25.0, 0.0, 100.0, &[p]);
        assert!(
            (r1 - r2).abs() > 0.1,
            "چندضلعی باید با ارتفاع بچرخد: r1={r1} r2={r2}"
        );
    }
}
