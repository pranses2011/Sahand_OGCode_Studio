//! ============================================================================
//! designer/mod.rs — موتور طراح چاپ مارپیچی (vase-mode)
//! ============================================================================
//! طبق Docs/PROMPT_03 §5 زنجیرهٔ «منبع واحد»:
//!
//!   پروفایل 2D (r,h)
//!     → اعمال ارتفاع/قطر/taper + کف
//!     → patternهای هندسی: r(z,θ) = r_profile(z) + Σ pattern(z,θ)
//!     → mesh پایه (STL از همین زنجیره ساخته می‌شود)
//!     → مسیر spiral پیوسته (θ پیوسته، Z صعودی با pitch)
//!     → textureهای toolpath (تعدیل شعاع نقطه‌به‌نقطه — فقط در G-code)
//!     → محاسبهٔ E و F + افزودن start/end کد
//!     → Toolpath IR واحد → همان داده برای 3D preview و خروجی G-code
//!
//! تمایز حیاتی (تأییدشده از پژوهش مرجع Docs/REFERENCE_AUDIT.md):
//!   - Pattern هندسهٔ مدل را تغییر می‌دهد ⇒ در STL می‌ماند؛
//!   - Texture مسیر نازل را تغییر می‌دهد ⇒ در STL ذخیره نمی‌شود.
//! ============================================================================

pub mod pattern;
pub mod printer_presets;
pub mod profile;
pub mod spiral;
pub mod texture;

// re-export های پرکاربرد
pub use pattern::{PatternKind, PatternParams};
pub use printer_presets::{brand_fa, printer_presets};
pub use profile::{Interpolation, ProfilePreset, ProfilePoint};
pub use spiral::{DesignerInput, DesignerOutput};
pub use texture::{TextureKind, TextureParams};

use serde::{Deserialize, Serialize};

/// وزن پنجرهٔ زاویه‌ای «اسپات» — نسخهٔ ۱.۶ (درخواست کاربر ۵).
/// الگو/بافت به‌جای کل دور فرم، فقط روی یک قوس زاویه‌ای اعمال می‌شود:
///   * span ≥ ۳۶۰ → همه‌جا ۱.۰ (رفتار قدیمی — سازگاری پروژه‌های قبلی)؛
///   * فاصلهٔ زاویه‌ای از مرکز (با wrap) خارج از نیم‌دهانه → ۰؛
///   * لبه‌ها با گذار نرم smoothstep روی ۱۵٪ دهانه (حداقل ۶°) —
///     لبهٔ تیز چاپ خوب نیست (شروع/توقف ناگهانی اکستروژن).
/// خالص و قطعی — قابل‌آزمون (test_spot_angular_window).
pub fn angular_window_weight(center_deg: f64, span_deg: f64, theta: f64) -> f64 {
    if !span_deg.is_finite() || span_deg >= 360.0 {
        return 1.0; // پوشش کامل — رفتار نسخه‌های ≤۱.۵
    }
    if span_deg <= 1e-9 {
        return 0.0;
    }
    let c = center_deg.to_radians();
    let half = (span_deg * 0.5).to_radians();
    // فاصلهٔ زاویه‌ای علامت‌دار با wrap به (−π, π]
    let d = (theta - c + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI)
        - std::f64::consts::PI;
    let a = d.abs();
    if a >= half {
        return 0.0;
    }
    // حاشیهٔ نرم لبه — ۱۵٪ نیم‌دهانه، حداقل ۶ درجه
    let edge = (half * 0.15).max(0.1);
    if half - a >= edge {
        1.0
    } else {
        let t = ((half - a) / edge).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t) // smoothstep
    }
}

