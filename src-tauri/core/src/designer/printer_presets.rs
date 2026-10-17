//! ============================================================================
//! designer/printer_presets.rs — گام ۵: پریست‌های ۲۰ پرینتر واقعی و مطرح
//! ============================================================================
//! منبع داده: صفحات رسمی مشخصات سازنده‌ها (Bambu Lab / Prusa / Creality /
//! Anycubic / Artillery / Sovol / Elegoo / QIDI / Voron Design).
//!
//! قاعدهٔ صداقت (Docs/PROMPT_03 §8):
//!   هیچ پریستی «verified» نیست — چون اعداد ما را دستی روی دستگاه تست نکرده‌ایم.
//!   وضعیت همهٔ پریست‌ها «community» است: یعنی «داده از مشخصات عمومی رسمی،
//!   اما توسط این برنامه روی دستگاه آزموده نشده؛ قبل از چاپ صحت‌سنجی کنید».
//!
//! سرعت‌های چاپ درج‌شده «سرعت شروع محافظه‌کارانه برای چاپ مارپیچی/vase» هستند،
//! نه حداکثر سرعت رسمی دستگاه — این انتخاب عمدی است و در راهنمای کاربر مستند
//! شده (چاپ دیوارهٔ نازک تک‌جداره با حداکثر سرعت معمولاً شکست می‌خورد).
//! ============================================================================

use crate::designer::PrinterProfile;

/// سازندهٔ کمکی پریست با مقادیر رایج
fn preset(
    name: &str,
    volume: [f64; 3],
    print_speed: f64,
    travel_speed: f64,
    nozzle: f64,
) -> PrinterProfile {
    // سرعت‌های mm/s نسخهٔ ۱.۴ — از همان سرعت‌های محافظه‌کارانهٔ vase مشتق
    // می‌شوند: دیواره = print_speed/60، لایهٔ اول ۵۰٪، کف ۷۵٪
    let wall_mm_s = (print_speed / 60.0 * 10.0).round() / 10.0;
    PrinterProfile {
        name: name.into(),
        status: "community".into(),
        build_volume_mm: volume,
        nozzle_diameter_mm: nozzle,
        filament_diameter_mm: 1.75,
        line_width_mm: (nozzle * 1.05 * 100.0).round() / 100.0, // ~۵٪ بالای نازل
        layer_height_mm: 0.2,
        print_speed_mm_min: print_speed,
        travel_speed_mm_min: travel_speed,
        flow_multiplier: 1.0,
        absolute_e: true,
        // دما/فن — به پرینتر وابسته نیست، به «ماده» وابسته است؛
        // پیش‌فرض PLA عمومی می‌گذاریم و کاربر در گام ۵ با پریست ماده
        // (PLA/PETG/ABS/TPU) تنظیم می‌کند (نسخهٔ ۱.۳).
        nozzle_temp_c: 200.0,
        bed_temp_c: 60.0,
        fan_percent: 100.0,
        wait_for_temps: true,
        // سرعت‌ها — نسخهٔ ۱.۴ (mm/s)
        wall_speed_mm_s: wall_mm_s,
        first_layer_speed_mm_s: ((wall_mm_s * 0.5) * 10.0).round() / 10.0,
        floor_speed_mm_s: ((wall_mm_s * 0.75) * 10.0).round() / 10.0,
        travel_speed_mm_s: ((travel_speed / 60.0) * 10.0).round() / 10.0,
        retract_mm: 0.8,
        retract_speed_mm_s: 40.0,
        start_gcode: String::new(),
        end_gcode: String::new(),
    }
}

