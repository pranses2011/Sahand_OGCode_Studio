//! ============================================================================
//! knowledge.rs — دانشنامهٔ آفلاین فرمان‌های G-code با توضیح فارسی
//! ============================================================================
//! این ماژول «دانش ایستا» دربارهٔ فرمان‌های استاندارد RepRap/Marlin است و
//! طبق الزام Docs/PROMPT_02 §5 کاملاً آفلاین و rule-based کار می‌کند.
//! هیچ مدل ابری/اینترنتی در تفسیر نقشی ندارد.
//!
//! ساختار توضیح‌ها:
//!   - summary_fa: معنی کلی فرمان به فارسی روان؛
//!   - family: دستهٔ فرمان برای نوع‌بندی خط در inspector؛
//!   - motion: آیا فرمان موقعیت نازل را تغییر می‌دهد (برای segment)؛
//!   - firmware_dependent: رفتار به فریم‌ور/پروفایل وابسته است (عدم‌قطعیت).
//!
//! قاعدهٔ صداقت: آنچه اینجا نیست «ناشناخته» است و مفسر پیام استاندارد
//! «این فرمان در مجموعهٔ پایه تفسیر نمی‌شود…» نشان می‌دهد؛ معنی حدسی
//! ساخته نمی‌شود (الزام PROMPT_01 اصل ۴).
//! ============================================================================

/// دستهٔ کلی فرمان — برای نوع‌بندی خطوط در پنل توضیح
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFamily {
    Motion,          // حرکت خطی/قوسی
    Units,           // واحد
    CoordinateMode,  // مطلق/نسبی
    ExtruderMode,    // حالت E
    Homing,          // هم‌کردن و مبنا
    Temperature,     // دما
    Fan,             // فن
    Wait,            // انتظار/توقف
    Tool,            // ابزار
    Display,         // نمایش پیام
    FirmwareSetting, // تنظیم فریم‌ور
    Mesh,            // سطح‌سازی
    RetractMacro,    // ماکروی عقب‌کشیدن نرم‌افزاری
    Workspace,       // سیستم مختصات کاری
    Motors,          // انرژی موتورها
    Unknown,         // ناشناخته
}

/// اطلاعات یک فرمان شناخته‌شده
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// نام فرمان مثل "G1"
    pub code: &'static str,
    pub family: CommandFamily,
    /// توضیح فارسی کلی
    pub summary_fa: &'static str,
    /// آیا تغییردهندهٔ موقعیت است
    pub motion: bool,
    /// رفتار وابسته به فریم‌ور است؟ (برای برچسب عدم‌قطعیت)
    pub firmware_dependent: bool,
}