/// الگوی چاپ کف — نسخهٔ ۱.۲ (درخواست کاربر: الگوهای کف بیشتر)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloorPattern {
    /// حلقه‌های هم‌مرکز از بیرون به داخل (رفتار ۱.۰/۱.۱)
    Rings,
    /// مارپیچ آرشیمدسی پیوسته از مرکز (بدون travel — مناسب vase-mode)
    Spiral,
    /// شبکهٔ متقاطع: خطوط موازی X و Y روی دیسک
    Grid,
    /// پرتوهای شعاعی از سوراخ مرکزی تا لبه
    Radial,
    /// ردیف‌های رفت‌وبرگشتی (مارپیچ خطی) روی دیسک
    ZigZag,
    /// شانه‌زنبوری — نسخهٔ ۱٫۶: شبکهٔ شش‌گوش (استحکام بالا/فیلامنت کم)
    Honeycomb,
    /// موج دایره‌ای — نسخهٔ ۱٫۶: مارپیچ با موج شعاعی سینوسی (کف تزئینی)
    Wave,
    /// ستاره‌ای — نسخهٔ ۱٫۶: چند ستارهٔ تودرتو با پرتوهای کامل
    Star,
    /// دانه‌برفی — نسخهٔ ۱٫۶: ۶ شاخه + شاخه‌های فرعی (کف نمایشی)
    Snowflake,
    /// گره هندسی — نسخهٔ ۱٫۶: ستارهٔ هشت‌پر تکرارشونده (نقش هندسی)
    Octagram,
}

impl Default for FloorPattern {
    fn default() -> Self {
        FloorPattern::Rings
    }
}

impl FloorPattern {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            FloorPattern::Rings => "حلقه‌های هم‌مرکز",
            FloorPattern::Spiral => "مارپیچ پیوسته",
            FloorPattern::Grid => "شبکهٔ متقاطع",
            FloorPattern::Radial => "پرتوهای شعاعی",
            FloorPattern::ZigZag => "رفت‌وبرگشتی",
            FloorPattern::Honeycomb => "شانه‌زنبوری",
            FloorPattern::Wave => "موج دایره‌ای",
            FloorPattern::Star => "ستاره‌ای",
            FloorPattern::Snowflake => "دانه‌برفی",
            FloorPattern::Octagram => "گره هندسی (هشت‌پر)",
        }
    }
}

/// تنظیمات کف دیسکی — گام ۴ گردش‌کار
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorSettings {
    /// آیا کف ساخته شود
    pub enabled: bool,
    /// الگوی چاپ کف (نسخهٔ ۱.۲ — پیش‌فرض حلقه‌های هم‌مرکز)
    #[serde(default)]
    pub pattern: FloorPattern,
    /// قطر سوراخ مرکزی کف (mm) — 0 یعنی بدون سوراخ
    pub center_hole_diameter_mm: f64,
    /// ارتفاع شروع کف (معمولاً 0)
    pub z_start_mm: f64,
    /// فاصلهٔ خطوط هم‌مرکز کف (mm) — معمولاً برابر line width
    pub ring_gap_mm: f64,
    /// تعداد پرتوها در الگوی شعاعی (نسخهٔ ۱.۲)
    #[serde(default = "default_spokes")]
    pub spokes: u32,
    /// تعداد لایه‌های کف (نسخهٔ ۱.۴ — درخواست کاربر).
    /// کف به‌صورت توپر با این تعداد لایه چاپ می‌شود و سپس
    /// حالت مارپیچی/vase از بالای کف ادامه می‌یابد.
    #[serde(default = "default_floor_layers")]
    pub layers: u32,
    /// تنظیمات حرفه‌ای کف — نسخهٔ ۱٫۶ (درخواست کاربر ۷: لامپ/آویز):
    /// تعداد سوراخ‌های لبه (۰..۱۲) برای رد کردن سیم/نخ آویز یا کابل لامپ؛
    /// الگوهای کف این نقاط را خالی می‌گذارند و حلقهٔ تقویت دورشان چاپ می‌شود.
    #[serde(default)]
    pub rim_holes: u32,
    /// قطر هر سوراخ لبه (mm) — پیش‌فرض ۳mm (عبور نخ/سیم)
    #[serde(default = "default_rim_hole_d")]
    pub rim_hole_diameter_mm: f64,
    /// حلقه‌های تقویت اضافی دور لبه/سوراخ مرکزی (۰..۴) — نسخهٔ ۱٫۶:
    /// برای آویزکردن و سوکت لامپ؛ لایهٔ آخر کف ضخیم‌تر/مقاوم‌تر می‌شود.
    #[serde(default)]
    pub reinforce_rings: u32,
}