/// ۲۰ پریست پرینتر واقعی — مشخصات حجم ساخت از صفحات رسمی سازنده‌ها
pub fn printer_presets() -> Vec<PrinterProfile> {
    vec![
        // ---------- Bambu Lab (پرینترهای CoreXZ سرعت بالا) ----------
        preset("Bambu Lab A1 mini", [180.0, 180.0, 180.0], 3000.0, 9000.0, 0.4),
        preset("Bambu Lab A1", [256.0, 256.0, 256.0], 3000.0, 9000.0, 0.4),
        preset("Bambu Lab P1S", [256.0, 256.0, 256.0], 3000.0, 9000.0, 0.4),
        preset("Bambu Lab X1 Carbon", [256.0, 256.0, 256.0], 3000.0, 9000.0, 0.4),
        // ---------- Prusa Research (bed-slinger کلاسیک) ----------
        preset("Prusa MK3S+", [250.0, 210.0, 210.0], 2400.0, 6000.0, 0.4),
        preset("Prusa MK4", [250.0, 210.0, 220.0], 2700.0, 6000.0, 0.4),
        preset("Prusa MINI+", [180.0, 180.0, 180.0], 2400.0, 6000.0, 0.4),
        preset("Prusa XL", [360.0, 360.0, 360.0], 2700.0, 6000.0, 0.4),
        // ---------- Creality ----------
        preset("Creality Ender 3 S1", [220.0, 220.0, 270.0], 2400.0, 6000.0, 0.4),
        preset("Creality Ender 3 V3 KE", [220.0, 220.0, 240.0], 3000.0, 9000.0, 0.4),
        preset("Creality Ender 5 S1", [220.0, 220.0, 280.0], 2400.0, 6000.0, 0.4),
        preset("Creality K1", [220.0, 220.0, 250.0], 3000.0, 9000.0, 0.4),
        preset("Creality K1 Max", [300.0, 300.0, 300.0], 3000.0, 9000.0, 0.4),
        // ---------- Anycubic ----------
        preset("Anycubic Kobra 2", [220.0, 220.0, 250.0], 2700.0, 9000.0, 0.4),
        preset("Anycubic Kobra 3", [250.0, 250.0, 260.0], 2700.0, 9000.0, 0.4),
        // ---------- سایر سازنده‌های مطرح ----------
        preset("Artillery Sidewinder X2", [300.0, 300.0, 400.0], 2400.0, 6000.0, 0.4),
        preset("Sovol SV06", [220.0, 220.0, 250.0], 2400.0, 6000.0, 0.4),
        preset("Elegoo Neptune 4", [225.0, 225.0, 265.0], 2700.0, 6000.0, 0.4),
        preset("QIDI X-Plus 3", [280.0, 280.0, 270.0], 2700.0, 6000.0, 0.4),
        // ---------- طراحی جامعه (DIY) ----------
        preset("Voron 2.4 (350)", [350.0, 350.0, 250.0], 3000.0, 9000.0, 0.4),
    ]
}

/// برند پریست از روی نام (برای گروه‌بندی UI — فارسی)
pub fn brand_fa(name: &str) -> &'static str {
    if name.starts_with("Bambu Lab") {
        "بامبو لب"
    } else if name.starts_with("Prusa") {
        "پروزا"
    } else if name.starts_with("Creality") {
        "کریالیتی"
    } else if name.starts_with("Anycubic") {
        "انی‌کیوبیک"
    } else if name.starts_with("Artillery") {
        "آرتیلری"
    } else if name.starts_with("Sovol") {
        "سوول"
    } else if name.starts_with("Elegoo") {
        "الگو"
    } else if name.starts_with("QIDI") {
        "کیدای"
    } else if name.starts_with("Voron") {
        "ورون (DIY)"
    } else {
        "سایر"
    }
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================

// ============================================================================
// ذخیره‌گاه پروفایل‌های شخصی کاربر — نسخهٔ ۱٫۶ (درخواست کاربر ۴)
// ============================================================================
// پروفایل دلخواه (تنظیم‌شده در گام ۵) با نام ذخیره می‌شود و در اجراهای
// بعدی از همان مسیر بارگذاری می‌شود. توابع «خالص» با پارامتر دایرکتوری‌اند
// تا در cargo test آزمودنی باشند؛ پوستهٔ Tauri مسیر app_data را می‌دهد.
//
// قواعد:
//   * نام فایل از نام پروفایل «بهداشتی‌سازی» می‌شود (حروف/اعداد/-/_ فقط)؛
//   * هر پروفایل یک فایل JSON مستقل در <dir>/printer_profiles/؛
//   * status ذخیره‌شده همیشه «custom» است (صداقت: آزموده‌نشده)؛
//   * حذف/بازنویسی فقط فایل «همان نام» را لمس می‌کند.

/// بهداشتی‌سازی نام برای نام فایل — حروف فارسی/نمادها به «-»
pub fn sanitize_profile_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // فشرده‌سازی «--»های پشت‌سرهم و حذف «-» ابتدا/انتها
    let mut out = String::new();
    let mut last_dash = false;
    for c in cleaned.chars() {
        if c == '-' {
            if !last_dash {
                out.push(c);
            }
            last_dash = true;
        } else {
            out.push(c);
            last_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "my-printer".into()
    } else {
        out.chars().take(64).collect()
    }
}

