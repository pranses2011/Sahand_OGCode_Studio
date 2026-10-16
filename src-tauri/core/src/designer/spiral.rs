//! ============================================================================
//! designer/spiral.rs — تولید مسیر مارپیچی + mesh + Toolpath IR مشترک
//! ============================================================================
//! این ماژول «زنجیرهٔ منبع واحد» را کامل می‌کند (Docs/PROMPT_03 §5):
//!
//!   پروفایل 2D → patternها → mesh پایه (STL)
//!                                ↘
//!                                  مسیر spiral → textureها → E/F → Toolpath IR
//!
//! Toolpath IR همان ساختار interpreter است تا «آنچه کاربر می‌بیند همان
//! چیزی باشد که صادر می‌شود» — مختصات قابل‌تطبیق در آزمون.
//!
//! محاسبهٔ E (الزام §5):
//!   حجم bead تقریبی = line_width × layer_height × طول گام
//!   E_filament = حجم / سطح مقطع فیلامنت × ضریب جریان
//!   روش و فرض‌ها در این کامنت و در راهنمای کاربر مستند است.
//! ============================================================================

use crate::designer::pattern::apply_patterns;
use crate::designer::profile::{build_radius_fn, FormAdjust, ProfilePoint};
use crate::designer::texture::apply_texture;
use crate::designer::{FloorSettings, PrinterProfile};
use crate::diagnostics::Diagnostic;
use crate::toolpath::{Certainty, LayerInfo, SegmentKind, ToolpathSegment, ToolpathStats};
use serde::{Deserialize, Serialize};

/// ورودی کامل موتور طراح
#[derive(Debug, Clone)]
pub struct DesignerInput {
    /// نقاط پروفایل خام
    pub profile: Vec<ProfilePoint>,
    /// اصلاح فرم
    pub form: FormAdjust,
    /// الگوهای هندسی (ترتیب اعمال)
    pub patterns: Vec<crate::designer::pattern::PatternParams>,
    /// بافت‌های مسیر
    pub textures: Vec<crate::designer::texture::TextureParams>,
    /// کف
    pub floor: FloorSettings,
    /// پرینتر
    pub printer: PrinterProfile,
}

/// خروجی کامل موتور طراح
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerOutput {
    /// Toolpath IR مشترک — منبع preview و G-code
    pub segments: Vec<ToolpathSegment>,
    /// خطوط متناظر برای ویرایشگر (هر segment به «خط» فرضی خودش اشاره می‌کند)
    pub line_count: usize,
    /// mesh مثلثی فرم (برای STL) — الگوها اعمال شده‌اند، بافت‌ها نه
    pub mesh: MeshData,
    /// آمار مسیر
    pub stats: ToolpathStats,
    /// هشدارها
    pub diagnostics: Vec<Diagnostic>,
    /// اطلاعات لایه‌ها (برای مارپیچ: خالی طبق الزام)
    pub layers: Vec<LayerInfo>,
    /// خلاصهٔ متنی فارسی برای نمایش پیش از خروجی
    pub summary_fa: String,
}

/// mesh مثلثی — رأس‌ها و مثلث‌ها با normals محاسبه‌شده
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    /// رأس‌ها: سه‌تایی (x,y,z) به mm
    pub vertices: Vec<[f64; 3]>,
    /// مثلث‌ها: ایندکس سه‌تایی به vertices — winding بیرونی
    pub triangles: Vec<[u32; 3]>,
}

impl MeshData {
    /// محاسبهٔ normal یک مثلث (برای STL)
    pub fn triangle_normal(&self, tri: &[u32; 3]) -> [f64; 3] {
        let a = self.vertices[tri[0] as usize];
        let b = self.vertices[tri[1] as usize];
        let c = self.vertices[tri[2] as usize];
        // ضرب خارجی (b-a)×(c-a)
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-12 {
            [0.0, 0.0, 1.0]
        } else {
            [n[0] / len, n[1] / len, n[2] / len]
        }
    }
}

