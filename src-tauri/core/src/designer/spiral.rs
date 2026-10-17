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
use crate::designer::texture::apply_texture_ex;
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
    /// مرکز‌گذاری فرم روی صفحهٔ چاپ — نسخهٔ ۱.۲ (رفع باگ 0,0)
    pub center_on_bed: bool,
    /// چرخش کل فرم حول محور Z (درجه) — نسخهٔ ۱.۳.
    /// زاویهٔ شروع مارپیچ = جای درز (seam) + جهت‌گیری الگوها؛
    /// با اسپات 🔄 روی نمای 3D قابل‌درگ است.
    pub phase_start_deg: f64,
    /// پرینتر
    pub printer: PrinterProfile,
    /// ضریب کیفیت mesh خروجی STL — نسخهٔ ۱.۵ (۱ = استاندارد؛
    /// ۰٫۵ = سریع/پیش‌نویس؛ ۲ = ریز؛ ۳ = بسیار ریز).
    /// فقط mesh/STL را ریز می‌کند؛ مسیر G-code مستقل است.
    pub mesh_quality: f64,
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
    /// نمونه‌های شعاع بیرونی (z, r) — نسخهٔ ۱.۴: اسپات UI بدون انتقال mesh کامل
    #[serde(default)]
    pub radius_samples: Vec<[f64; 2]>,
    /// mesh دیوارهٔ چاپی برای پیش‌نمایش جامد — نسخهٔ ۱٫۶ (درخواست کاربر ۱).
    /// «نوار مارپیچی» از نقاط «واقعی مسیر»: هر سلول بین دو انقلاب متوالی
    /// مارپیچ → دقیقاً همان سطحی که چاپ می‌شود، با الگو «و» بافت.
    /// رفع باگ ۱٫۵: نمای جامد فقط پوش شعاعی حداکثری (سطح صاف) نشان می‌داد.
    #[serde(default)]
    pub wall_mesh: WallMesh,
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

/// mesh دیوارهٔ چاپی برای پیش‌نمایش جامد — نسخهٔ ۱٫۶ (درخواست کاربر ۱)
/// ==========================================================================
/// شبکهٔ منظم (rows+1)×(cols+1) از نقاط «مسیر واقعی» مارپیچ:
///   * ردیف = انقلاب مارپیچ (فاصلهٔ عمودی row_step×layer_h)؛
///   * ستون = زاویه؛ ستون آخر (j=cols) «ستونِ بسته‌شدن» است: دقیقاً یک
///     انقلاب جلوتر از ستون ۰ (هم‌زاویه) — حلقه بدون درز بسته می‌شود.
///   * ایندکس مثلث‌ها در فرانت‌اند از (rows, cols) ساخته می‌شود —
///     پیلود نصف می‌شود (قاعدهٔ کارایی ۱٫۴).
///
/// چرا از خود مسیر؟ «آنچه کاربر می‌بیند همان چیزی باشد که صادر می‌شود»:
/// نقاط مسیر الگو + بافت (شعاعی و عمودی) + کلمپ‌ها را گرفته‌اند؛ نوارِ بین
/// دو انقلاب متوالی دقیقاً سطح چاپی واقعی است — رفع باگ «سطح صاف» ۱٫۵
/// (پیش‌نمایش جامد الگو/بافت را نشان نمی‌داد).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WallMesh {
    /// تعداد ردیف‌ها (انقلاب‌ها) — رأس‌ها (rows+1)×(cols+1)
    pub rows: usize,
    /// تعداد ستون‌های زاویه‌ای هر انقلاب (ستون cols = بسته‌شدن)
    pub cols: usize,
    /// رأس‌های تخت (x,y,z)×۳ به‌ترتیب ردیف‌محور — گردشده به ۰٫۰۱mm
    pub positions: Vec<f64>,
}