/// پیش‌فرض قطر سوراخ لبه — ۳mm (عبور نخ/سیم آویز)
fn default_rim_hole_d() -> f64 {
    3.0
}

/// پیش‌فرض تعداد پرتوهای کف شعاعی
fn default_spokes() -> u32 {
    12
}

/// پیش‌فرض تعداد لایه‌های کف — ۲ لایهٔ توپر استاندارد ظرف
fn default_floor_layers() -> u32 {
    2
}

impl Default for FloorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            pattern: FloorPattern::Rings,
            center_hole_diameter_mm: 0.0,
            z_start_mm: 0.0,
            ring_gap_mm: 0.45,
            spokes: 12,
            layers: default_floor_layers(),
            rim_holes: 0,
            rim_hole_diameter_mm: default_rim_hole_d(),
            reinforce_rings: 0,
        }
    }
}

/// تنظیمات پرینتر/خروجی — گام ۵ گردش‌کار
/// الزام: presetها باید «وضعیت اعتبار» داشته باشند؛ ساختگی/خطرناک ممنوع
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterProfile {
    /// نام نمایشی پروفایل
    pub name: String,
    /// وضعیت اعتبار طبق الزام: verified | community | custom | unverified
    pub status: String,
    /// حجم ساخت [x, y, z] به mm — از دادهٔ واقعی profile
    pub build_volume_mm: [f64; 3],
    /// قطر نازل (mm) — 0.4/0.6/0.8 رایج؛ سفارشی مجاز
    pub nozzle_diameter_mm: f64,
    /// قطر فیلامنت (mm)
    pub filament_diameter_mm: f64,
    /// عرض خط چاپ (mm) — معمولاً ≥ قطر نازل
    pub line_width_mm: f64,
    /// ارتفاع لایه (mm) — همان گام مارپیچ Z
    pub layer_height_mm: f64,
    /// سرعت چاپ دیواره (mm/min)
    pub print_speed_mm_min: f64,
    /// سرعت travel (mm/min)
    pub travel_speed_mm_min: f64,
    /// ضریب تصحیح جریان (1.0 = بدون تصحیح) — مستند و قابل‌تنظیم
    pub flow_multiplier: f64,
    /// حالت E خروجی: مطلق (M82) یا نسبی (M83)
    pub absolute_e: bool,
    // ---------- تنظیمات حرارتی — نسخهٔ ۱.۳ (درخواست کاربر) ----------
    /// دمای هدف نازل (°C) — ۰ یعنی «کنترل دما در خروجی ننویس»
    #[serde(default = "default_nozzle_temp")]
    pub nozzle_temp_c: f64,
    /// دمای هدف بستر/میز کار (°C) — ۰ یعنی عدم صدور فرمان دما
    #[serde(default = "default_bed_temp")]
    pub bed_temp_c: f64,
    /// سرعت فن خنک‌کنندهٔ قطعه (٪ ۰..۱۰۰) — M106 S معادل ۰..۲۵۵
    #[serde(default = "default_fan_percent")]
    pub fan_percent: f64,
    /// پس از تنظیم دما، با M190/M109 منتظر رسیدن به هدف بمانیم؟
    /// (توصیه‌شده روشن — چاپ روی بستر سرد پیچیدگی پیچ‌خوردگی دارد)
    #[serde(default = "default_true")]
    pub wait_for_temps: bool,
    // ---------- تنظیمات سرعت — نسخهٔ ۱.۴ (درخواست کاربر: mm/s) ----------
    /// سرعت چاپ دیوارهٔ مارپیچی (mm/s) — ۰ یعنی از print_speed_mm_min قدیمی پیروی کن
    #[serde(default)]
    pub wall_speed_mm_s: f64,
    /// سرعت لایهٔ اول کف (mm/s) — ۰ یعنی ۵۰٪ سرعت دیواره (استاندارد اسلایسرها)
    #[serde(default)]
    pub first_layer_speed_mm_s: f64,
    /// سرعت لایه‌های بعدی کف (mm/s) — ۰ یعنی ۷۵٪ سرعت دیواره
    #[serde(default)]
    pub floor_speed_mm_s: f64,
    /// سرعت جابجایی/travel (mm/s) — ۰ یعنی از travel_speed_mm_min قدیمی پیروی کن
    #[serde(default)]
    pub travel_speed_mm_s: f64,
    // ---------- رتراکشن — نسخهٔ ۱.۴ (استانداردسازی G-code) ----------
    /// طول رتراکشن (mm) — ۰ یعنی بدون رتراکشن؛ فقط روی travelهای کف اعمال می‌شود
    /// (مسیر مارپیچی vase پیوسته است و رتراکشن وسط آن زخم می‌اندازد)
    #[serde(default = "default_retract_mm")]
    pub retract_mm: f64,
    /// سرعت رتراکشن/آنرتراکشن (mm/s)
    #[serde(default = "default_retract_speed")]
    pub retract_speed_mm_s: f64,
    /// کد start سفارشی (قبل از مسیر) — پیش‌نمایش و هشدار پیش از الحاق
    pub start_gcode: String,
    /// کد end سفارشی (بعد از مسیر)
    pub end_gcode: String,
}

