/**
 * ============================================================================
 * i18n/fa.ts — همهٔ رشته‌های فارسی رابط کاربری
 * ============================================================================
 * زبان پایهٔ برنامه فارسی است (الزام Docs/PROMPT_04 §5):
 * «زبان پایه فارسی، dir="rtl" روی shell؛ بخش کد، parameterها و مقادیر فنی
 *  در container با dir="ltr"»
 * ساختار برای افزودن انگلیسی در آینده آماده است (کلید→متن) ولی نسخهٔ
 * اول فقط فارسی دارد (الزام: «امکان ترجیح زبان را به ساختار محدود نکن»).
 * ============================================================================
 */

/** کلید رشته‌ها — تایپ‌دار برای جلوگیری از غلط املایی در کامپوننت‌ها */
export interface FaStrings {
  // ---------- پوسته و ناوبری ----------
  appTitle: string;
  navEditor: string;
  navDesigner: string;
  // ---------- نوار ابزار ----------
  openFile: string;
  saveAs: string;
  undo: string;
  redo: string;
  search: string;
  gotoLine: string;
  unsavedChanges: string;
  noFile: string;
  // ---------- ویرایشگر ----------
  codeColumn: string;
  viewColumn: string;
  lineCount: string;
  segmentCount: string;
  layerCount: string;
  spiralMode: string;
  // ---------- پنل توضیح خط ----------
  lineInspector: string;
  lineNo: string;
  rawText: string;
  lineType: string;
  commands: string;
  parameters: string;
  paramLetter: string;
  paramValue: string;
  paramMeaning: string;
  whatHappens: string;
  stateBefore: string;
  stateAfter: string;
  relatedSegments: string;
  warnings: string;
  editThisLine: string;
  applyEdit: string;
  cancelEdit: string;
  revertLine: string;
  noMovementLine: string;
  // ---------- شبیه‌سازی ----------
  simulation: string;
  play: string;
  pause: string;
  stepForward: string;
  stepBack: string;
  toStart: string;
  toEnd: string;
  speed: string;
  progress: string;
  layer: string;
  // ---------- رنگ‌بندی viewport ----------
  legendExtrusion: string;
  legendTravel: string;
  legendRetract: string;
  legendArc: string;
  legendUnknown: string;
  // ---------- فیلتر نمایش ----------
  showAll: string;
  showUpToLine: string;
  showOnlyLayer: string;
  showOnlyLine: string;
  // ---------- طراح مارپیچی ----------
  designerTitle: string;
  step1Profile: string;
  step2Pattern: string;
  step3Texture: string;
  step4Floor: string;
  step5Printer: string;
  height: string;
  bottomDiameter: string;
  topDiameter: string;
  taper: string;
  smoothing: string;
  addPoint: string;
  removePoint: string;
  presets: string;
  presetVase: string;
  presetLamp: string;
  presetPlanter: string;
  patternAdd: string;
  patternAmplitude: string;
  patternCount: string;
  patternFrequency: string;
  patternPhase: string;
  textureAdd: string;
  textureAmplitude: string;
  texturePeriod: string;
  textureDirection: string;
  textureRadial: string;
  textureVertical: string;
  fade: string;
  floorEnabled: string;
  floorHole: string;
  nozzleDiameter: string;
  lineWidth: string;
  layerHeight: string;
  printSpeed: string;
  filamentDiameter: string;
  flowMultiplier: string;
  absoluteE: string;
  relativeE: string;
  startGcode: string;
  endGcode: string;
  buildVolume: string;
  printerProfileStatus: string;
  // ---------- خروجی ----------
  exportGcode: string;
  exportStl: string;
  saveProject: string;
  loadProject: string;
  sendToEditor: string;
  exportSummary: string;
  exportExperimental: string;
  stlNote: string;
  // ---------- وضعیت‌ها ----------
  statusReady: string;
  statusParsing: string;
  statusUpdating: string;
  statusError: string;
  statusEmpty: string;
  openHint: string;
  webglMissing: string;
  // ---------- هشدارها و ایمنی ----------
  safetyNotice: string;
  checksumInvalid: string;
  confirmSave: string;
  oldPreview: string;
  // ---------- اعتبارسنجی ----------
  validationTitle: string;
  syntaxErrors: string;
  unknownCommands: string;
  checksumWarnings: string;
  boundsWarnings: string;
  noProblems: string;
}

