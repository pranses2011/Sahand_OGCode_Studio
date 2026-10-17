/**
 * ============================================================================
 * features/spiral-designer/gcodeCommands.ts — کتابخانهٔ دستورات شروع/پایان
 * ============================================================================
 * درخواست کاربر ۳ (نسخهٔ ۱٫۶):
 *   «در تب پرینتر و خروجی برای هر کدام از فیلدهای کد شروع و کد پایان یک
 *    لیست از دستورات مرتبط همراه با توضیح فارسی آن کد باشد و بتونیم از
 *    لیست به بخش دستورات شروع یا پایان اضافه کنیم»
 *
 * هر دستور: کد + توضیح فارسی + قطعهٔ آمادهٔ درج (با {پارامتر} قابل‌ویرایش)
 * + برچسب «شروع» / «پایان» / «هر دو» + دسته. فهرست بر پایهٔ مجموعهٔ
 * Marlin/Prusa (رایج‌ترین فریم‌ورها) و مستندات RepRap است.
 * ============================================================================
 */

export type CommandSlot = "start" | "end" | "both";

export interface GcodeCommandEntry {
  /** کد اصلی (نمایش) */
  code: string;
  /** نام کوتاه فارسی */
  name: string;
  /** توضیح فارسی کامل */
  desc: string;
  /** کجا مناسب است: شروع / پایان / هر دو */
  slot: CommandSlot;
  /** قطعهٔ آمادهٔ درج — {نام} = جای‌نگهدار قابل‌ویرایش */
  snippet: string;
  /** دستهٔ نمایش */
  category: string;
}

/** دسته‌های نمایش */
export const COMMAND_CATEGORIES = [
  "آماده‌سازی",
  "دما و فن",
  "حرکت و هوم",
  "اکستروژن",
  "پاک‌سازی و نگهداری",
  "پایان چاپ",
] as const;

