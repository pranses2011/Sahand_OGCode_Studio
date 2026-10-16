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
  // ---------- نسخهٔ ۱.۲: طراح گسترده‌شده ----------
  interpolationMode: string;
  interpolationSpline: string;
  interpolationLinear: string;
  interpolationHint: string;
  centerOnBed: string;
  centerOnBedHint: string;
  floorPattern: string;
  floorSpokes: string;
  floorGap: string;
  // ---------- نسخهٔ ۱.۴: کف چندلایه + سرعت‌ها + اسپات سطحی ----------
  floorLayers: string;
  floorLayersHint: string;
  speedsTitle: string;
  speedsHint: string;
  wallSpeed: string;
  firstLayerSpeed: string;
  floorSpeed: string;
  travelSpeed: string;
  retractLength: string;
  retractSpeed: string;
  retractHint: string;
  spotHint: string;
  spotPatternRange: string;
  spotTextureFade: string;
  spotFloorHole: string;
  designerUndo: string;
  designerRedo: string;
  // ---------- نسخهٔ ۱.۳: چرخش فاز θ + تنظیمات حرارتی ----------
  phaseRotate: string;
  phaseRotateHint: string;
  phaseRotateSeam: string;
  spotPhaseRotate: string;
  tempsTitle: string;
  materialPresets: string;
  materialHint: string;
  nozzleTemp: string;
  bedTemp: string;
  fanSpeed: string;
  waitTemps: string;
  waitTempsHint: string;
  matPla: string;
  matPetg: string;
  matAbs: string;
  matTpu: string;
  matSilk: string;
  tempsZeroNote: string;
  stylePresets: string;
  stylePresetHint: string;
  cardAdd: string;
  cardActive: string;
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
  printerPresets: string;
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

  // ---------- نسخهٔ ۱.۲: طراح گسترده‌شده ----------
  interpolationMode: "درون‌یابی پروفایل",
  interpolationSpline: "اسپلاین نرم (بدون لبهٔ تیز)",
  interpolationLinear: "خط مستقیم",
  interpolationHint:
    "اسپلاین مونوتون از نقاط دقیقاً می‌گذرد ولی گوشه‌ها را نرم می‌کند (بدون فراجهش).",
  centerOnBed: "مرکز‌گذاری روی صفحهٔ چاپ",
  centerOnBedHint:
    "طرح در مرکز صفحه (نصف ابعاد) می‌نشیند نه روی گوشهٔ (0,0) — برای چاپ واقعی لازم است.",
  floorPattern: "الگوی چاپ کف",
  floorSpokes: "تعداد پرتوها",
  floorGap: "فاصلهٔ خطوط کف (mm)",
  // ---------- نسخهٔ ۱.۴ ----------
  floorLayers: "تعداد لایه‌های کف",
  floorLayersHint:
    "کف با این تعداد لایهٔ توپر چاپ می‌شود و سپس حالت مارپیچی (vase) از بالای کف ادامه می‌یابد — ۲ تا ۳ لایه برای ظرف معمولی کافی است.",
  speedsTitle: "سرعت‌ها (mm/s)",
  speedsHint:
    "سرعت‌ها به میلی‌متر بر ثانیه وارد می‌شوند؛ در G-code استاندارد به mm/min (F) تبدیل می‌شوند. مقادیر صفر یعنی پیروی از پیش‌فرض‌های اسلایسر (لایهٔ اول ۵۰٪ و کف ۷۵٪ سرعت دیواره).",
  wallSpeed: "سرعت دیواره/مارپیچ (mm/s)",
  firstLayerSpeed: "سرعت لایهٔ اول کف (mm/s)",
  floorSpeed: "سرعت لایه‌های کف (mm/s)",
  travelSpeed: "سرعت جابجایی (mm/s)",
  retractLength: "طول رتراکشن (mm)",
  retractSpeed: "سرعت رتراکشن (mm/s)",
  retractHint:
    "۰ = بدون رتراکشن. فقط روی جابجایی‌های کف اعمال می‌شود؛ مسیر پیوستهٔ vase رتراکشن نمی‌گیرد. برای اکسترودر Bowden مقدار بزرگ‌تر (۴ تا ۷mm) لازم است.",
  spotHint:
    "دستهٔ روی فرم را بکشید: عمودی = جابه‌جایی ارتفاع، افقی = چرخش زاویهٔ الگو (Shift = چفت ۱۵°)",
  spotPatternRange: "جابه‌جایی بازهٔ الگو",
  spotTextureFade: "جابه‌جایی ناحیهٔ بافت",
  spotFloorHole: "تغییر قطر سوراخ کف",
  designerUndo: "واگرد",
  designerRedo: "ازنو",
  // ---------- نسخهٔ ۱.۳: چرخش فاز θ + دما ----------
  phaseRotate: "چرخش فرم θ (درجه)",
  phaseRotateHint:
    "زاویهٔ شروع مارپیچ حول محور — جای درز (seam) و جهت گلبرگ/شیارها را می‌چرخاند.",
  phaseRotateSeam: "درز چاپ به سمت انتخابی شما می‌افتد — برای پنهان‌کردن درز، سمت پشت مدل را انتخاب کنید",
  spotPhaseRotate: "چرخش فاز θ با کشیدن",
  tempsTitle: "تنظیمات دما و خنک‌کاری",
  materialPresets: "ماده (پریست دما)",
  materialHint:
    "مقادیر «شروع مناسب رایج» هستند نه عدد قطعی — به برند/رنگ فیلامنت و تنظیمات سازنده شما وابسته‌اند؛ قبل از چاپ نهایی تست بگیرید.",
  nozzleTemp: "دمای نازل (°C)",
  bedTemp: "دمای میز کار/بستر (°C)",
  fanSpeed: "فن خنک‌کنندهٔ قطعه (٪)",
  waitTemps: "منتظر رسیدن دما بمان (M190/M109)",
  waitTempsHint:
    "روشن: چاپ فقط بعد از رسیدن نازل/بستر به دمای هدف شروع می‌شود (توصیه‌شده).",
  matPla: "PLA — 200/60",
  matPetg: "PETG — 240/80",
  matAbs: "ABS — 250/100",
  matTpu: "TPU — 225/45",
  matSilk: "PLA ابریشمی — 215/65",
  tempsZeroNote: "دمای ۰ یعنی فرمان دما در G-code نوشته نمی‌شود (کنترل دستی پرینتر).",
  stylePresets: "استایل‌های آماده",
  stylePresetHint: "ترکیب آمادهٔ پروفایل + الگو + بافت — بعد از اعمال کاملاً قابل‌ویرایش است",
  cardAdd: "افزودن",
  cardActive: "فعال",
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
  printerPresets: "پرینتر آماده (۲۰ مدل واقعی)",

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