/// پوشهٔ ذخیره‌گاه — ساخته‌شده در صورت نبود
fn profiles_dir(base_dir: &std::path::Path) -> std::path::PathBuf {
    let dir = base_dir.join("printer_profiles");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// ذخیرهٔ پروفایل شخصی — status به «custom» تنظیم می‌شود (صداقت)
pub fn save_custom_profile(
    base_dir: &std::path::Path,
    profile: &PrinterProfile,
) -> Result<String, String> {
    let name = if profile.name.trim().is_empty() {
        "my-printer".to_string()
    } else {
        profile.name.trim().to_string()
    };
    let mut p = profile.clone();
    p.name = name.clone();
    p.status = "custom".into(); // آزموده‌نشده — برچسب صادقانه
    let file = profiles_dir(base_dir).join(format!("{}.json", sanitize_profile_name(&name)));
    let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
    std::fs::write(&file, json.as_bytes())
        .map_err(|e| format!("نوشتن پروفایل ناموفق بود: {e}"))?;
    Ok(file.to_string_lossy().into_owned())
}

/// فهرست پروفایل‌های شخصی — نام‌های فایل مرتب (پایدار برای UI)
pub fn list_custom_profiles(base_dir: &std::path::Path) -> Vec<PrinterProfile> {
    let dir = profiles_dir(base_dir);
    let mut out: Vec<PrinterProfile> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut files: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();
    files.sort();
    for f in files {
        if let Ok(text) = std::fs::read_to_string(&f) {
            if let Ok(p) = serde_json::from_str::<PrinterProfile>(&text) {
                // پروفایل خراب نصفه‌شده هم رد نمی‌شود اگر فیلدهای اصلی دارد
                if p.build_volume_mm[0] > 0.0 && p.layer_height_mm > 0.0 {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// حذف پروفایل شخصی با نام
pub fn delete_custom_profile(base_dir: &std::path::Path, name: &str) -> Result<(), String> {
    let file = profiles_dir(base_dir).join(format!("{}.json", sanitize_profile_name(name)));
    match std::fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // حذفِ نبوده = موفق
        Err(e) => Err(format!("حذف پروفایل ناموفق بود: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exactly_twenty_presets() {
        // الزام کاربر: دقیقاً ۲۰ پرینتر واقعی
        assert_eq!(printer_presets().len(), 20, "تعداد پریست باید ۲۰ باشد");
    }

    #[test]
    fn test_names_unique() {
        let presets = printer_presets();
        let mut names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        for w in names.windows(2) {
            assert_ne!(w[0], w[1], "نام تکراری: {}", w[0]);
        }
    }

    #[test]
    fn test_no_preset_claims_verified() {
        // قاعدهٔ صداقت: هیچ پریستی verified نیست (تست دستی نداشتیم)
        for p in printer_presets() {
            assert_eq!(
                p.status, "community",
                "{} نباید verified باشد — ما آن را تست نکرده‌ایم",
                p.name
            );
        }
    }

    #[test]
    fn test_volumes_sane() {
        // ابعاد حجم ساخت باید در بازهٔ منطقی پرینترهای واقعی باشد
        for p in printer_presets() {
            for dim in p.build_volume_mm {
                assert!(
                    dim >= 180.0 && dim <= 400.0,
                    "{}: بعد غیرمنطقی {}",
                    p.name, dim
                );
            }
        }
    }

    #[test]
    fn test_all_params_sane() {
        // پارامترهای چاپی هر پریست باید منطقی و ایمن باشند
        for p in printer_presets() {
            assert!(p.nozzle_diameter_mm >= 0.3 && p.nozzle_diameter_mm <= 0.8, "{}", p.name);
            assert!((p.filament_diameter_mm - 1.75).abs() < 0.01, "{}", p.name);
            assert!(p.line_width_mm >= p.nozzle_diameter_mm, "{}", p.name);
            assert!(p.layer_height_mm > 0.1 && p.layer_height_mm <= 0.3, "{}", p.name);
            // سرعت‌ها «سرعت شروع vase» — نه بیشینهٔ دستگاه؛ بازهٔ ایمن
            assert!(p.print_speed_mm_min >= 1800.0 && p.print_speed_mm_min <= 3600.0, "{}", p.name);
            assert!(p.travel_speed_mm_min >= 3000.0 && p.travel_speed_mm_min <= 12000.0, "{}", p.name);
            assert!((p.flow_multiplier - 1.0).abs() < 1e-9, "{}", p.name);
        }
    }

    #[test]
    fn test_known_volumes_accurate() {
        // نمونه‌های شاخص — ارقام عمومی مستند
        let presets = printer_presets();
        let find = |n: &str| {
            presets
                .iter()
                .find(|p| p.name == n)
                .unwrap_or_else(|| panic!("پریست {n} پیدا نشد"))
        };
        assert_eq!(find("Bambu Lab A1 mini").build_volume_mm, [180.0, 180.0, 180.0]);
        assert_eq!(find("Bambu Lab X1 Carbon").build_volume_mm, [256.0, 256.0, 256.0]);
        assert_eq!(find("Prusa MK3S+").build_volume_mm, [250.0, 210.0, 210.0]);
        assert_eq!(find("Prusa XL").build_volume_mm, [360.0, 360.0, 360.0]);
        assert_eq!(find("Creality K1 Max").build_volume_mm, [300.0, 300.0, 300.0]);
        assert_eq!(find("Artillery Sidewinder X2").build_volume_mm, [300.0, 300.0, 400.0]);
        assert_eq!(find("Voron 2.4 (350)").build_volume_mm, [350.0, 350.0, 250.0]);
    }

    #[test]
    fn test_brand_fa_covers_all() {
        // هر پریست باید برند فارسی معتبر بگیرد
        for p in printer_presets() {
            assert_ne!(brand_fa(&p.name), "سایر", "{} برند ندارد", p.name);
        }
    }

    #[test]
    fn test_presets_fit_typical_vase() {
        // کوچک‌ترین پرینتر (A1 mini / MINI+ 180³) باید گلدان نمونه (⌀60×100) را جا دهد
        let presets = printer_presets();
        let smallest_z = presets.iter().map(|p| p.build_volume_mm[2]).fold(f64::MAX, f64::min);
        assert!(smallest_z >= 180.0);
        assert!(smallest_z > 100.0, "حجم Z حداقل باید گلدان ۱۰۰mm را جا دهد");
    }

    // ================= پروفایل شخصی — نسخهٔ ۱٫۶ =================

    #[test]
    fn test_custom_profile_roundtrip() {
        // ذخیره → فهرست → همان پروفایل (status=custom)
        use super::*;
        let dir = std::env::temp_dir().join(format!("gcs_prof_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut p = PrinterProfile::default();
        p.name = "پرینتر من ۲".into();
        p.build_volume_mm = [300.0, 300.0, 350.0];
        p.nozzle_temp_c = 235.0;
        p.start_gcode = "G28 ; home\nM104 S200".into();
        let saved = save_custom_profile(&dir, &p).unwrap();
        assert!(saved.contains("printer_profiles"));
        assert!(saved.ends_with(".json"));
        let list = list_custom_profiles(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "پرینتر من ۲");
        assert_eq!(list[0].status, "custom");
        assert_eq!(list[0].build_volume_mm, [300.0, 300.0, 350.0]);
        assert_eq!(list[0].start_gcode, "G28 ; home\nM104 S200");
        // بازنویسی همان نام → همچنان ۱ فایل
        save_custom_profile(&dir, &p).unwrap();
        assert_eq!(list_custom_profiles(&dir).len(), 1);
        // حذف
        delete_custom_profile(&dir, "پرینتر من ۲").unwrap();
        assert_eq!(list_custom_profiles(&dir).len(), 0);
        // حذف دوباره (نبوده) = موفق
        delete_custom_profile(&dir, "پرینتر من ۲").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sanitize_profile_name() {
        assert_eq!(sanitize_profile_name("My Printer X1"), "My-Printer-X1");
        assert_eq!(sanitize_profile_name("Ender//3??"), "Ender-3");
        assert_eq!(sanitize_profile_name("   "), "my-printer");
        assert_eq!(sanitize_profile_name("a__b"), "a__b");
        // فارسی کاملاً حذف می‌شود → نام جایگزین پیش‌فرض
        assert_eq!(sanitize_profile_name("سلام دوست"), "my-printer");
    }

    #[test]
    fn test_multiple_profiles_and_order() {
        use super::*;
        let dir = std::env::temp_dir().join(format!("gcs_prof_multi_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for name in ["Zeta", "Alpha", "Mid"] {
            let mut p = PrinterProfile::default();
            p.name = name.into();
            save_custom_profile(&dir, &p).unwrap();
        }
        let list = list_custom_profiles(&dir);
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Mid", "Zeta"]); // مرتب الفبایی فایل
        let _ = std::fs::remove_dir_all(&dir);
    }
}
