//! ============================================================================
//! designer/text2vase.rs — تبدیل متن به گلدان/حروف سه‌بعدی حالت مارپیچی
//! ============================================================================
//! نسخهٔ ۱٫۷ — درخواست کاربر ۱۱:
//!   «می‌خوام متن بنویسم و اون رو تبدیل کنه به حالت vase mode و خودش بتونه
//!    به بهترین نحو حروف جدا از هم رو به هم متصل بکنه بطوری که محل وصل
//!    شدنشون تو چشم نباشه و تنظیمات کامل و حرفه‌ای برای این بخش هم میخوام
//!    به همراه پشتیبانی از فونت های ویندوز و زبان فارسی که حروف جدا از
//!    هم و برعکس نشن»
//!
//! معماری «منبع واحد» (سازگار با بقیهٔ برنامه):
//!   * فرانت‌اند متن را با «هر فونتی از ویندوز» (از جمله وزیرمتن باندل‌شده)
//!     روی Canvas رندر می‌کند — موتور رندر مرورگر خودش فارسی RTL و
//!     اتصال حروف را درست می‌سازد (برعکس/جداشدن حروف ناممکن است)؛
//!   * «اتصال نامرئی حروف»: fillText + strokeText هم‌پوشان ⇒ کانتورهای
//!     حروف مجاور در یک شکل به‌هم‌جوشیده از دل contour tracer خارج
//!     می‌شوند ⇒ دیوارهٔ مارپیچی روی «شکل یکپارچه» چاپ می‌شود و جای
//!     اتصال به‌هیچ‌وجه دیده نمی‌شود (خوانندهٔ کانتور اینجاست نه در حروف)؛
//!   * این ماژول (Rust) کانتورهای ۲بعدی (mm) را می‌گیرد و:
//!       ۱) کف چندلایه (پیرامون + پرکردن زیگزاگ متقاطع even-odd)؛
//!       ۲) دیوارهٔ هر کانتور به‌صورت «مارپیچ پیوسته» (هر دور = یک لایه؛
//!          z در طول مسیر به‌نرمی بالا می‌رود ⇒ بدون درز، حالت vase)؛
//!       ۳) E/F از همان فرمول هسته؛ travel بین کانتورها (رتراکشن خودکار
//!          در serialize_gcode).
//!
//! خروجی G-code از همان serialize_gcode هسته می‌گذرد ⇒ دما/سرعت/رتراکشن/
//! پایان استاندارد (رتراکشن → خاموشی‌ها → Z+1 → هوم X → M84) یکسان.
//! ============================================================================

use crate::designer::spiral::{extrusion_mm, DesignerOutput, MeshData, WallMesh};
use crate::designer::PrinterProfile;
use crate::diagnostics::Diagnostic;
use crate::toolpath::{Certainty, SegmentKind, ToolpathStats, ToolpathSegment};
use serde::{Deserialize, Serialize};

/// یک کانتور بستهٔ ۲بعدی (mm) — خروجی ردیاب کانتور فرانت‌اند
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContour {
    /// نقاط بسته (x,y) به mm در فضای مدل (مرکز متن ≈ 0,0)
    pub points: Vec<[f64; 2]>,
    /// آیا کانتور «حفره» است (داخل حروف مثل «و»، «ه»، «a»، «o»)؟
    /// فقط برای مرتب‌چاپ بهینه و توضیح فارسی؛ مسیر همان حلقهٔ بسته است.
    #[serde(default)]
    pub is_hole: bool,
}

