//! ============================================================================
//! toolpath/arcs.rs — قوس‌های G2/G3 و تبدیل آن‌ها به polyline
//! ============================================================================
//! چرا قوس جدا از مفسر است؟ (Docs/PROMPT_02 §4)
//!   - قوس «هندسهٔ خالص» است و باید با tolerance قابل‌تنظیم به قطعات خطی
//!     (tessellation) تبدیل شود؛ این منطق باید مستقل و unit-test شدنی باشد.
//!   - هر قطعهٔ حاصل به «خط مبدأ» نسبت داده می‌شود تا انتخاب خط، کل قوس را
//!     در viewport برجسته کند.
//!
//! پشتیبانی:
//!   - فرم مرکزی I/J (یا I/K و J/K بسته به plane) + تشخیص قوس کامل
//!   - فرم شعاعی R با قاعدهٔ علامت (R مثبت → کمان کوچک، R منفی → کمان بزرگ)
//!   - سه صفحهٔ G17 (XY) و G18 (ZX) و G19 (YZ) با نگاشت محورها
//!   - حرکت مارپیچی (تغییر محور سوم به‌صورت خطی روی طول قوس)
//!   - خطای شعاع نامعتبر: فاصلهٔ دو سر از مرکز ناهم‌خوان یا d > 2|R|
//! ============================================================================

use serde::{Deserialize, Serialize};

/// صفحهٔ قوس بر اساس فرمان‌های modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plane {
    /// G17 — صفحهٔ XY (پیش‌فرض)، محور سوم Z
    Xy,
    /// G18 — صفحهٔ ZX، محور سوم Y
    Zx,
    /// G19 — صفحهٔ YZ، محور سوم X
    Yz,
}

impl Plane {
    /// استخراج مختصات صفحهٔ (u,v) از نقطهٔ سه‌بعدی
    /// ترتیب به‌گونه‌ای انتخاب شده که «چرخش مثبت u→v» همیشه پادساعت‌گردِ
    /// دیده‌شده از محور مثبتِ سوم باشد (استاندارد NIST).
    pub fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        match self {
            Plane::Xy => (p[0], p[1]),
            Plane::Zx => (p[2], p[0]),
            Plane::Yz => (p[1], p[2]),
        }
    }

    /// ساخت نقطهٔ سه‌بعدی از (u,v) و مقدار محور سوم w
    pub fn from_uv_w(&self, u: f64, v: f64, w: f64) -> [f64; 3] {
        match self {
            Plane::Xy => [u, v, w],
            Plane::Zx => [v, w, u],
            Plane::Yz => [w, u, v],
        }
    }

    /// مقدار محور سوم نقطه
    pub fn w(&self, p: [f64; 3]) -> f64 {
        match self {
            Plane::Xy => p[2],
            Plane::Zx => p[1],
            Plane::Yz => p[0],
        }
    }
}

/// پارامترهای کامل یک قوس
#[derive(Debug, Clone)]
pub struct ArcParams {
    /// نقطهٔ شروع (mm)
    pub start: [f64; 3],
    /// نقطهٔ پایان (mm)
    pub end: [f64; 3],
    /// صفحهٔ قوس
    pub plane: Plane,
    /// جهت: true = G2 (ساعت‌گرد)، false = G3 (پادساعت‌گرد)
    pub clockwise: bool,
    /// آفست مرکز نسبت به مبدأ به مختصات (u,v) — فرم I/J
    pub center_uv: Option<(f64, f64)>,
    /// شعاع مستقیم R (مثبت: کمان کوچک، منفی: کمان بزرگ)
    pub radius: Option<f64>,
    /// خطای مجاز انحراف chord از قوس واقعی (mm) — قابل‌تنظیم از UI
    pub tolerance_mm: f64,
    /// سقف تعداد قطعات برای محافظت از حافظه
    pub max_segments: usize,
}

