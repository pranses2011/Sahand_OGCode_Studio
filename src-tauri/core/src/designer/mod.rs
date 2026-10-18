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
pub mod text2vase;
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
    /// وب لامپ — نسخهٔ ۱٫۷: حلقه‌های هم‌مرکز + پره‌های شعاعی (شبکهٔ نور)
    LampWeb,
    /// خورشیدشکاف — نسخهٔ ۱٫۷: پرتوهای پهن از سوکت مرکزی تا لبه (نور مستقیم)
    Sunburst,
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
            FloorPattern::LampWeb => "وب لامپ (حلقه + پره)",
            FloorPattern::Sunburst => "خورشیدشکاف (پرتو پهن)",
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

// ============================================================================
// تنظیمات دیواره — نسخهٔ ۱٫۷ (درخواست کاربر ۳ و ۹)
// ============================================================================

/// حالت دیواره: جامد پیوسته یا توری (درخواست کاربر ۳ — «چاپ توری مانند مرجع»)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WallMode {
    /// دیوارهٔ مارپیچی پیوسته — رفتار نسخه‌های ≤۱٫۶
    #[default]
    Solid,
    /// توری بافته: مسیر زیگزاگ روی سطح با خانه‌های باز (الماس/مربع/مهری)
    Lattice,
}

impl WallMode {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            WallMode::Solid => "دیوارهٔ جامد",
            WallMode::Lattice => "توری (خانه‌های باز)",
        }
    }
}

/// سبک بافت توری — نسخهٔ ۱٫۷
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LatticeStyle {
    /// الماسی: زیگزاگ متقارن — خانه‌های لوزی کلاسیک گلدان توری
    #[default]
    Diamond,
    /// مربعی: زیگزاگ با سکوهای افقی بالا/پایین خانه‌ها
    Square,
    /// مهری: زیگزاگ تیز شخورداده (شبیه لوزی با دندانه‌ها)
    Chevron,
    /// پیچان: الماس با جابه‌جایی فاز هر انقلاب — چرخش مارپیچی شبکه
    Twisted,
}

impl LatticeStyle {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            LatticeStyle::Diamond => "الماسی",
            LatticeStyle::Square => "مربعی",
            LatticeStyle::Chevron => "مهری (شخورداده)",
            LatticeStyle::Twisted => "پیچان",
        }
    }
}

/// تنظیمات توری — درخواست کاربر ۳: «مسیر رو تقویت کن تا مانند مرجع
/// بتونم بصورت توری هم چاپ بکنم و تنظیمات کامل داشته باشم»
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeSettings {
    /// سبک خانه‌ها
    #[serde(default)]
    pub style: LatticeStyle,
    /// عرض خانه در طول محیط (mm) — ۶..۴۰
    #[serde(default = "default_lat_cell_w")]
    pub cell_width_mm: f64,
    /// ارتفاع خانه (mm) — ۴..۳۰
    #[serde(default = "default_lat_cell_h")]
    pub cell_height_mm: f64,
    /// نسبت صعود هر انقلاب به ارتفاع خانه — ۰٫۵ = الماس متقارن؛
    /// کوچک‌تر = خانه‌های کشیده‌تر عمودی (۰٫۲..۰٫۸)
    #[serde(default = "default_lat_rise")]
    pub rise_ratio: f64,
    /// سرعت تارهای توری نسبت به سرعت دیواره (۰٫۳..۱) — تارهای مورب
    /// روی هوا چاپ می‌شوند؛ آرام‌تر = چسبندگی بهتر در تقاطع‌ها
    #[serde(default = "default_lat_speed")]
    pub speed_scale: f64,
    /// ضریب جریان تارها — ۱ = مثل دیواره؛ کمی بیشتر برای پیوند قوی
    #[serde(default = "default_lat_flow")]
    pub flow_scale: f64,
    /// چرخش شبکهٔ توری (درجه) — جهت تارهای مورب
    #[serde(default)]
    pub phase_deg: f64,
    /// ارتفاع شروع توری (mm) — پایین دیواره جامد بماند (لبهٔ مقاوم)
    #[serde(default = "default_lat_z0")]
    pub z_start_mm: f64,
    /// ارتفاع پایان توری (None = تا بالای فرم)
    #[serde(default)]
    pub z_end_mm: Option<f64>,
}

