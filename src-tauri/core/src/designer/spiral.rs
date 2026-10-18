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
use crate::designer::{FloorSettings, PendantSettings, PrinterProfile, WallSettings};
use crate::designer::LampSettings;
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
    /// کیفیت خروجی G-code — نسخهٔ ۱٫۶ (درخواست کاربر ۶).
    /// ۰٫۵ = پیش‌نمایشی/زبر (گام قوس ۴mm و اعشار ۲)؛
    /// ۱ = استاندارد (۲mm، اعشار ۳)؛ ۲ = ریز (۱mm)؛ ۳ = بسیار ریز (۰٫۶۷mm، اعشار ۴).
    /// تراکم نقاط مسیر (steps_per_rev) را مقیاس می‌کند؛ mesh/STL مستقل است.
    pub gcode_quality: f64,
    /// تنظیمات دیواره (جامد/توری + سوراخ‌ها) — نسخهٔ ۱٫۷ (درخواست ۳ و ۹)
    pub wall: WallSettings,
    /// تنظیمات آویز حرفه‌ای — نسخهٔ ۱٫۷ (درخواست ۷)
    pub pendant: PendantSettings,
    /// تنظیمات حرفه‌ای لامپ — نسخهٔ ۱٫۷ (درخواست ۸)
    pub lamp: LampSettings,
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
    /// ماسک هر رأس: ۱ = سطح چاپی واقعی؛ ۰ = شکاف (سوراخ/travel) — نسخهٔ ۱٫۷:
    /// مثلث‌هایی که گوشه‌شان ماسک ۰ دارد در فرانت‌اند رندر نمی‌شوند ⇒
    /// دهانهٔ سوراخ‌های دیواره در نمای جامد واقعاً «باز» دیده می‌شود.
    #[serde(default)]
    pub mask: Vec<u8>,
    /// حالت دیواره: ۰ = جامد (نوار بین انقلاب‌ها)؛ ۱ = توری (نوار تارها در
    /// همان انقلاب) — نسخهٔ ۱٫۷: اتصال مثلث‌ها در فرانت‌اند بر اساس حالت
    #[serde(default)]
    pub mode: u8,
}

/// ساخت نوار مارپیچی از نقاط واقعی دیواره — نسخهٔ ۱٫۶ + ماسک/حالت ۱٫۷
/// wall_points[0] = اولین نقطهٔ چاپ دیواره؛ هر spr نقطه = یک انقلاب کامل.
/// flags[i] = ۱ اگر نقطهٔ i روی «چاپ» بوده؛ ۰ اگر travel (داخل سوراخ).
fn build_wall_ribbon(wall_points: &[[f64; 3]], flags: &[u8], spr: usize, lattice: bool) -> WallMesh {
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
    let mut mask: Vec<u8> = Vec::with_capacity((rows + 1) * stride);
    // گردکردن به ۰٫۰۱mm — دقت نمایش؛ JSON فشرده‌تر
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    for i in 0..=rows {
        let base = i * row_step * spr;
        for j in 0..cols {
            let idx = (base + j * col_step).min(total - 1);
            let p = wall_points[idx];
            positions.extend_from_slice(&[r2(p[0]), r2(p[1]), r2(p[2])]);
            mask.push(*flags.get(idx).unwrap_or(&1));
        }
        // ستون بسته‌شدن: دقیقاً یک انقلاب جلوتر (هم‌زاویهٔ ستون ۰)؛
        // ردیف آخر به آخرین نقطهٔ مسیر کلمپ می‌شود (هم‌زاویه در mod 2π)
        let idx_c = (base + spr).min(total - 1);
        let p = wall_points[idx_c];
        positions.extend_from_slice(&[r2(p[0]), r2(p[1]), r2(p[2])]);
        mask.push(*flags.get(idx_c).unwrap_or(&1));
    }
    WallMesh {
        rows,
        cols,
        positions,
        mask,
        mode: if lattice { 1 } else { 0 },
    }
}

/// محاسبهٔ E برای یک گام مسیر — مستند در راهنما
/// فرمول: E = (line_width × layer_height × طول) / (π × (d/2)²) × flow
/// مساحت مقطع فیلامنت از قطر فیلامنت پروفایل می‌آید.
/// عمومی crate — text2vase هم از همان فرمول واحد استفاده می‌کند.
pub(crate) fn extrusion_mm(
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

// ============================================================================
// توری — نسخهٔ ۱٫۷ (درخواست کاربر ۳): «چاپ توری مانند مرجع»
// ============================================================================
// اصل: روی استوانهٔ بازشده (u = r·θ، v = z) تارها «موج مثلثی» چاپ می‌شوند؛
// دورهٔ موج طوری انتخاب می‌شود که هر انقلاب «نیم‌دوره» جابه‌جا باشد
// (P_θ = 2π/(n+½)) ⇒ موج انقلاب بعدی با موج فعلی تقاطع می‌کند و در نقاط
// تقاطع روی هم می‌نشیند (مثل بافت توری/سبد) ⇒ خانه‌های لوزی باز.
//
// صعود هر انقلاب Δ = rise_ratio × A است (A = ارتفاع خانه)؛
// Δ = A/2 ⇒ الماس متقارن کلاسیک (پیش‌فرض).

/// موج مثلثی دورهٔ ۱ — قله در اعداد صحیح، دره در نیم‌دوره
#[inline]
fn tri_wave(t: f64) -> f64 {
    let f = t - t.floor();
    2.0 * (f - 0.5).abs() // ∈ [0,1]
}

/// موج بر اساس سبک توری — خروجی در [−۰٫۵، +۰٫۵]
fn lattice_wave(style: crate::designer::LatticeStyle, t: f64) -> f64 {
    let f = t - t.floor();
    match style {
        // الماسی: مثلث متقارن
        crate::designer::LatticeStyle::Diamond => tri_wave(t) - 0.5,
        // مربعی: ذوزنقه با سکوهای بالا/پایین (۲۰٪ هر ضلع)
        crate::designer::LatticeStyle::Square => {
            let s = ((tri_wave(t) - 0.2) / 0.6).clamp(0.0, 1.0);
            s - 0.5
        }
        // مهری: بالا رفتن تیز (۳۵٪ دوره) و پایین آمدن ملایم
        crate::designer::LatticeStyle::Chevron => {
            let v = if f < 0.35 { f / 0.35 } else { (1.0 - f) / 0.65 };
            v - 0.5
        }
        // پیچان: مثل الماسی (شیفت فاز در آرگومان اعمال می‌شود)
        crate::designer::LatticeStyle::Twisted => tri_wave(t) - 0.5,
    }
}

// ============================================================================
// سوراخ دیواره — نسخهٔ ۱٫۷ (درخواست کاربر ۹)
// ============================================================================
// هر سوراخ در فضای بازشدهٔ استوانه یک «مستطیل مرکزدار» است:
//   (θc, zc, half_w_arc, half_h). آزمون عضویت شکل‌دار؛ دهانهٔ واقعی با
//   travel عبور می‌کند و «قاب تقویتی» بسته دور آن چاپ می‌شود.

/// هندسهٔ یک سوراخ دیواره (فضای بازشده)
#[derive(Debug, Clone, Copy)]
struct WallHoleGeom {
    /// زاویهٔ مرکز (رادیان)
    theta_c: f64,
    /// ارتفاع مرکز نسبت به شروع دیواره (mm)
    z_c: f64,
    /// نیم‌عرض قوس (mm روی شعاع مرجع)
    half_w: f64,
    /// نیم‌ارتفاع (mm)
    half_h: f64,
}

/// آزمون شکل سوراخ با مختصات نرمال‌شده (u/hw, v/hh)
fn inside_hole_shape(shape: crate::designer::HoleShape, un: f64, vn: f64) -> bool {
    let au = un.abs();
    let av = vn.abs();
    match shape {
        crate::designer::HoleShape::Circle => un * un + vn * vn <= 1.0,
        crate::designer::HoleShape::RoundedRect => {
            // گوشه‌ها با شعاع ۲۵٪ کوچک‌ترین نیم‌ضلع گرد می‌شوند
            let r = 0.25;
            if au <= 1.0 - r || av <= 1.0 - r {
                au <= 1.0 && av <= 1.0
            } else {
                let dx = au - (1.0 - r);
                let dy = av - (1.0 - r);
                dx * dx + dy * dy <= r * r
            }
        }
        crate::designer::HoleShape::Diamond => au + av <= 1.0,
        crate::designer::HoleShape::Hexagon => {
            // شش‌ضلعی با راس چپ/راست: |u|≤1 و سقف |v| پله‌ای-شیبی
            au <= 1.0 && av <= (if au <= 0.5 { 1.0 } else { 2.0 * (1.0 - au) })
        }
        crate::designer::HoleShape::Star => {
            // ستارهٔ چهارپر (اضلاع مقعر): |u|^½ + |v|^½ ≤ ۱
            au.sqrt() + av.sqrt() <= 1.0
        }
    }
}

/// فاصلهٔ زاویه‌ای علامت‌دار به (−π, π]
#[inline]
fn wrap_pi(d: f64) -> f64 {
    (d + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI)
        - std::f64::consts::PI
}

/// آزمون: نقطهٔ (θ, z) داخل یکی از سوراخ‌هاست؟ (شعاع مرجع r_ref)
fn point_in_holes(
    holes: &[WallHoleGeom],
    shape: crate::designer::HoleShape,
    theta: f64,
    z_wall: f64,
    r_ref: f64,
) -> bool {
    if holes.is_empty() || r_ref <= 0.1 {
        return false;
    }
    for h in holes {
        let dth = wrap_pi(theta - h.theta_c);
        let u = dth * r_ref; // فاصلهٔ قوسی از مرکز
        let un = u / h.half_w.max(0.1);
        let vn = (z_wall - h.z_c) / h.half_h.max(0.1);
        if inside_hole_shape(shape, un, vn) {
            return true;
        }
    }
    false
}

/// نقطهٔ سطح دیواره در (θ, z مطلق) با الگوها — برای قاب‌های سوراخ و حلقه‌ها
fn wall_surface_point(
    radius_fn: &dyn Fn(f64) -> f64,
    patterns: &[crate::designer::pattern::PatternParams],
    theta: f64,
    z_abs: f64,
    z_start: f64,
    height: f64,
    bed_offset: [f64; 2],
) -> [f64; 3] {
    let zp = (z_abs - z_start).clamp(0.0, height);
    let (r, _) = apply_patterns(radius_fn(zp), zp, theta, height, patterns);
    let r = r.max(0.05);
    [r * theta.cos() + bed_offset[0], r * theta.sin() + bed_offset[1], z_abs]
}

/// چاپ قاب تقویتی بستهٔ دور یک سوراخ — نسخهٔ ۱٫۷ (درخواست کاربر ۹)
///
/// قاب در فضای بازشدهٔ استوانه یک «کپسول/مستطیل» است (گسترش pass_offset):
///   * شکل‌های گرد (دایره/شش‌ضلعی/ستاره/گوشه‌گرد): کپسول — دو نیم‌دایرهٔ
///     عمودی در دو طرف + دو قوس افقی بالا/پایین؛
///   * شکل‌های گوشه‌دار (لوزی): مستطیل ساده با ۴ گوشه.
/// همهٔ اضلاع با زیربخش ≤ ۱٫۵mm چاپ می‌شوند و «روی سطح فرم» می‌مانند
/// (شعاع از radius_fn + الگوها) ⇒ قاب واقعاً به دیواره می‌چسبد.
fn emit_hole_frame(
    ctx: &mut WallEmitCtx<'_>,
    hole: &WallHoleGeom,
    pass_offset: f64,
    shape: crate::designer::HoleShape,
) {
    let hw = (hole.half_w + pass_offset).max(0.5);
    let hh = (hole.half_h + pass_offset).max(0.5);
    let zc_abs = ctx.z_start + hole.z_c;
    let z_bottom = zc_abs - hh;
    let z_top = zc_abs + hh;
    let r_ref = ctx.r_ref.max(0.5);
    let th_half = (hw / r_ref).min(std::f64::consts::PI * 0.9);
    let th_l = hole.theta_c - th_half;
    let th_r = hole.theta_c + th_half;

    // مسیر بسته (θ, z_abs)
    let mut path: Vec<(f64, f64)> = Vec::new();
    let round_capsule = !matches!(shape, crate::designer::HoleShape::Diamond);
    if round_capsule {
        // کپسول: قوس چپ (بالا→پایین) + قوس پایین + قوس راست (پایین→بالا) + قوس بالا
        let steps = 10;
        // قوس چپ: از (θ_l, z+h) به (θ_l, z−h)
        for k in 0..=steps {
            let a = std::f64::consts::FRAC_PI_2 + k as f64 * std::f64::consts::PI / steps as f64;
            path.push((th_l, zc_abs + hh * a.sin()));
        }
        // قوس پایین: از (θ_l, z−h) به (θ_r, z−h) — نیم‌دایرهٔ پایین
        for k in 0..=steps {
            let a = std::f64::consts::PI + k as f64 * std::f64::consts::PI / steps as f64;
            // زاویه روی دایرهٔ سوراخ: x = cos a ∈ [−1..1]، y = sin a ≤ ۰
            let un = a.cos();
            let vn = a.sin();
            let th = hole.theta_c + th_half * un;
            let z = zc_abs + hh * vn;
            path.push((th, z));
        }
        // قوس راست: از (θ_r, z−h) به (θ_r, z+h)
        for k in 0..=steps {
            let a = -std::f64::consts::FRAC_PI_2 + k as f64 * std::f64::consts::PI / steps as f64;
            path.push((th_r, zc_abs + hh * a.sin()));
        }
        // قوس بالا: از (θ_r, z+h) به (θ_l, z+h) — نیم‌دایرهٔ بالا
        for k in 0..=steps {
            let a = k as f64 * std::f64::consts::PI / steps as f64;
            let un = a.cos();
            let vn = a.sin();
            let th = hole.theta_c + th_half * un;
            let z = zc_abs + hh * vn;
            path.push((th, z));
        }
    } else {
        // مستطیل ساده (لوزی‌وار برای شکل لوزی از قاب مستطیلی رد می‌شود)
        path.push((th_l, z_bottom));
        path.push((th_r, z_bottom));
        path.push((th_r, z_top));
        path.push((th_l, z_top));
    }
    if path.len() < 2 {
        return;
    }

    // سفر به نقطهٔ اول + چاپ مسیر بسته
    let first = wall_surface_point(
        ctx.radius_fn, ctx.patterns, path[0].0, path[0].1,
        ctx.z_start, ctx.height, ctx.bed_offset,
    );
    ctx.travel(first);
    for w in path.windows(2) {
        let (th0, z0) = w[0];
        let (th1, z1) = w[1];
        let du = (wrap_pi(th1 - th0) * r_ref).abs();
        let dv = (z1 - z0).abs();
        let n = ((du.max(dv) / 1.5).ceil() as usize).clamp(1, 64);
        for k in 1..=n {
            let t = k as f64 / n as f64;
            let th = th0 + (th1 - th0) * t;
            let z = z0 + (z1 - z0) * t;
            let p = wall_surface_point(
                ctx.radius_fn, ctx.patterns, th, z,
                ctx.z_start, ctx.height, ctx.bed_offset,
            );
            ctx.extrude(p);
        }
    }
    // بستن قاب
    let close = wall_surface_point(
        ctx.radius_fn, ctx.patterns, path[0].0, path[0].1,
        ctx.z_start, ctx.height, ctx.bed_offset,
    );
    ctx.extrude(close);
}

/// زمینهٔ انتشار segment دیواره — ابزار مشترک قاب/حلقه
struct WallEmitCtx<'a> {
    out: &'a mut Vec<ToolpathSegment>,
    next_id: &'a mut u64,
    stats: &'a mut ToolpathStats,
    printer: &'a PrinterProfile,
    radius_fn: &'a dyn Fn(f64) -> f64,
    patterns: &'a [crate::designer::pattern::PatternParams],
    z_start: f64,
    height: f64,
    bed_offset: [f64; 2],
    r_ref: f64,
    point: [f64; 3],
    travel_feed: f64,
    feed: f64,
}