/** رشته‌های فارسی — منبع واحد متن‌های UI */
export const fa: FaStrings = {
  // ---------- پوسته ----------
  appTitle: "G-code Studio — استودیوی ویرایش و طراحی G-code",
  navEditor: "ویرایشگر G-code",
  navDesigner: "طراح مارپیچی",

  // ---------- نوار ابزار ----------
  openFile: "بازکردن فایل",
  saveAs: "ذخیره با نام جدید",
  undo: "واگرد",
  redo: "ازنو",
  search: "جست‌وجو",
  gotoLine: "رفتن به خط",
  unsavedChanges: "تغییرات ذخیره‌نشده",
  noFile: "فایلی باز نشده است",

  // ---------- ویرایشگر ----------
  codeColumn: "کد",
  viewColumn: "نمای سه‌بعدی مسیر نازل",
  lineCount: "خطوط",
  segmentCount: "قطعه‌های مسیر",
  layerCount: "لایه‌ها",
  spiralMode: "حالت مارپیچی (لایهٔ گسسته ندارد)",

  // ---------- پنل توضیح ----------
  lineInspector: "توضیح خط انتخاب‌شده",
  lineNo: "خط",
  rawText: "متن خام",
  lineType: "نوع خط",
  commands: "فرمان‌ها",
  parameters: "پارامترها",
  paramLetter: "حرف",
  paramValue: "مقدار",
  paramMeaning: "معنا",
  whatHappens: "چه اتفاقی می‌افتد",
  stateBefore: "وضعیت قبل",
  stateAfter: "وضعیت بعد",
  relatedSegments: "قطعه‌های مرتبط",
  warnings: "هشدارها",
  editThisLine: "ویرایش همین خط",
  applyEdit: "اعمال",
  cancelEdit: "انصراف",
  revertLine: "بازگردانی متن قبلی",
  noMovementLine: "این خط حرکت هندسی ندارد؛ اثر آن در وضعیت ماشین (جدول پایین) دیده می‌شود.",

  // ---------- شبیه‌سازی ----------
  simulation: "شبیه‌سازی",
  play: "پخش",
  pause: "توقف",
  stepForward: "خط بعد",
  stepBack: "خط قبل",
  toStart: "ابتدای مسیر",
  toEnd: "انتهای مسیر",
  speed: "سرعت",
  progress: "پیشرفت",
  layer: "لایه",

  // ---------- legend ----------
  legendExtrusion: "چاپ (اکستروژن)",
  legendTravel: "حرکت خالی",
  legendRetract: "عقب‌کشیدن",
  legendArc: "قوس",
  legendUnknown: "نامطمئن",

  // ---------- فیلتر ----------
  showAll: "کل مسیر",
  showUpToLine: "تا این خط",
  showOnlyLayer: "فقط این لایه",
  showOnlyLine: "فقط این خط",

  // ---------- طراح ----------
  designerTitle: "طراح چاپ مارپیچی (vase-mode تک‌دیواره)",
  step1Profile: "۱. پروفایل مقطع",
  step2Pattern: "۲. الگوی هندسی",
  step3Texture: "۳. بافت مسیر",
  step4Floor: "۴. کف",
  step5Printer: "۵. پرینتر و خروجی",
  height: "ارتفاع (mm)",
  bottomDiameter: "قطر پایین (mm)",
  topDiameter: "قطر بالا (mm)",
  taper: "باریک‌شدن (%)",
  smoothing: "هموارسازی",
  addPoint: "افزودن نقطه",
  removePoint: "حذف نقطه",
  presets: "پروفایل‌های آماده",
  presetVase: "گلدان",
  presetLamp: "سایه‌روشن",
  presetPlanter: "گلدان کشت",
  patternAdd: "افزودن الگو",
  patternAmplitude: "دامنه (mm)",
  patternCount: "تعداد",
  patternFrequency: "فرکانس",
  patternPhase: "فاز (درجه)",
  textureAdd: "افزودن بافت",
  textureAmplitude: "دامنه (mm)",
  texturePeriod: "دوره (mm)",
  textureDirection: "جهت",
  textureRadial: "شعاعی",
  textureVertical: "عمودی",
  fade: "محو شدن",
  floorEnabled: "کف ساخته شود",
  floorHole: "قطر سوراخ مرکزی (mm)",
  nozzleDiameter: "قطر نازل (mm)",
  lineWidth: "عرض خط (mm)",
  layerHeight: "ارتفاع لایه (mm)",
  printSpeed: "سرعت چاپ (mm/min)",
  filamentDiameter: "قطر فیلامنت (mm)",
  flowMultiplier: "ضریب جریان",
  absoluteE: "E مطلق",
  relativeE: "E نسبی",
  startGcode: "کد شروع (start)",
  endGcode: "کد پایان (end)",
  buildVolume: "حجم ساخت X×Y×Z (mm)",
  printerProfileStatus: "وضعیت اعتبار پروفایل",

  // ---------- خروجی ----------
  exportGcode: "خروجی G-code",
  exportStl: "خروجی STL",
  saveProject: "ذخیرهٔ پروژه",
  loadProject: "بازکردن پروژه",
  sendToEditor: "ارسال به ویرایشگر G-code",
  exportSummary: "خلاصهٔ خروجی",
  exportExperimental: "آزمایشی — پیش از چاپ بازبینی کنید",
  stlNote: "STL فقط «فرم هندسی و الگوها» را ذخیره می‌کند؛ بافت‌های مسیر فقط در G-code می‌مانند.",

  // ---------- وضعیت ----------
  statusReady: "آماده",
  statusParsing: "در حال تفسیر…",
  statusUpdating: "در حال به‌روزرسانی پیش‌نمایش…",
  statusError: "خطا",
  statusEmpty: "خالی",
  openHint: "برای شروع، یک فایل G-code باز کنید یا از طراح مارپیچی خروجی بگیرید.",
  webglMissing: "WebGL در دسترس نیست؛ پیش‌نمایش سه‌بعدی غیرفعال است ولی ویرایش و توضیح خطوط کار می‌کند.",

  // ---------- ایمنی ----------
  safetyNotice:
    "این برنامه صحت چاپ فیزیکی را تضمین نمی‌کند. فایل را با ابزار/سازندهٔ معتبر بررسی کنید و اولین چاپ را زیر نظر داشته باشید.",
  checksumInvalid: "checksum نامعتبر",
  confirmSave: "ذخیره",
  oldPreview: "پیش‌نمایش قدیمی (خط نامعتبر)",

  // ---------- اعتبارسنجی ----------
  validationTitle: "خلاصهٔ اعتبارسنجی پیش از ذخیره",
  syntaxErrors: "خطاهای نحوی",
  unknownCommands: "فرمان‌های ناشناخته",
  checksumWarnings: "هشدارهای checksum",
  boundsWarnings: "خارج از حجم ساخت",
  noProblems: "مشکلی یافت نشد",
};

/** تاریخ‌گذاری دسترسی — رشته در کلیدها جست‌وجو می‌شود */
export function t(key: keyof FaStrings): string {
  return fa[key];
}