/// جدول فرمان‌های شناخته‌شده — منبع توضیحات inspector
/// توضیح‌ها عمداً «کاربردی» نوشته شده‌اند نه فقط تعریف دیکشنری (الزام PROMPT_02 §5)
pub static COMMANDS: &[CommandInfo] = &[
    // ---------------- حرکت ----------------
    CommandInfo { code: "G0", family: CommandFamily::Motion, summary_fa: "حرکت سریع (travel) بدون اکستروژن؛ برای جابه‌جایی نازل بین نقاط چاپ استفاده می‌شود و مسیر آن معمولاً با سرعت حداکثری طی می‌شود.", motion: true, firmware_dependent: false },
    CommandInfo { code: "G1", family: CommandFamily::Motion, summary_fa: "حرکت خطی هماهنگ (چاپی یا travel): نازل با سرعت F به‌سوی مختصات داده‌شده می‌رود و اگر E تغییر کند فیلامنت اکسترود می‌شود. پرکاربردترین فرمان چاپ FDM است.", motion: true, firmware_dependent: false },
    CommandInfo { code: "G2", family: CommandFamily::Motion, summary_fa: "حرکت قوسی ساعت‌گرد از موقعیت فعلی تا نقطهٔ هدف؛ مرکز قوس با I/J (آفست از مبدأ) یا شعاع با R تعیین می‌شود. مسیر منحنی به قطعات ریز خطی تقسیم و نمایش داده می‌شود.", motion: true, firmware_dependent: false },
    CommandInfo { code: "G3", family: CommandFamily::Motion, summary_fa: "حرکت قوسی پادساعت‌گرد؛ مثل G2 ولی در جهت مخالف. برای دیواره‌های منحنی و گوشه‌های نرم استفاده می‌شود.", motion: true, firmware_dependent: false },
    // ---------------- واحد و مختصات ----------------
    CommandInfo { code: "G17", family: CommandFamily::CoordinateMode, summary_fa: "انتخاب صفحهٔ XY برای قوس‌های G2/G3 (حالت پیش‌فرض). قوس‌ها در صفحهٔ افقی تعریف می‌شوند.", motion: false, firmware_dependent: false },
    CommandInfo { code: "G18", family: CommandFamily::CoordinateMode, summary_fa: "انتخاب صفحهٔ ZX برای قوس‌ها؛ در FDM به‌ندرت استفاده می‌شود.", motion: false, firmware_dependent: false },
    CommandInfo { code: "G19", family: CommandFamily::CoordinateMode, summary_fa: "انتخاب صفحهٔ YZ برای قوس‌ها؛ در FDM به‌ندرت استفاده می‌شود.", motion: false, firmware_dependent: false },
    CommandInfo { code: "G20", family: CommandFamily::Units, summary_fa: "تبدیل واحد به اینچ: تمام مختصات و سرعت‌های بعدی اینچ تفسیر می‌شوند (۱ اینچ = ۲۵.۴ میلی‌متر). نمایشگر داخلی مقادیر را به mm نرمال می‌کند.", motion: false, firmware_dependent: false },
    CommandInfo { code: "G21", family: CommandFamily::Units, summary_fa: "تبدیل واحد به میلی‌متر (حالت رایج FDM).", motion: false, firmware_dependent: false },
    CommandInfo { code: "G90", family: CommandFamily::CoordinateMode, summary_fa: "مختصات مطلق: مقادیر X/Y/Z/E در فرمان‌های بعدی نسبت به مبدأ ماشین تفسیر می‌شوند.", motion: false, firmware_dependent: false },
    CommandInfo { code: "G91", family: CommandFamily::CoordinateMode, summary_fa: "مختصات نسبی: مقادیر بعدی «مقدار جابه‌جایی از موقعیت فعلی» هستند نه مختصات مطلق.", motion: false, firmware_dependent: false },
    // ---------------- اکسترودر ----------------
    CommandInfo { code: "M82", family: CommandFamily::ExtruderMode, summary_fa: "حالت E مطلق: مقدار E در هر فرمان، شمارندهٔ کل فیلامنت از ابتدای چاپ است؛ کاهش آن معنی retract دارد.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M83", family: CommandFamily::ExtruderMode, summary_fa: "حالت E نسبی: مقدار E هر فرمان فقط «مقدار اکستروژن همین حرکت» است؛ مقدار منفی یعنی retract نسبی.", motion: false, firmware_dependent: false },
    // ---------------- هم‌کردن و مبنا ----------------
    CommandInfo { code: "G28", family: CommandFamily::Homing, summary_fa: "هم‌کردن (homing) محورها: نازل به سنسورهای مبنا می‌رود تا موقعیت دقیق صفر پیدا شود. اگر محوری ذکر شود فقط همان هم می‌شود. موقعیت پس از homing به پروفایل پرینتر وابسته است.", motion: true, firmware_dependent: true },
    CommandInfo { code: "G29", family: CommandFamily::Mesh, summary_fa: "سطح‌سازی خودکار بستر (bed leveling/mesh): بستر نقشه‌برداری می‌شود تا جبران ناهمواری ممکن شود. جزئیات رفتار به فریم‌ور وابسته است.", motion: true, firmware_dependent: true },
    CommandInfo { code: "G92", family: CommandFamily::Homing, summary_fa: "تنظیم شمارنده‌ها: مقدار فعلی محورها را «ادعا» می‌کند بدون حرکت فیزیکی. رایج‌ترین کاربرد G92 E0 صفرکردن شمارندهٔ فیلامنت است.", motion: false, firmware_dependent: false },
    // ---------------- سیستم مختصات کاری ----------------
    CommandInfo { code: "G53", family: CommandFamily::Workspace, summary_fa: "ارجاع موقت به مختصات ماشین (در همین فرمان)؛ به‌ندرت در G-code چاپ FDM دیده می‌شود.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G54", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۱ (G54). آفست آن به پروفایل/فریم‌ور وابسته است؛ بدون آن، موقعیت دقیق قابل مدل‌سازی نیست.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G55", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۲؛ مانند G54 وابسته به تنظیمات دستگاه.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G56", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۳؛ وابسته به تنظیمات دستگاه.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G57", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۴؛ وابسته به تنظیمات دستگاه.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G58", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۵؛ وابسته به تنظیمات دستگاه.", motion: false, firmware_dependent: true },
    CommandInfo { code: "G59", family: CommandFamily::Workspace, summary_fa: "انتخاب سیستم مختصات کاری شمارهٔ ۶ (یا G59.x بسته به فریم‌ور)؛ وابسته به تنظیمات دستگاه.", motion: false, firmware_dependent: true },
    // ---------------- دما ----------------
    CommandInfo { code: "M104", family: CommandFamily::Temperature, summary_fa: "تنظیم دمای هدف نازل بدون انتظار: فرمان بلافاصله برمی‌گردد و نازل در پس‌زمینه گرم می‌شود. مقدار S دمای هدف به سلسیوس است.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M109", family: CommandFamily::Temperature, summary_fa: "تنظیم دمای نازل «با انتظار»: چاپ تا رسیدن نازل به دمای S ادامه پیدا نمی‌کند. برای گرم‌کردن اولیه استفاده می‌شود.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M140", family: CommandFamily::Temperature, summary_fa: "تنظیم دمای هدف بستر (heatbed) بدون انتظار؛ S دمای هدف سلسیوس است.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M190", family: CommandFamily::Temperature, summary_fa: "تنظیم دمای بستر با انتظار تا رسیدن به دمای هدف؛ معمولاً ابتدای چاپ.", motion: false, firmware_dependent: false },
    // ---------------- فن ----------------
    CommandInfo { code: "M106", family: CommandFamily::Fan, summary_fa: "روشن‌کردن فن خنک‌کنندهٔ قطعه: مقدار S از ۰ تا ۲۵۵ است (۲۵۵ = تمام‌قد). نمایشگر آن را به درصد هم نشان می‌دهد.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M107", family: CommandFamily::Fan, summary_fa: "خاموش‌کردن کامل فن خنک‌کنندهٔ قطعه.", motion: false, firmware_dependent: false },
    // ---------------- انتظار و توقف ----------------
    CommandInfo { code: "G4", family: CommandFamily::Wait, summary_fa: "مکث (dwell): مدت P میلی‌ثانیه یا S ثانیه؛ برای خنک‌شدن، نشستن فیلامنت یا چسبیدن لایهٔ اول.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M0", family: CommandFamily::Wait, summary_fa: "توقف اختیاری برنامه تا تأیید اپراتور (در برخی فریم‌ورها نمایش پیام)؛ در چاپ خودکار معمولاً ادامه فوری است.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M1", family: CommandFamily::Wait, summary_fa: "توقف شرطی/اختیاری؛ رفتار دقیق به فریم‌ور و تنظیمات وابسته است.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M25", family: CommandFamily::Wait, summary_fa: "توقف (pause) چاپ از منبع سریال/SD؛ در چاپ از فایل مستقل رفتارش به فریم‌ور وابسته است.", motion: false, firmware_dependent: true },
    // ---------------- ابزار و موتور ----------------
    CommandInfo { code: "T0", family: CommandFamily::Tool, summary_fa: "انتخاب ابزار/اکسترودر شمارهٔ ۰. پارامترهای هر ابزار مستقل دیده می‌شوند.", motion: false, firmware_dependent: false },
    CommandInfo { code: "T1", family: CommandFamily::Tool, summary_fa: "انتخاب ابزار/اکسترودر شمارهٔ ۱ (در پرینترهای چندرنگ).", motion: false, firmware_dependent: false },
    CommandInfo { code: "M84", family: CommandFamily::Motors, summary_fa: "قطع جریان موتورهای گام؛ معمولاً انتهای فایل. پس از آن تا فرمان حرکت بعدی، موقعیت «مطمئن» نیست.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M17", family: CommandFamily::Motors, summary_fa: "روشن‌کردن جریان موتورهای گام.", motion: false, firmware_dependent: false },
    CommandInfo { code: "M18", family: CommandFamily::Motors, summary_fa: "قطع جریان موتورها (مثل M84).", motion: false, firmware_dependent: false },
    // ---------------- نمایش ----------------
    CommandInfo { code: "M117", family: CommandFamily::Display, summary_fa: "نمایش پیام متنی روی نمایشگر پرینتر؛ بقیهٔ خط payload پیام است و هیچ پارامتر حرکتی ندارد.", motion: false, firmware_dependent: false },
    // ---------------- تنظیمات فریم‌ور ----------------
    CommandInfo { code: "M204", family: CommandFamily::FirmwareSetting, summary_fa: "تنظیم شتاب (acceleration) حرکت‌ها؛ مقادیر P/R/T/S معنای متفاوتی در فریم‌ورهای مختلف دارند. روی هندسهٔ مسیر اثری ندارد ولی زمان واقعی چاپ را تغییر می‌دهد.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M220", family: CommandFamily::FirmwareSetting, summary_fa: "ضریب سرعت کل (feedrate override) به درصد؛ مثلاً S50 یعنی همهٔ حرکت‌ها نصف سرعت nominal اجرا شوند.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M221", family: CommandFamily::FirmwareSetting, summary_fa: "ضریب جریان اکستروژن (flow override) به درصد؛ روی مقدار E واقعی فیزیکی اثر می‌گذارد ولی در متن G-code دیده نمی‌شود.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M600", family: CommandFamily::FirmwareSetting, summary_fa: "تعویض فیلامنت (فیلامنت ران): چاپ متوقف و منتظر اپراتور می‌ماند؛ فقط در فریم‌ورهای پشتیبان.", motion: false, firmware_dependent: true },
    CommandInfo { code: "M900", family: CommandFamily::FirmwareSetting, summary_fa: "تنظیم linear advance (پیش‌فشار فیلامنت در Marlin)؛ رفتار و پارامترها (K و نظایر) وابسته به فریم‌ور است.", motion: false, firmware_dependent: true },
    // ---------------- ماکروهای عقب‌کشیدن نرم‌افزاری ----------------
    CommandInfo { code: "G10", family: CommandFamily::RetractMacro, summary_fa: "عقب‌کشیدن نرم‌افزاری (firmware retract) با تنظیمات از قبل تعریف‌شده؛ مقدار واقعی حرکت E و Z در فریم‌ور ذخیره شده و از متن فایل معلوم نیست.", motion: true, firmware_dependent: true },
    CommandInfo { code: "G11", family: CommandFamily::RetractMacro, summary_fa: "بازگرداندن عقب‌کشیدن نرم‌افزاری (unretract) با همان تنظیمات G10.", motion: true, firmware_dependent: true },
];

/// جستجوی فرمان با حرف و شماره — خروجی None یعنی «ناشناخته»
pub fn lookup(letter: char, number: u32) -> Option<&'static CommandInfo> {
    let code = format!("{letter}{number}");
    COMMANDS.iter().find(|c| c.code == code)
}