/// پیش‌فرض طول رتراکشن — ۰٫۸mm (کوچک؛ مناسب دایرکت‌درایو،
/// برای بودن مقدار را بالا ببرید — در UI مستند است)
fn default_retract_mm() -> f64 {
    0.8
}

/// پیش‌فرض سرعت رتراکشن — ۴۰ mm/s
fn default_retract_speed() -> f64 {
    40.0
}

impl PrinterProfile {
    /// سرعت مؤثر دیواره (mm/min) — mm/s جدید بر mm/min قدیمی تقدم دارد
    pub fn effective_wall_mm_min(&self) -> f64 {
        if self.wall_speed_mm_s > 0.0 {
            self.wall_speed_mm_s * 60.0
        } else {
            self.print_speed_mm_min
        }
    }
    /// سرعت مؤثر travel (mm/min)
    pub fn effective_travel_mm_min(&self) -> f64 {
        if self.travel_speed_mm_s > 0.0 {
            self.travel_speed_mm_s * 60.0
        } else {
            self.travel_speed_mm_min
        }
    }
    /// سرعت مؤثر لایهٔ اول کف (mm/min) — پیش‌فرض ۵۰٪ دیواره (چسبندگی به میز)
    pub fn effective_first_layer_mm_min(&self) -> f64 {
        if self.first_layer_speed_mm_s > 0.0 {
            self.first_layer_speed_mm_s * 60.0
        } else {
            self.effective_wall_mm_min() * 0.5
        }
    }
    /// سرعت مؤثر لایه‌های بعدی کف (mm/min) — پیش‌فرض ۷۵٪ دیواره
    pub fn effective_floor_mm_min(&self) -> f64 {
        if self.floor_speed_mm_s > 0.0 {
            self.floor_speed_mm_s * 60.0
        } else {
            self.effective_wall_mm_min() * 0.75
        }
    }
}

impl Default for PrinterProfile {
    fn default() -> Self {
        // پروفایل عمومی «سفارشی/تأییدنشده» — طبق الزام هیچ preset جعلی
        // «verified» نیست؛ اعداد حجم ساخت، نمایندهٔ پرینتر عمومی ۲۲۰mm است
        // و صراحتاً unverified برچسب می‌خورد.
        Self {
            name: "Custom / Unverified".into(),
            status: "unverified".into(),
            build_volume_mm: [220.0, 220.0, 250.0],
            nozzle_diameter_mm: 0.4,
            filament_diameter_mm: 1.75,
            line_width_mm: 0.45,
            layer_height_mm: 0.25,
            print_speed_mm_min: 2400.0, // 40 mm/s
            travel_speed_mm_min: 6000.0, // 100 mm/s
            flow_multiplier: 1.0,
            absolute_e: true,
            // پیش‌فرض‌های حرارتی — PLA عمومی (کاربر باید مطابق فیلامنت
            // خود و پریست ماده تنظیم کند؛ راهنمای کاربر مستند کرده)
            nozzle_temp_c: default_nozzle_temp(),
            bed_temp_c: default_bed_temp(),
            fan_percent: default_fan_percent(),
            wait_for_temps: true,
            // سرعت‌ها — نسخهٔ ۱.۴: پیش‌فرض mm/s (معادل مقادیر mm/min بالا)
            wall_speed_mm_s: 40.0,
            first_layer_speed_mm_s: 20.0,
            floor_speed_mm_s: 30.0,
            travel_speed_mm_s: 100.0,
            retract_mm: default_retract_mm(),
            retract_speed_mm_s: default_retract_speed(),
            start_gcode: String::new(),
            end_gcode: String::new(),
        }
    }
}