/// محاسبهٔ E برای یک گام مسیر — مستند در راهنما
/// فرمول: E = (line_width × layer_height × طول) / (π × (d/2)²) × flow
/// مساحت مقطع فیلامنت از قطر فیلامنت پروفایل می‌آید.
fn extrusion_mm(
    length_mm: f64,
    line_width_mm: f64,
    layer_height_mm: f64,
    filament_diameter_mm: f64,
    flow: f64,
) -> f64 {
    let bead_area = line_width_mm * layer_height_mm; // mm² تقریبی مستطیلی
    let filament_area =
        std::f64::consts::PI * (filament_diameter_mm / 2.0).powi(2);
    (bead_area * length_mm / filament_area) * flow
}

/// اجرای کامل زنجیرهٔ طراح
pub fn compute(input: &DesignerInput) -> DesignerOutput {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let printer = &input.printer;
    let height = input.form.height_mm;
    let layer_h = printer.layer_height_mm.max(0.01);

    // ---------- اعتبارسانی ورودی‌ها ----------
    if let Err(e) = crate::designer::profile::validate_profile(&input.profile) {
        diagnostics.push(Diagnostic::file_level(crate::diagnostics::Severity::Error, "E-PROFILE", e));
        return DesignerOutput {
            segments: vec![],
            line_count: 0,
            mesh: MeshData { vertices: vec![], triangles: vec![] },
            stats: ToolpathStats::default(),
            diagnostics,
            layers: vec![],
            summary_fa: "پروفایل نامعتبر است".into(),
        };
    }

    // ---------- شعاع پایه ----------
    let radius_fn = build_radius_fn(&input.profile, &input.form);

    // ---------- پارامترهای گام زاویه‌ای ----------
    // طول گام مسیر نباید خیلی بلند شود (هندسه نرم)؛ در عین حال تعداد
    // نقاط منطقی بماند. قوس مؤثر هر گام ≈ ۲mm در شعاع متوسط.
    let r_max = (0..=100)
        .map(|i| radius_fn(i as f64 * height / 100.0))
        .fold(0.0f64, f64::max)
        .max(1.0);
    // گام زاویه‌ای از طول قوس هدف
    let arc_target_mm = 2.0;
    let circumference_max = 2.0 * std::f64::consts::PI * r_max;
    let steps_per_rev =
        ((circumference_max / arc_target_mm).ceil() as usize).clamp(24, 720);
    let d_theta = 2.0 * std::f64::consts::PI / steps_per_rev as f64;

    // حرکت مارپیچ: در هر دور کامل، Z به اندازهٔ layer_height بالا می‌رود
    let total_revolutions = (height / layer_h).ceil().max(1.0);
    let dz_per_step = layer_h / steps_per_rev as f64;

    // ---------- mesh پایه (برای STL) ----------
    let mesh = build_mesh(input, &radius_fn, steps_per_rev, &mut diagnostics);

    // ---------- مسیر مارپیچی + بافت‌ها → IR ----------
    let mut segments: Vec<ToolpathSegment> = Vec::new();
    let mut stats = ToolpathStats::default();
    let mut next_id: u64 = 0;
    let mut collided_any = false;

    // شروع از مرکز (travel به نقطهٔ اول)
    let theta_start = 0.0f64;
    let z_start = layer_h; // نازل از ارتفاع لایهٔ اول
    let r0 = {
        let (r, clamped) =
            apply_patterns(radius_fn(z_start.min(height)), z_start, theta_start, height, &input.patterns);
        if clamped {
            collided_any = true;
        }
        r.max(0.05)
    };
    let first_point = [r0 * theta_start.cos(), r0 * theta_start.sin(), z_start];

    // بافت روی نقطهٔ اول
    let (first_point, c1) = apply_texture(first_point, 0.0, r0, z_start / height, &input.textures);
    collided_any |= c1;

    // travel اولیه از مبدأ (0,0,layer_h) — بدون اکستروژن
    let mut prev_point = [0.0, 0.0, layer_h];
    let seg = ToolpathSegment {
        id: next_id,
        source_line_index: 0,
        kind: SegmentKind::Travel,
        start_mm: Some(prev_point),
        end_mm: Some(first_point),
        e_delta_mm: None,
        feedrate_mm_min: Some(printer.travel_speed_mm_min),
        layer_index: None,
        certainty: Certainty::Known,
    };
    next_id += 1;
    stats.accumulate(&seg);
    segments.push(seg);
    prev_point = first_point;

    // ---------- حلقهٔ مارپیچ ----------
    let mut e_counter = 0.0f64; // برای E مطلق؛ در نسبی delta مستقیم می‌رود
    let total_steps = (total_revolutions * steps_per_rev as f64).ceil() as usize;
    let mut path_length = 0.0f64;
    let mut clamped_radius_events = 0usize;

    for step in 1..=total_steps {
        // θ پیوسته بالا می‌رود؛ z خطی با گام
        let theta = step as f64 * d_theta;
        let z = (z_start + step as f64 * dz_per_step).min(height + layer_h);
        let z_for_profile = (z - layer_h).clamp(0.0, height); // پروفایل از 0 تا height

        // شعاع با الگوها
        let (r, clamped) = apply_patterns(
            radius_fn(z_for_profile),
            z_for_profile,
            theta,
            height,
            &input.patterns,
        );
        if clamped {
            clamped_radius_events += 1;
            collided_any = true;
        }
        let r = r.max(0.05);

        // نقطهٔ خام قبل از بافت
        let raw_point = [r * theta.cos(), r * theta.sin(), z];
        path_length += (d_theta * r).abs(); // طول قوس تقریبی این گام
        // بافت روی طول پیموده‌شده
        let (point, collided) =
            apply_texture(raw_point, path_length, r, z_for_profile / height.max(1e-9), &input.textures);
        collided_any |= collided;

        // طول واقعی این گام (پس از بافت)
        let d = [
            point[0] - prev_point[0],
            point[1] - prev_point[1],
            point[2] - prev_point[2],
        ];
        let seg_len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();

        // E این گام
        let e_delta = extrusion_mm(
            seg_len,
            printer.line_width_mm,
            layer_h,
            printer.filament_diameter_mm,
            printer.flow_multiplier,
        );
        e_counter += e_delta;

        // segment چاپی — ΔE مثبت؛ feedrate چاپ
        let e_val = if printer.absolute_e { e_counter } else { e_delta };
        let _ = e_val; // E نهایی در خروجی‌ساز G-code ساخته می‌شود؛ IR فقط delta
        let seg = ToolpathSegment {
            id: next_id,
            source_line_index: segments.len(),
            kind: SegmentKind::Extrusion,
            start_mm: Some(prev_point),
            end_mm: Some(point),
            e_delta_mm: Some(e_delta),
            feedrate_mm_min: Some(printer.print_speed_mm_min),
            layer_index: None, // مارپیچ: لایهٔ گسسته نه (الزام)
            certainty: Certainty::Known,
        };
        // اگر جابه‌جایی ناچیز است (بافت صفر و r ثابت) همچنان چاپ حساب می‌شود
        next_id += 1;
        stats.accumulate(&seg);
        segments.push(seg);
        prev_point = point;
    }

    // ---------- کف هم‌مرکز (اگر فعال) ----------
    let floor_segments = build_floor(input, &radius_fn, &mut next_id, &mut stats);
    let floor_count = floor_segments.len();
    // کف «قبل» از دیواره چاپ می‌شود؛ برای سادگی IR، در ادامهٔ segments
    // می‌آید ولی kind=Floor ندارد — به‌صورت Extrusion با z ثابت
    segments.extend(floor_segments);

    // ---------- هشدارها ----------
    if clamped_radius_events > 0 {
        diagnostics.push(Diagnostic::warning(
            "W-PATTERN-CLAMP",
            format!(
                "شعاع منفی در {clamped_radius_events} نقطه با الگوها به صفر محدود شد؛ فرم را تعدیل کنید"
            ),
            segments.len(),
        ));
    }
    if collided_any {
        diagnostics.push(Diagnostic::warning(
            "W-TEXTURE-COLLISION",
            "بافت در برخی نقاط شعاع را منفی کرد؛ خروجی ممکن است چاپ‌ناپذیر باشد",
            segments.len(),
        ));
    }
    // بازرسی حجم ساخت با پروفایل پرینتر
    let vol = printer.build_volume_mm;
    let r_outer = mesh
        .vertices
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt())
        .fold(0.0f64, f64::max);
    if r_outer * 2.0 > vol[0].min(vol[1]) {
        diagnostics.push(Diagnostic::warning(
            "W-DESIGN-BOUNDS",
            format!(
                "قطر بیرونی فرم ({:.1} mm) از عرض حجم ساخت ({:.1} mm) بزرگ‌تر است",
                r_outer * 2.0,
                vol[0].min(vol[1])
            ),
            segments.len(),
        ));
    }
    if height > vol[2] {
        diagnostics.push(Diagnostic::warning(
            "W-DESIGN-HEIGHT",
            format!("ارتفاع فرم ({height} mm) از ارتفاع چاپ ({:.1} mm) بیشتر است", vol[2]),
            segments.len(),
        ));
    }

    // ---------- خلاصهٔ فارسی ----------
    let est_time_min = estimate_time_min(&segments);
    let summary_fa = format!(
        "فرم مارپیچی: ارتفاع {} mm، {} دور، {} قطعه مسیر{}؛ طول چاپ ≈ {} m؛ فیلامنت ≈ {} m؛ زمان تخمینی ≈ {} دقیقه",
        format_num(height),
        format_num(total_revolutions),
        segments.len(),
        if floor_count > 0 { format!(" (+{floor_count} قطعهٔ کف)") } else { String::new() },
        format_num(stats.extrusion_length_mm / 1000.0),
        format_num(stats.total_e_mm / 1000.0),
        format_num(est_time_min),
    );

    DesignerOutput {
        segments,
        line_count: total_steps + 1,
        mesh,
        stats,
        diagnostics,
        layers: Vec::new(), // مارپیچ: بدون لایهٔ گسسته
        summary_fa,
    }
}