/// پارامترهای کامل متن‌به‌گلدان — «تنظیمات حرفه‌ای» (درخواست ۱۱)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextVaseParams {
    /// کانتورهای متن (mm، فضای مدل)
    pub contours: Vec<TextContour>,
    /// ارتفاع دیواره‌های متن (mm) — قد حروف
    pub height_mm: f64,
    /// لایه‌های کف توپر (۰..۸) — ۲ استاندارد
    #[serde(default = "default_floor_layers")]
    pub floor_layers: u32,
    /// فاصلهٔ خطوط پرکردن کف (mm؛ ۰ = خودکار ۱٫۱۵×عرض خط)
    #[serde(default)]
    pub floor_spacing_mm: f64,
    /// گام کیفیت کانتور (mm) — ریزتر = دیواره نرم‌تر/فایل بزرگ‌تر
    #[serde(default = "default_quality")]
    pub quality_step_mm: f64,
    /// مرکز‌گذاری روی صفحهٔ چاپ (مثل طراح مارپیچی)
    #[serde(default = "default_true")]
    pub center_on_bed: bool,
    /// سرعت نسبی دیوارهٔ متن (۰٫۳..۱ نسبت به پروفایل) — حروف ریز آرام‌تر
    #[serde(default = "default_text_speed")]
    pub speed_scale: f64,
    /// پرینتر (دما/سرعت/رتراکشن/کد شروع و پایان)
    pub printer: PrinterProfile,
}

fn default_floor_layers() -> u32 {
    2
}
fn default_quality() -> f64 {
    1.1
}
fn default_text_speed() -> f64 {
    0.8
}
fn default_true() -> bool {
    true
}

impl Default for TextVaseParams {
    fn default() -> Self {
        Self {
            contours: Vec::new(),
            height_mm: 40.0,
            floor_layers: default_floor_layers(),
            floor_spacing_mm: 0.0,
            quality_step_mm: default_quality(),
            center_on_bed: true,
            speed_scale: default_text_speed(),
            printer: PrinterProfile::default(),
        }
    }
}

/// نتیجهٔ محاسبه — برای پیش‌نمایش و خروجی
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextVaseResult {
    /// Toolpath IR — منبع پیش‌نمایش و G-code
    pub segments: Vec<ToolpathSegment>,
    /// آمار مسیر
    pub stats: ToolpathStats,
    /// هشدارها
    pub diagnostics: Vec<Diagnostic>,
    /// خلاصهٔ فارسی
    pub summary_fa: String,
    /// جعبهٔ محدودکنندهٔ متن در فضای مدل (x0, y0, x1, y1) — mm
    pub bbox: [f64; 4],
}

// ============================================================================
// ابزارهای داخلی
// ============================================================================