/// خطاهای مشخص قوس — مفسر این‌ها را به Diagnostic فارسی تبدیل می‌کند
#[derive(Debug, Clone, PartialEq)]
pub enum ArcError {
    /// نه I/J/K داده شده و نه R
    MissingCenterOrRadius,
    /// فاصلهٔ دو سر بیشتر از قطر است: d > 2|R|
    RadiusTooSmall { distance: f64, radius: f64 },
    /// در فرم I/J فاصلهٔ سر و ته از مرکز ناهم‌خوان است
    EndpointRadiusMismatch { start_r: f64, end_r: f64 },
    /// قوس R با مبدأ و مقصد یکسان قابل تعریف نیست
    DegenerateR,
}

/// نتیجهٔ موفق tessellation
#[derive(Debug, Clone)]
pub struct ArcResult {
    /// نقاط مسیر «بدون نقطهٔ شروع» و «با نقطهٔ پایان» (mm)
    pub points: Vec<[f64; 3]>,
    /// مرکز قوس در مختصات صفحهٔ (u,v)
    pub center_uv: (f64, f64),
    /// شعاع مؤثر (mm)
    pub radius: f64,
    /// زاویهٔ جاروب (رادیان؛ علامت جهت را نشان می‌دهد)
    pub sweep: f64,
    /// آیا قوس کامل (۳۶۰ درجه) بود
    pub is_full_circle: bool,
}

/// محاسبهٔ مرکز قوس از فرم R (روش استاندارد GRBL/Marlin)
/// دو مرکز نامزد روی عمودمنصفِ وتر وجود دارد؛ علامت R و جهت چرخش یکی را
/// انتخاب می‌کند: R مثبت → کمان ≤ ۱۸۰°، R منفی → کمان ≥ ۱۸۰°
fn center_from_r(
    su: f64,
    sv: f64,
    eu: f64,
    ev: f64,
    radius: f64,
    clockwise: bool,
) -> Result<(f64, f64, f64, f64), ArcError> {
    let du = eu - su;
    let dv = ev - sv;
    let distance = (du * du + dv * dv).sqrt();
    let r_abs = radius.abs();

    // دو سر از هم دورتر از قطرند → قوس وجود ندارد
    if distance > 2.0 * r_abs + 1e-9 {
        return Err(ArcError::RadiusTooSmall { distance, radius });
    }
    // مبدأ و مقصد یکسان → مرکز تعریف‌شده نیست
    if distance < 1e-9 {
        return Err(ArcError::DegenerateR);
    }

    // فاصلهٔ مرکز تا وسط وتر (قضیهٔ فیثاغورس)
    let half_d = distance / 2.0;
    let h = (r_abs * r_abs - half_d * half_d).max(0.0).sqrt();
    // بردار یکهٔ عمود بر وتر (چرخش +۹۰ درجه)
    let perp_u = -dv / distance;
    let perp_v = du / distance;

    // دو مرکز نامزد
    let mid_u = su + du / 2.0;
    let mid_v = sv + dv / 2.0;
    let c1 = (mid_u + perp_u * h, mid_v + perp_v * h);
    let c2 = (mid_u - perp_u * h, mid_v - perp_v * h);

    // انتخاب نامزد درست بر اساس جهت و علامت R:
    // برای هر نامزد، جاروب جهت‌خواسته را حساب می‌کنیم و کمان ≤ ۱۸۰° یا > ۱۸۰°
    // را طبق علامت R انتخاب می‌کنیم (روش مستقل از فرمول‌های علامتِ پیچیده)
    let minor_arc = radius > 0.0;
    for (cu, cv) in [c1, c2] {
        let sweep = compute_sweep(su, sv, eu, ev, cu, cv, clockwise);
        let abs_sweep = sweep.abs();
        let is_minor = abs_sweep <= std::f64::consts::PI + 1e-9;
        if is_minor == minor_arc {
            let r_start = ((su - cu).powi(2) + (sv - cv).powi(2)).sqrt();
            let r_end = ((eu - cu).powi(2) + (ev - cv).powi(2)).sqrt();
            return Ok((cu, cv, r_start, r_end));
        }
    }
    // حالت دقیقاً ۱۸۰°: هر دو نامزد کمان نیم‌دورند؛ اولی را می‌گیریم
    let (cu, cv) = c1;
    let r_start = ((su - cu).powi(2) + (sv - cv).powi(2)).sqrt();
    let r_end = ((eu - cu).powi(2) + (ev - cv).powi(2)).sqrt();
    Ok((cu, cv, r_start, r_end))
}