impl<'a> WallEmitCtx<'_> {
    fn travel(&mut self, to: [f64; 3]) {
        let seg = ToolpathSegment {
            id: *self.next_id,
            source_line_index: self.out.len(),
            kind: SegmentKind::Travel,
            start_mm: Some(self.point),
            end_mm: Some(to),
            e_delta_mm: None,
            feedrate_mm_min: Some(self.travel_feed),
            layer_index: None,
            certainty: Certainty::Known,
        };
        *self.next_id += 1;
        self.stats.accumulate(&seg);
        self.out.push(seg);
        self.point = to;
    }

    fn extrude(&mut self, to: [f64; 3]) {
        let d = [to[0] - self.point[0], to[1] - self.point[1], to[2] - self.point[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let e = extrusion_mm(
            len,
            self.printer.line_width_mm,
            self.printer.layer_height_mm,
            self.printer.filament_diameter_mm,
            self.printer.flow_multiplier,
        );
        let seg = ToolpathSegment {
            id: *self.next_id,
            source_line_index: self.out.len(),
            kind: SegmentKind::Extrusion,
            start_mm: Some(self.point),
            end_mm: Some(to),
            e_delta_mm: Some(e),
            feedrate_mm_min: Some(self.feed),
            layer_index: None,
            certainty: Certainty::Known,
        };
        *self.next_id += 1;
        self.stats.accumulate(&seg);
        self.out.push(seg);
        self.point = to;
    }
}

/// چاپ حلقهٔ بستهٔ کامل در z ثابت (تقویت لبهٔ بالا — آویز) — نسخهٔ ۱٫۷
fn emit_ring_at_z(
    ctx: &mut WallEmitCtx<'_>,
    z_abs: f64,
    radius_scale: f64,
    theta0: f64,
) {
    let r_ref = ctx.r_ref.max(0.5);
    let steps = ((2.0 * std::f64::consts::PI * r_ref) / 1.8).ceil() as usize;
    let steps = steps.clamp(48, 400);
    let mut th = theta0;
    let first = wall_surface_point(
        ctx.radius_fn,
        ctx.patterns,
        th,
        z_abs,
        ctx.z_start,
        ctx.height,
        ctx.bed_offset,
    );
    // حلقه شعاع کمی بزرگ‌تر از فرم (روی لبه بنشیند): شعاع × scale
    let first = [
        ctx.bed_offset[0] + (first[0] - ctx.bed_offset[0]) * radius_scale,
        ctx.bed_offset[1] + (first[1] - ctx.bed_offset[1]) * radius_scale,
        first[2],
    ];
    ctx.travel(first);
    let d = 2.0 * std::f64::consts::PI / steps as f64;
    for k in 1..=steps {
        th = theta0 + k as f64 * d;
        let p = wall_surface_point(
            ctx.radius_fn,
            ctx.patterns,
            th,
            z_abs,
            ctx.z_start,
            ctx.height,
            ctx.bed_offset,
        );
        let p = [
            ctx.bed_offset[0] + (p[0] - ctx.bed_offset[0]) * radius_scale,
            ctx.bed_offset[1] + (p[1] - ctx.bed_offset[1]) * radius_scale,
            p[2],
        ];
        ctx.extrude(p);
    }
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
    // گام زاویه‌ای از طول قوس هدف — نسخهٔ ۱٫۶: کیفیت G-code گام را مقیاس می‌کند
    // (۰٫۵→۴mm زبر/فایل کوچک؛ ۱→۲mm استاندارد؛ ۲→۱mm ریز؛ ۳→۰٫۶۷mm بسیار ریز)
    let arc_target_mm = (2.0 / input.gcode_quality.clamp(0.25, 4.0)).clamp(0.6, 5.0);
    let circumference_max = 2.0 * std::f64::consts::PI * r_max;
    let steps_per_rev =
        ((circumference_max / arc_target_mm).ceil() as usize).clamp(24, 720);
    let d_theta = 2.0 * std::f64::consts::PI / steps_per_rev as f64;

    // حرکت مارپیچ: در هر دور کامل، Z به اندازهٔ layer_height بالا می‌رود
    let total_revolutions = (height / layer_h).ceil().max(1.0);
    // نسخهٔ ۱٫۷: z از انتگرال‌گیری z_center (نرخ وابسته به توری) می‌آید؛
    // dz_per_step ثابت قدیمی فقط مستندسازی است: layer_h / steps_per_rev

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
    // ماسک چاپ هر نقطه — نسخهٔ ۱٫۷ (سوراخ دیواره): ۱ = چاپ، ۰ = دهانه
    let mut wall_flags: Vec<u8> = Vec::with_capacity(wall_points.capacity());

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
    wall_flags.push(1);

    // ---------- حلقهٔ مارپیچ — نسخهٔ ۱٫۷: توری + سوراخ دیواره ----------
    let mut _e_counter = 0.0f64; // مستندسازی: E مطلق در خروجی‌ساز ساخته می‌شود
    let mut total_steps = (total_revolutions * steps_per_rev as f64).ceil() as usize;
    let mut path_length = 0.0f64;
    let mut clamped_radius_events = 0usize;
    // بیشینهٔ شدت بافت دیده‌شده (۰..۱) — نسخهٔ ۱.۵: مدولاسیون سرعت
    let mut tex_intensity_max: f64 = 0.0;

    // ===== توری — نسخهٔ ۱٫۷ (درخواست کاربر ۳) =====
    // موج مثلثی روی z با دورهٔ «n+½» در هر انقلاب ⇒ تارهای انقلاب‌های
    // مجاور ضدفازند، همدیگر را قطع می‌کنند و در تقاطع‌ها جوش می‌کنند
    // ⇒ خانه‌های لوزی باز مثل توری/سبد (مرجع درخواستی کاربر).
    let lattice_on = matches!(input.wall.mode, crate::designer::WallMode::Lattice);
    let lat = &input.wall.lattice;
    let lat_z0 = lat.z_start_mm.clamp(0.0, height * 0.95);
    let lat_z1 = lat
        .z_end_mm
        .unwrap_or(height)
        .clamp(lat_z0.min(height), height);
    let cell_a = lat.cell_height_mm.clamp(2.0, 80.0); // ارتفاع خانه
    let delta_rise = (cell_a * lat.rise_ratio.clamp(0.15, 0.9)).max(layer_h * 1.5);
    // دورهٔ زاویه‌ای موج: n+½ دوره در هر انقلاب (ضدفاز ⇒ تقاطع تارها)
    let lat_phase = lat.phase_deg.to_radians();
    let twist_k = if matches!(lat.style, crate::designer::LatticeStyle::Twisted) {
        0.055 // برش آرام شبکه در ارتفاع — چرخش مارپیچی توری
    } else {
        0.0
    };
    let p_theta = {
        let circ_mean = 2.0 * std::f64::consts::PI * r_max;
        let n = ((circ_mean / lat.cell_width_mm.max(3.0)) - 0.5).round().max(1.0);
        2.0 * std::f64::consts::PI / (n + 0.5)
    };
    // حالت موج: ۰ = جامد؛ ۱ = توری کامل (گذار نرم یک انقلاب)
    let mut lat_s = 0.0f64;
    // z مرکز (بدون موج) — انتگرال‌گیری تدریجی با نرخ وابسته به lat_s
    let mut z_center = z_start;

    // ===== سوراخ دیواره — نسخهٔ ۱٫۷ (درخواست کاربر ۹) =====
    let hole_cfg = &input.wall.holes;
    let holes_enabled = hole_cfg.enabled && hole_cfg.count_around >= 1 && hole_cfg.rows >= 1;
    let hole_shape = hole_cfg.shape;
    let hole_hw = (hole_cfg.width_mm * 0.5).max(1.0);
    let hole_hh = (hole_cfg.height_mm * 0.5).max(1.0);
    // ردیف‌ها: فاصلهٔ مرکز (خودکار = ارتفاع + ۶mm حاشیه)
    let hole_pitch = if hole_cfg.row_pitch_mm > 0.0 {
        hole_cfg.row_pitch_mm.max(hole_cfg.height_mm + 2.0)
    } else {
        hole_cfg.height_mm + 6.0
    };
    let hole_phase = hole_cfg.phase_deg.to_radians();
    // شعاع مرجع برای تبدیل قوس↔زاویه (میانهٔ فرم)
    let r_ref = {
        let mut rr = 0.0f64;
        for i in 0..8 {
            rr += radius_fn(height * i as f64 / 7.0);
        }
        (rr / 8.0).max(1.0)
    };
    let mut wall_holes: Vec<WallHoleGeom> = Vec::new();
    let mut frame_passes_of: Vec<usize> = Vec::new();
    let mut frame_printed: Vec<bool> = Vec::new();
    if holes_enabled {
        let n_around = hole_cfg.count_around.clamp(1, 12) as usize;
        let rows_n = hole_cfg.rows.clamp(1, 8) as usize;
        for row in 0..rows_n {
            // مرکز ردیف: از حاشیهٔ پایین + row·pitch؛ اگر از بالا بیرون زد رد می‌شود
            let zc = hole_cfg.margin_bottom_mm.max(2.0) + row as f64 * hole_pitch;
            if zc + hole_hh > height - hole_cfg.margin_top_mm.max(0.0) {
                continue;
            }
            let stagger_off = if hole_cfg.stagger && row % 2 == 1 {
                std::f64::consts::PI / n_around as f64
            } else {
                0.0
            };
            for k in 0..n_around {
                let th = hole_phase + stagger_off
                    + k as f64 * 2.0 * std::f64::consts::PI / n_around as f64;
                wall_holes.push(WallHoleGeom {
                    theta_c: th,
                    z_c: zc,
                    half_w: hole_hw,
                    half_h: hole_hh,
                });
                frame_passes_of.push(hole_cfg.frame_passes.min(3) as usize);
                frame_printed.push(false);
            }
        }
    }
    // ===== سوراخ‌های آویز بالای دیواره — نسخهٔ ۱٫۷ (درخواست کاربر ۷) =====
    let pendant = &input.pendant;
    let pendant_top = matches!(
        pendant.style,
        crate::designer::HangingStyle::TopHoles | crate::designer::HangingStyle::Both
    );
    let pendant_top_half = (pendant.top_hole_diameter_mm * 0.5).max(1.2);
    if pendant_top {
        let n = pendant.top_holes_count.clamp(2, 6) as usize;
        // مرکز سوراخ: top_hole_from_rim زیر لبهٔ بالا
        let zc = (height - pendant.top_hole_from_rim_mm.max(3.0))
            .min(height - pendant_top_half - 1.0);
        if zc - pendant_top_half > 2.0 {
            for k in 0..n {
                let th = phase0 + k as f64 * 2.0 * std::f64::consts::PI / n as f64;
                wall_holes.push(WallHoleGeom {
                    theta_c: th,
                    z_c: zc,
                    half_w: pendant_top_half,
                    half_h: pendant_top_half,
                });
                frame_passes_of.push(if pendant.top_hole_frames { 1 } else { 0 });
                frame_printed.push(false);
            }
        }
    }
    let n_holes = wall_holes.len();

    let total_steps_cap = total_steps + steps_per_rev * 4; // سقف ایمنی
    let mut step = 0usize;
    // آیا قدم قبلی داخل سوراخ بود؟ (پاک‌سازی خروج از دهانه — درخواست ۹:
    // هیچ اکستروژنی نباید از «داخل» سوراخ شروع شود و روی دهانه اثر بگذارد)
    let mut prev_in_hole = false;
    while step < total_steps_cap {
        step += 1;
        if z_center >= z_start + height {
            break; // فرم کامل شد (توری با Δ بزرگ سریع‌تر صعود می‌کند)
        }
        // θ پیوسته بالا می‌رود (از فاز شروع)
        let theta = phase0 + step as f64 * d_theta;

        // ---------- نرخ صعود و حالت توری ----------
        let z_profile_center = (z_center - z_start).clamp(0.0, height);
        let s_target = if lattice_on && z_profile_center >= lat_z0 && z_profile_center < lat_z1 {
            1.0
        } else {
            0.0
        };
        // گذار نرم: یک انقلاب کامل (steps_per_rev گام) از ۰→۱ یا ۱→۰
        let ds = 1.0 / steps_per_rev as f64;
        let d_s = (s_target - lat_s).clamp(-ds, ds);
        lat_s += d_s;
        // نرخ صعود: lerp(layer_h, Δ, s) — در هر انقلاب
        let rise_rate = layer_h + (delta_rise - layer_h) * lat_s;
        z_center += rise_rate / steps_per_rev as f64;

        // ---------- z نهایی: مرکز + موج توری ----------
        let z = if lat_s > 1e-6 {
            let arg = (theta - phase0 - lat_phase) / p_theta * (1.0 + twist_k);
            let w = lattice_wave(lat.style, arg) * cell_a * lat_s;
            (z_center + w).clamp(z_start, z_start + height)
        } else {
            z_center.min(z_start + height)
        };
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
        // بافت روی طول پیموده‌شده — نسخهٔ ۱٫۵: با دامنهٔ کلمپ‌شدهٔ نازل
        // + شدت (intensity) برای مدولاسیون سرعت
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

        // ---------- قاب سوراخ‌ها: هنگام رسیدن z مرکز به کف ردیف ----------
        // کف قاب = zc − hh؛ چاپ وقتی z مرکز نیم‌لایه از آن گذشته باشد
        // (قاب روی لایهٔ زیرین خود می‌نشیند و دیوارهٔ واقعی می‌سازد).
        if n_holes > 0 {
            let mut any_frame = false;
            for hi in 0..n_holes {
                if frame_printed[hi] {
                    continue;
                }
                let zb = wall_holes[hi].z_c - wall_holes[hi].half_h - layer_h * 0.5;
                if z_profile_center >= zb {
                    // قاب‌ها با travel خودشان از نقطهٔ فعلی مسیر شروع می‌شوند و
                    // به همان نقطه برمی‌گردند؛ سپس مارپیچ ادامه می‌یابد.
                    // (نقطهٔ شروع پیش از ساخت زمینه خوانده می‌شود — تفکیک borrow)
                    let start_pt = if any_frame {
                        segments.last().and_then(|s| s.end_mm).unwrap_or(prev_point)
                    } else {
                        prev_point
                    };
                    let mut emit_ctx = WallEmitCtx {
                        out: &mut segments,
                        next_id: &mut next_id,
                        stats: &mut stats,
                        printer,
                        radius_fn: &radius_fn,
                        patterns: &input.patterns,
                        z_start,
                        height,
                        bed_offset,
                        r_ref,
                        point: start_pt,
                        travel_feed: printer.effective_travel_mm_min(),
                        feed: printer.effective_wall_mm_min().min(900.0),
                    };
                    for p in 0..frame_passes_of[hi] {
                        let off = printer.line_width_mm * 0.55 * p as f64;
                        emit_hole_frame(&mut emit_ctx, &wall_holes[hi], off, hole_shape);
                    }
                    frame_printed[hi] = true;
                    any_frame = true;
                }
            }
            // ادامهٔ مسیر مارپیچ: برگشت به نقطهٔ فعلی با travel
            if any_frame {
                let back = prev_point;
                let seg = ToolpathSegment {
                    id: next_id,
                    source_line_index: segments.len(),
                    kind: SegmentKind::Travel,
                    start_mm: segments.last().and_then(|s| s.end_mm),
                    end_mm: Some(back),
                    e_delta_mm: None,
                    feedrate_mm_min: Some(printer.effective_travel_mm_min()),
                    layer_index: None,
                    certainty: Certainty::Known,
                };
                next_id += 1;
                stats.accumulate(&seg);
                segments.push(seg);
            }
        }

        // ---------- آزمون سوراخ: نقطهٔ جاری داخل دهانه؟ ----------
        let in_hole = n_holes > 0
            && z_for_profile > 1.0 // لایه‌های شروع دیواره سوراخ نمی‌شوند
            && point_in_holes(&wall_holes, hole_shape, theta, z_for_profile, r_ref);

        // طول واقعی این گام (پس از بافت)
        let d = [
            point[0] - prev_point[0],
            point[1] - prev_point[1],
            point[2] - prev_point[2],
        ];
        let seg_len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();

        // ---------- مدولاسیون سرعت بافت ۱٫۵ + توری ۱٫۷ ----------
        let wall_f = printer.effective_wall_mm_min();
        let mut tex_f = wall_f * (1.0 - 0.4 * tex_intensity.clamp(0.0, 1.0));
        if lat_s > 1e-6 {
            // تارهای توری مورب/معلق: آرام‌تر (سرعت نسبی توری)
            tex_f *= lat.speed_scale.clamp(0.3, 1.0).max(0.3);
        }

        if in_hole {
            // ---------- دهانهٔ سوراخ: travel بدون اکستروژن (درخواست ۹) ----------
            let seg = ToolpathSegment {
                id: next_id,
                source_line_index: segments.len(),
                kind: SegmentKind::Travel,
                start_mm: Some(prev_point),
                end_mm: Some(point),
                e_delta_mm: None,
                feedrate_mm_min: Some(printer.effective_travel_mm_min()),
                layer_index: None,
                certainty: Certainty::Known,
            };
            next_id += 1;
            stats.accumulate(&seg);
            segments.push(seg);
            prev_point = point;
            wall_points.push(point);
            wall_flags.push(0); // ماسک: دهانهٔ باز — مثلث رندر نشود
            prev_in_hole = true;
        } else if prev_in_hole {
            // ---------- خروج از سوراخ: travel تا نقطهٔ بیرونی، سپس چاپ ----------
            // قدم قبلی داخل دهانه بود؛ اکستروژن مستقیم یعنی شروع رشته از
            // داخل سوراخ و اثر روی دهانه. یک گام travel تمیز می‌کند و چاپ
            // از قدم بعد از «مرز بیرونی» آغاز می‌شود.
            let seg = ToolpathSegment {
                id: next_id,
                source_line_index: segments.len(),
                kind: SegmentKind::Travel,
                start_mm: Some(prev_point),
                end_mm: Some(point),
                e_delta_mm: None,
                feedrate_mm_min: Some(printer.effective_travel_mm_min()),
                layer_index: None,
                certainty: Certainty::Known,
            };
            next_id += 1;
            stats.accumulate(&seg);
            segments.push(seg);
            prev_point = point;
            wall_points.push(point);
            wall_flags.push(1); // نقطهٔ بیرونی — سطح چاپی
            prev_in_hole = false;
        } else {
            // ---------- چاپ عادی (جامد/توری) ----------
            // E این گام — توری: ضریب جریان تار
            let mut e_delta = extrusion_mm(
                seg_len,
                printer.line_width_mm,
                layer_h,
                printer.filament_diameter_mm,
                printer.flow_multiplier,
            );
            if lat_s > 1e-6 {
                e_delta *= lat.flow_scale.clamp(0.5, 2.0);
            }
            _e_counter += e_delta;

            let seg = ToolpathSegment {
                id: next_id,
                source_line_index: segments.len(),
                kind: SegmentKind::Extrusion,
                start_mm: Some(prev_point),
                end_mm: Some(point),
                e_delta_mm: Some(e_delta),
                feedrate_mm_min: Some(tex_f.max(wall_f * 0.3)),
                layer_index: None, // مارپیچ: لایهٔ گسسته نه (الزام)
                certainty: Certainty::Known,
            };
            next_id += 1;
            stats.accumulate(&seg);
            segments.push(seg);
            prev_point = point;
            wall_points.push(point);
            wall_flags.push(1);
        }
    }
    total_steps = step; // گام‌های واقعی طی‌شده (توری ممکن است زودتر تمام شود)

    // ---------- حلقه‌های تقویت لبهٔ بالا — آویز (درخواست کاربر ۷) ----------
    // لبهٔ بالای فرم تحت کشش آویز است؛ ۱..۶ حلقهٔ بسته در بالای دیواره
    // (هر حلقه یک دور کامل در z ثابت روی خود فرم) ⇒ لبهٔ ضخیم مقاوم.
    let pendant_any = !matches!(pendant.style, crate::designer::HangingStyle::None);
    if pendant.rim_reinforce_rings > 0 && pendant_any {
        let rings = pendant.rim_reinforce_rings.min(6) as usize;
        let mut cur = prev_point;
        for j in 0..rings {
            let z_ring = (z_start + height - j as f64 * printer.line_width_mm * 0.9)
                .max(z_start + height - printer.line_width_mm * 5.0);
            let mut emit_ctx = WallEmitCtx {
                out: &mut segments,
                next_id: &mut next_id,
                stats: &mut stats,
                printer,
                radius_fn: &radius_fn,
                patterns: &input.patterns,
                z_start,
                height,
                bed_offset,
                r_ref,
                point: cur,
                travel_feed: printer.effective_travel_mm_min(),
                feed: printer.effective_wall_mm_min(),
            };
            emit_ring_at_z(&mut emit_ctx, z_ring, 1.0, phase0);
            cur = emit_ctx.point;
        }
        let _ = cur; // نقطهٔ پایانی حلقه‌ها — مسیر بعدی travel خودش را دارد
    }

    // ---------- نوار مارپیچی پیش‌نمایش — ۱٫۶ + ماسک/حالت ۱٫۷ ----------
    // از نقاط «واقعی مسیر» → الگو و بافت در نمای جامد دیده می‌شوند؛
    // ماسک ۰ = دهانهٔ سوراخ؛ حالت ۱ = توری (اتصال تارها در فرانت‌اند).
    let wall_mesh = build_wall_ribbon(&wall_points, &wall_flags, steps_per_rev, lattice_on);

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
/// ساخت کف دیسکی چندلایه با ۱۰ الگو + تنظیمات حرفه‌ای — نسخهٔ ۱٫۶
/// ==========================================================================
/// همهٔ الگوها مسیر واقعی تولید می‌کنند (الزام: «مسیر واقعی تولید کن»).
/// مختصات در فضای صفحه (با bed_offset) برمی‌گردد.
///
/// معماری ۱٫۶ — «زمینهٔ کف» (FloorCtx) با روش‌های کمکی مشترک:
///   * line(a,b): پاره‌خط محلی با «برش سوراخ‌های لبه» (clip_to_holes)؛
///   * arc/ring: قوس‌های گام‌کوتاه با پرهیز از سوراخ‌ها؛
///   * circle_around: حلقهٔ تقویت دور سوراخ دلخواه؛
///   * تقویت لایهٔ آخر: دور لبه (آویز) + سوراخ مرکزی (سوکت لامپ).
///
/// الگوهای جدید (درخواست کاربر ۷):
///   Honeycomb شانه‌زنبوری · Wave موج دایره‌ای · Star ستاره‌ای ·
///   Snowflake دانه‌برفی · Octagram گره هندسی هشت‌پر
///
/// تغییرات ۱٫۴ (پایه): کف چندلایه توپر + سرعت first/floor/travel +
/// cross-hatch لایه‌های فرد + خروجی z بالای کف برای شروع مارپیچ.
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

    // =========================================================================
    // برش پاره‌خط به بازه‌های «خارج از سوراخ‌های لبه» — نسخهٔ ۱٫۶
    // هندسهٔ واقعی: تقاطع پاره‌خط با دایرهٔ هر سوراخ (درجه۲)؛ بازه‌های
    // داخل سوراخ ادغام و حذف می‌شوند → خروجی: پاره‌های آزاد (محلی).
    // =========================================================================
    fn clip_to_holes(
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
        holes: &[(f64, f64, f64)],
    ) -> Vec<(f64, f64, f64, f64)> {
        if holes.is_empty() {
            return vec![(ax, ay, bx, by)];
        }
        let dx = bx - ax;
        let dy = by - ay;
        let len2 = dx * dx + dy * dy;
        if len2 < 1e-12 {
            let inside = holes.iter().any(|(hx, hy, hr)| (ax - hx).hypot(ay - hy) < *hr);
            return if inside { vec![] } else { vec![(ax, ay, bx, by)] };
        }
        let mut blocked: Vec<(f64, f64)> = Vec::new();
        for &(hx, hy, hr) in holes {
            let t0 = ((hx - ax) * dx + (hy - ay) * dy) / len2;
            let d2 = (hx - ax) * (hx - ax) + (hy - ay) * (hy - ay) - t0 * t0 * len2;
            if d2 >= hr * hr {
                continue; // خط سوراخ را قطع نمی‌کند
            }
            let dt = ((hr * hr - d2) / len2).sqrt();
            let s = (t0 - dt).clamp(0.0, 1.0);
            let e = (t0 + dt).clamp(0.0, 1.0);
            if e > s {
                blocked.push((s, e));
            }
        }
        if blocked.is_empty() {
            return vec![(ax, ay, bx, by)];
        }
        blocked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (s, e) in blocked {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1e-9 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }
        let mut out = Vec::new();
        let mut cur = 0.0f64;
        for (s, e) in merged {
            if s > cur + 1e-4 {
                out.push((ax + dx * cur, ay + dy * cur, ax + dx * s, ay + dy * s));
            }
            cur = cur.max(e);
        }
        if cur < 1.0 - 1e-4 {
            out.push((ax + dx * cur, ay + dy * cur, bx, by));
        }
        out
    }

    /// نقطهٔ (r, θ) — داخل یکی از سوراخ‌های لبه است؟
    fn angle_in_hole(r: f64, theta: f64, holes: &[(f64, f64, f64)]) -> bool {
        let x = r * theta.cos();
        let y = r * theta.sin();
        holes.iter().any(|(hx, hy, hr)| (x - hx).hypot(y - hy) < *hr)
    }

    /// زمینهٔ کف — تبدیل مختصات + انتشار segment + برش سوراخ
    struct FloorCtx<'a> {
        out: &'a mut Vec<ToolpathSegment>,
        next_id: &'a mut u64,
        stats: &'a mut ToolpathStats,
        printer: &'a PrinterProfile,
        holes: Vec<(f64, f64, f64)>,
        point: [f64; 3],
        travel_feed: f64,
        feed: f64,
        cph: f64,
        sph: f64,
        cx: f64,
        cy: f64,
        z: f64,
    }

    impl<'a> FloorCtx<'a> {
        /// نقطهٔ محلی دیسک → مختصات صفحه (با چرخش φ+rot لایه)
        fn bed_pt(&self, x: f64, y: f64) -> [f64; 3] {
            let xr = x * self.cph - y * self.sph;
            let yr = x * self.sph + y * self.cph;
            [xr + self.cx, yr + self.cy, self.z]
        }

        /// مختصات صفحه → محلی دیسک (وارون تبدیل — برای اتصال ZigZag)
        fn local_pt(&self, p: [f64; 3]) -> (f64, f64) {
            let dx = p[0] - self.cx;
            let dy = p[1] - self.cy;
            (dx * self.cph + dy * self.sph, -dx * self.sph + dy * self.cph)
        }

        /// travel به نقطهٔ صفحه
        fn travel(&mut self, to: [f64; 3]) {
            let seg = ToolpathSegment {
                id: *self.next_id,
                source_line_index: self.out.len(),
                kind: SegmentKind::Travel,
                start_mm: Some(self.point),
                end_mm: Some(to),
                e_delta_mm: None,
                feedrate_mm_min: Some(self.travel_feed),
                layer_index: None,
                certainty: Certainty::Known,
            };
            *self.next_id += 1;
            self.stats.accumulate(&seg);
            self.out.push(seg);
            self.point = to;
        }

        /// اکستروژن بین دو نقطهٔ صفحه
        fn extrude_bed(&mut self, a: [f64; 3], b: [f64; 3]) {
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let e = extrusion_mm(
                len,
                self.printer.line_width_mm,
                self.printer.layer_height_mm,
                self.printer.filament_diameter_mm,
                self.printer.flow_multiplier,
            );
            let seg = ToolpathSegment {
                id: *self.next_id,
                source_line_index: self.out.len(),
                kind: SegmentKind::Extrusion,
                start_mm: Some(a),
                end_mm: Some(b),
                e_delta_mm: Some(e),
                feedrate_mm_min: Some(self.feed),
                layer_index: None,
                certainty: Certainty::Known,
            };
            *self.next_id += 1;
            self.stats.accumulate(&seg);
            self.out.push(seg);
            self.point = b;
        }

        /// پاره‌خط محلی با برش سوراخ — هر پارهٔ آزاد چاپ؛ بین پاره‌ها travel
        fn line(&mut self, ax: f64, ay: f64, bx: f64, by: f64) {
            for (x0, y0, x1, y1) in clip_to_holes(ax, ay, bx, by, &self.holes) {
                let a = self.bed_pt(x0, y0);
                let b = self.bed_pt(x1, y1);
                let d = (a[0] - self.point[0]).hypot(a[1] - self.point[1]);
                if d > 0.05 {
                    self.travel(a);
                }
                self.extrude_bed(a, b);
            }
        }

        /// قوس دایرهٔ شعاع r حول مرکز از θ1 تا θ2 (گام ~۲mm)؛
        /// نقاط داخل سوراخ لبه travel می‌شوند (حلقه در سوراخ قطع می‌شود).
        fn arc(&mut self, r: f64, theta1: f64, theta2: f64) {
            let span = theta2 - theta1;
            if span.abs() < 1e-9 {
                return;
            }
            let steps = ((r * span.abs() / 2.0).ceil() as usize).clamp(3, 480);
            let dt = span / steps as f64;
            for i in 1..=steps {
                let th = theta1 + i as f64 * dt;
                let th_mid = th - dt * 0.5;
                let to = self.bed_pt(r * th.cos(), r * th.sin());
                if angle_in_hole(r, th_mid, &self.holes) {
                    self.travel(to);
                } else {
                    let from = self.point;
                    self.extrude_bed(from, to);
                }
            }
        }

        /// دایرهٔ کامل حول مرکز دیسک (شروع از θ0)
        fn ring(&mut self, r: f64, theta0: f64) {
            let start = self.bed_pt(r * theta0.cos(), r * theta0.sin());
            let d = (start[0] - self.point[0]).hypot(start[1] - self.point[1]);
            if d > 0.05 {
                self.travel(start);
            }
            self.arc(r, theta0, theta0 + 2.0 * std::f64::consts::PI);
        }

        /// دایرهٔ کامل حول مرکز دلخواه (سوراخ لبه) — تقویت
        fn circle_around(&mut self, hcx: f64, hcy: f64, r: f64) {
            let steps =
                ((2.0 * std::f64::consts::PI * r / 2.0).ceil() as usize).clamp(12, 240);
            let dt = 2.0 * std::f64::consts::PI / steps as f64;
            let start = self.bed_pt(hcx + r, hcy);
            self.travel(start);
            for i in 1..=steps {
                let th = i as f64 * dt;
                let to = self.bed_pt(hcx + r * th.cos(), hcy + r * th.sin());
                let from = self.point;
                self.extrude_bed(from, to);
            }
        }
    }

    let mut out: Vec<ToolpathSegment> = Vec::new();
    if !input.floor.enabled {
        return (out, 0.0);
    }
    let floor = &input.floor;
    let printer = &input.printer;
    let lh = printer.layer_height_mm.max(0.01);
    let layers = floor.layers.clamp(1, 12) as usize;
    let z_base = floor.z_start_mm.max(0.0);
    let r_outer = radius_fn(0.0).max(0.5);
    // ---------- لامپ — نسخهٔ ۱٫۷ (درخواست کاربر ۸) ----------
    // سوکت لامپ بر سوراخ مرکزی معمولی تقدم دارد؛ ضریب فاصله، کفِ
    // «نورپذیر» بازتر می‌سازد (خطوط کمتر = عبور نور بیشتر).
    let lamp = &input.lamp;
    let lamp_on = lamp.enabled;
    let r_hole = if lamp_on {
        (lamp.center_hole_diameter_mm / 2.0).clamp(0.0, r_outer - 1.5)
    } else {
        (floor.center_hole_diameter_mm / 2.0).clamp(0.0, r_outer - 1.0)
    };
    let gap = if lamp_on {
        (floor.ring_gap_mm * lamp.floor_gap_multiplier.clamp(0.5, 3.0))
            .max(printer.line_width_mm * 0.9)
    } else {
        floor.ring_gap_mm.max(printer.line_width_mm * 0.9)
    };
    let lw = printer.line_width_mm.max(0.1);
    let cx = bed_offset[0];
    let cy = bed_offset[1];
    let travel_feed = printer.effective_travel_mm_min();
    let first_feed = printer.effective_first_layer_mm_min();
    let floor_feed = printer.effective_floor_mm_min();

    // ---------- سوراخ‌های لبه — ۱٫۶ + آویز حرفه‌ای ۱٫۷ (درخواست ۷) ----------
    // N سوراخ با قطر قابل‌تنظیم نزدیک لبهٔ کف؛ برای رد کردن نخ آویز،
    // سیم/کابل لامپ یا تهویه. الگوها این نقاط را خالی می‌گذارند و
    // حلقهٔ تقویت دورشان در لایهٔ آخر چاپ می‌شود.
    // ۱٫۷: تنظیمات آویز (تعداد/قطر/موقعیت ٪شعاع/فاز) بر کف تقدم دارد.
    let pendant = &input.pendant;
    let pendant_rim = matches!(
        pendant.style,
        crate::designer::HangingStyle::RimHoles | crate::designer::HangingStyle::Both
    );
    let rim_holes = if pendant_rim {
        pendant.rim_holes_count.min(12)
    } else {
        floor.rim_holes.min(12)
    };
    let hr = if pendant_rim {
        (pendant.rim_hole_diameter_mm / 2.0).clamp(0.4, r_outer * 0.25)
    } else {
        (floor.rim_hole_diameter_mm / 2.0).clamp(0.4, r_outer * 0.25)
    };
    let rim_phase = if pendant_rim {
        phase0 + pendant.rim_phase_deg.to_radians()
    } else {
        phase0
    };
    let holes: Vec<(f64, f64, f64)> = if rim_holes > 0 && r_outer > hr * 3.0 {
        let r_ring = if pendant_rim {
            // موقعیت ٪ شعاعی (۶۰..۹۲٪) — قابل‌تنظیم کاربر
            (r_outer * pendant.rim_hole_radius_percent.clamp(40.0, 95.0) / 100.0)
                .clamp(r_hole + hr + 1.0, (r_outer - hr - lw).max(r_hole + hr + 1.0))
        } else {
            (r_outer - hr - lw * 2.5).max(r_hole + hr + 2.0)
        };
        (0..rim_holes)
            .map(|k| {
                let th = rim_phase + k as f64 * 2.0 * std::f64::consts::PI / rim_holes as f64;
                (r_ring * th.cos(), r_ring * th.sin(), hr)
            })
            .collect()
    } else {
        Vec::new()
    };

    // پیوستگی نازل بین لایه‌ها — نقطهٔ پایان هر لایه شروع لایهٔ بعد
    let mut point: [f64; 3] = [cx, cy, (z_base + lh).max(0.2)];
    let mut z_top = 0.0f64;
    for layer_idx in 0..layers {
        let z = (z_base + (layer_idx + 1) as f64 * lh).max(0.2);
        z_top = z;
        let feed = if layer_idx == 0 { first_feed } else { floor_feed };
        // چرخش متقاطع لایه‌ها (cross-hatch استاندارد کف)
        let rot = match floor.pattern {
            FloorPattern::Grid | FloorPattern::ZigZag | FloorPattern::Honeycomb => {
                if layer_idx % 2 == 1 { std::f64::consts::FRAC_PI_2 } else { 0.0 }
            }
            FloorPattern::Radial => {
                if layer_idx % 2 == 1 {
                    std::f64::consts::PI / floor.spokes.clamp(3, 96) as f64
                } else {
                    0.0
                }
            }
            FloorPattern::Star | FloorPattern::Snowflake => {
                if layer_idx % 2 == 1 { std::f64::consts::PI / 6.0 } else { 0.0 }
            }
            FloorPattern::Octagram => {
                if layer_idx % 2 == 1 { std::f64::consts::PI / 8.0 } else { 0.0 }
            }
            FloorPattern::LampWeb | FloorPattern::Sunburst => {
                let n = if lamp_on {
                    lamp.spokes_count.clamp(3, 16) as f64
                } else {
                    floor.spokes.clamp(3, 16) as f64
                };
                if layer_idx % 2 == 1 {
                    std::f64::consts::PI / n
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        let (cph, sph) = ((phase0 + rot).cos(), (phase0 + rot).sin());
        let mut ctx = FloorCtx {
            out: &mut out,
            next_id,
            stats,
            printer,
            holes: holes.clone(),
            point,
            travel_feed,
            feed,
            cph,
            sph,
            cx,
            cy,
            z,
        };

        match floor.pattern {
            // ---------------- حلقه‌های هم‌مرکز (رفتار ۱.۰/۱.۱) ----------------
            FloorPattern::Rings => {
                let mut r = r_outer;
                while r > r_hole + gap {
                    ctx.ring(r, 0.0);
                    r -= gap;
                    if r <= r_hole {
                        break;
                    }
                }
            }

            // ---------------- مارپیچ آرشیمدسی پیوسته ----------------
            FloorPattern::Spiral => {
                let r_start = if r_hole > 0.5 { r_hole } else { (gap * 0.5).max(0.3) };
                let mut theta = 0.0f64;
                let mut r = r_start;
                let start = ctx.bed_pt(r, 0.0);
                ctx.travel(start);
                let d_theta_max = 0.35;
                while r < r_outer {
                    let d_theta = (2.0 / r.max(0.5)).min(d_theta_max).max(0.02);
                    theta += d_theta;
                    r = r_start + gap * theta / (2.0 * std::f64::consts::PI);
                    let r_clamped = r.min(r_outer);
                    let to = ctx.bed_pt(r_clamped * theta.cos(), r_clamped * theta.sin());
                    let th_mid = theta - d_theta * 0.5;
                    if angle_in_hole(r_clamped, th_mid, &ctx.holes) {
                        ctx.travel(to);
                    } else {
                        let from = ctx.point;
                        ctx.extrude_bed(from, to);
                    }
                }
            }

            // ---------------- شبکهٔ متقاطع (X سپس Y) ----------------
            FloorPattern::Grid => {
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
                            ((-half, y), (half, y))
                        } else {
                            ((y, -half), (y, half))
                        };
                        let (a, b) = if idx % 2 == 0 { (a, b) } else { (b, a) };
                        ctx.line(a.0, a.1, b.0, b.1);
                    }
                }
            }

            // ---------------- پرتوهای شعاعی ----------------
            FloorPattern::Radial => {
                let spokes = floor.spokes.clamp(3, 96) as usize;
                let r_in = r_hole.max(0.3);
                for i in 0..spokes {
                    let ang = i as f64 * 2.0 * std::f64::consts::PI / spokes as f64;
                    ctx.line(r_in * ang.cos(), r_in * ang.sin(), r_outer * ang.cos(), r_outer * ang.sin());
                }
            }

            // ---------------- رفت‌وبرگشتی (مارپیچ خطی) ----------------
            FloorPattern::ZigZag => {
                let n_rows = ((2.0 * r_outer) / gap).floor() as usize;
                if n_rows > 0 {
                    let row_y = |idx: usize| -r_outer + idx as f64 * gap;
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
                            ((-half, y), (half, y))
                        } else {
                            ((half, y), (-half, y))
                        };
                        // اتصال مارپیچی به ردیف قبلی (اگر نزدیک است)
                        let (lx, ly) = ctx.local_pt(ctx.point);
                        let a_bed = ctx.bed_pt(a.0, a.1);
                        let conn_len = (a_bed[0] - ctx.point[0]).hypot(a_bed[1] - ctx.point[1]);
                        if conn_len > 0.05 && conn_len < gap * 2.5 {
                            ctx.line(lx, ly, a.0, a.1);
                        }
                        ctx.line(a.0, a.1, b.0, b.1);
                        if idx > 4000 {
                            break; // محدودیت ایمنی
                        }
                    }
                }
            }

            // ---------------- شانه‌زنبوری — نسخهٔ ۱٫۶ ----------------
            // شبکهٔ شش‌گوش: استحکام بالا با فیلامنت کم؛ اندازهٔ سلول از gap
            // (حداقل ۵mm — سلول‌های ریزتر از این برای کف تزئینی بی‌معناست).
            FloorPattern::Honeycomb => {
                let s = (gap * 4.0).clamp(5.0, r_outer * 0.6);
                let dy = s * 0.866;
                let n_rows = (r_outer / dy).ceil() as i64;
                for row in -n_rows..=n_rows {
                    let yv = row as f64 * dy;
                    if yv.abs() > r_outer {
                        continue;
                    }
                    let xoff = if (row % 2).abs() == 1 { s * 0.5 } else { 0.0 };
                    let n_cols = ((r_outer + s) / s).ceil() as i64;
                    for col in -n_cols..=n_cols {
                        let cxh = col as f64 * s + xoff;
                        if (cxh * cxh + yv * yv).sqrt() + s <= r_outer + 0.2 {
                            let mut pts: [(f64, f64); 6] = [(0.0, 0.0); 6];
                            for (vi, pt) in pts.iter_mut().enumerate() {
                                let va = (30.0 + 60.0 * vi as f64).to_radians();
                                *pt = (cxh + s * va.cos(), yv + s * va.sin());
                            }
                            for ei in 0..6 {
                                let a = pts[ei];
                                let b = pts[(ei + 1) % 6];
                                ctx.line(a.0, a.1, b.0, b.1);
                            }
                        }
                    }
                }
            }

            // ---------------- موج دایره‌ای — نسخهٔ ۱٫۶ ----------------
            // مارپیچ آرشیمدسی با موج شعاعی سینوسی (k موج در هر دور) —
            // کف تزئینی «طرح‌دار»؛ k از فیلد spokes (بازهٔ ۳..۲۴).
            FloorPattern::Wave => {
                let r_start = if r_hole > 0.5 { r_hole } else { (gap * 0.5).max(0.3) };
                let amp = (gap * 2.0).clamp(1.0, 3.5);
                let waves = floor.spokes.clamp(3, 24) as f64;
                let mut theta = 0.0f64;
                let mut base_r = r_start;
                let start = ctx.bed_pt(r_start, 0.0);
                ctx.travel(start);
                let d_theta_max = 0.3;
                while base_r < r_outer {
                    let r_target = (base_r + amp * (waves * theta).sin()).max(0.5);
                    let d_theta = (2.0 / r_target).min(d_theta_max).max(0.02);
                    theta += d_theta;
                    base_r = r_start + gap * theta / (2.0 * std::f64::consts::PI);
                    let r_now = (base_r + amp * (waves * theta).sin()).clamp(0.3, r_outer);
                    let to = ctx.bed_pt(r_now * theta.cos(), r_now * theta.sin());
                    let th_mid = theta - d_theta * 0.5;
                    let r_mid = (base_r + amp * (waves * th_mid).sin()).clamp(0.3, r_outer);
                    if angle_in_hole(r_mid, th_mid, &ctx.holes) {
                        ctx.travel(to);
                    } else {
                        let from = ctx.point;
                        ctx.extrude_bed(from, to);
                    }
                }
            }

            // ---------------- ستاره‌ای — نسخهٔ ۱٫۶ ----------------
            // ستارهٔ چندپر پیوسته: شعاع بین r_in و r_out نوسان می‌کند.
            FloorPattern::Star => {
                let points = (floor.spokes.clamp(5, 24) as usize / 2).max(5);
                let r_out = r_outer * 0.98;
                let r_in = (r_hole + lw * 3.0).max(r_outer * 0.42).min(r_out - lw * 2.0);
                let n = points * 2;
                let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
                for i in 0..n {
                    let th = i as f64 * std::f64::consts::TAU / n as f64;
                    let r = if i % 2 == 0 { r_out } else { r_in };
                    pts.push((r * th.cos(), r * th.sin()));
                }
                for i in 0..n {
                    let a = pts[i];
                    let b = pts[(i + 1) % n];
                    ctx.line(a.0, a.1, b.0, b.1);
                }
                // حلقهٔ داخلی برای پوشش مرکز (وقتی سوراخ مرکزی نیست)
                if r_hole < 0.5 {
                    ctx.ring((r_in * 0.55).max(1.0), 0.0);
                }
            }

            // ---------------- دانه‌برفی — نسخهٔ ۱٫۶ ----------------
            // ۶ شاخهٔ اصلی + شاخه‌های فرعی ۴۵° در سه ارتفاع — تقارن شش‌گانه.
            FloorPattern::Snowflake => {
                let branches = 6usize;
                let r_in = r_hole.max(lw * 2.0);
                let arm = (r_outer - r_in).max(1.0);
                let side = arm * 0.3;
                for b in 0..branches {
                    let th = b as f64 * std::f64::consts::TAU / branches as f64;
                    let (ux, uy) = (th.cos(), th.sin());
                    ctx.line(r_in * ux, r_in * uy, r_outer * ux, r_outer * uy);
                    for frac in [0.4f64, 0.65, 0.85] {
                        let r_at = r_in + arm * frac;
                        let px = r_at * ux;
                        let py = r_at * uy;
                        for sign in [-1.0f64, 1.0] {
                            let a = th + sign * std::f64::consts::FRAC_PI_4;
                            ctx.line(px, py, px + side * frac * a.cos(), py + side * frac * a.sin());
                        }
                    }
                }
                if r_in > 0.6 {
                    ctx.ring(r_in, 0.0);
                }
            }

            // ---------------- گره هندسی هشت‌پر — نسخهٔ ۱٫۶ ----------------
            // دو مربع چرخیدهٔ ۴۵° روی هم + هشت‌ضلعی داخلی — نقش هندسی.
            FloorPattern::Octagram => {
                let rr = r_outer * 0.96;
                let rot45 = std::f64::consts::FRAC_PI_4;
                for sq in 0..2 {
                    let a0 = sq as f64 * rot45;
                    let mut pts: [(f64, f64); 4] = [(0.0, 0.0); 4];
                    for (vi, pt) in pts.iter_mut().enumerate() {
                        let va = a0 + vi as f64 * std::f64::consts::FRAC_PI_2;
                        *pt = (rr * va.cos(), rr * va.sin());
                    }
                    for ei in 0..4 {
                        let a = pts[ei];
                        let b = pts[(ei + 1) % 4];
                        ctx.line(a.0, a.1, b.0, b.1);
                    }
                }
                let r_mid = rr * 0.414;
                if r_mid > r_hole + lw * 2.0 {
                    let n = 8;
                    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
                    for i in 0..n {
                        let th = i as f64 * std::f64::consts::TAU / n as f64
                            + std::f64::consts::FRAC_PI_8;
                        pts.push((r_mid * th.cos(), r_mid * th.sin()));
                    }
                    for i in 0..n {
                        let a = pts[i];
                        let b = pts[(i + 1) % n];
                        ctx.line(a.0, a.1, b.0, b.1);
                    }
                }
            }

            // ---------------- وب لامپ — نسخهٔ ۱٫۷ (درخواست کاربر ۸) ----------------
            // حلقه‌های هم‌مرکز درشت (با ضریب نور لامپ) + پره‌های شعاعی ⇒
            // شبکهٔ توری نور: نور از خانه‌های باز عبور می‌کند و وزن کم است.
            FloorPattern::LampWeb => {
                let mut r = r_outer;
                while r > r_hole + gap {
                    ctx.ring(r, 0.0);
                    r -= gap;
                    if r <= r_hole {
                        break;
                    }
                }
                let spokes = if lamp_on {
                    lamp.spokes_count.clamp(3, 12) as usize
                } else {
                    floor.spokes.clamp(3, 24) as usize
                };
                let r_in = r_hole.max(0.3);
                for i in 0..spokes {
                    let ang = i as f64 * 2.0 * std::f64::consts::PI / spokes as f64;
                    ctx.line(
                        r_in * ang.cos(),
                        r_in * ang.sin(),
                        r_outer * ang.cos(),
                        r_outer * ang.sin(),
                    );
                }
            }

            // ---------------- خورشیدشکاف — نسخهٔ ۱٫۷ (درخواست کاربر ۸) ----------------
            // پرتوهای پهن واگرا از سوکت مرکزی تا لبه (هر پرتو ۳ خط:
            // دو لبهٔ واگرا + مرکز) — نور مستقیم و ظاهر «خورشید» کف لامپ.
            FloorPattern::Sunburst => {
                let rays = if lamp_on {
                    lamp.spokes_count.clamp(4, 16) as usize
                } else {
                    floor.spokes.clamp(5, 16) as usize
                };
                let r_in = r_hole.max(0.4);
                for i in 0..rays {
                    let th = i as f64 * 2.0 * std::f64::consts::PI / rays as f64;
                    let half0 = lw * 0.9;
                    let half1 =
                        (std::f64::consts::PI * r_outer / rays as f64) * 0.42;
                    let (ux, uy) = (th.cos(), th.sin());
                    let (px, py) = (-uy, ux);
                    ctx.line(
                        r_in * ux - half0 * px,
                        r_in * uy - half0 * py,
                        r_outer * ux - half1 * px,
                        r_outer * uy - half1 * py,
                    );
                    ctx.line(
                        r_in * ux + half0 * px,
                        r_in * uy + half0 * py,
                        r_outer * ux + half1 * px,
                        r_outer * uy + half1 * py,
                    );
                    ctx.line(r_in * ux, r_in * uy, r_outer * ux, r_outer * uy);
                }
            }
        }