fn default_lat_cell_w() -> f64 {
    12.0
}
fn default_lat_cell_h() -> f64 {
    10.0
}
fn default_lat_rise() -> f64 {
    0.5
}
fn default_lat_speed() -> f64 {
    0.6
}
fn default_lat_flow() -> f64 {
    1.1
}
fn default_lat_z0() -> f64 {
    8.0
}

impl Default for LatticeSettings {
    fn default() -> Self {
        Self {
            style: LatticeStyle::default(),
            cell_width_mm: default_lat_cell_w(),
            cell_height_mm: default_lat_cell_h(),
            rise_ratio: default_lat_rise(),
            speed_scale: default_lat_speed(),
            flow_scale: default_lat_flow(),
            phase_deg: 0.0,
            z_start_mm: default_lat_z0(),
            z_end_mm: None,
        }
    }
}

/// شکل سوراخ دیواره — درخواست کاربر ۹
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HoleShape {
    /// دایره
    #[default]
    Circle,
    /// مستطیل با گوشهٔ گرد
    RoundedRect,
    /// لوزی (الماس)
    Diamond,
    /// شش‌ضلعی
    Hexagon,
    /// ستارهٔ چهارپر (برای پنجره‌های تزئینی)
    Star,
}

impl HoleShape {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            HoleShape::Circle => "دایره",
            HoleShape::RoundedRect => "مستطیل گردگوشه",
            HoleShape::Diamond => "لوزی",
            HoleShape::Hexagon => "شش‌ضلعی",
            HoleShape::Star => "ستارهٔ چهارپر",
        }
    }
}

/// تنظیمات سوراخ دیواره — درخواست کاربر ۹:
/// «سوراخ‌ها رو در با دیواره بساز نه اینکه فقط در لایه آخر یک دیواره
///  روش بکشی و تنظیمات کامل براش بزار»
///
/// پیاده‌سازی: مارپیچ هنگام عبور از ناحیهٔ سوراک «travel» می‌شود (دهانهٔ
/// واقعی باز می‌ماند) و دور هر سوراخ «قاب تقویتی» (حلقهٔ بستهٔ مستطیلی
/// در فضای بازشدهٔ استوانه) چاپ می‌شود — سوراخ واقعی با لبهٔ مسلح، نه
/// پوشش سطحی.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallHoleSettings {
    /// ساخت سوراخ در دیواره؟
    #[serde(default)]
    pub enabled: bool,
    /// شکل سوراخ
    #[serde(default)]
    pub shape: HoleShape,
    /// تعداد سوراخ دور محیط (۲..۱۲)
    #[serde(default = "default_hole_around")]
    pub count_around: u32,
    /// تعداد ردیف عمودی (۱..۸)
    #[serde(default = "default_hole_rows")]
    pub rows: u32,
    /// عرض سوراخ در طول محیط (mm)
    #[serde(default = "default_hole_w")]
    pub width_mm: f64,
    /// ارتفاع سوراخ (mm)
    #[serde(default = "default_hole_h")]
    pub height_mm: f64,
    /// فاصلهٔ مرکز ردیف‌ها (mm) — ۰ = خودکار از ارتفاع سوراخ + حاشیه
    #[serde(default)]
    pub row_pitch_mm: f64,
    /// حاشیهٔ پایین اولین ردیف (mm)
    #[serde(default = "default_hole_mb")]
    pub margin_bottom_mm: f64,
    /// حاشیهٔ بالای آخرین ردیف (mm)
    #[serde(default = "default_hole_mt")]
    pub margin_top_mm: f64,
    /// دورهای قاب تقویتی دور هر سوراخ (۰..۳) — ۲ = لبهٔ ضخیم مقاوم
    #[serde(default = "default_hole_frames")]
    pub frame_passes: u32,
    /// فاز زاویه‌ای چینش (درجه)
    #[serde(default)]
    pub phase_deg: f64,
    /// ردیف‌های زوج نیم‌گام جابه‌جا شوند (چینش آجری)
    #[serde(default = "default_true")]
    pub stagger: bool,
}