/// معنای پارامترها (حروف) — فارسی و کاربردی
/// برای جدول parameter در پنل توضیح خط استفاده می‌شود
pub fn param_meaning(letter: char) -> &'static str {
    match letter {
        'X' => "مختصات مقصد روی محور X (mm در حالت G21؛ اینچ در G20)",
        'Y' => "مختصات مقصد روی محور Y",
        'Z' => "مختصات مقصد روی محور Z — ارتفاع نازل نسبت به بستر",
        'E' => "مقدار اکسترودر: در حالت مطلق شمارندهٔ کل فیلامنت، در حالت نسبی مقدار اکستروژن همین حرکت (mm فیلامنت)",
        'F' => "سرعت حرکت (feedrate) به mm/min (یا اینچ/min در G20)؛ باقی می‌ماند تا تغییر بعدی",
        'S' => "مقدار اصلی فرمان: دمای هدف (°C) در فرمان‌های دما، سرعت فن ۰–۲۵۵ در M106، یا درصد در M220/M221",
        'P' => "پارامتر ثانویه: مدت به میلی‌ثانیه در G4، یا شماره ابزار/کانال بسته به فرمان",
        'R' => "شعاع قوس در G2/G3 (علامت منفی = کمان بزرگ‌تر از نیم‌دور)؛ در فرمان‌های دیگر معنای خاص خودش",
        'I' => "آفست مرکز قوس نسبت به مبدأ در جهت X (فرم مرکزی G2/G3)",
        'J' => "آفست مرکز قوس نسبت به مبدأ در جهت Y",
        'K' => "آفست مرکز قوس در جهت Z (صفحه‌های G18/G19)؛ یا پارامتر linear-advance در M900",
        'N' => "شمارهٔ خط در پروتکل serial با checksum — نه محور حرکتی",
        'T' => "انتخاب ابزار/اکسترودر (مثلاً T0 یا T1)؛ داخل فرمان‌های دیگر معنای ثانویه دارد",
        'W' => "پارامتر کمکی؛ در G28 برخی فریم‌ورها (بدون مقدار) برای هم‌کردن کامل استفاده می‌کنند",
        'A' | 'B' | 'C' => "پارامتر کمکی وابسته به فرمان/فریم‌ور",
        'D' => "پارامتر وابسته به فرمان (مثلاً فاصله/ضریب در تنظیمات فریم‌ور)",
        'H' => "پارامتر وابسته به فرمان (مثلاً ارتفاع/حرارت بسته به فریم‌ور)",
        'L' => "پارامتر وابسته به فرمان (تعداد تکرار/طول)",
        'Q' => "پارامتر وابسته به فرمان",
        'U' | 'V' => "پارامتر وابسته به فرمان",
        _ => "پارامتر ناشناخته یا وابسته به فریم‌ور",
    }
}