/// محاسبهٔ زاویهٔ جاروب با علامت جهت
/// خروجی: مثبت = پادساعت‌گرد، منفی = ساعت‌گرد، ±2π = قوس کامل
fn compute_sweep(
    su: f64,
    sv: f64,
    eu: f64,
    ev: f64,
    cu: f64,
    cv: f64,
    clockwise: bool,
) -> f64 {
    let a0 = (sv - cv).atan2(su - cu); // زاویهٔ شروع
    let a1 = (ev - cv).atan2(eu - cu); // زاویهٔ پایان
    let mut sweep = a1 - a0;

    if clockwise {
        // ساعت‌گرد: جاروب باید منفی باشد؛ اگر مثبت یا صفر شد یک دور کامل کم می‌کنیم
        if sweep > -1e-9 {
            sweep -= 2.0 * std::f64::consts::PI;
        }
    } else {
        // پادساعت‌گرد: جاروب باید مثبت باشد؛ اگر منفی یا صفر شد یک دور اضافه می‌کنیم
        if sweep < 1e-9 {
            sweep += 2.0 * std::f64::consts::PI;
        }
    }
    sweep
}

/// تبدیل قوس به دنبالهٔ نقاط خطی (tessellation)
/// خطاها با ArcError برمی‌گردند تا مفسر Diagnostic فارسی بسازد (نه panic!)
pub fn tessellate_arc(params: &ArcParams) -> Result<ArcResult, ArcError> {
    let (su, sv) = params.plane.uv(params.start);
    let (eu, ev) = params.plane.uv(params.end);
    let w_start = params.plane.w(params.start);
    let w_end = params.plane.w(params.end);

    // ---------- تعیین مرکز، شعاع و زاویهٔ جاروب ----------
    let (cu, cv, r_start, r_end, mut sweep, is_full) = if let Some((iu, iv)) =
        params.center_uv
    {
        // فرم مرکزی: مرکز = مبدأ + آفست
        let cu = su + iu;
        let cv = sv + iv;
        let r_start = ((su - cu).powi(2) + (sv - cv).powi(2)).sqrt();
        let r_end = ((eu - cu).powi(2) + (ev - cv).powi(2)).sqrt();
        // آیا دو سر روی شعاع یکسان‌اند؟ تلورانس ترکیبی نسبی+مطلق
        let mismatch = (r_start - r_end).abs();
        let allowed = 0.005 * r_start.max(r_end) + 0.05;
        if mismatch > allowed {
            return Err(ArcError::EndpointRadiusMismatch { start_r: r_start, end_r: r_end });
        }
        // قوس کامل: مبدأ و مقصد یکی‌اند (قرارداد Marlin/RepRap)
        let same_point = (su - eu).abs() < 1e-9 && (sv - ev).abs() < 1e-9;
        let sweep = if same_point {
            // جهت چرخش مقدار جاروب را تعیین می‌کند
            if params.clockwise {
                -2.0 * std::f64::consts::PI
            } else {
                2.0 * std::f64::consts::PI
            }
        } else {
            compute_sweep(su, sv, eu, ev, cu, cv, params.clockwise)
        };
        (cu, cv, r_start, r_end, sweep, same_point)
    } else if let Some(r) = params.radius {
        // فرم شعاعی R
        let (cu, cv, r_start, r_end) =
            center_from_r(su, sv, eu, ev, r, params.clockwise)?;
        let sweep = compute_sweep(su, sv, eu, ev, cu, cv, params.clockwise);
        (cu, cv, r_start, r_end, sweep, false)
    } else {
        return Err(ArcError::MissingCenterOrRadius);
    };

    // گارد NaN/Infinity: هیچ قوس بدشکل نباید به هندسه نشت کند
    if !sweep.is_finite() || !r_start.is_finite() || !cu.is_finite() || !cv.is_finite() {
        return Err(ArcError::MissingCenterOrRadius);
    }

    // ---------- تعداد قطعات از روی تلورانس chord ----------
    // خطای chord برای زاویهٔ گام θ برابر است با r(1 − cos(θ/2))؛
    // پس θ_max = 2·acos(1 − tol/r)
    let r_eff = r_start;
    let tol = params.tolerance_mm.max(1e-4);
    let theta_max = if r_eff <= tol {
        std::f64::consts::PI / 2.0 // شعاع خیلی کوچک: قطعات ۹۰ درجه
    } else {
        let cos_arg = (1.0 - tol / r_eff).clamp(-1.0, 1.0);
        2.0 * cos_arg.acos().max(1e-3)
    };
    let wanted = (sweep.abs() / theta_max).ceil() as usize;
    // سقف محافظتی + حداقل یک قطعه
    let n = wanted.clamp(1, params.max_segments.max(1));

    // ---------- تولید نقاط ----------
    let a0 = (sv - cv).atan2(su - cu);
    let w_delta = w_end - w_start;
    let mut points = Vec::with_capacity(n);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let angle = a0 + sweep * t;
        // شعاع را روی طول قوس خطی درون‌یابی می‌کنیم تا اگر سر و ته
        // اختلاف جزئی شعاع داشتند، انتها دقیقاً به مقصد برسد
        let r_i = r_start + (r_end - r_start) * t;
        let u = cu + r_i * angle.cos();
        let v = cv + r_i * angle.sin();
        // محور سوم به‌صورت خطی روی طول قوس تغییر می‌کند (حرکت مارپیچی)
        let w = w_start + w_delta * t;
        points.push(params.plane.from_uv_w(u, v, w));
    }
    // نقطهٔ آخر باید دقیقاً مقصد باشد (خطای گرد شدن float را حذف می‌کنیم)
    if let Some(last) = points.last_mut() {
        *last = params.end;
    }

    // قوس کامل با فرم R منطقی نیست؛ فقط I/J پشتیبانی می‌شود (در بالا مدیریت شد)
    if is_full {
        sweep = if params.clockwise {
            -2.0 * std::f64::consts::PI
        } else {
            2.0 * std::f64::consts::PI
        };
    }

    Ok(ArcResult {
        points,
        center_uv: (cu, cv),
        radius: r_eff,
        sweep,
        is_full_circle: is_full,
    })
}