fn default_hole_around() -> u32 {
    4
}
fn default_hole_rows() -> u32 {
    3
}
fn default_hole_w() -> f64 {
    14.0
}
fn default_hole_h() -> f64 {
    18.0
}
fn default_hole_mb() -> f64 {
    10.0
}
fn default_hole_mt() -> f64 {
    10.0
}
fn default_hole_frames() -> u32 {
    2
}

impl Default for WallHoleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            shape: HoleShape::default(),
            count_around: default_hole_around(),
            rows: default_hole_rows(),
            width_mm: default_hole_w(),
            height_mm: default_hole_h(),
            row_pitch_mm: 0.0,
            margin_bottom_mm: default_hole_mb(),
            margin_top_mm: default_hole_mt(),
            frame_passes: default_hole_frames(),
            phase_deg: 0.0,
            stagger: true,
        }
    }
}

/// تنظیمات کامل دیواره — نسخهٔ ۱٫۷
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WallSettings {
    /// جامد یا توری
    #[serde(default)]
    pub mode: WallMode,
    /// پارامترهای توری (وقتی mode = Lattice)
    #[serde(default)]
    pub lattice: LatticeSettings,
    /// سوراخ‌های دیواره (در هر دو حالت کار می‌کند)
    #[serde(default)]
    pub holes: WallHoleSettings,
}

// ============================================================================
// تنظیمات آویز (معلق‌کردن) — نسخهٔ ۱٫۷ (درخواست کاربر ۷)
// ============================================================================

/// سبک آویز — «تنظیمات آویز رو زیادتر و حرفه‌ای تر بکن»
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HangingStyle {
    /// بدون آویز
    #[default]
    None,
    /// سوراخ‌های لبهٔ کف — نخ/سیم از زیر رد شود (گلدان آویز کلاسیک)
    RimHoles,
    /// سوراخ‌های بالای دیواره — زنجیر/نخ از لبهٔ بالا (شیک‌تر)
    TopHoles,
    /// ترکیبی: سوراخ بالا + سوراخ کف (تعادل ۳ نقطه‌ای و بیشتر)
    Both,
}

impl HangingStyle {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            HangingStyle::None => "بدون آویز",
            HangingStyle::RimHoles => "سوراخ لبهٔ کف (نخ از زیر)",
            HangingStyle::TopHoles => "سوراخ بالای دیواره (زنجیر از بالا)",
            HangingStyle::Both => "ترکیبی (کف + بالا)",
        }
    }
}

/// تنظیمات آویز حرفه‌ای — نسخهٔ ۱٫۷ (درخواست کاربر ۷)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendantSettings {
    /// سبک آویز
    #[serde(default)]
    pub style: HangingStyle,
    /// تعداد سوراخ کف (۳..۱۲) — برای تعادل، ۳ یا بیشتر
    #[serde(default = "default_pen_rim_n")]
    pub rim_holes_count: u32,
    /// قطر سوراخ کف (mm) — ۳ = نخ؛ ۵ = سیم پارچه؛ ۸ = زنجیر ظریف
    #[serde(default = "default_pen_rim_d")]
    pub rim_hole_diameter_mm: f64,
    /// موقعیت سوراخ کف (٪ شعاع — ۶۰..۹۲)؛ نزدیک لبه = گشتاور کمتر
    #[serde(default = "default_pen_rim_pos")]
    pub rim_hole_radius_percent: f64,
    /// فاصلهٔ زاویه‌ای سوراخ‌های کف (درجه) — ۰ = پخش مساوی
    #[serde(default)]
    pub rim_phase_deg: f64,
    /// تعداد سوراخ بالای دیواره (۲..۶)
    #[serde(default = "default_pen_top_n")]
    pub top_holes_count: u32,
    /// قطر سوراخ بالای دیواره (mm)
    #[serde(default = "default_pen_top_d")]
    pub top_hole_diameter_mm: f64,
    /// فاصلهٔ مرکز سوراخ‌های بالا از لبهٔ بالای فرم (mm)
    #[serde(default = "default_pen_top_from")]
    pub top_hole_from_rim_mm: f64,
    /// حلقه‌های تقویت لبهٔ بالا — دورهای اضافه در انتهای مارپیچ؛
    /// لبهٔ آویز تحت کشش است و لبهٔ ضخیم = پارگی دیرتر
    #[serde(default = "default_pen_rings")]
    pub rim_reinforce_rings: u32,
    /// گوشه‌های اتصال سوراخ بالا به بدنه (ستون‌های کوچک عمودی)
    #[serde(default = "default_true")]
    pub top_hole_frames: bool,
}