/// فایل پروژهٔ طراح — قالب JSON نسخه‌دار (الزام §7)
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

/// مرکز‌گذاری روی صفحهٔ چاپ — پیش‌فرض روشن (رفع باگ نسخهٔ ۱.۱: طرح روی 0,0)
fn default_true() -> bool {
    true
}

/// پیش‌فرض دمای نازل (°C) — PLA عمومی
fn default_nozzle_temp() -> f64 {
    200.0
}

/// پیش‌فرض دمای بستر (°C) — PLA عمومی
fn default_bed_temp() -> f64 {
    60.0
}

/// پیش‌فرض فن (٪) — خنک‌کاری کامل برای PLA
fn default_fan_percent() -> f64 {
    100.0
}

/// کل پروژهٔ طراح قابل ذخیره/بازیابی
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerProject {
    /// نسخهٔ schema برای migration ایمن (الزام: reject نسخهٔ ناشناخته)
    pub schema_version: u32,
    /// پروفایل مقطع
    pub profile: Vec<ProfilePoint>,
    /// تنظیمات فرم (ارتفاع/قطر/taper/smoothing)
    pub form_height_mm: f64,
    pub form_bottom_diameter_mm: f64,
    pub form_top_diameter_mm: f64,
    pub taper_percent: f64,
    /// روش درون‌یابی پروفایل — نسخهٔ ۱.۲ (پیش‌فرض اسپلاین مونوتون؛
    /// فایل‌های قدیمی بدون این فیلد هم اسپلاین می‌گیرند)
    #[serde(default)]
    pub profile_interpolation: Interpolation,
    /// مرکز‌گذاری فرم روی صفحهٔ چاپ — نسخهٔ ۱.۲ (پیش‌فرض روشن)
    #[serde(default = "default_true")]
    pub center_on_bed: bool,
    /// چرخش کل فرم حول محور Z (درجه) — نسخهٔ ۱.۳.
    /// زاویهٔ شروع مارپیچ/درز (seam) و جهت گلبرگ‌ها را تعیین می‌کند؛
    /// با اسپات 🔄 روی نمای 3D هم قابل‌درگ است.
    #[serde(default)]
    pub phase_start_deg: f64,
    /// الگوهای هندسی فعال (ترتیب اعمال)
    pub patterns: Vec<PatternParams>,
    /// بافت‌های مسیر فعال
    pub textures: Vec<TextureParams>,
    /// کف
    pub floor: FloorSettings,
    /// پروفایل پرینتر
    pub printer: PrinterProfile,
    /// فراداده
    pub created_at_unix: u64,
    pub note: String,
}

impl Default for DesignerProject {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            profile: ProfilePreset::vase_default(),
            form_height_mm: 100.0,
            form_bottom_diameter_mm: 60.0,
            form_top_diameter_mm: 60.0,
            taper_percent: 0.0,
            profile_interpolation: Interpolation::Spline,
            center_on_bed: true,
            phase_start_deg: 0.0,
            patterns: Vec::new(),
            textures: Vec::new(),
            floor: FloorSettings::default(),
            printer: PrinterProfile::default(),
            created_at_unix: 0,
            note: String::new(),
        }
    }
}