export const GCODE_COMMANDS: GcodeCommandEntry[] = [
  // ================= آماده‌سازی =================
  {
    code: "G21",
    name: "میلی‌متر",
    desc: "واحد مختصات را روی میلی‌متر می‌گذارد (استاندارد؛ همراه خروجی برنامه خودکار می‌آید).",
    slot: "start",
    snippet: "G21 ; millimeters",
    category: "آماده‌سازی",
  },
  {
    code: "G90",
    name: "مختصات مطلق",
    desc: "همهٔ حرکت‌ها با مختصات مطلق تفسیر می‌شوند (استاندارد اسلایسرها).",
    slot: "both",
    snippet: "G90 ; absolute XYZ",
    category: "آماده‌سازی",
  },
  {
    code: "G91",
    name: "مختصات نسبی",
    desc: "حرکت‌ها نسبت به موقعیت فعلی تفسیر می‌شوند — برای جابه‌جایی‌های کوچک مثل بالارفتن پایانی.",
    slot: "both",
    snippet: "G91 ; relative mode",
    category: "آماده‌سازی",
  },
  {
    code: "M82",
    name: "E مطلق",
    desc: "موقعیت اکسترودر مطلق است (M82 — پیش‌فرض خروجی برنامه).",
    slot: "start",
    snippet: "M82 ; absolute E",
    category: "آماده‌سازی",
  },
  {
    code: "M83",
    name: "E نسبی",
    desc: "موقعیت اکسترودر نسبی است (M83) — هر خط E مستقل تفسیر می‌شود.",
    slot: "start",
    snippet: "M83 ; relative E",
    category: "آماده‌سازی",
  },
  {
    code: "G92 E0",
    name: "صفر کردن E",
    desc: "موقعیت اکسترودر را صفر می‌کند؛ معمولاً پس از کد شروع و پیش از چاپ.",
    slot: "start",
    snippet: "G92 E0 ; reset extruder",
    category: "آماده‌سازی",
  },
  {
    code: "M117",
    name: "پیام روی نمایشگر",
    desc: "متن را روی نمایشگر LCD پرینتر نشان می‌دهد — مثلاً «در حال گرم‌شدن…».",
    slot: "both",
    snippet: "M117 گرم شدن نازل...",
    category: "آماده‌سازی",
  },
  {
    code: "M75",
    name: "شروع تایمر چاپ",
    desc: "تایمر چاپ روی نمایشگر را آغاز می‌کند (Prusa/Marlin).",
    slot: "start",
    snippet: "M75 ; start print timer",
    category: "آماده‌سازی",
  },

  // ================= دما و فن =================
  {
    code: "M104 S{t}",
    name: "دمای نازل (تنظیم)",
    desc: "دمای هدف نازل را تنظیم می‌کند بدون انتظار — گرم‌شدن در پس‌زمینه.",
    slot: "start",
    snippet: "M104 S200 ; set nozzle target",
    category: "دما و فن",
  },
  {
    code: "M109 S{t}",
    name: "دمای نازل (انتظار)",
    desc: "دمای نازل را تنظیم و تا رسیدن به هدف منتظر می‌ماند — پیش از اکستروژن.",
    slot: "start",
    snippet: "M109 S200 ; wait for nozzle",
    category: "دما و فن",
  },
  {
    code: "M140 S{t}",
    name: "دمای میز (تنظیم)",
    desc: "دمای هدف بستر/میز را تنظیم می‌کند بدون انتظار.",
    slot: "start",
    snippet: "M140 S60 ; set bed target",
    category: "دما و فن",
  },
  {
    code: "M190 S{t}",
    name: "دمای میز (انتظار)",
    desc: "دمای میز را تنظیم و منتظر می‌ماند — برای چسبندگی لایهٔ اول ضروری.",
    slot: "start",
    snippet: "M190 S60 ; wait for bed",
    category: "دما و فن",
  },
  {
    code: "M106 S{0..255}",
    name: "روشن کردن فن",
    desc: "فن خنک‌کنندهٔ قطعه را با سرعت مشخص (S0=S خاموش تا S255=۱۰۰٪) روشن می‌کند.",
    slot: "start",
    snippet: "M106 S255 ; fan 100%",
    category: "دما و فن",
  },
  {
    code: "M107",
    name: "خاموش کردن فن",
    desc: "فن خنک‌کنندهٔ قطعه را خاموش می‌کند.",
    slot: "end",
    snippet: "M107 ; fan off",
    category: "دما و فن",
  },
  {
    code: "M104 S0",
    name: "خاموشی نازل",
    desc: "هیتر نازل را خاموش می‌کند — در پایان چاپ (خروجی برنامه خودکار می‌گذارد).",
    slot: "end",
    snippet: "M104 S0 ; nozzle off",
    category: "پایان چاپ",
  },
  {
    code: "M140 S0",
    name: "خاموشی میز",
    desc: "هیتر بستر را خاموش می‌کند — در پایان چاپ.",
    slot: "end",
    snippet: "M140 S0 ; bed off",
    category: "پایان چاپ",
  },
  {
    code: "M104 S0 + M140 S0 + M107",
    name: "خاموشی کامل حرارتی",
    desc: "نازل + میز + فن با هم خاموش می‌شوند — بستهٔ استاندارد پایان چاپ.",
    slot: "end",
    snippet: "M140 S0 ; bed off\nM104 S0 ; nozzle off\nM107 ; fan off",
    category: "پایان چاپ",
  },

  // ================= حرکت و هوم =================
  {
    code: "G28",
    name: "هوم همهٔ محورها",
    desc: "همهٔ محورها را هوم می‌کند — پیش از چاپ ضروری است (برنامه اگر نبود خودش می‌گذارد).",
    slot: "start",
    snippet: "G28 ; home all axes",
    category: "حرکت و هوم",
  },
  {
    code: "G28 X",
    name: "هوم فقط X",
    desc: "فقط محور X را هوم می‌کند — Z و Y دست‌نخورده می‌مانند.",
    slot: "end",
    snippet: "G28 X ; home X only",
    category: "حرکت و هوم",
  },
  {
    code: "G28 Z",
    name: "هوم فقط Z",
    desc: "فقط محور Z را هوم می‌کند — برای کالیبراسیون ارتفاع.",
    slot: "start",
    snippet: "G28 Z ; home Z only",
    category: "حرکت و هوم",
  },
  {
    code: "G1 Z{z} F{f}",
    name: "بالارفتن/پایین‌آمدن Z",
    desc: "نازل را در محور Z جابه‌جا می‌کند — مثلاً ۵mm بالارفتن پیش از شروع یا در پایان.",
    slot: "both",
    snippet: "G1 Z5 F600 ; lift 5mm",
    category: "حرکت و هوم",
  },
  {
    code: "G0 X{x} Y{y} F{f}",
    name: "حرکت سریع",
    desc: "حرکت خالی (بدون اکستروژن) به مختصات مشخص — پارک‌کردن نازل یا جابه‌جایی.",
    slot: "both",
    snippet: "G0 X110 Y110 F6000 ; park center",
    category: "حرکت و هوم",
  },
  {
    code: "G4 P{ms}",
    name: "توقف زمانی",
    desc: "برنامه به‌مدت میلی‌ثانیه توقف می‌کند — برای تثبیت دما یا چک کردن چسبندگی.",
    slot: "both",
    snippet: "G4 P2000 ; wait 2s",
    category: "حرکت و هوم",
  },
  {
    code: "M84",
    name: "خاموشی موتورها",
    desc: "موتورهای پله‌ای را آزاد/خاموش می‌کند — پایان استاندارد چاپ.",
    slot: "end",
    snippet: "M84 ; motors off",
    category: "پایان چاپ",
  },
  {
    code: "M18",
    name: "خاموشی موتورها (بدون توقف)",
    desc: "همان M84 — موتورها را آزاد می‌کند (نام قدیمی RepRap).",
    slot: "end",
    snippet: "M18 ; motors off",
    category: "پایان چاپ",
  },

  // ================= اکستروژن =================
  {
    code: "G1 E{e} F{f}",
    name: "رتراکشن",
    desc: "فیلامنت را عقب می‌کشد (E منفی) — جلوگیری از ریسه در حرکت‌های خالی.",
    slot: "both",
    snippet: "G1 E-2 F2400 ; retract 2mm",
    category: "اکستروژن",
  },
  {
    code: "G1 E{+e} F{f}",
    name: "آنرتراکشن",
    desc: "فیلامنت عقب‌کشیده‌شده را برمی‌گرداند — پیش از ادامهٔ چاپ.",
    slot: "both",
    snippet: "G1 E2 F2400 ; unretract",
    category: "اکستروژن",
  },
  {
    code: "G1 E{e} F{f}",
    name: "خروج فیلامنت اضافی",
    desc: "چند میلی‌متر فیلامنت را بیرون می‌راند تا نازل پر شود (پورج پیش از شروع).",
    slot: "start",
    snippet: "G1 E3 F300 ; purge 3mm",
    category: "اکستروژن",
  },
  {
    code: "M302",
    name: "اکستروژن سرد",
    desc: "کنترل اکستروژن در دمای سرد: S1 = اجازهٔ اکستروژن سرد (برای تعویض فیلامنت).",
    slot: "start",
    snippet: "M302 S1 ; allow cold extrusion",
    category: "اکستروژن",
  },
  {
    code: "M701",
    name: "بارگذاری فیلامنت",
    desc: "مراحل بارگذاری فیلامنت (Marlin — با منوی پرینتر هم‌اهنگ).",
    slot: "start",
    snippet: "M701 ; load filament",
    category: "اکستروژن",
  },
  {
    code: "M702",
    name: "تخلیهٔ فیلامنت",
    desc: "فیلامنت را کامل بیرون می‌کشد — برای تعویض رنگ/شروع تمیز.",
    slot: "both",
    snippet: "M702 ; unload filament",
    category: "اکستروژن",
  },
  {
    code: "M600",
    name: "تعویض فیلامنت",
    desc: "چاپ متوقف و فیلامنت تعویض می‌شود — برای چندرنگی دستی.",
    slot: "start",
    snippet: "M600 ; filament change",
    category: "اکستروژن",
  },

  // ================= پاک‌سازی و نگهداری =================
  {
    code: "G1 X{x} Y{y} Z{z}",
    name: "خط پاک‌سازی",
    desc: "چند خط اکستروژن در لبهٔ میز قبل از شروع — نازل تمیز و جریان پایدار.",
    slot: "start",
    snippet:
      "G0 X20 Y10 Z0.3 F6000\nG1 X200 Y10 E10 F1500 ; purge line\nG92 E0",
    category: "پاک‌سازی و نگهداری",
  },
  {
    code: "G1 X{x} Y{y} Z{z} F{f}",
    name: "پارک‌کردن نازل",
    desc: "نازل را به نقطهٔ امن می‌برد (بالای میز) — پایان چاپ یا توقف.",
    slot: "end",
    snippet: "G0 X110 Y200 Z50 F6000 ; park at back",
    category: "پاک‌سازی و نگهداری",
  },
  {
    code: "M420 S1",
    name: "فعال‌سازی Mesh میز",
    desc: "جدول هم‌ترازی مش (ABL) فعال می‌شود — اگر میز کالیبره شده باشد.",
    slot: "start",
    snippet: "M420 S1 ; enable bed mesh",
    category: "پاک‌سازی و نگهداری",
  },
  {
    code: "G29",
    name: "هم‌ترازی خودکار میز",
    desc: "نقشهٔ هم‌ترازی میز را از نو می‌سازد (ABL) — قبل از چاپ؛ کند است.",
    slot: "start",
    snippet: "G29 ; auto bed level",
    category: "پاک‌سازی و نگهداری",
  },
  {
    code: "M500",
    name: "ذخیرهٔ تنظیمات در EEPROM",
    desc: "تنظیمات جاری را در حافظهٔ دائمی پرینتر ذخیره می‌کند.",
    slot: "end",
    snippet: "M500 ; save settings",
    category: "پاک‌سازی و نگهداری",
  },
  {
    code: "M486",
    name: "کنترل قطعه (Cuttlefish)",
    desc: "شمارهٔ قطعهٔ جاری — مکث/ادامهٔ قطعه‌محور (فریم‌ورهای خاص).",
    slot: "start",
    snippet: "M486 S0 ; object 0",
    category: "پاک‌سازی و نگهداری",
  },

  // ================= پایان چاپ =================
  {
    code: "بستهٔ پایان استاندارد",
    name: "پایان کامل استاندارد",
    desc: "رتراکشن + خاموشی بستر/نازل/فن + بالارفتن ۱mm + هوم X بدون پایین‌آمدن + خاموشی موتورها.",
    slot: "end",
    snippet:
      "G1 E-2 F2400 ; retract\nM140 S0 ; bed off\nM104 S0 ; nozzle off\nM107 ; fan off\nG91 ; relative\nG1 Z1 F600 ; lift 1mm\nG90 ; absolute\nG28 X ; home X only\nM84 ; motors off",
    category: "پایان چاپ",
  },
  {
    code: "M117",
    name: "پیام پایان",
    desc: "روی نمایشگر نشان می‌دهد چاپ تمام شده است.",
    slot: "end",
    snippet: "M117 چاپ تمام شد",
    category: "پایان چاپ",
  },
  {
    code: "M77",
    name: "توقف تایمر چاپ",
    desc: "تایمر چاپ نمایشگر را متوقف می‌کند (Prusa/Marlin).",
    slot: "end",
    snippet: "M77 ; stop print timer",
    category: "پایان چاپ",
  },
];

/** جست‌وجوی دستور — نام کد/نام/توضیح (فارسی حروف بزرگ/کوچک بی‌اثر) */
export function filterCommands(query: string, slot: CommandSlot): GcodeCommandEntry[] {
  const q = query.trim().toLowerCase();
  return GCODE_COMMANDS.filter((c) => {
    // مناسب اسلات: start فقط شروع‌ها + bothها؛ end فقط پایان‌ها + bothها
    if (c.slot !== slot && c.slot !== "both") return false;
    if (!q) return true;
    return (
      c.code.toLowerCase().includes(q) ||
      c.name.toLowerCase().includes(q) ||
      c.desc.toLowerCase().includes(q)
    );
  });
}