fn default_pen_rim_n() -> u32 {
    3
}
fn default_pen_rim_d() -> f64 {
    3.0
}
fn default_pen_rim_pos() -> f64 {
    85.0
}
fn default_pen_top_n() -> u32 {
    3
}
fn default_pen_top_d() -> f64 {
    5.0
}
fn default_pen_top_from() -> f64 {
    8.0
}
fn default_pen_rings() -> u32 {
    3
}

impl Default for PendantSettings {
    fn default() -> Self {
        Self {
            style: HangingStyle::default(),
            rim_holes_count: default_pen_rim_n(),
            rim_hole_diameter_mm: default_pen_rim_d(),
            rim_hole_radius_percent: default_pen_rim_pos(),
            rim_phase_deg: 0.0,
            top_holes_count: default_pen_top_n(),
            top_hole_diameter_mm: default_pen_top_d(),
            top_hole_from_rim_mm: default_pen_top_from(),
            rim_reinforce_rings: default_pen_rings(),
            top_hole_frames: true,
        }
    }
}

// ============================================================================
// تنظیمات لامپ — نسخهٔ ۱٫۷ (درخواست کاربر ۸)
// «تنظیمات لامپ رو زیادتر و حرفه‌ای تر بکن و همچنین طرح های زیادی برای
//  کف لامپ و سوراخ وسط و پایه های متصل کننده سوراخ به بدنه»
// ============================================================================

/// سبک پره‌های متصل‌کنندهٔ سوکت به بدنه
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpokesStyle {
    /// هر پره چند خط موازی (پهن و محکم — پیش‌فرض)
    #[default]
    Parallel,
    /// تک‌خط باریک (نور بیشتر، استحکام کمتر)
    Single,
    /// شبکهٔ وب: پره + کمان‌های اتصال بین پره‌ها (استحکام حداکثر)
    Web,
}

impl SpokesStyle {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            SpokesStyle::Parallel => "پره‌های پهن موازی",
            SpokesStyle::Single => "تک‌خط باریک",
            SpokesStyle::Web => "شبکهٔ وب (پره + کمان)",
        }
    }
}

