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
}

impl TextureKind {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            TextureKind::Ripple => "موجک (ریپل)",
            TextureKind::Weave => "تور بافته",
            TextureKind::Stitch => "بخیه‌ای",
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
    let [x, y, z] = point;
    let mut new_r_offset = 0.0; // آفست شعاعی جمع‌شده
    let mut new_z_offset = 0.0; // آفست عمودی جمع‌شده
    let phase = 0.0f64; // فاز هر بافت داخل حلقه محاسبه می‌شود

    for t in textures {
        let w = fade_weight(t, z_norm);
        if w <= 0.0 {
            continue;
        }
        let ph = t.phase_deg.to_radians();
        // فاز طول مسیر: چند دوره تا اینجا
        let cycles = s / t.period_mm;
        let u = cycles * 2.0 * std::f64::consts::PI;
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
    (out, collided)
}

/// بررسی «سلامت مسیر» پس از بافت: فاصلهٔ نقاط مجاور نباید از حدی بیشتر شود
/// (الزام: «اگر texture باعث جداشدن مسیر… هشدار بساز»)
/// خروجی: بیشینهٔ فاصلهٔ بین نقاط متوالی (mm)
pub fn max_gap_after_texture(points: &[[f64; 3]]) -> f64 {
    let mut max_gap = 0.0;
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
        let p = [10.0, 0.0, 50.0];
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let (p1, _) = apply_texture(p, 0.0, 10.0, 0.5, &[t.clone()]);
        let (p2, _) = apply_texture(p, 2.0, 10.0, 0.5, &[t]); // نیم‌دوره بعد
        let r1 = (p1[0] * p1[0] + p1[1] * p1[1]).sqrt();
        let r2 = (p2[0] * p2[0] + p2[1] * p2[1]).sqrt();
        // در s=0 سینوس صفر و در s=2 (نیم‌دوره) بیشینه است
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
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            radial: false,
            ..Default::default()
        };
        let p = [10.0, 0.0, 50.0];
        let (out, _) = apply_texture(p, 2.0, 10.0, 0.5, &[t]);
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
        let (out0, _) = apply_texture(p, 2.0, 10.0, 0.0, &[t.clone()]);
        assert_eq!(out0, p);
        // وسط فرم → وزن ۱
        let (out_mid, _) = apply_texture(p, 2.0, 10.0, 0.5, &[t]);
        assert!(out_mid != p);
    }

    #[test]
    fn test_collision_detection() {
        // بافت بزرگ‌تر از شعاع → برخورد
        let t = TextureParams {
            kind: TextureKind::Ripple,
            amplitude_mm: 20.0,
            period_mm: 4.0,
            ..Default::default()
        };
        let p = [5.0, 0.0, 50.0];
        // s=2 → مقدار منفی سینوس؟ s=2 با period 4 → نیم‌دوره → sin(π)=0
        // بهتر: s=3 → sin(3π/2)=-1 → r منفی
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
}