/// ساخت نوار مارپیچی از نقاط واقعی دیواره — نسخهٔ ۱٫۶
/// wall_points[0] = اولین نقطهٔ چاپ دیواره؛ هر spr نقطه = یک انقلاب کامل.
fn build_wall_ribbon(wall_points: &[[f64; 3]], spr: usize) -> WallMesh {
    let total = wall_points.len();
    if spr < 8 || total < spr + 2 {
        return WallMesh::default(); // فرم خیلی کوتاه/خیلی درشت — بدون نوار
    }
    // انقلاب‌های کامل موجود (total_steps = total-1 مضرب spr است)
    let revs = (total - 1) / spr;
    if revs < 2 {
        return WallMesh::default();
    }
    // نمونه‌گیری محدود برای پیلود سبک: حداکثر ۷۲ ستون زاویه‌ای و ۲۴۰ ردیف
    let col_step = ((spr as f64) / 72.0).ceil().max(1.0) as usize;
    let cols = spr / col_step;
    if cols < 4 {
        return WallMesh::default();
    }
    let row_step = ((revs.saturating_sub(1)) as f64 / 240.0).ceil().max(1.0) as usize;
    // ردیف آخر باید «یک انقلاب کامل + ستون بسته‌شدن» جا داشته باشد:
    // rows·row_step·spr + spr ≤ total-1 ⇒ rows·row_step ≤ revs-1
    let rows = if row_step >= revs { 0 } else { (revs - 1) / row_step };
    if rows < 1 {
        return WallMesh::default();
    }

    let stride = cols + 1;
    let mut positions: Vec<f64> = Vec::with_capacity((rows + 1) * stride * 3);
    // گردکردن به ۰٫۰۱mm — دقت نمایش؛ JSON فشرده‌تر
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    for i in 0..=rows {
        let base = i * row_step * spr;
        for j in 0..cols {
            let idx = (base + j * col_step).min(total - 1);
            let p = wall_points[idx];
            positions.extend_from_slice(&[r2(p[0]), r2(p[1]), r2(p[2])]);
        }
        // ستون بسته‌شدن: دقیقاً یک انقلاب جلوتر (هم‌زاویهٔ ستون ۰)؛
        // ردیف آخر به آخرین نقطهٔ مسیر کلمپ می‌شود (هم‌زاویه در mod 2π)
        let idx_c = (base + spr).min(total - 1);
        let p = wall_points[idx_c];
        positions.extend_from_slice(&[r2(p[0]), r2(p[1]), r2(p[2])]);
    }
    WallMesh { rows, cols, positions }
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

    // ---------- آفست مرکز صفحهٔ چاپ (نسخهٔ ۱.۲) ----------
    // G-code مختصات واقعی صفحه دارد (مبدأ = گوشه)؛ طرح باید در مرکز
    // (vol/2, vol/2) بنشیند نه روی (0,0) — رفع باگ «بیرون صفحه چاپ".
    // mesh/STL در فضای مدل (محور مرکز) می‌ماند — STL هندسه است نه چاپ.
    let bed_offset = if input.center_on_bed {
        [printer.build_volume_mm[0] / 2.0, printer.build_volume_mm[1] / 2.0]
    } else {
        [0.0, 0.0]
    };
    if !input.center_on_bed {
        diagnostics.push(Diagnostic::warning(
            "W-NO-CENTER",
            "مرکز‌گذاری روی صفحه خاموش است: محور فرم روی (0,0) یعنی گوشهٔ صفحه می‌نشیند و بخش بزرگی از طرح بیرون از ناحیهٔ چاپ قرار می‌گیرد",
            0,
        ));
    }

    // ---------- چرخش فاز θ — نسخهٔ ۱.۳ ----------
    // زاویهٔ شروع مارپیچ حول محور Z: جای درز (seam) چاپ مارپیچی و
    // جهت‌گیری گلبرگ‌ها/شیارها را می‌چرخاند. mesh و مسیر و کف همه
    // «هم‌زاویه» می‌مانند تا پیش‌نمایش = خروجی (قاعدهٔ منبع واحد).
    let phase0 = input.phase_start_deg.to_radians();

    // ---------- اعتبارسانی ورودی‌ها ----------
    if let Err(e) = crate::designer::profile::validate_profile(&input.profile) {
        diagnostics.push(Diagnostic::file_level(crate::diagnostics::Severity::Error, "E-PROFILE", e));
        return DesignerOutput {
            segments: vec![],
            line_count: 0,
            mesh: MeshData { vertices: vec![], triangles: vec![] },
            radius_samples: vec![],
            wall_mesh: WallMesh::default(),
            stats: ToolpathStats::default(),
            diagnostics,
            layers: vec![],
            summary_fa: "پروفایل نامعتبر است".into(),
        };
    }

    // ---------- بافتهای مؤثر — کلمپ دامنه بر پایهٔ نازل (نسخهٔ ۱.۵) ----------
    // انحراف شعاعی بزرگ‌تر از قطر نازل ⇒ لایهٔ بعدی روی هوا می‌نشیند
    // (برخورد نازل با دیوارهٔ موج‌دار و افت چسبندگی). دامنهٔ مؤثر به
    // ۰٫۹×قطر نازل محدود می‌شود و هشدار W-TEX-NOZZLE صادر می‌شود.
    let nozzle_amp_limit = (printer.nozzle_diameter_mm * 0.9).max(0.15);
    let mut nozzle_clamped = 0usize;
    let textures_eff: Vec<crate::designer::texture::TextureParams> = input
        .textures
        .iter()
        .map(|t| {
            if t.amplitude_mm > nozzle_amp_limit {
                nozzle_clamped += 1;
                let mut t2 = t.clone();
                t2.amplitude_mm = nozzle_amp_limit;
                t2
            } else {
                t.clone()
            }
        })
        .collect();
    if nozzle_clamped > 0 {
        diagnostics.push(Diagnostic::warning(
            "W-TEX-NOZZLE",
            format!(
                "دامنهٔ {nozzle_clamped} بافت به {nozzle_amp_limit:.2}mm محدود شد (۰٫۹×قطر نازل {0:.2}mm) — شیار عمیق‌تر از نازل باعث برخورد/لایهٔ معلق می‌شود",
                printer.nozzle_diameter_mm
            ),
            0,
        ));
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
    let mesh = build_mesh(
        input,
        &radius_fn,
        steps_per_rev,
        phase0,
        &mut diagnostics,
        input.mesh_quality.clamp(0.25, 4.0),
    );

    // ---------- مسیر مارپیچی + بافت‌ها → IR ----------
    let mut segments: Vec<ToolpathSegment> = Vec::new();
    let mut stats = ToolpathStats::default();
    let mut next_id: u64 = 0;
    let mut collided_any = false;

    // ---------- نمونه‌های شعاع فرم — نسخهٔ ۱.۴ (کارایی preview) ----------
    // UI اسپات‌ها از همین نمونه‌ها شعاع واقعی را می‌خواند؛ mesh کامل
    // فقط برای STL صادر می‌شود و دیگر در هر تغییر اسلایدر منتقل نمی‌شود.
    let radius_samples = build_radius_samples(&radius_fn, &input.patterns, height);

    // ---------- کف چندلایه — «قبل» از دیواره (نسخهٔ ۱.۴) ----------
    // رفع باگ ۱.۳: کف به انتهای segments اضافه می‌شد و در G-code بعد از
    // کل دیواره چاپ می‌شد — یعنی کف عملاً پرینت نمی‌شد و نازل پس از
    // اتمام فرم به کف می‌رفت. ترتیب صحیح: کف لایه‌به‌لایه، سپس vase.
    let (floor_segments, floor_top_z) =
        build_floor(input, &radius_fn, &mut next_id, &mut stats, bed_offset, phase0);
    let floor_count = floor_segments.len();
    segments.extend(floor_segments);

    // شروع مارپیچ — زاویهٔ شروع = فاز θ؛ z از «بالای کف» (نسخهٔ ۱.۴)
    let theta_start = phase0;
    let z_start = if input.floor.enabled && floor_count > 0 {
        floor_top_z + layer_h // اولین لایهٔ vase روی سطح کف
    } else {
        layer_h // بدون کف: از ارتفاع لایهٔ اول
    };
    let r0 = {
        // شعاع نقطهٔ نخست = شکل در انتهای کف (profile z=0) — نه z لایه
        let (r, clamped) =
            apply_patterns(radius_fn(0.0), 0.0, theta_start, height, &input.patterns);
        if clamped {
            collided_any = true;
        }
        r.max(0.05)
    };
    let first_point_model = [r0 * theta_start.cos(), r0 * theta_start.sin(), z_start];

    // بافت روی نقطهٔ اول (در فضای مدل — نسبت به محور)؛ zn=0 = انتهای کف
    let (first_point_model, c1, _int0) =
        apply_texture_ex(first_point_model, 0.0, r0, 0.0, &textures_eff);
    collided_any |= c1;
    // انتقال به مختصات صفحه (مرکز‌گذاری)
    let first_point = [
        first_point_model[0] + bed_offset[0],
        first_point_model[1] + bed_offset[1],
        first_point_model[2],
    ];

    // ---------- نقاط دیواره برای پیش‌نمایش جامد — نسخهٔ ۱٫۶ ----------
    // همین نقاط در حلقهٔ مارپیچ جمع می‌شوند؛ نوار بین انقلاب‌ها = سطح چاپی
    let mut wall_points: Vec<[f64; 3]> =
        Vec::with_capacity((total_revolutions * steps_per_rev as f64) as usize + 2);

    // travel اولیه — از آخرین نقطهٔ کف (اگر هست) وگرنه مرکز صفحه
    let mut prev_point = segments
        .last()
        .and_then(|s| s.end_mm)
        .unwrap_or([bed_offset[0], bed_offset[1], z_start]);
    let seg = ToolpathSegment {
        id: next_id,
        source_line_index: 0,
        kind: SegmentKind::Travel,
        start_mm: Some(prev_point),
        end_mm: Some(first_point),
        e_delta_mm: None,
        feedrate_mm_min: Some(printer.effective_travel_mm_min()),
        layer_index: None,
        certainty: Certainty::Known,
    };
    next_id += 1;
    stats.accumulate(&seg);
    segments.push(seg);
    prev_point = first_point;
    wall_points.push(first_point);

    // ---------- حلقهٔ مارپیچ ----------
    let mut e_counter = 0.0f64; // برای E مطلق؛ در نسبی delta مستقیم می‌رود
    let total_steps = (total_revolutions * steps_per_rev as f64).ceil() as usize;
    let mut path_length = 0.0f64;
    let mut clamped_radius_events = 0usize;
    // بیشینهٔ شدت بافت دیده‌شده (۰..۱) — نسخهٔ ۱.۵: مدولاسیون سرعت
    let mut tex_intensity_max: f64 = 0.0;

    for step in 1..=total_steps {
        // θ پیوسته بالا می‌رود (از فاز شروع)؛ z خطی با گام
        let theta = phase0 + step as f64 * d_theta;
        let z = (z_start + step as f64 * dz_per_step).min(z_start + height);
        // پروفایل از 0 تا height — شکل از بالای کف (z_start) شروع می‌شود
        let z_for_profile = (z - z_start).clamp(0.0, height);

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

        // نقطهٔ خام قبل از بافت (فضای مدل — نسبت به محور مرکزی)
        let raw_point = [r * theta.cos(), r * theta.sin(), z];
        path_length += (d_theta * r).abs(); // طول قوس تقریبی این گام
        // بافت روی طول پیموده‌شده — نسخهٔ ۱.۵: با دامنهٔ کلمپ‌شدهٔ نازل
        // + شدت (intensity) برای مدولاسیون سرعت (مثل ویدئوهای مرجع:
        // نازل روی فرورفتگی‌های بافت آرام‌تر می‌رود)
        let (point_model, collided, tex_intensity) = apply_texture_ex(
            raw_point,
            path_length,
            r,
            z_for_profile / height.max(1e-9),
            &textures_eff,
        );
        collided_any |= collided;
        tex_intensity_max = tex_intensity_max.max(tex_intensity);
        // انتقال به مختصات صفحه
        let point = [
            point_model[0] + bed_offset[0],
            point_model[1] + bed_offset[1],
            point_model[2],
        ];

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
        // ---------- مدولاسیون سرعت بافت — نسخهٔ ۱.۵ ----------
        // مثل ویدئوهای مرجع: نازل روی قله/درهٔ بافت آرام می‌رود تا
        // bead یکنواخت بماند و نازل به موج قبلی نکوبد:
        //   F = F_wall × (۱ − ۰٫۴×شدت) — حداقل ۶۰٪ سرعت دیواره
        let wall_f = printer.effective_wall_mm_min();
        let tex_f = wall_f * (1.0 - 0.4 * tex_intensity.clamp(0.0, 1.0));
        let seg = ToolpathSegment {
            id: next_id,
            source_line_index: segments.len(),
            kind: SegmentKind::Extrusion,
            start_mm: Some(prev_point),
            end_mm: Some(point),
            e_delta_mm: Some(e_delta),
            feedrate_mm_min: Some(tex_f.max(wall_f * 0.6)),
            layer_index: None, // مارپیچ: لایهٔ گسسته نه (الزام)
            certainty: Certainty::Known,
        };
        // اگر جابه‌جایی ناچیز است (بافت صفر و r ثابت) همچنان چاپ حساب می‌شود
        next_id += 1;
        stats.accumulate(&seg);
        segments.push(seg);
        prev_point = point;
        wall_points.push(point);
    }

    // ---------- نوار مارپیچی پیش‌نمایش — نسخهٔ ۱٫۶ (درخواست کاربر ۱) ----------
    // از نقاط «واقعی مسیر» → الگو و بافت هر دو در نمای جامد دیده می‌شوند
    let wall_mesh = build_wall_ribbon(&wall_points, steps_per_rev);

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
    // یادداشت مدولاسیون سرعت — فقط وقتی بافت فعال و اثرگذار بوده
    let tex_note = if !input.textures.is_empty() && tex_intensity_max > 0.05 {
        format!(
            "؛ بافت: سرعت تا {:.0}٪ دیواره در نقاط برجسته کاهش یافت",
            40.0 * tex_intensity_max
        )
    } else {
        String::new()
    };
    let summary_fa = format!(
        "فرم مارپیچی: ارتفاع {} mm، {} دور، {} قطعه مسیر{}؛ طول چاپ ≈ {} m؛ فیلامنت ≈ {} m؛ زمان تخمینی ≈ {} دقیقه؛ فاز θ = {}°؛ دمای نازل/بستر = {}/{} °C",
        format_num(height),
        format_num(total_revolutions),
        segments.len(),
        if floor_count > 0 {
            format!(" (+{floor_count} قطعهٔ کف در {} لایه)", input.floor.layers.clamp(1, 12))
        } else {
            String::new()
        },
        format_num(stats.extrusion_length_mm / 1000.0),
        format_num(stats.total_e_mm / 1000.0),
        format_num(est_time_min),
        format_num(input.phase_start_deg),
        format_num(printer.nozzle_temp_c),
        format_num(printer.bed_temp_c),
    );
    let summary_fa = format!("{summary_fa}{tex_note}");

    DesignerOutput {
        segments,
        line_count: total_steps + 1,
        mesh,
        radius_samples,
        wall_mesh,
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
    phase0: f64,
    diagnostics: &mut Vec<Diagnostic>,
    mesh_quality: f64,
) -> MeshData {
    let height = input.form.height_mm;
    // تراکم عمودی mesh: هر نیم‌لایه یک ردیف (سازگار با کیفیت مسیر)
    // نسخهٔ ۱.۵: ضریب کیفیت (دیالوگ خروجی STL) تراکم را مقیاس می‌کند؛
    // سقف‌ها برای جلوگیری از انفجار حافظه (حداکثر ~۱٫۷م مثلث)
    let rows_base = ((height / input.printer.layer_height_mm).ceil() as usize).clamp(8, 400) * 2;
    let rows = ((rows_base as f64) * mesh_quality.max(0.25)).round() as usize;
    let rows = rows.clamp(8, 1600);
    let cols = ((steps_per_rev as f64) * mesh_quality).round() as usize;
    let cols = cols.clamp(24, 1080);

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity((rows + 1) * (cols + 1));
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(rows * cols * 2);
    let mut degenerate = 0usize;

    for i in 0..=rows {
        let z = i as f64 * height / rows as f64;
        for j in 0..=cols {
            // چرخش شبکهٔ رأس‌ها با فاز θ (نسخهٔ ۱.۳)
            let theta = phase0 + j as f64 * 2.0 * std::f64::consts::PI / cols as f64;
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

/// ساخت کف دیسکی چندلایه با ۵ الگو — گام ۴ (نسخهٔ ۱.۴)
/// همهٔ الگوها مسیر واقعی تولید می‌کنند (الزام: «مسیر واقعی تولید کن»).
/// مختصات در فضای صفحه (با bed_offset) برگردانده می‌شود.
///
/// تغییرات نسخهٔ ۱.۴ (درخواست کاربر):
///   * `floor.layers` لایهٔ توپر — پس از اتمام کف، حالت vase ادامه می‌یابد؛
///   * لایهٔ اول با سرعت first_layer (آرام‌تر = چسبندگی به میز)، لایه‌های
///     بعدی کف با سرعت floor، travelها با سرعت travel؛
///   * لایه‌های فرد الگوهای خطی ۹۰° چرخیده چاپ می‌شوند (cross-hatch
///     استاندارد برای استحکام کف)؛ Radial نیم‌گام پرتو جابه‌جا می‌شود؛
///   * خروجی دوم: z لایهٔ آخر کف (سطح بالای کف) برای شروع مارپیچ.
#[allow(clippy::too_many_lines)]
fn build_floor(
    input: &DesignerInput,
    radius_fn: &impl Fn(f64) -> f64,
    next_id: &mut u64,
    stats: &mut ToolpathStats,
    bed_offset: [f64; 2],
    phase0: f64,
) -> (Vec<ToolpathSegment>, f64) {
    use crate::designer::FloorPattern;

    /// افزودن segment کمکی (travel یا extrusion) با سرعت صریح
    fn push_seg(
        out: &mut Vec<ToolpathSegment>,
        next_id: &mut u64,
        stats: &mut ToolpathStats,
        start: [f64; 3],
        end: [f64; 3],
        kind: SegmentKind,
        e: Option<f64>,
        feed_mm_min: f64,
    ) {
        let seg = ToolpathSegment {
            id: *next_id,
            source_line_index: out.len(),
            kind,
            start_mm: Some(start),
            end_mm: Some(end),
            e_delta_mm: e,
            feedrate_mm_min: Some(feed_mm_min),
            layer_index: None,
            certainty: Certainty::Known,
        };
        *next_id += 1;
        stats.accumulate(&seg);
        out.push(seg);
    }

    /// اکستروژن بین دو نقطه با E متناسب طول و سرعت صریح
    fn push_extrusion(
        out: &mut Vec<ToolpathSegment>,
        next_id: &mut u64,
        stats: &mut ToolpathStats,
        start: [f64; 3],
        end: [f64; 3],
        printer: &PrinterProfile,
        feed_mm_min: f64,
    ) {
        let d = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let e = extrusion_mm(
            len,
            printer.line_width_mm,
            printer.layer_height_mm,
            printer.filament_diameter_mm,
            printer.flow_multiplier,
        );
        push_seg(out, next_id, stats, start, end, SegmentKind::Extrusion, Some(e), feed_mm_min);
    }

    let mut out: Vec<ToolpathSegment> = Vec::new();
    if !input.floor.enabled {
        return (out, 0.0);
    }
    let floor = &input.floor;
    let printer = &input.printer;
    let lh = printer.layer_height_mm.max(0.01);
    // تعداد لایه‌های کف — نسخهٔ ۱.۴ (گیر ۱..۱۲ برای ایمنی)
    let layers = floor.layers.clamp(1, 12) as usize;
    let z_base = floor.z_start_mm.max(0.0);
    // شعاع بیرونی کف = شعاع پایین فرم
    let r_outer = radius_fn(0.0).max(0.5);
    let r_hole = (floor.center_hole_diameter_mm / 2.0).clamp(0.0, r_outer - 1.0);
    let gap = floor.ring_gap_mm.max(printer.line_width_mm * 0.9);
    // مرکز کف روی صفحه
    let cx = bed_offset[0];
    let cy = bed_offset[1];
    // سرعت‌های مؤثر — نسخهٔ ۱.۴
    let travel_feed = printer.effective_travel_mm_min();
    let first_feed = printer.effective_first_layer_mm_min();
    let floor_feed = printer.effective_floor_mm_min();
    // نقطهٔ جاری نازل — بین لایه‌ها پیوسته می‌ماند (travel بین لایه‌ها)
    let mut point = [cx, cy, (z_base + lh).max(0.2)];

    let mut z_top = 0.0f64;
    for layer_idx in 0..layers {
        let z = (z_base + (layer_idx + 1) as f64 * lh).max(0.2);
        z_top = z;
        let feed = if layer_idx == 0 { first_feed } else { floor_feed };
        // چرخش متقاطع لایه‌ها (cross-hatch استاندارد کف):
        // Grid/ZigZag لایهٔ فرد ۹۰°؛ Radial نیم‌گام پرتو؛ بقیه متقارن
        let rot = match floor.pattern {
            FloorPattern::Grid | FloorPattern::ZigZag => {
                if layer_idx % 2 == 1 { std::f64::consts::FRAC_PI_2 } else { 0.0 }
            }
            FloorPattern::Radial => {
                if layer_idx % 2 == 1 {
                    std::f64::consts::PI / floor.spokes.clamp(3, 96) as f64
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        // چرخش فاز θ — کل کف با فرم هم‌زاویه می‌چرخد (نسخهٔ ۱.۳)؛
        // برای الگوهای متقارن (حلقه/شبکه) بی‌اثر و برای شعاعی معنادار
        let (cph, sph) = ((phase0 + rot).cos(), (phase0 + rot).sin());
        // نقطهٔ صفحه از مختصات دیسک (نسبت به محور) — با چرخش φ+rot
        let bed_pt = |x: f64, y: f64| {
            let xr = x * cph - y * sph;
            let yr = x * sph + y * cph;
            [xr + cx, yr + cy, z]
        };

        match floor.pattern {
            // ---------------- حلقه‌های هم‌مرکز (رفتار ۱.۰/۱.۱) ----------------
            FloorPattern::Rings => {
                let mut r = r_outer;
                // travel به شروع حلقهٔ بیرونی (از نقطهٔ جاری — پیوستگی لایه‌ها)
                let start = bed_pt(r, 0.0);
                push_seg(&mut out, next_id, stats, point, start, SegmentKind::Travel, None, travel_feed);
                point = start;
                while r > r_hole + gap {
                    let steps = ((2.0 * std::f64::consts::PI * r / 2.0).ceil() as usize).clamp(24, 360);
                    let d_theta = 2.0 * std::f64::consts::PI / steps as f64;
                    for i in 1..=steps {
                        let theta = i as f64 * d_theta;
                        let np = bed_pt(r * theta.cos(), r * theta.sin());
                        push_extrusion(&mut out, next_id, stats, point, np, printer, feed);
                        point = np;
                    }
                    r -= gap;
                    if r <= r_hole {
                        break;
                    }
                    let inner = bed_pt(r * 0.997, 0.0);
                    push_seg(&mut out, next_id, stats, point, inner, SegmentKind::Travel, None, travel_feed);
                    point = inner;
                }
            }

            // ---------------- مارپیچ آرشیمدسی پیوسته ----------------
            // r(θ) = r_start + gap·θ/2π — یک مسیر پیوسته بدون travel
            // (بهترین گزینه برای کف حالت vase: بدون زخم travel)
            FloorPattern::Spiral => {
                let r_start = if r_hole > 0.5 { r_hole } else { (gap * 0.5).max(0.3) };
                let mut theta = 0.0f64;
                let mut r = r_start;
                // گام زاویه‌ای تطبیقی: قوس ≈ ۲mm
                let start = bed_pt(r, 0.0);
                push_seg(&mut out, next_id, stats, point, start, SegmentKind::Travel, None, travel_feed);
                point = start;
                let d_theta_max = 0.35; // حداکثر ~۲۰ درجه هر گام
                while r < r_outer {
                    let d_theta = (2.0 / r.max(0.5)).min(d_theta_max).max(0.02);
                    theta += d_theta;
                    r = r_start + gap * theta / (2.0 * std::f64::consts::PI);
                    let r_clamped = r.min(r_outer);
                    let np = bed_pt(r_clamped * theta.cos(), r_clamped * theta.sin());
                    push_extrusion(&mut out, next_id, stats, point, np, printer, feed);
                    point = np;
                }
            }

            // ---------------- شبکهٔ متقاطع (X سپس Y) ----------------
            FloorPattern::Grid => {
                // ردیف‌های هر دو جهت؛ ترتیب: 0، +g، −g، +2g، −2g…
                let mut rows: Vec<f64> = Vec::new();
                let mut k = 0u32;
                loop {
                    let off = k as f64 * gap;
                    if off > r_outer && k > 0 {
                        break;
                    }
                    rows.push(off);
                    if off > 0.0 {
                        rows.push(-off);
                    }
                    k += 1;
                    if k > 2000 {
                        break;
                    }
                }
                for axis in 0..2 {
                    for (idx, &y) in rows.iter().enumerate() {
                        if y.abs() > r_outer {
                            continue;
                        }
                        let half = (r_outer * r_outer - y * y).max(0.0).sqrt();
                        if half <= gap * 0.8 {
                            continue;
                        }
                        let (a, b) = if axis == 0 {
                            (bed_pt(-half, y), bed_pt(half, y))
                        } else {
                            (bed_pt(y, -half), bed_pt(y, half))
                        };
                        // جهت‌گیری یک‌درمیان برای کاهش travel
                        let (a, b) = if idx % 2 == 0 { (a, b) } else { (b, a) };
                        push_seg(&mut out, next_id, stats, point, a, SegmentKind::Travel, None, travel_feed);
                        push_extrusion(&mut out, next_id, stats, a, b, printer, feed);
                        point = b;
                    }
                }
            }

            // ---------------- پرتوهای شعاعی ----------------
            FloorPattern::Radial => {
                let spokes = floor.spokes.clamp(3, 96) as usize;
                let r_in = r_hole.max(0.3);
                // شروع از اولین پرتو — travel از نقطهٔ جاری
                let a0 = bed_pt(r_in, 0.0);
                push_seg(&mut out, next_id, stats, point, a0, SegmentKind::Travel, None, travel_feed);
                point = a0;
                for i in 0..spokes {
                    let ang = i as f64 * 2.0 * std::f64::consts::PI / spokes as f64;
                    let a = bed_pt(r_in * ang.cos(), r_in * ang.sin());
                    let b = bed_pt(r_outer * ang.cos(), r_outer * ang.sin());
                    // travel به ابتدای پرتو (از داخل به بیرون چاپ می‌کنیم)
                    push_seg(&mut out, next_id, stats, point, a, SegmentKind::Travel, None, travel_feed);
                    push_extrusion(&mut out, next_id, stats, a, b, printer, feed);
                    point = b;
                }
            }

            // ---------------- رفت‌وبرگشتی (مارپیچ خطی) ----------------
            // ردیف‌های موازی از -r تا +r که به‌صورت مارپیچ به هم وصل می‌شوند —
            // بدون travel در میانهٔ کف (بهترین پوشش بدون درز)
            FloorPattern::ZigZag => {
                // ردیف‌ها با گام gap از پایین دیسک تا بالا
                let n_rows = ((2.0 * r_outer) / gap).floor() as usize;
                if n_rows == 0 {
                    return (out, z_top);
                }
                let row_y = |idx: usize| -r_outer + idx as f64 * gap;
                // شروع از اولین ردیف — travel از نقطهٔ جاری
                let half0 = (r_outer * r_outer - row_y(0) * row_y(0)).max(0.0).sqrt();
                let start = bed_pt(-half0, row_y(0));
                push_seg(&mut out, next_id, stats, point, start, SegmentKind::Travel, None, travel_feed);
                point = start;
                for idx in 0..=n_rows {
                    let y = row_y(idx);
                    if y > r_outer {
                        break;
                    }
                    let half = (r_outer * r_outer - y * y).max(0.0).sqrt();
                    if half < gap * 0.5 {
                        continue;
                    }
                    let (a, b) = if idx % 2 == 0 {
                        (bed_pt(-half, y), bed_pt(half, y))
                    } else {
                        (bed_pt(half, y), bed_pt(-half, y))
                    };
                    // اتصال مارپیچی به ردیف قبلی (فقط اگر فاصله معنا‌دار باشد)
                    let conn = [
                        a[0] - point[0],
                        a[1] - point[1],
                        a[2] - point[2],
                    ];
                    let conn_len = (conn[0] * conn[0] + conn[1] * conn[1]).sqrt();
                    if conn_len > 0.05 {
                        push_extrusion(&mut out, next_id, stats, point, a, printer, feed);
                    }
                    push_extrusion(&mut out, next_id, stats, a, b, printer, feed);
                    point = b;
                    if idx > 4000 {
                        break; // محدودیت ایمنی
                    }
                }
            }
        }
    }
    (out, z_top)
}

/// نمونه‌گیری شعاع بیرونی فرم در ۱۲۹ ارتفاع — نسخهٔ ۱.۴
/// برای اسپات UI (شعاع واقعی فرم در هر z) بدون انتقال mesh کامل؛
/// الگوها لحاظ می‌شوند (max روی ۱۶ زاویه) — بافت‌ها نه (فقط مسیرند).
fn build_radius_samples(
    radius_fn: &impl Fn(f64) -> f64,
    patterns: &[crate::designer::pattern::PatternParams],
    height: f64,
) -> Vec<[f64; 2]> {
    let mut v = Vec::with_capacity(129);
    for i in 0..=128usize {
        let z = height * i as f64 / 128.0;
        let mut r_max = 0.0f64;
        for k in 0..16usize {
            let th = k as f64 * std::f64::consts::PI / 8.0;
            let (r, _) = apply_patterns(radius_fn(z), z, th, height, patterns);
            if r > r_max {
                r_max = r;
            }
        }
        v.push([z, r_max.max(0.1)]);
    }
    v
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
            center_on_bed: true,
            phase_start_deg: 0.0,
            printer: PrinterProfile::default(),
            mesh_quality: 1.0,
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

    // ================= آزمون‌های نسخهٔ ۱.۴ — کف چندلایه + سرعت =================

    #[test]
    fn test_floor_prints_before_wall() {
        // باگ ۱.۳: کف به «انتهای» مسیر اضافه می‌شد (بعد از کل دیواره).
        // اکنون کف باید «قبل» از دیواره چاپ شود: همهٔ segmentهای کف
        // (z پایین) قبل از همهٔ segmentهای دیواره (z بالا) می‌آیند.
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 2;
        let lh = inp.printer.layer_height_mm;
        let out = compute(&inp);
        let floor_top = 2.0 * lh; // سطح بالای کف
        // مرز کف/دیواره را پیدا کن: آخرین segment با z ≤ floor_top
        let last_floor_idx = out
            .segments
            .iter()
            .rposition(|s| s.end_mm.map(|e| e[2] <= floor_top + 1e-6).unwrap_or(false))
            .expect("باید segment کف وجود داشته باشد");
        // هیچ segment دیواره (z > floor_top) نباید قبل از مرز باشد
        for (i, s) in out.segments.iter().enumerate() {
            if let Some(e) = s.end_mm {
                if e[2] > floor_top + 1e-6 {
                    assert!(
                        i > last_floor_idx,
                        "segment دیواره (z={:.2}) قبل از کف (ایندکس {i} ≤ {last_floor_idx})",
                        e[2]
                    );
                }
            }
        }
        // و اولین segment اصلاً باید مربوط به کف باشد (travel به حلقهٔ بیرونی)
        let first_end = out.segments[0].end_mm.unwrap();
        assert!(first_end[2] <= floor_top + 1e-6, "اولین segment باید کف باشد");
    }

    #[test]
    fn test_floor_multi_layer_z_levels() {
        // layers=3 → اکستروژن‌های کف در ۳ سطح z متمایز
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 3;
        let lh = inp.printer.layer_height_mm;
        let out = compute(&inp);
        let floor_top = 3.0 * lh;
        let mut z_levels: Vec<f64> = Vec::new();
        for s in &out.segments {
            if s.kind == SegmentKind::Extrusion {
                if let Some(e) = s.end_mm {
                    if e[2] <= floor_top + 1e-6 {
                        if !z_levels.iter().any(|z| (z - e[2]).abs() < lh * 0.25) {
                            z_levels.push(e[2]);
                        }
                    }
                }
            }
        }
        assert!(
            z_levels.len() >= 3,
            "باید ≥۳ سطح z در کف باشد ولی {} سطح یافت شد: {z_levels:?}",
            z_levels.len()
        );
    }

    #[test]
    fn test_spiral_starts_above_floor() {
        // اولین اکستروژن دیواره باید دقیقاً بالای سطح کف باشد:
        // z = floor_top + layer_height (اولین لایهٔ vase روی کف)
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 2;
        let lh = inp.printer.layer_height_mm;
        let out = compute(&inp);
        let floor_top = 2.0 * lh;
        let first_wall = out
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Extrusion && s.end_mm.map(|e| e[2] > floor_top + 1e-6).unwrap_or(false))
            .expect("باید segment دیواره باشد");
        let z = first_wall.end_mm.unwrap()[2];
        assert!(
            (z - (floor_top + lh)).abs() < lh * 0.51,
            "اولین لایهٔ vase باید z≈{:.3} باشد نه {:.3}",
            floor_top + lh,
            z
        );
        // و بدون کف، همان رفتار قدیمی: شروع از layer_height
        let no_floor = compute(&base_input());
        let first0 = no_floor
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Extrusion)
            .unwrap();
        // «شروع» اولین اکستروژن روی لایهٔ اول است (پایانش یک گام dz بالاتر)
        let z0 = first0.start_mm.unwrap()[2];
        assert!((z0 - lh).abs() < 1e-9, "بدون کف: شروع از layer_height");
    }

    #[test]
    fn test_floor_speeds_by_layer() {
        // سرعت‌ها — لایهٔ اول کف آرام‌تر از لایه‌های بعدی و از دیواره
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 2;
        let out = compute(&inp);
        let lh = inp.printer.layer_height_mm;
        let floor_top = 2.0 * lh;
        let first_layer_feed = out
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Extrusion && s.end_mm.map(|e| e[2] <= lh + 1e-6).unwrap_or(false))
            .expect("اکستروژن لایهٔ اول کف")
            .feedrate_mm_min
            .unwrap();
        let second_layer_feed = out
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Extrusion && s.end_mm.map(|e| e[2] > lh + 1e-6 && e[2] <= floor_top + 1e-6).unwrap_or(false))
            .expect("اکستروژن لایهٔ دوم کف")
            .feedrate_mm_min
            .unwrap();
        let wall_feed = out
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Extrusion && s.end_mm.map(|e| e[2] > floor_top + 1e-6).unwrap_or(false))
            .expect("اکستروژن دیواره")
            .feedrate_mm_min
            .unwrap();
        // پیش‌فرض: دیواره ۲۴۰۰ (mm_min قدیمی) → اول ۱۲۰۰، کف ۱۸۰۰
        assert!((first_layer_feed - 1200.0).abs() < 1e-6, "لایهٔ اول = {first_layer_feed}");
        assert!((second_layer_feed - 1800.0).abs() < 1e-6, "لایهٔ دوم کف = {second_layer_feed}");
        assert!((wall_feed - 2400.0).abs() < 1e-6, "دیواره = {wall_feed}");
        assert!(first_layer_feed < second_layer_feed);
        assert!(second_layer_feed < wall_feed);
    }

    #[test]
    fn test_floor_layers_change_segment_count() {
        // لایه‌های بیشتر → segmentهای بیشتر (کف ضخیم‌تر)
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 1;
        let one = compute(&inp).segments.len();
        inp.floor.layers = 4;
        let four = compute(&inp).segments.len();
        assert!(four > one, "۴ لایه ({four}) باید بیشتر از ۱ لایه ({one}) باشد");
    }

    // ================= آزمون‌های نسخهٔ ۱.۵ =================

    #[test]
    fn test_mesh_quality_scales_triangles() {
        // کیفیت بالاتر → مثلث بیشتر (دیالوگ کیفیت STL)
        let q05 = {
            let mut inp = base_input();
            inp.mesh_quality = 0.5;
            compute(&inp).mesh.triangles.len()
        };
        let q1 = {
            let mut inp = base_input();
            inp.mesh_quality = 1.0;
            compute(&inp).mesh.triangles.len()
        };
        let q2 = {
            let mut inp = base_input();
            inp.mesh_quality = 2.0;
            compute(&inp).mesh.triangles.len()
        };
        assert!(q1 > q05, "q1={q1} q05={q05}");
        assert!(q2 > q1, "q2={q2} q1={q1}");
        // نسبت تقریباً مجذوری (rows×cols)
        let ratio = q2 as f64 / q1 as f64;
        assert!(ratio > 2.5 && ratio < 5.5, "ratio={ratio}");
    }

    #[test]
    fn test_mesh_quality_does_not_change_toolpath() {
        // کیفیت mesh فقط STL است — مسیر G-code نباید عوض شود
        let a = {
            let mut inp = base_input();
            inp.mesh_quality = 0.5;
            compute(&inp).segments.len()
        };
        let b = {
            let mut inp = base_input();
            inp.mesh_quality = 3.0;
            compute(&inp).segments.len()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_texture_speed_modulation() {
        // بافت فعال → سرعت‌های متفاوت روی دیواره (کاهش تا ۶۰٪)
        use crate::toolpath::SegmentKind;
        let mut inp = base_input();
        inp.textures.push(crate::designer::texture::TextureParams {
            kind: crate::designer::texture::TextureKind::Ripple,
            amplitude_mm: 0.3,
            period_mm: 4.0,
            radial: true,
            phase_deg: 0.0,
            fade_in_percent: 0.0,
            fade_out_percent: 0.0,
            ..Default::default()
        });
        let out = compute(&inp);
        let wall_f: Vec<f64> = out
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Extrusion && s.feedrate_mm_min.is_some())
            .map(|s| s.feedrate_mm_min.unwrap())
            .collect();
        assert!(wall_f.len() > 10);
        let max_f = wall_f.iter().cloned().fold(0.0f64, f64::max);
        let min_f = wall_f.iter().cloned().fold(f64::MAX, f64::min);
        // دامنهٔ کاهش: تا ۴۰٪ روی شدت کامل (حداقل ۶۰٪ دیواره)
        let wall = inp.printer.effective_wall_mm_min();
        assert!(max_f <= wall + 1e-6, "max={max_f} wall={wall}");
        assert!(min_f < wall * 0.99, "min={min_f} wall={wall} — باید کمتر شود");
        assert!(min_f >= wall * 0.6 - 1e-6, "min={min_f} — کف ۶۰٪");
    }

    #[test]
    fn test_texture_amplitude_nozzle_clamp() {
        // دامنهٔ بزرگ‌تر از ۰٫۹×نازل → هشدار W-TEX-NOZZLE
        let mut inp = base_input();
        inp.textures.push(crate::designer::texture::TextureParams {
            kind: crate::designer::texture::TextureKind::Ripple,
            amplitude_mm: 5.0, // نازل ۰٫۴ → کلمپ به ۰٫۳۶
            period_mm: 4.0,
            radial: true,
            phase_deg: 0.0,
            fade_in_percent: 0.0,
            fade_out_percent: 0.0,
            ..Default::default()
        });
        let out = compute(&inp);
        assert!(
            out.diagnostics.iter().any(|d| d.code == "W-TEX-NOZZLE"),
            "هشدار W-TEX-NOZZLE باید صادر شود"
        );
    }

    #[test]
    fn test_explicit_tangent_profile_computes() {
        // پروفایل با مماس صریح m — کل زنجیره بدون خطا اجرا می‌شود
        let mut inp = base_input();
        inp.profile = vec![
            crate::designer::profile::ProfilePoint { r: 10.0, h: 0.0, m: None },
            crate::designer::profile::ProfilePoint { r: 30.0, h: 50.0, m: Some(-0.35) },
            crate::designer::profile::ProfilePoint { r: 10.0, h: 100.0, m: None },
        ];
        inp.form.height_mm = 60.0;
        let out = compute(&inp);
        assert!(!out.segments.is_empty());
        assert!(out.mesh.triangles.len() > 100);
    }

    #[test]
    fn test_radius_samples_present() {
        // ۱۲۹ نمونه (z, r) برای اسپات UI — بدون الگو تقریباً ثابت
        let out = compute(&base_input());
        assert_eq!(out.radius_samples.len(), 129);
        assert!((out.radius_samples[0][0] - 0.0).abs() < 1e-9, "شروع از z=0");
        // با الگو: نمونه‌ها دامنهٔ بیشتری دارند (max روی θ)
        let mut inp = base_input();
        inp.patterns.push(PatternParams {
            kind: crate::designer::pattern::PatternKind::Petal,
            amplitude_mm: 4.0,
            count: 6,
            ..Default::default()
        });
        let with_pat = compute(&inp);
        // در میانهٔ ارتفاع، شعاع با الگو ≥ بدون الگو
        let mid = 64;
        assert!(with_pat.radius_samples[mid][1] >= out.radius_samples[mid][1] - 1e-9);
    }

    // ================= آزمون‌های چرخش فاز θ — نسخهٔ ۱.۳ =================

    #[test]
    fn test_phase_rotates_start_point() {
        // فاز ۹۰° → اولین نقطهٔ چاپ روی +Y محور (نه +X)
        // مرکز‌گذاری خاموش تا مختصات نسبت به محور سنجیده شود
        let mut inp = base_input();
        inp.center_on_bed = false;
        let p0 = compute(&inp);
        // بدون چرخش: اولین اکستروژن نزدیک +X
        let first0 = p0.segments[1].end_mm.unwrap();
        assert!(first0[0] > 10.0, "x باید مثبت بزرگ باشد: {first0:?}");
        assert!(first0[1].abs() < 5.0, "y باید نزدیک صفر باشد: {first0:?}");

        inp.phase_start_deg = 90.0;
        let p90 = compute(&inp);
        let first90 = p90.segments[1].end_mm.unwrap();
        // با چرخش ۹۰°: نقطه نزدیک +Y و x کوچک
        assert!(first90[1] > 10.0, "y باید مثبت بزرگ باشد: {first90:?}");
        assert!(first90[0].abs() < 5.0, "x باید نزدیک صفر باشد: {first90:?}");
        // شعاع (فاصله از محور) تغییر نکرده
        let r0 = (first0[0] * first0[0] + first0[1] * first0[1]).sqrt();
        let r90 = (first90[0] * first90[0] + first90[1] * first90[1]).sqrt();
        assert!((r0 - r90).abs() < 0.01, "شعاع نباید تغییر کند: {r0} vs {r90}");
    }

    #[test]
    fn test_phase_rotates_mesh_consistently() {
        // رأس j=0 ردیف z=0 دقیقاً روی زاویهٔ فاز می‌نشیند —
        // mesh با مسیر هم‌زاویه می‌چرخد (منبع واحد)
        let mut inp = base_input();
        inp.center_on_bed = false;
        inp.phase_start_deg = 90.0;
        let out = compute(&inp);
        // ردیف اول mesh: z=0، رأس اول (j=0) باید (≈0, +r) باشد
        let v0 = out.mesh.vertices[0];
        assert!(v0[2].abs() < 1e-9, "ردیف اول باید z=0 باشد");
        assert!(v0[0].abs() < 0.01, "x رأس اول باید ≈0 باشد: {v0:?}");
        assert!(v0[1] > 5.0, "y رأس اول باید مثبت بزرگ باشد: {v0:?}");
        // شعاع همان ردیف با حالت بدون چرخش برابر است
        let out0 = compute(&base_input());
        let v00 = out0.mesh.vertices[0];
        let r_a = (v0[0] * v0[0] + v0[1] * v0[1]).sqrt();
        let r_b = (v00[0] * v00[0] + v00[1] * v00[1]).sqrt();
        assert!((r_a - r_b).abs() < 0.01, "شعاع mesh نباید تغییر کند");
    }

    #[test]
    fn test_phase_full_circle_equivalence() {
        // چرخش ۳۶۰° = بدون چرخش (برای mesh و نقاط مسیر)
        let mut inp = base_input();
        inp.center_on_bed = false;
        inp.phase_start_deg = 360.0;
        let a = compute(&inp);
        let mut base = base_input();
        base.center_on_bed = false;
        let b = compute(&base);
        // اولین نقطهٔ چاپ
        let pa = a.segments[1].end_mm.unwrap();
        let pb = b.segments[1].end_mm.unwrap();
        assert!((pa[0] - pb[0]).abs() < 0.01 && (pa[1] - pb[1]).abs() < 0.01);
        // mesh هم همان است (مجموعهٔ رأس‌ها)
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        for (va, vb) in a.mesh.vertices.iter().zip(b.mesh.vertices.iter()) {
            assert!((va[0] - vb[0]).abs() < 0.01);
            assert!((va[1] - vb[1]).abs() < 0.01);
        }
    }

    #[test]
    fn test_phase_in_summary() {
        // خلاصهٔ فارسی فاز و دما را ذکر می‌کند (نسخهٔ ۱.۳)
        let mut inp = base_input();
        inp.phase_start_deg = 45.0;
        let out = compute(&inp);
        assert!(out.summary_fa.contains("فاز θ = 45"), "خلاصه: {}", out.summary_fa);
        assert!(out.summary_fa.contains("200/60"), "دماها: {}", out.summary_fa);
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
        inp.profile = vec![crate::designer::profile::ProfilePoint { r: 10.0, h: 0.0, m: None }];
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

    // ================= آزمون‌های نسخهٔ ۱.۲ =================

    #[test]
    fn test_center_on_bed_offsets_points() {
        // رفع باگ اصلی کاربر: طرح باید در مرکز صفحه (110,110) بنشیند
        // نه روی (0,0) — همهٔ نقاط درون صفحهٔ ۲۲۰×۲۲۰ می‌مانند
        let inp = base_input();
        let out = compute(&inp);
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        // مرکز واقعی مسیر ≈ مرکز صفحه
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut n = 0.0;
        for s in &out.segments {
            if let Some(p) = s.end_mm {
                sx += p[0];
                sy += p[1];
                n += 1.0;
            }
        }
        let (mx, my) = (sx / n, sy / n);
        assert!(
            (mx - cx).abs() < 2.0 && (my - cy).abs() < 2.0,
            "مرکز مسیر ({mx:.1},{my:.1}) باید نزدیک مرکز صفحه ({cx},{cy}) باشد"
        );
        // همهٔ نقاط درون صفحه
        for s in &out.segments {
            for p in [s.start_mm, s.end_mm].into_iter().flatten() {
                assert!(p[0] >= -1.0 && p[0] <= vol[0] + 1.0, "x خارج صفحه: {}", p[0]);
                assert!(p[1] >= -1.0 && p[1] <= vol[1] + 1.0, "y خارج صفحه: {}", p[1]);
            }
        }
    }

    #[test]
    fn test_no_center_warning_fires() {
        // خاموش‌کردن مرکز‌گذاری → هشدار صادقانه W-NO-CENTER
        let mut inp = base_input();
        inp.center_on_bed = false;
        let out = compute(&inp);
        assert!(
            out.diagnostics.iter().any(|d| d.code == "W-NO-CENTER"),
            "باید هشدار مرکز‌گذاری بیاید"
        );
        // و نقاط حول مبدأ می‌مانند (رفتار قدیمی)
        let first = out.segments[1].end_mm.unwrap();
        assert!(first[0].abs() < 60.0 && first[1].abs() < 60.0);
    }

    #[test]
    fn test_floor_patterns_all_generate() {
        // هر ۵ الگوی کف باید مسیر واقعی بسازند و در صفحه بمانند
        use crate::designer::FloorPattern;
        for pat in [
            FloorPattern::Rings,
            FloorPattern::Spiral,
            FloorPattern::Grid,
            FloorPattern::Radial,
            FloorPattern::ZigZag,
        ] {
            let mut inp = base_input();
            inp.floor.enabled = true;
            inp.floor.pattern = pat;
            inp.floor.center_hole_diameter_mm = 10.0;
            let out = compute(&inp);
            let floor_segs: Vec<_> = out
                .segments
                .iter()
                .filter(|s| s.end_mm.map(|p| (p[2] - 0.25).abs() < 0.3).unwrap_or(false))
                .collect();
            assert!(
                !floor_segs.is_empty(),
                "{pat:?} هیچ قطعهٔ کفی نساخت"
            );
            // همهٔ نقاط کف درون دیسک (فاصله از مرکز صفحه ≤ r_outer + تلورانس)
            let vol = inp.printer.build_volume_mm;
            let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
            for s in &floor_segs {
                for p in [s.start_mm, s.end_mm].into_iter().flatten() {
                    let d = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
                    assert!(
                        d <= 33.0,
                        "{pat:?}: نقطهٔ کف بیرون دیسک (d={d:.1})"
                    );
                }
            }
            // خلاصه باید کف را ذکر کند
            assert!(out.summary_fa.contains("کف"), "{pat:?} خلاصهٔ کف ندارد");
        }
    }

    #[test]
    fn test_floor_spiral_is_continuous() {
        // کف مارپیچی تک‌لایه: فقط یک travel شروع کف + یک travel شروع دیواره
        // (مزیت اعلام‌شدهٔ این الگو در UI)؛ لایه‌های بیشتر طبعاً travel بین‌لایه دارند
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 1;
        inp.floor.pattern = crate::designer::FloorPattern::Spiral;
        let out = compute(&inp);
        // فقط travel اولیهٔ دیواره + travel شروع کف = ۲ travel
        let travels = out.segments.iter().filter(|s| s.kind == SegmentKind::Travel).count();
        assert!(travels <= 2, "مارپیچ کف نباید travel میانه داشته باشد: {travels}");
    }

    #[test]
    fn test_floor_radial_spoke_count() {
        // کف شعاعی: تعداد پرتوها قابل‌کنترل است (پیش‌فرض ۱۲)
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.pattern = crate::designer::FloorPattern::Radial;
        inp.floor.spokes = 8;
        let out = compute(&inp);
        // پرتوها = extrusionهای هم‌سطح کف که شعاعشان max است (لبهٔ دیسک)
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        let mut edge_hits = 0;
        for s in &out.segments {
            if s.kind != SegmentKind::Extrusion {
                continue;
            }
            if let Some(p) = s.end_mm {
                if (p[2] - 0.25).abs() < 0.3 {
                    let d = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
                    if d > 28.0 {
                        edge_hits += 1;
                    }
                }
            }
        }
        assert!(edge_hits >= 8, "تعداد نقاط لبه باید ≥ ۸ باشد: {edge_hits}");
    }

    // ================= آزمون‌های نوار مارپیچی — نسخهٔ ۱٫۶ =================

    #[test]
    fn test_wall_mesh_built_from_path() {
        // نمای جامد دقیق: شبکهٔ منظم از نقاط واقعی مسیر
        let out = compute(&base_input());
        let wm = &out.wall_mesh;
        assert!(wm.rows >= 1, "ردیف انقلاب: {}", wm.rows);
        assert!(wm.cols >= 4, "ستون زاویه‌ای: {}", wm.cols);
        assert_eq!(wm.positions.len(), (wm.rows + 1) * (wm.cols + 1) * 3);
        // همهٔ مختصات متناهی و گردشده به ۰٫۰۱
        for v in &wm.positions {
            assert!(v.is_finite());
            let s = v * 100.0;
            assert!((s - s.round()).abs() < 1e-6, "گردشدن ۰٫۰۱: {v}");
        }
    }

    #[test]
    fn test_wall_mesh_vertex_matches_toolpath() {
        // رأس (۰,۰) = اولین نقطهٔ چاپ دیواره (گردشده) — «همان چیزی که چاپ می‌شود»
        let inp = base_input();
        let out = compute(&inp);
        // اولین نقطهٔ دیواره = پایان travel اولیه (شروع اکستروژن مارپیچ)
        let first_wall = out.segments[0].end_mm.unwrap();
        let v0 = &out.wall_mesh.positions[..3];
        assert!((v0[0] - first_wall[0]).abs() <= 0.01);
        assert!((v0[1] - first_wall[1]).abs() <= 0.01);
        assert!((v0[2] - first_wall[2]).abs() <= 0.01);
        // ستون بسته‌شدن ردیف ۰ (j=cols، ایندکس تخت cols) = نقطهٔ یک انقلاب
        // بعد از ستون ۰ — هم‌زاویه و layer_h بالاتر
        let cols = out.wall_mesh.cols;
        let vc = &out.wall_mesh.positions[cols * 3..cols * 3 + 3];
        let lh = inp.printer.layer_height_mm;
        assert!(
            (vc[2] - first_wall[2] - lh).abs() <= 0.02,
            "ستون بسته‌شدن باید یک لایه بالاتر باشد: {} vs {}",
            vc[2],
            first_wall[2] + lh
        );
        // رأس ردیف ۱ ستون ۰: row_step انقلاب بالاتر (row_step ≥ ۱)
        let stride = cols + 1;
        let v10 = &out.wall_mesh.positions[stride * 3..stride * 3 + 3];
        assert!(v10[2] > first_wall[2], "ردیف بعد باید بالاتر باشد");
    }

    #[test]
    fn test_wall_mesh_reflects_texture() {
        // رفع باگ ۱٫۵: بافت باید در نمای جامد دیده شود — نوار از نقاط مسیر
        let mut inp = base_input();
        let base = compute(&inp);
        inp.textures.push(TextureParams {
            kind: crate::designer::texture::TextureKind::Ripple,
            amplitude_mm: 0.35, // زیر کلمپ نازل
            period_mm: 3.0,
            ..Default::default()
        });
        let with_tex = compute(&inp);
        assert_eq!(base.wall_mesh.rows, with_tex.wall_mesh.rows);
        assert_eq!(base.wall_mesh.cols, with_tex.wall_mesh.cols);
        let mut diff = 0usize;
        for (a, b) in base.wall_mesh.positions.iter().zip(with_tex.wall_mesh.positions.iter()) {
            if (a - b).abs() > 1e-9 {
                diff += 1;
            }
        }
        assert!(diff > 30, "بافت باید نوار را تغییر دهد: {diff} مختصات");
    }

    #[test]
    fn test_wall_mesh_reflects_pattern() {
        // الگو هم در نمای جامد دیده می‌شود (قبلاً فقط پوش حداکثری دیده می‌شد)
        let mut inp = base_input();
        let base = compute(&inp);
        inp.patterns.push(PatternParams {
            kind: crate::designer::pattern::PatternKind::Flute,
            amplitude_mm: 2.0,
            count: 10,
            ..Default::default()
        });
        let with_pat = compute(&inp);
        // با الگوی شیار عمودی، رأس‌های مختلف زاویه‌ای باید شعاع متفاوت بگیرند
        let wm = &with_pat.wall_mesh;
        let stride = wm.cols + 1;
        let mut radii: Vec<f64> = Vec::new();
        for j in 0..wm.cols {
            let x = wm.positions[j * 3];
            let y = wm.positions[j * 3 + 1];
            radii.push((x * x + y * y).sqrt());
        }
        let _ = stride;
        let rmin = radii.iter().cloned().fold(f64::MAX, f64::min);
        let rmax = radii.iter().cloned().fold(0.0f64, f64::max);
        assert!(
            rmax - rmin > 1.0,
            "شیارها باید در ردیف اول دیده شوند: rmin={rmin} rmax={rmax}"
        );
    }

    #[test]
    fn test_wall_mesh_reflects_angular_spot() {
        // اسپات زاویه‌ای: الگو فقط روی قوس — رأس‌های خارج قوس شعاع پایه
        let mut inp = base_input();
        inp.patterns.push(PatternParams {
            kind: crate::designer::pattern::PatternKind::Petal,
            amplitude_mm: 4.0,
            count: 6,
            spot_center_deg: 0.0,
            spot_span_deg: 90.0,
            ..Default::default()
        });
        let out = compute(&inp);
        let wm = &out.wall_mesh;
        // شعاع رأس‌ها نسبت به محور فرم (مرکز صفحه)
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        let mut r_on = f64::MIN;
        let mut r_off = f64::MAX;
        for j in 0..wm.cols {
            let x = wm.positions[j * 3] - cx;
            let y = wm.positions[j * 3 + 1] - cy;
            let r = (x * x + y * y).sqrt();
            let theta = y.atan2(x).to_degrees();
            let d = (theta - 0.0 + 180.0).rem_euclid(360.0) - 180.0;
            if d.abs() < 20.0 {
                r_on = r_on.max(r);
            } else if d.abs() > 120.0 {
                r_off = r_off.min(r);
            }
        }
        assert!(r_on > r_off + 0.5, "داخل اسپات برجسته، خارج پایه: {r_on} vs {r_off}");
    }
}