/// بازنمونه‌گیری کانتور با گام وتری حداکثر `step` — دیواره نرم و E دقیق
fn resample(points: &[[f64; 2]], step: f64) -> Vec<[f64; 2]> {
    let step = step.max(0.3);
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(points.len() * 2);
    if points.len() < 2 {
        return out;
    }
    for w in points.windows(2) {
        let a = w[0];
        let b = w[1];
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        out.push(a);
        if len > step {
            let n = (len / step).ceil() as usize;
            for k in 1..n {
                let t = k as f64 / n as f64;
                out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
    }
    // بستن حلقه
    out.push(points[0]);
    out
}

/// مساحت علامت‌دار کانتور (قانون کفش‌بندی) — جهت دوران را می‌گوید
fn signed_area(pts: &[[f64; 2]]) -> f64 {
    let mut s = 0.0;
    for w in pts.windows(2) {
        s += w[0][0] * w[1][1] - w[1][0] * w[0][1];
    }
    if let (Some(&f), Some(&l)) = (pts.first(), pts.last()) {
        s += l[0] * f[1] - f[0] * l[1];
    }
    s * 0.5
}

/// پرکردن زیگزاگ اسکن‌خطی با قاعدهٔ even-odd روی مجموعهٔ کانتورها
/// direction: 'x' = خطوط موازی X، 'y' = موازی Y
fn scanline_fill(
    contours: &[&TextContour],
    spacing: f64,
    direction: bool, // true = X
) -> Vec<([f64; 2], [f64; 2])> {
    let mut out = Vec::new();
    if contours.is_empty() || spacing < 0.1 {
        return out;
    }
    // محدوده
    let mut lo = [f64::MAX, f64::MAX];
    let mut hi = [f64::MIN, f64::MIN];
    for c in contours {
        for p in &c.points {
            lo[0] = lo[0].min(p[0]);
            lo[1] = lo[1].min(p[1]);
            hi[0] = hi[0].max(p[0]);
            hi[1] = hi[1].max(p[1]);
        }
    }
    if lo[0] > hi[0] || lo[1] > hi[1] {
        return out;
    }
    // اسکن‌ها
    let scan_from = if direction { lo[1] } else { lo[0] };
    let scan_to = if direction { hi[1] } else { hi[0] };
    let mut s = scan_from + spacing * 0.5;
    while s < scan_to {
        // تقاطع‌های خط اسکن با همهٔ یال‌ها (even-odd)
        let mut crossings: Vec<f64> = Vec::new();
        for c in contours {
            let pts = &c.points;
            if pts.len() < 2 {
                continue;
            }
            for i in 0..pts.len() {
                let a = pts[i];
                let b = pts[(i + 1) % pts.len()];
                let (a_c, b_c) = if direction { (a[1], b[1]) } else { (a[0], b[0]) };
                let (a_o, b_o) = if direction { (a[0], b[0]) } else { (a[1], b[1]) };
                // یال افقی؟ رد
                if (a_c - b_c).abs() < 1e-12 {
                    continue;
                }
                let t = (s - a_c) / (b_c - a_c);
                if t >= 0.0 && t < 1.0 {
                    crossings.push(a_o + (b_o - a_o) * t);
                }
            }
        }
        crossings.sort_by(|x, y| x.partial_cmp(y).unwrap());
        // جفت‌ها = داخل ناحیه (even-odd)
        let mut k = 0;
        while k + 1 < crossings.len() {
            let (c0, c1) = (crossings[k], crossings[k + 1]);
            if c1 - c0 > spacing * 0.25 {
                // فقط تکه‌های معنادار
                let seg = if direction {
                    ([c0, s], [c1, s])
                } else {
                    ([s, c0], [s, c1])
                };
                out.push(seg);
            }
            k += 2;
        }
        s += spacing;
    }
    out
}

// ============================================================================
// موتور اصلی
// ============================================================================

/// محاسبهٔ کامل مسیر متن‌به‌گلدان
pub fn compute(params: &TextVaseParams) -> TextVaseResult {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let printer = &params.printer;
    let layer_h = printer.layer_height_mm.max(0.05);
    let height = params.height_mm.max(1.0);
    let quality = params.quality_step_mm.clamp(0.4, 4.0);
    let spacing = if params.floor_spacing_mm > 0.1 {
        params.floor_spacing_mm
    } else {
        printer.line_width_mm * 1.15
    };

    // ---------- آفست مرکز صفحه ----------
    let bed_offset = if params.center_on_bed {
        [printer.build_volume_mm[0] / 2.0, printer.build_volume_mm[1] / 2.0]
    } else {
        [0.0, 0.0]
    };

    // ---------- اعتبارسانی ----------
    if params.contours.is_empty() {
        diagnostics.push(Diagnostic::warning(
            "W-TEXT-EMPTY",
            "هیچ کانتوری از متن استخراج نشد — متن یا ردیاب کانتور خروجی ندارد",
            0,
        ));
        return TextVaseResult {
            segments: Vec::new(),
            stats: ToolpathStats::default(),
            diagnostics,
            summary_fa: "متن خالی است".into(),
            bbox: [0.0; 4],
        };
    }
    if height > printer.build_volume_mm[2] {
        diagnostics.push(Diagnostic::warning(
            "W-TEXT-HEIGHT",
            format!(
                "ارتفاع متن ({height:.0}mm) از ارتفاع چاپ ({:.0}mm) بیشتر است",
                printer.build_volume_mm[2]
            ),
            0,
        ));
    }

    // ---------- آماده‌سازی کانتورها ----------
    struct Prep {
        pts: Vec<[f64; 2]>, // بازنمونه‌شده + بسته
        is_hole: bool,
        area: f64,
        start: [f64; 2],
    }
    let mut preps: Vec<Prep> = Vec::with_capacity(params.contours.len());
    let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for c in &params.contours {
        if c.points.len() < 3 {
            continue;
        }
        let pts = resample(&c.points, quality);
        if pts.len() < 4 {
            continue;
        }
        let area = signed_area(&pts);
        for p in &pts {
            bbox[0] = bbox[0].min(p[0]);
            bbox[1] = bbox[1].min(p[1]);
            bbox[2] = bbox[2].max(p[0]);
            bbox[3] = bbox[3].max(p[1]);
        }
        preps.push(Prep {
            start: pts[0],
            pts,
            is_hole: c.is_hole,
            area,
        });
    }
    if preps.is_empty() {
        diagnostics.push(Diagnostic::warning(
            "W-TEXT-DEGENERATE",
            "کانتورهای متن خیلی ریز یا نامعتبرند",
            0,
        ));
        return TextVaseResult {
            segments: Vec::new(),
            stats: ToolpathStats::default(),
            diagnostics,
            summary_fa: "کانتورهای نامعتبر".into(),
            bbox: [0.0; 4],
        };
    }

    // ---------- گزینش ترتیب چاپ: نزدیک‌ترین‌همسایه از مرکز ----------
    // حروف به ترتیب فاصله از مرکز چاپ می‌شوند (travel کمینه تقریبی)
    let mut order: Vec<usize> = (0..preps.len()).collect();
    let cxm = (bbox[0] + bbox[2]) * 0.5;
    let cym = (bbox[1] + bbox[3]) * 0.5;
    order.sort_by(|&a, &b| {
        let da = (preps[a].start[0] - cxm).hypot(preps[a].start[1] - cym);
        let db = (preps[b].start[0] - cxm).hypot(preps[b].start[1] - cym);
        da.partial_cmp(&db).unwrap()
    });

    let mut segments: Vec<ToolpathSegment> = Vec::new();
    let mut stats = ToolpathStats::default();
    let mut next_id: u64 = 0;
    let wall_f = printer.effective_wall_mm_min() * params.speed_scale.clamp(0.2, 1.2);
    let first_f = printer.effective_first_layer_mm_min();
    let floor_f = printer.effective_floor_mm_min();
    let travel_f = printer.effective_travel_mm_min();

    // نقطهٔ شروع مجازی — مرکز متن روی صفحه
    let mut cur = [cxm + bed_offset[0], cym + bed_offset[1], layer_h];

    let mut emit = |segments: &mut Vec<ToolpathSegment>,
                    next_id: &mut u64,
                    stats: &mut ToolpathStats,
                    cur: &mut [f64; 3],
                    to: [f64; 3],
                    kind: SegmentKind,
                    feed: f64| {
        let e = if kind == SegmentKind::Extrusion {
            let d = [
                to[0] - cur[0],
                to[1] - cur[1],
                to[2] - cur[2],
            ];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            Some(extrusion_mm(
                len,
                printer.line_width_mm,
                layer_h,
                printer.filament_diameter_mm,
                printer.flow_multiplier,
            ))
        } else {
            None
        };
        let seg = ToolpathSegment {
            id: *next_id,
            source_line_index: segments.len(),
            kind,
            start_mm: Some(*cur),
            end_mm: Some(to),
            e_delta_mm: e,
            feedrate_mm_min: Some(feed),
            layer_index: None,
            certainty: Certainty::Known,
        };
        *next_id += 1;
        stats.accumulate(&seg);
        segments.push(seg);
        *cur = to;
    };

    // =====================================================================
    // ۱) کف چندلایه — پیرامون + پرکردن متقاطع (even-odd)
    // =====================================================================
    let floor_layers = params.floor_layers.min(8) as usize;
    for layer in 0..floor_layers {
        let z = ((layer + 1) as f64 * layer_h).max(0.2);
        let feed = if layer == 0 { first_f } else { floor_f };
        // پیرامون همهٔ کانتورها (مرز تمیز کف)
        for &ci in &order {
            let pts = &preps[ci].pts;
            let first = [pts[0][0] + bed_offset[0], pts[0][1] + bed_offset[1], z];
            emit(&mut segments, &mut next_id, &mut stats, &mut cur, first, SegmentKind::Travel, travel_f);
            for p in pts.iter().skip(1) {
                emit(
                    &mut segments, &mut next_id, &mut stats, &mut cur,
                    [p[0] + bed_offset[0], p[1] + bed_offset[1], z],
                    SegmentKind::Extrusion, feed,
                );
            }
        }
        // پرکردن زیگزاگ — جهت متناوب هر لایه (cross-hatch استاندارد)
        let refs: Vec<&TextContour> = params.contours.iter().filter(|c| c.points.len() >= 3).collect();
        let dir_x = layer % 2 == 0;
        for (i, (a, b)) in scanline_fill(&refs, spacing, dir_x).into_iter().enumerate() {
            let pa = [a[0] + bed_offset[0], a[1] + bed_offset[1], z];
            let pb = [b[0] + bed_offset[0], b[1] + bed_offset[1], z];
            emit(&mut segments, &mut next_id, &mut stats, &mut cur, pa, SegmentKind::Travel, travel_f);
            emit(&mut segments, &mut next_id, &mut stats, &mut cur, pb, SegmentKind::Extrusion, feed);
            let _ = i;
        }
    }

    // =====================================================================
    // ۲) دیواره‌های مارپیچی — هر کانتور یک «مMarپیچ مستقل» (vase)
    // z در طول هر دور به‌نرمی یک لایه بالا می‌رود ⇒ بدون درز لایه‌ای
    // =====================================================================
    let wall_z0 = (floor_layers as f64 * layer_h).max(layer_h);
    let wall_height = (height - wall_z0).max(layer_h);
    let loops_total = (wall_height / layer_h).ceil().max(1.0) as usize;
    for &ci in &order {
        let pts = &preps[ci].pts;
        let m = pts.len();
        // شروع مارپیچ
        let first = [pts[0][0] + bed_offset[0], pts[0][1] + bed_offset[1], wall_z0];
        emit(&mut segments, &mut next_id, &mut stats, &mut cur, first, SegmentKind::Travel, travel_f);
        for k in 0..loops_total {
            let z_a = wall_z0 + k as f64 * layer_h;
            let z_b = (wall_z0 + (k + 1) as f64 * layer_h).min(wall_z0 + wall_height);
            // یک دور کامل: z از z_a به z_b به‌نرمی
            for i in 1..=m {
                let t = i as f64 / m as f64;
                let p = pts[i % m];
                let z = z_a + (z_b - z_a) * t;
                emit(
                    &mut segments, &mut next_id, &mut stats, &mut cur,
                    [p[0] + bed_offset[0], p[1] + bed_offset[1], z],
                    SegmentKind::Extrusion, wall_f,
                );
            }
        }
    }

    // ---------- هشدارها ----------
    let n_holes = preps.iter().filter(|p| p.is_hole).count();
    let width_mm = bbox[2] - bbox[0];
    let depth_mm = bbox[3] - bbox[1];
    if width_mm > printer.build_volume_mm[0].min(printer.build_volume_mm[1]) * 0.98 {
        diagnostics.push(Diagnostic::warning(
            "W-TEXT-WIDE",
            format!(
                "پهنای متن ({width_mm:.0}mm) تقریباً برابر عرض صفحهٔ چاپ است — فونت کوچک‌تر یا متن چندخطی"
            ),
            0,
        ));
    }
    if n_holes > 0 {
        diagnostics.push(Diagnostic::file_level(
            crate::diagnostics::Severity::Info,
            "I-TEXT-HOLES",
            format!("{n_holes} حفرهٔ داخلی (داخل حروف) به‌صورت دیوارهٔ جداگانه چاپ می‌شود"),
        ));
    }

    // ---------- خلاصهٔ فارسی ----------
    let est_time_min = estimate_time(&segments);
    let summary_fa = format!(
        "متن به گلدان: {} شکل ({} حفره)، ابعاد {}×{} mm، ارتفاع {} mm، {} دور مارپیچ در {} کانتور؛ طول چاپ ≈ {} m؛ فیلامنت ≈ {} m؛ زمان تخمینی ≈ {} دقیقه",
        preps.len(),
        n_holes,
        format_num(width_mm),
        format_num(depth_mm),
        format_num(height),
        loops_total,
        preps.len(),
        format_num(stats.extrusion_length_mm / 1000.0),
        format_num(stats.total_e_mm / 1000.0),
        format_num(est_time_min),
    );

    TextVaseResult {
        segments,
        stats,
        diagnostics,
        summary_fa,
        bbox,
    }
}

/// تخمین زمان (دقیقه) — همان منطق spiral
fn estimate_time(segments: &[ToolpathSegment]) -> f64 {
    let mut total_min = 0.0;
    for s in segments {
        if let (Some(a), Some(b)) = (s.start_mm, s.end_mm) {
            let len = (b[0] - a[0]).hypot(b[1] - a[1]).hypot(b[2] - a[2]);
            if let Some(f) = s.feedrate_mm_min {
                if f > 1.0 {
                    total_min += len / f;
                }
            }
        }
    }
    total_min
}

fn format_num(v: f64) -> String {
    format!("{:.2}", v).trim_end_matches('0').trim_end_matches('.').to_string()
}

/// ساخت DesignerOutput برای بازاستفادهٔ serialize_gcode هسته
/// (دما/رتراکشن/پایان استاندارد بدون تکرار کد)
pub fn to_designer_output(result: &TextVaseResult) -> DesignerOutput {
    DesignerOutput {
        segments: result.segments.clone(),
        line_count: result.segments.len(),
        mesh: MeshData { vertices: Vec::new(), triangles: Vec::new() },
        radius_samples: Vec::new(),
        wall_mesh: WallMesh::default(),
        stats: result.stats.clone(),
        diagnostics: result.diagnostics.clone(),
        layers: Vec::new(),
        summary_fa: result.summary_fa.clone(),
    }
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn rect_contour(x0: f64, y0: f64, w: f64, h: f64, is_hole: bool) -> TextContour {
        TextContour {
            points: vec![
                [x0, y0],
                [x0 + w, y0],
                [x0 + w, y0 + h],
                [x0, y0 + h],
            ],
            is_hole,
        }
    }

    fn base_params() -> TextVaseParams {
        TextVaseParams {
            contours: vec![rect_contour(-20.0, -10.0, 40.0, 20.0, false)],
            height_mm: 20.0,
            floor_layers: 2,
            floor_spacing_mm: 0.0,
            quality_step_mm: 1.0,
            center_on_bed: true,
            speed_scale: 0.8,
            printer: PrinterProfile::default(),
        }
    }

    #[test]
    fn test_text2vase_basic_segments() {
        // یک مستطیل: کف (پیرامون + زیگزاگ) + دیوارهٔ مارپیچی
        let r = compute(&base_params());
        assert!(!r.segments.is_empty(), "segment تولید نشد");
        assert!(r.segments.len() > 100, "segment خیلی کم: {}", r.segments.len());
        // هم travel هست هم چاپ
        assert!(r.segments.iter().any(|s| s.kind == SegmentKind::Travel));
        assert!(r.segments.iter().any(|s| s.kind == SegmentKind::Extrusion));
        // E مثبت جمع می‌شود
        assert!(r.stats.total_e_mm > 0.0);
        // bbox درست
        assert!((r.bbox[2] - r.bbox[0] - 40.0).abs() < 1e-6);
        assert!((r.bbox[3] - r.bbox[1] - 20.0).abs() < 1e-6);
        // خلاصهٔ فارسی
        assert!(r.summary_fa.contains("متن به گلدان"));
    }

    #[test]
    fn test_text2vase_wall_reaches_height() {
        // دیوارهٔ مارپیچی باید تا ارتفاع خواسته‌شده برسد
        let r = compute(&base_params());
        let z_max = r
            .segments
            .iter()
            .filter_map(|s| s.end_mm.map(|p| p[2]))
            .fold(0.0f64, f64::max);
        assert!(
            (z_max - 20.0).abs() < 0.6,
            "ارتفاع نهایی {z_max} باید ≈ ۲۰ باشد"
        );
    }

    #[test]
    fn test_text2vase_hole_contour() {
        // مستطیل + حفرهٔ داخلی: کف حفره را خالی می‌گذارد (even-odd)
        let mut p = base_params();
        p.contours.push(rect_contour(-5.0, -5.0, 10.0, 10.0, true));
        let r = compute(&p);
        // نقطهٔ میانی حفره (مرکز 0,0) در لایهٔ کف نباید اکستروژن داشته باشد
        // (تلورانس نصف فاصلهٔ خط)
        let lw = p.printer.line_width_mm;
        for s in &r.segments {
            if s.kind != SegmentKind::Extrusion {
                continue;
            }
            if let (Some(a), Some(b)) = (s.start_mm, s.end_mm) {
                if a[2] > lw * 2.5 + 0.5 {
                    continue; // فقط کف (۲ لایه)
                }
                for pnt in [a, b] {
                    let d = (pnt[0] - 110.0).hypot(pnt[1] - 110.0);
                    assert!(
                        d > 4.0 - lw,
                        "اکستروژن کف داخل حفره: d={d:.2}"
                    );
                }
            }
        }
        assert!(r.summary_fa.contains("۱ حفره") || r.summary_fa.contains("حفره"));
    }

    #[test]
    fn test_text2vase_center_on_bed() {
        // مرکز‌گذاری: متن حول مرکز صفحه (110,110) می‌نشیند
        let r = compute(&base_params());
        let xs: Vec<f64> = r
            .segments
            .iter()
            .filter_map(|s| s.end_mm.map(|p| p[0]))
            .collect();
        let x_min = xs.iter().cloned().fold(f64::MAX, f64::min);
        let x_max = xs.iter().cloned().fold(f64::MIN, f64::max);
        // عرض ۴۰ ⇒ حدود ۹۰..۱۳۰
        assert!((x_min - 90.0).abs() < 2.0, "x_min={x_min}");
        assert!((x_max - 130.0).abs() < 2.0, "x_max={x_max}");
    }

    #[test]
    fn test_text2vase_two_letters_two_contours() {
        // دو مستطیل (دو «حرف») — هرکدام مارپیچ مستقل + کف مشترک
        let mut p = base_params();
        p.contours = vec![
            rect_contour(-30.0, -10.0, 20.0, 20.0, false),
            rect_contour(10.0, -10.0, 20.0, 20.0, false),
        ];
        let r = compute(&p);
        // ترتیب چاپ: نزدیک‌ترین به مرکز اول
        // هر دو کانتور باید مارپیچ داشته باشند: بیشینهٔ z در هر دو ناحیه
        let left_max = r
            .segments
            .iter()
            .filter_map(|s| s.end_mm.filter(|p| p[0] < 110.0).map(|p| p[2]))
            .fold(0.0f64, f64::max);
        let right_max = r
            .segments
            .iter()
            .filter_map(|s| s.end_mm.filter(|p| p[0] > 110.0).map(|p| p[2]))
            .fold(0.0f64, f64::max);
        assert!((left_max - 20.0).abs() < 0.6, "left_max={left_max}");
        assert!((right_max - 20.0).abs() < 0.6, "right_max={right_max}");
    }

    #[test]
    fn test_text2vase_empty_contours() {
        let mut p = base_params();
        p.contours = Vec::new();
        let r = compute(&p);
        assert!(r.segments.is_empty());
        assert!(r.summary_fa.contains("خالی"));
        assert!(!r.diagnostics.is_empty());
    }

    #[test]
    fn test_resample_step() {
        let pts = vec![[0.0, 0.0], [10.0, 0.0]];
        let out = resample(&pts, 2.0);
        assert!(out.len() >= 6, "باید زیربخش داشته باشد: {}", out.len());
        let out2 = resample(&pts, 20.0);
        assert_eq!(out2.len(), 2, "کوتاه‌تر از گام ⇒ دست‌نخورده");
    }

    #[test]
    fn test_scanline_even_odd() {
        // مستطیل بزرگ با حفره: خط اسکن وسط باید دو قطعه بدهد (داخل حفره خالی)
        let outer = rect_contour(-10.0, -10.0, 20.0, 20.0, false);
        let hole = rect_contour(-4.0, -4.0, 8.0, 8.0, true);
        let refs = vec![&outer, &hole];
        let segs = scanline_fill(&refs, 2.0, true);
        // خط y=0 (نزدیک): داخل outer ولی دور از hole ⇒ دو قطعه مجزا
        let mid: Vec<_> = segs
            .iter()
            .filter(|(a, b)| a[1].abs() < 1.5 && b[1].abs() < 1.5)
            .collect();
        assert!(
            mid.len() >= 2,
            "خط میانی باید قطعهٔ گسسته داشته باشد (حفره): {}",
            mid.len()
        );
    }
}
