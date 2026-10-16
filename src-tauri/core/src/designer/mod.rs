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
pub use profile::{ProfilePreset, ProfilePoint};
pub use spiral::{DesignerInput, DesignerOutput};
pub use texture::{TextureKind, TextureParams};

use serde::{Deserialize, Serialize};

/// تنظیمات کف دیسکی — گام ۴ گردش‌کار
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorSettings {
    /// آیا کف ساخته شود
    pub enabled: bool,
    /// قطر سوراخ مرکزی کف (mm) — 0 یعنی بدون سوراخ
    pub center_hole_diameter_mm: f64,
    /// ارتفاع شروع کف (معمولاً 0)
    pub z_start_mm: f64,
    /// فاصلهٔ خطوط هم‌مرکز کف (mm) — معمولاً برابر line width
    pub ring_gap_mm: f64,
}

impl Default for FloorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            center_hole_diameter_mm: 0.0,
            z_start_mm: 0.0,
            ring_gap_mm: 0.45,
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
    /// کد start سفارشی (قبل از مسیر) — پیش‌نمایش و هشدار پیش از الحاق
    pub start_gcode: String,
    /// کد end سفارشی (بعد از مسیر)
    pub end_gcode: String,
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
            start_gcode: String::new(),
            end_gcode: String::new(),
        }
    }
}

/// فایل پروژهٔ طراح — قالب JSON نسخه‌دار (الزام §7)
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

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
}