/// تنظیمات حرفه‌ای لامپ — نسخهٔ ۱٫۷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LampSettings {
    /// حالت لامپ روشن؟ (کف نور + سوکت + پره‌ها فعال می‌شود)
    #[serde(default)]
    pub enabled: bool,
    /// قطر سوراخ مرکزی برای سوکت/سیم (mm) —
    /// ۸ = سیم؛ ۱۶ = سوکت کوچک؛ ۲۸/۴۰ = سوکت E14/E27 استاندارد
    #[serde(default = "default_lamp_hole")]
    pub center_hole_diameter_mm: f64,
    /// حلقه‌های تقویت دور سوکت (۰..۶) — «سوراخ وسط» مسلح
    #[serde(default = "default_lamp_rings")]
    pub socket_rings: u32,
    /// تعداد پره‌های متصل‌کنندهٔ سوکت به بدنه (۳..۱۲)
    #[serde(default = "default_lamp_spokes")]
    pub spokes_count: u32,
    /// عرض هر پره (mm) — «پایه‌های متصل‌کننده»
    #[serde(default = "default_lamp_spoke_w")]
    pub spokes_width_mm: f64,
    /// سبک پره‌ها
    #[serde(default)]
    pub spokes_style: SpokesStyle,
    /// ضریب فاصلهٔ خطوط کف — بیشتر از ۱ = کفِ بازتر برای عبور نور
    /// (مثلاً ۱٫۶ = فاصلهٔ خطوط ۶۰٪ بیشتر → نور ملایم)
    #[serde(default = "default_lamp_gap")]
    pub floor_gap_multiplier: f64,
    /// حلقهٔ فرورفته دور سوکت در «لایهٔ دوم» کف (ظاهر حرفه‌ای)
    #[serde(default = "default_true")]
    pub recessed_socket: bool,
}

fn default_lamp_hole() -> f64 {
    16.0
}
fn default_lamp_rings() -> u32 {
    3
}
fn default_lamp_spokes() -> u32 {
    4
}
fn default_lamp_spoke_w() -> f64 {
    2.4
}
fn default_lamp_gap() -> f64 {
    1.6
}