        // ---------- فرورفتگی سوکت — لایهٔ دوم (ظاهر حرفه‌ای ۱٫۷) ----------
        if lamp_on && lamp.recessed_socket && layer_idx == 1 && layers > 1 && r_hole > 0.4 {
            let r = r_hole + lw * 2.5;
            if r < r_outer - lw * 2.0 {
                ctx.ring(r, 0.0);
            }
        }

        // ---------- تقویت — ۱٫۶ + سوکت/پرهٔ لامپ ۱٫۷ ----------
        // فقط در لایهٔ آخر کف: حلقه‌های ضخیم دور لبه (برای آویزکردن)،
        // دور سوراخ مرکزی (گریپ سوکت لامپ) و دور سوراخ‌های لبه (نخ/سیم).
        if layer_idx == layers - 1 {
            let rr_extra = floor.reinforce_rings.min(4);
            for i in 0..rr_extra {
                let r = r_outer - lw * (0.5 + i as f64);
                if r > r_hole + gap {
                    ctx.ring(r, 0.0);
                }
            }
            // حلقه‌های سوکت لامپ — «سوراخ وسط» مسلح (درخواست ۸)
            let socket_rings = if lamp_on {
                lamp.socket_rings.min(6)
            } else {
                0
            };
            if r_hole > 0.4 {
                let ring_n = (1 + rr_extra.min(2) + socket_rings).min(10);
                for i in 0..ring_n {
                    let r = r_hole + lw * (0.55 + i as f64);
                    if r < r_outer - lw {
                        ctx.ring(r, 0.0);
                    }
                }
            }
            for &(hx, hy, hrad) in &holes {
                let r = hrad + lw * 0.55;
                ctx.circle_around(hx, hy, r);
            }

            // ---------- پره‌های متصل‌کنندهٔ سوکت به بدنه — ۱٫۷ (درخواست ۸) ----------
            // «پایه‌های متصل‌کنندهٔ سوراخ به بدنه»: سه سبک —
            //   Parallel: هر پره چند خط موازی (پهن/مقاوم)؛
            //   Single: تک‌خط باریک (نور بیشتر)؛
            //   Web: پره + کمان‌های اتصال در دو شعاع میانی (استحکام حداکثر).
            if lamp_on && lamp.spokes_count >= 3 && r_hole > 0.4 {
                let n = lamp.spokes_count.clamp(3, 12) as usize;
                let spoke_w = lamp.spokes_width_mm.max(lw);
                let r_in = r_hole + lw * 0.6;
                let r_out2 = (r_outer - lw * 0.6).max(r_in + 1.0);
                let web = matches!(lamp.spokes_style, crate::designer::SpokesStyle::Web);
                for k in 0..n {
                    let th = phase0 + k as f64 * 2.0 * std::f64::consts::PI / n as f64;
                    let (ux, uy) = (th.cos(), th.sin());
                    match lamp.spokes_style {
                        crate::designer::SpokesStyle::Parallel => {
                            let n_lines =
                                ((spoke_w / lw).ceil() as usize).clamp(1, 6);
                            let half = (n_lines as f64 * lw) * 0.5;
                            for li in 0..n_lines {
                                let off = -half + lw * (li as f64 + 0.5);
                                let (px, py) = (-uy, ux);
                                ctx.line(
                                    r_in * ux + off * px,
                                    r_in * uy + off * py,
                                    r_out2 * ux + off * px,
                                    r_out2 * uy + off * py,
                                );
                            }
                        }
                        crate::designer::SpokesStyle::Single => {
                            ctx.line(r_in * ux, r_in * uy, r_out2 * ux, r_out2 * uy);
                        }
                        crate::designer::SpokesStyle::Web => {
                            ctx.line(r_in * ux, r_in * uy, r_out2 * ux, r_out2 * uy);
                        }
                    }
                }
                // شبکهٔ وب: کمان‌های اتصال بین پره‌ها در دو شعاع میانی
                if web {
                    let dth = 2.0 * std::f64::consts::PI / n as f64;
                    for frac in [0.33f64, 0.66] {
                        let r_m = r_in + (r_out2 - r_in) * frac;
                        for k in 0..n {
                            let th0 = phase0 + k as f64 * dth;
                            // کمان روی ۶۵٪ گپ زاویه‌ای بین دو پرهٔ مجاور
                            ctx.arc(r_m, th0 + dth * 0.175, th0 + dth * 0.825);
                        }
                    }
                }
            }
        }