/// ساخت mesh سطح انقلاب — الگوها اعمال شده، بافت‌ها نه (تمایز الگو/بافت)
fn build_mesh(
    input: &DesignerInput,
    radius_fn: &impl Fn(f64) -> f64,
    steps_per_rev: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> MeshData {
    let height = input.form.height_mm;
    // تراکم عمودی mesh: هر نیم‌لایه یک ردیف (سازگار با کیفیت مسیر)
    let rows = ((height / input.printer.layer_height_mm).ceil() as usize).clamp(8, 400) * 2;
    let cols = steps_per_rev.clamp(32, 360);

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity((rows + 1) * (cols + 1));
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(rows * cols * 2);
    let mut degenerate = 0usize;

    for i in 0..=rows {
        let z = i as f64 * height / rows as f64;
        for j in 0..=cols {
            let theta = j as f64 * 2.0 * std::f64::consts::PI / cols as f64;
            // الگو روی mesh هم اعمال می‌شود (الگو ⇒ در STL می‌ماند)
            let (r, _) = apply_patterns(radius_fn(z), z, theta, height, &input.patterns);
            let r = r.max(0.0);
            vertices.push([r * theta.cos(), r * theta.sin(), z]);
        }
    }

    // مثلث‌سازی شبکهٔ استوانه‌ای (بدون درپوش بالا/پایین — vase دیواره‌دار)
    let stride = cols + 1;
    for i in 0..rows {
        for j in 0..cols {
            let a = (i * stride + j) as u32;
            let b = (i * stride + j + 1) as u32;
            let c = ((i + 1) * stride + j) as u32;
            let d = ((i + 1) * stride + j + 1) as u32;
            // winding بیرونی (نرمال به دور از محور): بررسی با شعاع
            // مثلث‌های دو نیمهٔ quad
            let tri1 = [a, c, d];
            let tri2 = [a, d, b];
            // گارد degenerate (رأس‌های منطبق)
            let distinct = |t: &[u32; 3]| {
                let p = |i: u32| vertices[i as usize];
                p(t[0]) != p(t[1]) || p(t[0]) != p(t[2]) || p(t[1]) != p(t[2])
            };
            // فاصله‌دار بودن رأس‌ها را می‌سنجیم نه فقط برابری دقیق
            let far = |t: &[u32; 3]| {
                let p = |i: u32| vertices[i as usize];
                let d01 = (p(t[0])[0] - p(t[1])[0]).abs() + (p(t[0])[1] - p(t[1])[1]).abs();
                let d12 = (p(t[1])[0] - p(t[2])[0]).abs() + (p(t[1])[1] - p(t[2])[1]).abs();
                d01 + d12 > 1e-9
            };
            if far(&tri1) || distinct(&tri1) {
                triangles.push(tri1);
            } else {
                degenerate += 1;
            }
            if far(&tri2) || distinct(&tri2) {
                triangles.push(tri2);
            } else {
                degenerate += 1;
            }
        }
    }

    if degenerate > 0 {
        diagnostics.push(Diagnostic::warning(
            "W-MESH-DEGENERATE",
            format!("{degenerate} مثلث بدون مساحت حذف شد (رأس‌های منطبق)"),
            0,
        ));
    }

    MeshData { vertices, triangles }
}

/// ساخت کف هم‌مرکز با سوراخ مرکزی — گام ۴
/// کف به‌صورت مارپیچ‌های هم‌مرکز از بیرون به داخل (یا برعکس) با اتصال
/// نیم‌دوره‌ای؛ خط واقعی تولید می‌کند (الزام: «مسیر واقعی تولید کن»).
fn build_floor(
    input: &DesignerInput,
    radius_fn: &impl Fn(f64) -> f64,
    next_id: &mut u64,
    stats: &mut ToolpathStats,
) -> Vec<ToolpathSegment> {
    let mut out = Vec::new();
    if !input.floor.enabled {
        return out;
    }
    let floor = &input.floor;
    let printer = &input.printer;
    let z = floor.z_start_mm.max(printer.layer_height_mm * 0.5).max(0.2);
    // شعاع بیرونی کف = شعاع پایین فرم
    let r_outer = radius_fn(0.0).max(0.5);
    let r_hole = (floor.center_hole_diameter_mm / 2.0).clamp(0.0, r_outer - 1.0);
    let gap = floor.ring_gap_mm.max(printer.line_width_mm * 0.9);

    // حلقه‌های هم‌مرکز از بیرون به داخل
    let mut r = r_outer;
    let mut point = [r, 0.0, z];
    // travel به شروع کف
    let seg = ToolpathSegment {
        id: *next_id,
        source_line_index: 0,
        kind: SegmentKind::Travel,
        start_mm: Some([0.0, 0.0, z]),
        end_mm: Some(point),
        e_delta_mm: None,
        feedrate_mm_min: Some(printer.travel_speed_mm_min),
        layer_index: None,
        certainty: Certainty::Known,
    };
    *next_id += 1;
    stats.accumulate(&seg);
    out.push(seg);

    while r > r_hole + gap {
        // یک دور کامل (تقریباً) با گام زاویه‌ای معقول
        let steps = ((2.0 * std::f64::consts::PI * r / 2.0).ceil() as usize).clamp(24, 360);
        let d_theta = 2.0 * std::f64::consts::PI / steps as f64;
        for i in 1..=steps {
            let theta = i as f64 * d_theta;
            let np = [r * theta.cos(), r * theta.sin(), z];
            let d = [np[0] - point[0], np[1] - point[1], 0.0];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let e = extrusion_mm(
                len,
                printer.line_width_mm,
                printer.layer_height_mm,
                printer.filament_diameter_mm,
                printer.flow_multiplier,
            );
            let seg = ToolpathSegment {
                id: *next_id,
                source_line_index: out.len(),
                kind: SegmentKind::Extrusion,
                start_mm: Some(point),
                end_mm: Some(np),
                e_delta_mm: Some(e),
                feedrate_mm_min: Some(printer.print_speed_mm_min),
                layer_index: None,
                certainty: Certainty::Known,
            };
            *next_id += 1;
            stats.accumulate(&seg);
            out.push(seg);
            point = np;
        }
        // travel به حلقهٔ داخلی‌تر (نیم‌شعاع جابه‌جایی)
        r -= gap;
        if r <= r_hole {
            break;
        }
        let inner = [r * 0.997, 0.0, z]; // کمی داخل‌تر برای شروع تمیز
        let seg = ToolpathSegment {
            id: *next_id,
            source_line_index: out.len(),
            kind: SegmentKind::Travel,
            start_mm: Some(point),
            end_mm: Some(inner),
            e_delta_mm: None,
            feedrate_mm_min: Some(printer.travel_speed_mm_min),
            layer_index: None,
            certainty: Certainty::Known,
        };
        *next_id += 1;
        stats.accumulate(&seg);
        out.push(seg);
        point = inner;
    }
    out
}

/// تخمین زمان چاپ بر اساس طول/سرعت segmentها (دقیقه) — برچسب «تقریبی»
fn estimate_time_min(segments: &[ToolpathSegment]) -> f64 {
    let mut total_min = 0.0;
    for s in segments {
        if let (Some(a), Some(b)) = (s.start_mm, s.end_mm) {
            let d = [
                b[0] - a[0],
                b[1] - a[1],
                b[2] - a[2],
            ];
            let len_mm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if let Some(f) = s.feedrate_mm_min {
                if f > 1.0 {
                    total_min += len_mm / f;
                }
            }
        }
    }
    total_min
}

/// قالب‌کردن عدد (۲ رقم اعشار)
fn format_num(v: f64) -> String {
    format!("{:.2}", v).trim_end_matches('0').trim_end_matches('.').to_string()
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::designer::pattern::PatternParams;
    use crate::designer::profile::{FormAdjust, ProfilePreset};
    use crate::designer::texture::TextureParams;

    /// ورودی استاندارد آزمون
    fn base_input() -> DesignerInput {
        DesignerInput {
            profile: ProfilePreset::vase_default(),
            form: FormAdjust::default(),
            patterns: vec![],
            textures: vec![],
            floor: crate::designer::FloorSettings::default(),
            printer: PrinterProfile::default(),
        }
    }

    #[test]
    fn test_basic_spiral_generated() {
        // مسیر مارپیچی واقعاً ساخته می‌شود: پیوسته، صعودی Z
        let out = compute(&base_input());
        assert!(out.segments.len() > 100, "تعداد segment = {}", out.segments.len());
        // اولین segment travel است
        assert_eq!(out.segments[0].kind, SegmentKind::Travel);
        // همهٔ بعدی چاپی‌اند
        assert!(out.segments[1..].iter().all(|s| s.kind == SegmentKind::Extrusion));
        // Z صعودی و پیوسته
        for w in out.segments[1..].windows(2) {
            let z0 = w[0].end_mm.unwrap()[2];
            let z1 = w[1].end_mm.unwrap()[2];
            assert!(z1 >= z0 - 1e-9, "Z باید صعودی باشد: {z0} → {z1}");
        }
        // mesh هم ساخته شد
        assert!(!out.mesh.vertices.is_empty());
        assert!(!out.mesh.triangles.is_empty());
        // خلاصهٔ فارسی موجود است
        assert!(out.summary_fa.contains("مارپیچی"));
    }

    #[test]
    fn test_pattern_changes_mesh() {
        // الگو → تغییر mesh (در STL می‌ماند)
        let mut inp = base_input();
        let base = compute(&inp);
        inp.patterns.push(PatternParams {
            kind: crate::designer::pattern::PatternKind::Petal,
            amplitude_mm: 3.0,
            count: 8,
            ..Default::default()
        });
        let with_pattern = compute(&inp);
        // تعداد رأس mesh یکسان است ولی مختصات متفاوت
        assert_eq!(base.mesh.vertices.len(), with_pattern.mesh.vertices.len());
        let mut diff = false;
        for (a, b) in base.mesh.vertices.iter().zip(with_pattern.mesh.vertices.iter()) {
            if (a[0] - b[0]).abs() > 0.01 || (a[1] - b[1]).abs() > 0.01 {
                diff = true;
                break;
            }
        }
        assert!(diff, "الگو باید mesh را تغییر دهد");
    }

    #[test]
    fn test_texture_changes_path_not_mesh() {
        // بافت → تغییر مسیر ولی mesh ثابت (تمایز بنیادی!)
        let mut inp = base_input();
        let base = compute(&inp);
        inp.textures.push(TextureParams {
            kind: crate::designer::texture::TextureKind::Ripple,
            amplitude_mm: 0.8,
            period_mm: 3.0,
            ..Default::default()
        });
        let with_texture = compute(&inp);
        // mesh یکسان (بافت در STL نیست)
        assert_eq!(base.mesh.vertices.len(), with_texture.mesh.vertices.len());
        for (a, b) in base.mesh.vertices.iter().zip(with_texture.mesh.vertices.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-9);
            assert!((a[1] - b[1]).abs() < 1e-9);
        }
        // مسیر متفاوت
        assert_eq!(base.segments.len(), with_texture.segments.len());
        let mut path_diff = false;
        for (a, b) in base.segments.iter().zip(with_texture.segments.iter()) {
            let pa = a.end_mm.unwrap();
            let pb = b.end_mm.unwrap();
            if (pa[0] - pb[0]).abs() > 0.01 || (pa[1] - pb[1]).abs() > 0.01 {
                path_diff = true;
                break;
            }
        }
        assert!(path_diff, "بافت باید مسیر را تغییر دهد");
    }

    #[test]
    fn test_floor_adds_segments() {
        // کف فعال → segmentهای اضافه با z ثابت
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.center_hole_diameter_mm = 10.0;
        let no_floor = compute(&base_input());
        let with_floor = compute(&inp);
        assert!(with_floor.segments.len() > no_floor.segments.len());
        // هشدار یا خلاصه باید کف را ذکر کند
        assert!(with_floor.summary_fa.contains("کف"));
    }

    #[test]
    fn test_bounds_warning() {
        // فرم بزرگ‌تر از build volume → هشدار
        let mut inp = base_input();
        inp.form.height_mm = 400.0; // بیشتر از 250
        inp.form.bottom_diameter_mm = 300.0; // بیشتر از 220
        let out = compute(&inp);
        assert!(out.diagnostics.iter().any(|d| d.code == "W-DESIGN-BOUNDS"));
        assert!(out.diagnostics.iter().any(|d| d.code == "W-DESIGN-HEIGHT"));
    }

    #[test]
    fn test_invalid_profile_returns_error() {
        // پروفایل خراب → diagnostic خطا بدون panic
        let mut inp = base_input();
        inp.profile = vec![crate::designer::profile::ProfilePoint { r: 10.0, h: 0.0 }];
        let out = compute(&inp);
        assert!(out.diagnostics.iter().any(|d| d.code == "E-PROFILE"));
        assert!(out.segments.is_empty());
    }

    #[test]
    fn test_e_proportional_to_length() {
        // E باید متناسب طول باشد: مسیر بزرگ‌تر → E بیشتر
        let mut small = base_input();
        small.form.height_mm = 20.0;
        let mut big = base_input();
        big.form.height_mm = 100.0;
        let s = compute(&small);
        let b = compute(&big);
        assert!(b.stats.total_e_mm > s.stats.total_e_mm);
    }

    #[test]
    fn test_mesh_normals_valid() {
        // normals مثلث‌ها واحد و سالم‌اند
        let out = compute(&base_input());
        for tri in out.mesh.triangles.iter().take(50) {
            let n = out.mesh.triangle_normal(tri);
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-9, "طول normal = {len}");
        }
    }

    #[test]
    fn test_spiral_z_reaches_height() {
        // مسیر باید تا ارتفاع فرم برسد
        let inp = base_input();
        let out = compute(&inp);
        let max_z = out
            .segments
            .iter()
            .filter_map(|s| s.end_mm.map(|p| p[2]))
            .fold(f64::MIN, f64::max);
        assert!(max_z >= inp.form.height_mm - 1e-6, "max_z = {max_z}");
    }

    #[test]
    fn test_extrusion_formula_documented() {
        // فرمول E: بررسی دستی یک مورد
        // طول 10mm، lw=0.45، lh=0.25، فیلامنت 1.75 → E = 0.45*0.25*10/(π*0.875²)
        let e = extrusion_mm(10.0, 0.45, 0.25, 1.75, 1.0);
        let expected = 0.45 * 0.25 * 10.0 / (std::f64::consts::PI * 0.875f64.powi(2));
        assert!((e - expected).abs() < 1e-12);
        // ضریب جریان
        let e2 = extrusion_mm(10.0, 0.45, 0.25, 1.75, 1.1);
        assert!((e2 - expected * 1.1).abs() < 1e-12);
    }
}