// ============================================================================
// آزمون‌های واحد قوس — شامل fixtureهای سند
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// پارامترهای پیش‌فرض آزمون
    fn base(start: [f64; 3], end: [f64; 3], clockwise: bool) -> ArcParams {
        ArcParams {
            start,
            end,
            plane: Plane::Xy,
            clockwise,
            center_uv: None,
            radius: None,
            tolerance_mm: 0.05,
            max_segments: 1000,
        }
    }

    #[test]
    fn test_g2_semicircle_fixture() {
        // fixture: از (30,40) با I5 J0 → مرکز (35,40)؛ ساعت‌گرد تا (40,40)
        let mut p = base([30.0, 40.0, 0.2], [40.0, 40.0, 0.2], true);
        p.center_uv = Some((5.0, 0.0));
        let r = tessellate_arc(&p).unwrap();
        // نقطهٔ آخر دقیقاً مقصد است
        assert_eq!(r.points.last().unwrap(), &[40.0, 40.0, 0.2]);
        // قوس باید از نقطهٔ بالای مرکز (35,45) بگذرد (نزدیک‌ترین نقطه)
        let nearest = r
            .points
            .iter()
            .min_by(|a, b| {
                let da = (a[0] - 35.0).powi(2) + (a[1] - 45.0).powi(2);
                let db = (b[0] - 35.0).powi(2) + (b[1] - 45.0).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
        assert!((nearest[0] - 35.0).abs() < 0.3, "nearest = {nearest:?}");
        assert!((nearest[1] - 45.0).abs() < 0.3, "nearest = {nearest:?}");
        // جاروب نیم‌دور ساعت‌گرد (منفی)
        assert!((r.sweep.abs() - std::f64::consts::PI).abs() < 1e-6);
    }

    #[test]
    fn test_g3_semicircle_fixture() {
        // fixture: از (40,40) با I-5 J0 → مرکز (35,40)؛ پادساعت‌گرد تا (30,40)
        let mut p = base([40.0, 40.0, 0.2], [30.0, 40.0, 0.2], false);
        p.center_uv = Some((-5.0, 0.0));
        let r = tessellate_arc(&p).unwrap();
        assert_eq!(r.points.last().unwrap(), &[30.0, 40.0, 0.2]);
        // پادساعت‌گرد از 0° تا 180° هم از بالای مرکز می‌گذرد
        let nearest = r
            .points
            .iter()
            .min_by(|a, b| {
                let da = (a[0] - 35.0).powi(2) + (a[1] - 45.0).powi(2);
                let db = (b[0] - 35.0).powi(2) + (b[1] - 45.0).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
        assert!((nearest[1] - 45.0).abs() < 0.3, "nearest = {nearest:?}");
        assert!((r.sweep - std::f64::consts::PI).abs() < 1e-6);
    }

    #[test]
    fn test_full_circle_with_ij() {
        // مبدأ = مقصد + I/J → قوس کامل ۳۶۰ درجه
        let mut p = base([10.0, 0.0, 0.2], [10.0, 0.0, 0.2], false);
        p.center_uv = Some((-10.0, 0.0)); // مرکز (0,0)
        let r = tessellate_arc(&p).unwrap();
        assert!(r.is_full_circle);
        assert!((r.sweep - 2.0 * std::f64::consts::PI).abs() < 1e-9);
        // نقطهٔ پایانی = نقطهٔ شروع
        assert_eq!(r.points.last().unwrap(), &[10.0, 0.0, 0.2]);
    }

    #[test]
    fn test_r_form_minor_arc() {
        // R مثبت → کمان کوچک (کمتر از ۱۸۰°)
        let mut p = base([0.0, 0.0, 0.0], [10.0, 10.0, 0.0], false);
        p.radius = Some(8.0);
        let r = tessellate_arc(&p).unwrap();
        assert!(r.sweep.abs() < std::f64::consts::PI + 1e-6);
        assert!(r.sweep > 0.0); // پادساعت‌گرد
        // شعاع واقعی: مرکز روی یکی از دو نقطهٔ (10-... ) — فقط سازگاری می‌سنجیم
        let (cu, cv) = r.center_uv;
        let d_start = ((cu - 0.0).powi(2) + (cv - 0.0).powi(2)).sqrt();
        assert!((d_start - 8.0).abs() < 1e-9);
    }

    #[test]
    fn test_r_form_major_arc() {
        // R منفی → کمان بزرگ (بیشتر از ۱۸۰°)؛ دو سر باید کمتر از ۲|R| فاصله داشته باشند
        let mut p = base([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], false);
        p.radius = Some(-6.0); // فاصلهٔ ۱۰ < قطر ۱۲ → کمان بزرگ ممکن است
        let r = tessellate_arc(&p).unwrap();
        assert!(r.sweep > std::f64::consts::PI, "sweep = {}", r.sweep);
    }

    #[test]
    fn test_radius_too_small_error() {
        // دو سر ۱۰mm از هم و R=4 → d > 2R → خطا
        let mut p = base([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], false);
        p.radius = Some(4.0);
        assert!(matches!(
            tessellate_arc(&p),
            Err(ArcError::RadiusTooSmall { .. })
        ));
    }

    #[test]
    fn test_endpoint_mismatch_error() {
        // مرکز به‌گونه‌ای که فاصلهٔ سر و ته ناهم‌خوان است (شعاع نامعتبر)
        let mut p = base([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], false);
        p.center_uv = Some((-2.0, 0.0)); // مرکز (-2,0): r_start=2, r_end=12
        assert!(matches!(
            tessellate_arc(&p),
            Err(ArcError::EndpointRadiusMismatch { .. })
        ));
    }

    #[test]
    fn test_missing_params_error() {
        // بدون I/J و بدون R
        let p = base([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], false);
        assert!(matches!(tessellate_arc(&p), Err(ArcError::MissingCenterOrRadius)));
    }

    #[test]
    fn test_degenerate_r() {
        // R با مبدأ و مقصد یکسان → تعریف‌نشده
        let mut p = base([5.0, 5.0, 0.0], [5.0, 5.0, 0.0], true);
        p.radius = Some(5.0);
        assert!(matches!(tessellate_arc(&p), Err(ArcError::DegenerateR)));
    }

    #[test]
    fn test_helical_z_interpolation() {
        // قوس با تغییر Z (مارپیچ): z باید از 0.2 به 1.2 به‌صورت خطی برسد
        // مرکز مطلق (0,0) → آفست از مبدأ (0,10) برابر (0,-10) است
        let mut p = base([0.0, 10.0, 0.2], [0.0, -10.0, 1.2], false);
        p.center_uv = Some((0.0, -10.0));
        let r = tessellate_arc(&p).unwrap();
        // نیم‌دور پادساعت‌گرد از 90° تا 270°
        assert!((r.sweep - std::f64::consts::PI).abs() < 1e-6);
        // z باید صعودی خطی باشد از حدود 0.2 تا دقیقاً 1.2
        let z_first = r.points.first().unwrap()[2];
        let z_last = r.points.last().unwrap()[2];
        assert!(z_first > 0.2 && z_first < 0.4, "z_first = {z_first}");
        assert!((z_last - 1.2).abs() < 1e-12);
        for w in r.points.windows(2) {
            assert!(w[1][2] > w[0][2], "z باید صعودی باشد");
        }
        // نقطهٔ میانی باید z حدود وسط (0.6 تا 0.8) داشته باشد
        let mid = r.points[r.points.len() / 2];
        assert!(mid[2] > 0.5 && mid[2] < 0.9, "mid z = {}", mid[2]);
        // شعاع در صفحهٔ XY باید ۱۰ بماند
        for pt in &r.points {
            let rad = (pt[0] * pt[0] + pt[1] * pt[1]).sqrt();
            assert!((rad - 10.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_g18_plane_mapping() {
        // صفحهٔ ZX: قوس در (z,x)؛ محور سوم y
        let p = ArcParams {
            start: [10.0, 5.0, 0.0], // u=z=0, v=x=10, w=y=5
            end: [10.0, 5.0, 0.0],   // قوس کامل → بدون تغییر هیچ محوری نمی‌سنجیم؛ فقط نگاشت
            plane: Plane::Zx,
            clockwise: false,
            center_uv: Some((-0.0, -10.0)), // مرکز (u=0, v=0)
            radius: None,
            tolerance_mm: 0.05,
            max_segments: 64,
        };
        let r = tessellate_arc(&p).unwrap();
        assert!(r.is_full_circle);
        // همهٔ نقاط باید y=5 ثابت و شعاع در (z,x) برابر 10 بمانند
        for pt in &r.points {
            assert!((pt[1] - 5.0).abs() < 1e-9);
            let rad = (pt[0] * pt[0] + pt[2] * pt[2]).sqrt();
            assert!((rad - 10.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_tolerance_controls_resolution() {
        // تلورانس کوچک‌تر → قطعات بیشتر
        let mut p = base([0.0, 0.0, 0.0], [0.0, 20.0, 0.0], false);
        p.center_uv = Some((0.0, 10.0)); // نیم‌دور r=10
        let fine = {
            p.tolerance_mm = 0.01;
            tessellate_arc(&p).unwrap().points.len()
        };
        let coarse = {
            p.tolerance_mm = 0.5;
            tessellate_arc(&p).unwrap().points.len()
        };
        assert!(fine > coarse, "fine={fine} coarse={coarse}");
    }

    #[test]
    fn test_max_segments_cap() {
        // سقف قطعات رعایت می‌شود حتی با تلورانس خیلی ریز
        let mut p = base([0.0, 0.0, 0.0], [0.0, 20.0, 0.0], false);
        p.center_uv = Some((0.0, 10.0));
        p.tolerance_mm = 1e-6;
        p.max_segments = 32;
        assert_eq!(tessellate_arc(&p).unwrap().points.len(), 32);
    }
}