/// اعتبارسنجی پروژهٔ بارگذاری‌شده — schema ناشناخته/خراب رد می‌شود
/// بدون آنکه پروژهٔ جاری کاربر خراب شود (الزام §7)
pub fn validate_project(json_str: &str) -> Result<DesignerProject, String> {
    let proj: DesignerProject = serde_json::from_str(json_str)
        .map_err(|e| format!("فایل پروژه قابل خواندن نیست: {e}"))?;
    if proj.schema_version > PROJECT_SCHEMA_VERSION {
        return Err(format!(
            "نسخهٔ schema پروژه ({}) از نسخهٔ پشتیبانی‌شده ({}) جدیدتر است؛ فایل رد شد",
            proj.schema_version, PROJECT_SCHEMA_VERSION
        ));
    }
    if proj.profile.len() < 2 {
        return Err("پروفایل مقطع حداقل باید ۲ نقطه داشته باشد".into());
    }
    // گاردهای عددی: NaN/Infinity و مقادیر منفی منطقی
    if !(proj.form_height_mm.is_finite() && proj.form_height_mm > 0.0) {
        return Err("ارتفاع فرم نامعتبر است".into());
    }
    if !(proj.form_bottom_diameter_mm.is_finite() && proj.form_bottom_diameter_mm > 0.0) {
        return Err("قطر پایین فرم نامعتبر است".into());
    }
    if !(proj.printer.layer_height_mm.is_finite() && proj.printer.layer_height_mm > 0.01) {
        return Err("ارتفاع لایهٔ پروفایل پرینتر نامعتبر است".into());
    }
    Ok(proj)
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_roundtrip() {
        // ذخیره/بازیابی JSON پروژه باید بدون فقدان داده باشد
        let mut p = DesignerProject::default();
        p.note = "آزمون".into();
        p.patterns.push(PatternParams {
            kind: PatternKind::Wave,
            amplitude_mm: 2.0,
            count: 8,
            frequency: 1.0,
            phase_deg: 0.0,
            z_start_mm: 0.0,
            z_end_mm: None,
            ..Default::default()
        });
        let json = serde_json::to_string(&p).unwrap();
        let back: DesignerProject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.note, "آزمون");
        assert_eq!(back.patterns.len(), 1);
        assert_eq!(back.patterns[0].kind, PatternKind::Wave);
        assert_eq!(back.schema_version, PROJECT_SCHEMA_VERSION);
    }

    #[test]
    fn test_validate_rejects_unknown_schema() {
        // schema نسخهٔ آینده رد می‌شود
        let bad = r#"{"schema_version":99,"profile":[{"r":10.0,"h":0.0},{"r":10.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":false,"center_hole_diameter_mm":0.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        assert!(validate_project(bad).is_err());
    }

    #[test]
    fn test_validate_rejects_corrupt() {
        assert!(validate_project("{not json").is_err());
        // پروفایل خیلی کوتاه
        let short = r#"{"schema_version":1,"profile":[{"r":10.0,"h":0.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":false,"center_hole_diameter_mm":0.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        assert!(validate_project(short).is_err());
    }

    #[test]
    fn test_printer_profile_defaults_honest() {
        // preset پیش‌فرض نباید verified باشد (الزام صداقت)
        let p = PrinterProfile::default();
        assert_eq!(p.status, "unverified");
        assert!(p.build_volume_mm[0] > 0.0);
    }

    // ================= آزمون‌های نسخهٔ ۱.۶ =================

    #[test]
    fn test_spot_angular_window() {
        // پنجرهٔ زاویه‌ای اسپات — مرکز ۹۰، دهانه ۱۲۰ (پوشش ۳۰..۱۵۰)
        let w = |theta_deg: f64| angular_window_weight(90.0, 120.0, theta_deg.to_radians());
        // داخل کامل
        assert!((w(90.0) - 1.0).abs() < 1e-9, "مرکز باید ۱ باشد");
        assert!((w(60.0) - 1.0).abs() < 1e-9, "داخل پنجره ۱");
        // خارج کامل (روبروی مرکز)
        assert!(w(270.0) < 1e-9, "روبروی مرکز باید ۰ باشد");
        assert!(w(200.0) < 1e-9, "خارج پنجره ۰");
        // wrap: ۳۵۰° = فاصلهٔ ۱۰۰ از ۹۰ → خارج؛ ۳۰° = مرز داخل
        assert!(w(350.0) < 1e-9, "wrap: ۳۵۰ خارج");
        assert!(w(31.0) > 0.0, "۳۱ درجه داخل/لبه");
        // لبهٔ نرم: ۱۴۵ (نزدیک ۱۵۰) بین ۰ و ۱
        let e = w(145.0);
        assert!(e > 0.0 && e < 1.0, "لبهٔ نرم smoothstep: {e}");
        // دهانهٔ کامل = همه‌جا ۱ (رفتار قدیمی)
        assert!((angular_window_weight(0.0, 360.0, 3.0) - 1.0).abs() < 1e-9);
        assert!((angular_window_weight(0.0, 720.0, 2.5) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_old_project_json_loads_with_v16_spot_defaults() {
        // فایل ۱.۵ (بدون فیلدهای اسپات) → دهانهٔ کامل ۳۶۰ (رفتار قدیمی)
        let old_json = r#"{"schema_version":1,"profile":[{"r":22.0,"h":0.0},{"r":30.0,"h":35.0},{"r":21.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[{"kind":"Petal","amplitude_mm":2.0,"count":6,"frequency":1.0,"phase_deg":0.0,"z_start_mm":0.0,"z_end_mm":null}],"textures":[{"kind":"Ripple","amplitude_mm":0.4,"period_mm":4.0,"radial":true,"phase_deg":0.0,"fade_in_percent":0.0,"fade_out_percent":0.0}],"floor":{"enabled":false,"center_hole_diameter_mm":0.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        let p = validate_project(old_json).expect("پروژهٔ قدیمی باید بخواند");
        assert_eq!(p.patterns[0].spot_span_deg, 360.0, "پیش‌فرض اسپات = کامل");
        assert_eq!(p.patterns[0].spot_center_deg, 0.0);
        assert_eq!(p.textures[0].spot_span_deg, 360.0);
    }

    #[test]
    fn test_project_roundtrip_spot_fields() {
        // ذخیره/بازیابی فیلدهای اسپات بدون فقدان داده
        let mut p = DesignerProject::default();
        p.patterns.push(PatternParams {
            spot_center_deg: 45.0,
            spot_span_deg: 90.0,
            ..Default::default()
        });
        p.textures.push(TextureParams {
            spot_center_deg: 200.0,
            spot_span_deg: 60.0,
            ..Default::default()
        });
        let json = serde_json::to_string(&p).unwrap();
        let back: DesignerProject = serde_json::from_str(&json).unwrap();
        assert!((back.patterns[0].spot_center_deg - 45.0).abs() < 1e-9);
        assert!((back.patterns[0].spot_span_deg - 90.0).abs() < 1e-9);
        assert!((back.textures[0].spot_center_deg - 200.0).abs() < 1e-9);
        assert!((back.textures[0].spot_span_deg - 60.0).abs() < 1e-9);
    }

    // ================= آزمون‌های نسخهٔ ۱.۲ =================

    #[test]
    fn test_old_project_json_loads_with_defaults() {
        // فایل ۱.۱ بدون فیلدهای جدید باید با پیش‌فرض‌های ۱.۲ بارگذاری شود:
        // center_on_bed=true، interpolation=Spline، floor.pattern=Rings
        let old_json = r#"{"schema_version":1,"profile":[{"r":22.0,"h":0.0},{"r":30.0,"h":35.0},{"r":21.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":true,"center_hole_diameter_mm":8.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        let p = validate_project(old_json).expect("پروژهٔ قدیمی باید بخواند");
        assert!(p.center_on_bed, "پیش‌فرض مرکز‌گذاری روشن");
        assert_eq!(p.profile_interpolation, Interpolation::Spline);
        assert_eq!(p.floor.pattern, FloorPattern::Rings);
        assert_eq!(p.floor.spokes, 12);
        assert!(p.floor.enabled);
        assert!((p.floor.center_hole_diameter_mm - 8.0).abs() < 1e-9);
    }

    #[test]
    fn test_floor_pattern_names() {
        // هر الگوی کف نام فارسی دارد (برای کارت‌های UI)
        for fp in [
            FloorPattern::Rings,
            FloorPattern::Spiral,
            FloorPattern::Grid,
            FloorPattern::Radial,
            FloorPattern::ZigZag,
        ] {
            assert!(!fp.fa().is_empty());
        }
    }

    // ================= آزمون‌های نسخهٔ ۱.۳ =================

    #[test]
    fn test_old_project_json_loads_with_v13_defaults() {
        // فایل ۱.۲ (بدون phase_start_deg و بدون دماها) باید با
        // پیش‌فرض‌های ۱.۳ بخواند: فاز ۰، نازل ۲۰۰، بستر ۶۰، فن ۱۰۰، انتظار روشن
        let old_json = r#"{"schema_version":1,"profile":[{"r":22.0,"h":0.0},{"r":30.0,"h":35.0},{"r":21.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":true,"center_hole_diameter_mm":8.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        let p = validate_project(old_json).expect("پروژهٔ قدیمی باید بخواند");
        assert_eq!(p.phase_start_deg, 0.0);
        assert!((p.printer.nozzle_temp_c - 200.0).abs() < 1e-9);
        assert!((p.printer.bed_temp_c - 60.0).abs() < 1e-9);
        assert!((p.printer.fan_percent - 100.0).abs() < 1e-9);
        assert!(p.printer.wait_for_temps);
    }

    #[test]
    fn test_project_roundtrip_phase_and_temps() {
        // ذخیره/بازیابی فاز و دما بدون فقدان داده
        let mut p = DesignerProject::default();
        p.phase_start_deg = 137.5;
        p.printer.nozzle_temp_c = 240.0;
        p.printer.bed_temp_c = 80.0;
        p.printer.fan_percent = 30.0;
        p.printer.wait_for_temps = false;
        let json = serde_json::to_string(&p).unwrap();
        let back: DesignerProject = serde_json::from_str(&json).unwrap();
        assert!((back.phase_start_deg - 137.5).abs() < 1e-9);
        assert!((back.printer.nozzle_temp_c - 240.0).abs() < 1e-9);
        assert!((back.printer.bed_temp_c - 80.0).abs() < 1e-9);
        assert!((back.printer.fan_percent - 30.0).abs() < 1e-9);
        assert!(!back.printer.wait_for_temps);
    }

    // ================= آزمون‌های نسخهٔ ۱.۴ =================

    #[test]
    fn test_old_project_json_loads_with_v14_defaults() {
        // فایل ۱.۳ (بدون layers کف و بدون سرعت‌های mm/s) باید با
        // پیش‌فرض‌های ۱.۴ بخواند: کف ۲ لایه، سرعت mm/s = ۰ (پیروی از mm/min قدیمی)،
        // رتراکشن ۰٫۸mm@40mm/s
        let old_json = r#"{"schema_version":1,"profile":[{"r":22.0,"h":0.0},{"r":30.0,"h":35.0},{"r":21.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":true,"center_hole_diameter_mm":8.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        let p = validate_project(old_json).expect("پروژهٔ قدیمی باید بخواند");
        assert_eq!(p.floor.layers, 2, "پیش‌فرض کف ۲ لایه");
        assert_eq!(p.printer.wall_speed_mm_s, 0.0, "mm/s صفر = پیروی از mm/min");
        assert_eq!(p.printer.travel_speed_mm_s, 0.0);
        // حل تقدم: mm/min قدیمی معتبر می‌ماند
        assert_eq!(p.printer.effective_wall_mm_min(), 2400.0);
        assert_eq!(p.printer.effective_travel_mm_min(), 6000.0);
        // مشتقات پیش‌فرض لایهٔ اول/کف
        assert_eq!(p.printer.effective_first_layer_mm_min(), 1200.0);
        assert_eq!(p.printer.effective_floor_mm_min(), 1800.0);
        // رتراکشن پیش‌فرض
        assert!((p.printer.retract_mm - 0.8).abs() < 1e-9);
        assert!((p.printer.retract_speed_mm_s - 40.0).abs() < 1e-9);
    }

    #[test]
    fn test_speed_resolution_mm_s_priority() {
        // mm/s جدید بر mm/min قدیمی تقدم دارد
        let mut p = PrinterProfile::default();
        p.print_speed_mm_min = 2400.0;
        p.wall_speed_mm_s = 55.0; // 3300 mm/min
        assert_eq!(p.effective_wall_mm_min(), 3300.0);
        p.travel_speed_mm_s = 150.0;
        assert_eq!(p.effective_travel_mm_min(), 9000.0);
        p.first_layer_speed_mm_s = 25.0;
        assert_eq!(p.effective_first_layer_mm_min(), 1500.0);
        p.floor_speed_mm_s = 40.0;
        assert_eq!(p.effective_floor_mm_min(), 2400.0);
    }
}