impl Default for LampSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            center_hole_diameter_mm: default_lamp_hole(),
            socket_rings: default_lamp_rings(),
            spokes_count: default_lamp_spokes(),
            spokes_width_mm: default_lamp_spoke_w(),
            spokes_style: SpokesStyle::default(),
            floor_gap_multiplier: default_lamp_gap(),
            recessed_socket: true,
        }
    }
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
    /// تنظیمات دیواره: جامد/توری + سوراخ‌ها — نسخهٔ ۱٫۷ (درخواست ۳ و ۹)
    #[serde(default)]
    pub wall: WallSettings,
    /// تنظیمات آویز حرفه‌ای — نسخهٔ ۱٫۷ (درخواست ۷)
    #[serde(default)]
    pub pendant: PendantSettings,
    /// تنظیمات حرفه‌ای لامپ — نسخهٔ ۱٫۷ (درخواست ۸)
    #[serde(default)]
    pub lamp: LampSettings,
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
            wall: WallSettings::default(),
            pendant: PendantSettings::default(),
            lamp: LampSettings::default(),
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

    // ================= آزمون‌های نسخهٔ ۱٫۷ =================

    #[test]
    fn test_old_project_json_loads_with_v17_defaults() {
        // فایل ۱٫۶ (بدون wall/pendant/lamp) باید با پیش‌فرض‌های ۱٫۷ بخواند:
        // دیوارهٔ جامد + سوراخ خاموش + آویز none + لامپ خاموش
        let old_json = r#"{"schema_version":1,"profile":[{"r":22.0,"h":0.0},{"r":30.0,"h":35.0},{"r":21.0,"h":100.0}],"form_height_mm":100.0,"form_bottom_diameter_mm":60.0,"form_top_diameter_mm":60.0,"taper_percent":0.0,"patterns":[],"textures":[],"floor":{"enabled":true,"center_hole_diameter_mm":8.0,"z_start_mm":0.0,"ring_gap_mm":0.45},"printer":{"name":"x","status":"unverified","build_volume_mm":[220,220,250],"nozzle_diameter_mm":0.4,"filament_diameter_mm":1.75,"line_width_mm":0.45,"layer_height_mm":0.25,"print_speed_mm_min":2400.0,"travel_speed_mm_min":6000.0,"flow_multiplier":1.0,"absolute_e":true,"start_gcode":"","end_gcode":""},"created_at_unix":0,"note":""}"#;
        let p = validate_project(old_json).expect("پروژهٔ قدیمی ۱٫۶ باید بخواند");
        assert_eq!(p.wall.mode, WallMode::Solid, "پیش‌فرض دیوارهٔ جامد");
        assert!(!p.wall.holes.enabled, "پیش‌فرض سوراخ خاموش");
        assert_eq!(p.pendant.style, HangingStyle::None);
        assert!(!p.lamp.enabled);
        // پیش‌فرض‌های توری معتبر
        assert!((p.wall.lattice.cell_width_mm - 12.0).abs() < 1e-9);
        assert!((p.wall.lattice.rise_ratio - 0.5).abs() < 1e-9);
        assert_eq!(p.wall.holes.frame_passes, 2);
        assert_eq!(p.pendant.rim_reinforce_rings, 3);
        assert!((p.lamp.center_hole_diameter_mm - 16.0).abs() < 1e-9);
    }

    #[test]
    fn test_project_roundtrip_v17_wall_pendant_lamp() {
        // ذخیره/بازیابی کامل تنظیمات ۱٫۷ بدون فقدان داده
        let mut p = DesignerProject::default();
        p.wall.mode = WallMode::Lattice;
        p.wall.lattice.style = LatticeStyle::Twisted;
        p.wall.lattice.cell_width_mm = 9.5;
        p.wall.lattice.cell_height_mm = 7.0;
        p.wall.lattice.rise_ratio = 0.35;
        p.wall.holes.enabled = true;
        p.wall.holes.shape = HoleShape::Hexagon;
        p.wall.holes.count_around = 6;
        p.wall.holes.rows = 2;
        p.wall.holes.stagger = false;
        p.pendant.style = HangingStyle::TopHoles;
        p.pendant.top_holes_count = 4;
        p.pendant.top_hole_diameter_mm = 6.0;
        p.lamp.enabled = true;
        p.lamp.center_hole_diameter_mm = 28.0;
        p.lamp.spokes_count = 6;
        p.lamp.spokes_style = SpokesStyle::Web;
        p.floor.pattern = FloorPattern::LampWeb;
        let json = serde_json::to_string(&p).unwrap();
        let back: DesignerProject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.wall.mode, WallMode::Lattice);
        assert_eq!(back.wall.lattice.style, LatticeStyle::Twisted);
        assert!((back.wall.lattice.cell_width_mm - 9.5).abs() < 1e-9);
        assert_eq!(back.wall.holes.shape, HoleShape::Hexagon);
        assert_eq!(back.wall.holes.count_around, 6);
        assert!(!back.wall.holes.stagger);
        assert_eq!(back.pendant.style, HangingStyle::TopHoles);
        assert_eq!(back.pendant.top_holes_count, 4);
        assert!(back.lamp.enabled);
        assert!((back.lamp.center_hole_diameter_mm - 28.0).abs() < 1e-9);
        assert_eq!(back.lamp.spokes_style, SpokesStyle::Web);
        assert_eq!(back.floor.pattern, FloorPattern::LampWeb);
    }

    #[test]
    fn test_v17_fa_names_complete() {
        // همهٔ نام‌های فارسی پر هستند (UI کارت‌ها می‌سازد)
        for m in [WallMode::Solid, WallMode::Lattice] {
            assert!(!m.fa().is_empty());
        }
        for s in [
            LatticeStyle::Diamond,
            LatticeStyle::Square,
            LatticeStyle::Chevron,
            LatticeStyle::Twisted,
        ] {
            assert!(!s.fa().is_empty());
        }
        for h in [
            HoleShape::Circle,
            HoleShape::RoundedRect,
            HoleShape::Diamond,
            HoleShape::Hexagon,
            HoleShape::Star,
        ] {
            assert!(!h.fa().is_empty());
        }
        for h in [
            HangingStyle::None,
            HangingStyle::RimHoles,
            HangingStyle::TopHoles,
            HangingStyle::Both,
        ] {
            assert!(!h.fa().is_empty());
        }
        for s in [SpokesStyle::Parallel, SpokesStyle::Single, SpokesStyle::Web] {
            assert!(!s.fa().is_empty());
        }
        assert!(!FloorPattern::LampWeb.fa().is_empty());
        assert!(!FloorPattern::Sunburst.fa().is_empty());
    }

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