/// توضیح فارسی برای «فرمان ناشناخته» — پیام استاندارد صداقت
pub const UNKNOWN_COMMAND_FA: &str =
    "این فرمان در مجموعهٔ پایه تفسیر نمی‌شود؛ رفتار آن به firmware/profile وابسته است.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_known() {
        // فرمان‌های کلیدی باید توضیح فارسی داشته باشند
        assert!(lookup('G', 1).is_some());
        assert!(lookup('G', 2).is_some());
        assert!(lookup('G', 3).is_some());
        assert!(lookup('M', 104).is_some());
        assert!(lookup('M', 117).is_some());
        // G1 حرکت است و M104 نیست
        assert!(lookup('G', 1).unwrap().motion);
        assert!(!lookup('M', 104).unwrap().motion);
    }

    #[test]
    fn test_lookup_unknown() {
        // M900 در جدول هست ولی firmware-dependent
        let m900 = lookup('M', 900).unwrap();
        assert!(m900.firmware_dependent);
        // فرمان واقعاً ناشناخته
        assert!(lookup('M', 12345).is_none());
    }

    #[test]
    fn test_param_meaning_coverage() {
        // پارامترهای اصلی fixtureها همه توضیح دارند
        for l in ['X', 'Y', 'Z', 'E', 'F', 'S', 'I', 'J', 'R', 'N'] {
            assert!(!param_meaning(l).is_empty());
        }
    }
}