        // پیوستگی نازل: نقطهٔ پایان این لایه → شروع لایهٔ بعد
        point = ctx.point;
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
            gcode_quality: 1.0,
            wall: crate::designer::WallSettings::default(),
            pendant: crate::designer::PendantSettings::default(),
            lamp: crate::designer::LampSettings::default(),
        }
    }


    // ================= آزمون‌های نسخهٔ ۱٫۷ =================

    #[test]
    fn test_lattice_generates_zigzag_wall() {
        // توری (درخواست ۳): z باید هم بالا برود هم پایین (موج تارها)؛
        // مسیر پیوسته چاپ می‌ماند و ماسک نوار = حالت توری.
        let mut inp = base_input();
        inp.wall.mode = crate::designer::WallMode::Lattice;
        inp.wall.lattice.cell_width_mm = 14.0;
        inp.wall.lattice.cell_height_mm = 12.0;
        inp.wall.lattice.z_start_mm = 5.0;
        let out = compute(&inp);
        // همهٔ قطعات دیواره (بعد از travel اول) چاپ‌اند — توری پیوسته است
        assert!(out.segments.len() > 200);
        let mut down_moves = 0usize;
        for w in out.segments.windows(2) {
            let z0 = w[0].end_mm.unwrap()[2];
            let z1 = w[1].end_mm.unwrap()[2];
            if z1 < z0 - 0.5 {
                down_moves += 1;
            }
        }
        assert!(down_moves > 10, "توری باید حرکت نزولی موج داشته باشد: {down_moves}");
        // حالت نوار پیش‌نمایش = توری
        assert_eq!(out.wall_mesh.mode, 1, "نوار باید در حالت توری باشد");
        // همهٔ ماسک‌ها ۱ (بدون سوراخ)
        assert!(out.wall_mesh.mask.iter().all(|&m| m == 1));
        // z در محدودهٔ فرم می‌ماند
        for s in &out.segments {
            if let Some(p) = s.end_mm {
                assert!(p[2] <= 100.5, "z از فرم بیرون زد: {}", p[2]);
            }
        }
    }

    #[test]
    fn test_lattice_solid_default_unchanged() {
        // پیش‌فرض دیوارهٔ جامد: رفتار ≤۱٫۶ (z صعودی، بدون نزول)
        let out = compute(&base_input());
        let mut down = 0usize;
        for w in out.segments.windows(2) {
            let z0 = w[0].end_mm.unwrap()[2];
            let z1 = w[1].end_mm.unwrap()[2];
            if z1 < z0 - 1e-9 {
                down += 1;
            }
        }
        assert_eq!(down, 0, "جامد نباید نزول داشته باشد");
        assert_eq!(out.wall_mesh.mode, 0);
    }

    #[test]
    fn test_wall_holes_create_real_openings() {
        // سوراخ دیواره (درخواست ۹): travel روی دهانه + ماسک ۰ در نوار
        let mut inp = base_input();
        inp.wall.holes.enabled = true;
        inp.wall.holes.count_around = 4;
        inp.wall.holes.rows = 2;
        inp.wall.holes.width_mm = 14.0;
        inp.wall.holes.height_mm = 16.0;
        inp.wall.holes.margin_bottom_mm = 15.0;
        inp.wall.holes.frame_passes = 2;
        let out = compute(&inp);
        // travelهای میانه (دهانه‌ها) وجود دارند
        let travels = out
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Travel)
            .count();
        assert!(travels >= 8, "سوراخ‌ها باید travel بدهند: {travels}");
        // ماسک نوار صفر دارد (دهانهٔ باز در پیش‌نمایش)
        let zeros = out.wall_mesh.mask.iter().filter(|&&m| m == 0).count();
        assert!(zeros > 0, "ماسک باید دهانهٔ باز داشته باشد: {zeros}");
        // قاب‌ها: اکستروژن در z ثابت (کف/سقف قاب) — پربیش‌تر از حالت بدون سوراخ
        let base = compute(&base_input());
        assert!(
            out.segments.len() > base.segments.len() + 50,
            "قاب‌های تقویتی باید segment اضافه کنند: {} vs {}",
            out.segments.len(),
            base.segments.len()
        );
        // هیچ اکستروژنی «داخل» سوراخ نیست (نقطهٔ میانی segment)
        // مدل هندسی آزمون باید «جابه‌جایی آجرچین ردیف‌های زوج» را لحاظ کند
        // (stagger = π/۴ برای count_around=4) و r_ref واقعی هسته (~۳۴).
        let r_ref = 34.0;
        for s in &out.segments {
            if s.kind != SegmentKind::Extrusion {
                continue;
            }
            if let (Some(a), Some(b)) = (s.start_mm, s.end_mm) {
                let mx = (a[0] + b[0]) / 2.0;
                let my = (a[1] + b[1]) / 2.0;
                let mz = (a[2] + b[2]) / 2.0;
                // فقط ناحیهٔ دیواره (نه کف — کف خاموش است)
                if mz < 2.0 {
                    continue;
                }
                let th = ((my - 110.0).atan2(mx - 110.0)).rem_euclid(2.0 * std::f64::consts::PI);
                for hi in 0..8usize {
                    let row = hi / 4;
                    let k = hi % 4;
                    let thc = (k as f64 * std::f64::consts::FRAC_PI_2
                        + if row % 2 == 1 {
                            std::f64::consts::FRAC_PI_4
                        } else {
                            0.0
                        })
                    .rem_euclid(2.0 * std::f64::consts::PI);
                    let zc = 15.0 + row as f64 * 22.0;
                    let dth = (th - thc + std::f64::consts::PI)
                        .rem_euclid(2.0 * std::f64::consts::PI)
                        - std::f64::consts::PI;
                    let u = dth * r_ref;
                    let v = mz - zc;
                    // ناحیهٔ «هستهٔ» سوراخ (شعاع ۰٫۵) — قاب تقویتی مجاز است
                    // روی لبه (شعاع ~۱) باشد؛ وترهای قوس قاب کمی داخل‌تر می‌روند.
                    // ناحیهٔ «هستهٔ» سوراخ (شعاع ۰٫۵) — قاب تقویتی مجاز است
                    // روی لبه (شعاع ~۱) باشد؛ وترهای قوس قاب کمی داخل‌تر می‌روند.
                    let inside = (u / 7.0) * (u / 7.0) + (v / 8.0) * (v / 8.0) <= 0.25;
                    assert!(
                        !inside,
                        "اکستروژن داخل سوراخ: u={u:.1} v={v:.1} z={mz:.1}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_pendant_rim_rings_and_top_holes() {
        // آویز (درخواست ۷): حلقه‌های تقویت لبهٔ بالا + سوراخ‌های بالای دیواره
        let mut inp = base_input();
        inp.pendant.style = crate::designer::HangingStyle::TopHoles;
        inp.pendant.top_holes_count = 3;
        inp.pendant.top_hole_diameter_mm = 6.0;
        inp.pendant.top_hole_from_rim_mm = 8.0;
        inp.pendant.rim_reinforce_rings = 3;
        let out = compute(&inp);
        // حلقه‌ها: اکستروژن‌هایی که مجموع زاویه‌شان دور کامل است در z ثابت بالا
        let top_segs: Vec<_> = out
            .segments
            .iter()
            .filter(|s| {
                s.kind == SegmentKind::Extrusion
                    && s.end_mm.map(|p| p[2] > 99.0).unwrap_or(false)
            })
            .collect();
        assert!(
            !top_segs.is_empty(),
            "حلقه‌های تقویت لبه باید segment بالای فرم بسازند"
        );
        // سوراخ‌های بالا: travel روی دهانه‌ها نزدیک z = 100 − 8 − 3
        let top_hole_travels = out
            .segments
            .iter()
            .filter(|s| {
                s.kind == SegmentKind::Travel
                    && s.end_mm
                        .map(|p| (86.0..=95.0).contains(&p[2]))
                        .unwrap_or(false)
            })
            .count();
        assert!(top_hole_travels > 0, "سوراخ‌های بالای دیواره travel دارند");
    }

    #[test]
    fn test_pendant_none_no_extra_rings() {
        // بدون آویز: تعداد قطعهٔ ناحیهٔ بالا همان مارپیچ کلمپ‌شده است؛
        // با آویز (حلقه‌های تقویت) باید بیشتر شود — مقایسهٔ نسبی.
        let base = compute(&base_input());
        let base_top = base
            .segments
            .iter()
            .filter(|s| {
                s.kind == SegmentKind::Extrusion
                    && s.end_mm.map(|p| p[2] >= 99.0).unwrap_or(false)
            })
            .count();
        let mut inp = base_input();
        inp.pendant.style = crate::designer::HangingStyle::RimHoles;
        inp.pendant.rim_reinforce_rings = 4;
        let with = compute(&inp);
        let with_top = with
            .segments
            .iter()
            .filter(|s| {
                s.kind == SegmentKind::Extrusion
                    && s.end_mm.map(|p| p[2] >= 99.0).unwrap_or(false)
            })
            .count();
        assert!(
            with_top > base_top,
            "حلقه‌های تقویت باید ناحیهٔ بالا را ضخیم‌تر کنند: {with_top} vs {base_top}"
        );
    }

    #[test]
    fn test_lamp_floor_socket_and_spokes() {
        // لامپ (درخواست ۸): سوکت مرکزی + پره‌های متصل‌کننده + کف نور
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 2;
        inp.floor.pattern = crate::designer::FloorPattern::LampWeb;
        inp.lamp.enabled = true;
        inp.lamp.center_hole_diameter_mm = 20.0;
        inp.lamp.socket_rings = 3;
        inp.lamp.spokes_count = 5;
        inp.lamp.spokes_width_mm = 3.0;
        inp.lamp.spokes_style = crate::designer::SpokesStyle::Parallel;
        inp.lamp.floor_gap_multiplier = 1.8;
        let out = compute(&inp);
        // کف ساخته شد و خلاصه دارد
        assert!(out.summary_fa.contains("کف"));
        // سوراخ سوکت: هیچ اکستروژن کفی نزدیک‌تر از شعاع سوکت به مرکز نیست
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        let r_socket = 10.0;
        for s in &out.segments {
            if s.end_mm.map(|p| p[2] <= 0.6).unwrap_or(false) {
                if let Some(p) = s.end_mm {
                    let d = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
                    assert!(
                        d >= r_socket - 1.2,
                        "اکستروژن داخل سوکت لامپ: d={d:.2}"
                    );
                }
            }
        }
        // پره‌ها: نسبت به حالت بدون پره segment بیشتری در کف وجود دارد
        let mut inp2 = inp.clone();
        inp2.lamp.spokes_count = 0;
        inp2.lamp.socket_rings = 0;
        let out2 = compute(&inp2);
        assert!(
            out.segments.len() > out2.segments.len(),
            "پره‌ها/حلقه‌های سوکت باید segment اضافه کنند"
        );
    }

    #[test]
    fn test_lamp_hole_priority_over_floor_hole() {
        // قطر سوکت لامپ بر سوراخ مرکزی کف تقدم دارد
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.center_hole_diameter_mm = 8.0;
        inp.floor.pattern = crate::designer::FloorPattern::Rings;
        inp.lamp.enabled = true;
        inp.lamp.center_hole_diameter_mm = 24.0;
        let out = compute(&inp);
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        // کوچکترین فاصلهٔ اکستروژن کف از مرکز ≥ ~۱۲mm (شعاع سوکت ۲۴/۲)
        let mut min_d = f64::MAX;
        for s in &out.segments {
            if s.end_mm.map(|p| p[2] <= 0.6).unwrap_or(false) {
                if let Some(p) = s.end_mm {
                    let d = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
                    min_d = min_d.min(d);
                }
            }
        }
        assert!(
            min_d >= 11.0,
            "سوکت لامپ باید بر سوراخ کف تقدم داشته باشد (min_d={min_d:.2})"
        );
    }

    #[test]
    fn test_hole_shapes_all_valid() {
        // همهٔ شکل‌های سوراخ با هندسهٔ معتبر چاپ می‌شوند (بدون NaN)
        for shape in [
            crate::designer::HoleShape::Circle,
            crate::designer::HoleShape::RoundedRect,
            crate::designer::HoleShape::Diamond,
            crate::designer::HoleShape::Hexagon,
            crate::designer::HoleShape::Star,
        ] {
            let mut inp = base_input();
            inp.wall.holes.enabled = true;
            inp.wall.holes.shape = shape;
            inp.wall.holes.count_around = 3;
            inp.wall.holes.rows = 1;
            inp.wall.holes.margin_bottom_mm = 30.0;
            let out = compute(&inp);
            assert!(!out.segments.is_empty(), "{shape:?} قطعه نساخت");
            for s in &out.segments {
                if let Some(p) = s.end_mm {
                    assert!(p[0].is_finite() && p[1].is_finite() && p[2].is_finite());
                }
            }
        }
    }

    #[test]
    fn test_lattice_styles_all_valid() {
        // همهٔ سبک‌های توری مسیر معتبر می‌سازند
        for style in [
            crate::designer::LatticeStyle::Diamond,
            crate::designer::LatticeStyle::Square,
            crate::designer::LatticeStyle::Chevron,
            crate::designer::LatticeStyle::Twisted,
        ] {
            let mut inp = base_input();
            inp.wall.mode = crate::designer::WallMode::Lattice;
            inp.wall.lattice.style = style;
            inp.wall.lattice.z_start_mm = 4.0;
            let out = compute(&inp);
            assert!(out.segments.len() > 100, "{style:?}");
            for s in &out.segments {
                if let Some(p) = s.end_mm {
                    assert!(p[2] >= 0.0 && p[2] <= 100.5, "{style:?}: z={}", p[2]);
                }
            }
        }
    }

    #[test]
    fn test_lattice_with_holes_combined() {
        // توری + سوراخ با هم کار می‌کنند (دهانه روی تور)
        let mut inp = base_input();
        inp.wall.mode = crate::designer::WallMode::Lattice;
        inp.wall.lattice.z_start_mm = 6.0;
        inp.wall.holes.enabled = true;
        inp.wall.holes.count_around = 3;
        inp.wall.holes.rows = 1;
        inp.wall.holes.margin_bottom_mm = 40.0;
        let out = compute(&inp);
        let zeros = out.wall_mesh.mask.iter().filter(|&&m| m == 0).count();
        assert!(zeros > 0, "دهانه روی تور باید در ماسک دیده شود");
        assert_eq!(out.wall_mesh.mode, 1);
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
            // ---- نسخهٔ ۱٫۶: ۵ الگوی جدید ----
            FloorPattern::Honeycomb,
            FloorPattern::Wave,
            FloorPattern::Star,
            FloorPattern::Snowflake,
            FloorPattern::Octagram,
            // ---- نسخهٔ ۱٫۷: الگوهای لامپ (درخواست ۸) ----
            FloorPattern::LampWeb,
            FloorPattern::Sunburst,
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

    // ================= آزمون‌های کف ۱٫۶ — الگوهای جدید + حرفه‌ای =================

    #[test]
    fn test_floor_rim_holes_create_gaps() {
        // سوراخ لبه: هیچ اکستروژن کفی از داخل سوراخ نمی‌گذرد (درخواست ۷)
        let mut inp = base_input();
        inp.floor.enabled = true;
        inp.floor.layers = 1;
        inp.floor.pattern = crate::designer::FloorPattern::Grid;
        inp.floor.rim_holes = 4;
        inp.floor.rim_hole_diameter_mm = 6.0;
        let out = compute(&inp);
        let lh = inp.printer.layer_height_mm;
        // مرکز سوراخ‌ها: r_ring = r_outer - hr - lw*2.5 در زوایای phase0+k*90
        let vol = inp.printer.build_volume_mm;
        let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
        let r_outer = 30.0; // شعاع پایین فرم پیش‌فرض
        let hr = 3.0;
        let lw = inp.printer.line_width_mm;
        let r_ring = r_outer - hr - lw * 2.5;
        let mut violations = 0usize;
        for s in &out.segments {
            if s.kind != SegmentKind::Extrusion {
                continue;
            }
            if let (Some(a), Some(b)) = (s.start_mm, s.end_mm) {
                if (a[2] - lh).abs() > 0.3 || (b[2] - lh).abs() > 0.3 {
                    continue; // فقط کف لایهٔ اول
                }
                let mx = (a[0] + b[0]) / 2.0 - cx;
                let my = (a[1] + b[1]) / 2.0 - cy;
                for k in 0..4 {
                    let th = k as f64 * std::f64::consts::FRAC_PI_2;
                    let d = (mx - r_ring * th.cos()).hypot(my - r_ring * th.sin());
                    if d < hr * 0.9 {
                        violations += 1;
                    }
                }
            }
        }
        assert_eq!(violations, 0, "اکستروژن از داخل سوراخ لبه گذشت: {violations}");
        // و کلاً مسیر کف وجود دارد
        assert!(out.summary_fa.contains("کف"));
    }

    #[test]
    fn test_floor_reinforce_rings_add_segments() {
        // حلقه‌های تقویت: لایهٔ آخر کف قطعات دایره‌ای اضافه دارد (درخواست ۷)
        let mk = |rr: u32| {
            let mut inp = base_input();
            inp.floor.enabled = true;
            inp.floor.layers = 2;
            inp.floor.pattern = crate::designer::FloorPattern::Rings;
            inp.floor.center_hole_diameter_mm = 12.0;
            inp.floor.reinforce_rings = rr;
            compute(&inp).segments.len()
        };
        let without = mk(0);
        let with2 = mk(2);
        assert!(with2 > without, "تقویت باید segment اضافه کند: {with2} vs {without}");
    }

    #[test]
    fn test_floor_new_patterns_unique_paths() {
        // ۵ الگوی جدید مسیرهای متمایز می‌سازند (نه کپی هم)
        use crate::designer::FloorPattern;
        let mut lens: Vec<(usize, usize)> = Vec::new();
        for pat in [
            FloorPattern::Honeycomb,
            FloorPattern::Wave,
            FloorPattern::Star,
            FloorPattern::Snowflake,
            FloorPattern::Octagram,
        ] {
            let mut inp = base_input();
            inp.floor.enabled = true;
            inp.floor.layers = 1;
            inp.floor.pattern = pat;
            let out = compute(&inp);
            assert!(!out.segments.is_empty(), "{pat:?} خالی است");
            // همهٔ نقاط «کف» (z ≈ لایهٔ اول) داخل دیسک
            let vol = inp.printer.build_volume_mm;
            let (cx, cy) = (vol[0] / 2.0, vol[1] / 2.0);
            let lh = inp.printer.layer_height_mm;
            for s in &out.segments {
                for p in [s.start_mm, s.end_mm].into_iter().flatten() {
                    if (p[2] - lh).abs() > 0.3 {
                        continue; // فقط کف لایهٔ اول — نه دیواره/travelهای بالاتر
                    }
                    let d = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
                    assert!(d <= 34.0, "{pat:?}: نقطهٔ کف بیرون دیسک d={d:.1}");
                }
            }
            lens.push((out.segments.len(), out.stats.extrusion_segments));
        }
        // طول مسیرها همه متفاوت‌اند (هر الگو هندسهٔ خودش را دارد)
        for i in 0..lens.len() {
            for j in (i + 1)..lens.len() {
                assert_ne!(lens[i], lens[j], "دو الگو مسیر یکسان ساختند: {i} vs {j}");
            }
        }
    }

    #[test]
    fn test_floor_honeycomb_is_strong_and_light() {
        // شانه‌زنبوری: مسیر کمتر از شبکهٔ کامل ولی پوشش دارد (فیلامنت‌صرفه)
        let mk = |pat: crate::designer::FloorPattern| {
            let mut inp = base_input();
            inp.floor.enabled = true;
            inp.floor.layers = 1;
            inp.floor.pattern = pat;
            let out = compute(&inp);
            out.stats.extrusion_length_mm
        };
        let honey = mk(crate::designer::FloorPattern::Honeycomb);
        let grid = mk(crate::designer::FloorPattern::Grid);
        assert!(honey > 100.0, "شانه‌زنبوری باید مسیر واقعی داشته باشد: {honey:.0}mm");
        assert!(
            honey < grid,
            "شانه‌زنبوری باید از شبکهٔ کامل کمتر چاپ کند: {honey:.0} vs {grid:.0}mm"
        );
    }
}
