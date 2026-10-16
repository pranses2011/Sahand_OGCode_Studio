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
}
