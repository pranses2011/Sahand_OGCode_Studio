# 📋 گزارش نهایی تحویل — G-code Studio

> **تاریخ:** ۲۰۲۶-۱۰-۱۶ | **قالب:** طبق `Docs/PROMPT_05_QA_SECURITY_DELIVERY.md` §۶

---

## ۱. وضعیت: **کامل با محدودیت**

کل الزامات دو ماژول پیاده‌سازی و آزمون شده است. محدودیت‌های باقی‌مانده صادقانه در بخش ۷ فهرست شده‌اند.

## ۲. قابلیت‌های ساخته‌شده

### ماژول ۱ — ویرایشگر و تحلیل‌گر G-code ✅

- بازکردن فایل محلی با دیالوگ native (پسوندهای `.gcode/.gco/.g/.gc/.ngc/txt`)
- لغزندهٔ کامل: فرمت فاصله‌دار/فشرده/`N...*checksum` (XOR استاندارد RepRap)؛ کامنت `;` و `()` بدون اجرا؛ payload متنی `M117`؛ ردّ NaN/Infinity با حفظ متن خام
- مفسر modal-state کامل: G0–G4، G17–G21، G28 (موقعیت نامعلوم بدون پروفایل)، G29، G53–G59، G90/G91، G92، M0–M25، M82/M83، M104–M221، T؛ E مطلق/نسبی؛ تفکیک چاپ/travel/retract/unretract
- قوس G2/G3: هر سه صفحهٔ G17/G18/G19، فرم I/J/K و R (علامت کمان کوچک/بزرگ)، قوس کامل، مارپیچ محور سوم، خطاهای شعاع نامعتبر با Diagnostic فارسی
- بازسازی سه‌بعدی مسیر نازل (Three.js): BufferGeometry واحد، رنگ per-vertex، orbit/pan/zoom/fit، grid/محورها، build volume فقط با پروفایل
- همگام‌سازی دوسویهٔ خط↔segment (کلیک خط → برجسته‌سازی؛ کلیک مسیر → انتخاب/اسکرول خط)
- پنل توضیح هر ۹ بخش الزام: متن خام/فرمان‌ها/جدول پارامتر/شرح عمل/نوع خط/وضعیت قبل-بعد/segmentهای مرتبط/هشدارها/ویرایش همین خط
- ویرایش زنده با debounce ۴۰۰ms + undo/redo + dirty state؛ خط نامعتبر متن را حذف نمی‌کند و preview قدیمی برچسب می‌خورد
- شبیه‌سازی خط‌به‌خط با فیلتر «تا این خط»/«فقط این خط»
- Save As: نام پیش‌فرض `*-edited.gcode`، بازتولید BOM/پایان‌خط مبدأ، اعتبارسنجی پیش از ذخیره، سه سیاست checksum (بازمحاسبه/حفظ/حذف)
- تشخیص حالت مارپیچی (لایهٔ گسسته ساخته نمی‌شود)

### ماژول ۲ — طراح مارپیچی ✅

- گام ۱ پروفایل: ادیتور SVG تعاملی (کشیدن/افزودن/حذف نقاط)، سه preset قابل‌ویرایش، ارتفاع/قطر پایین-بالا/taper
- گام ۲ الگوهای هندسی (در STL می‌مانند): موج/گلبرگ/پیچش/شیار — composable با clamp شعاع منفی و بازهٔ z با falloff
- گام ۳ بافت‌های مسیر (فقط G-code): موجک/تور بافته/بخیه‌ای — شعاعی/عمودی، fade-in/out، تشخیص برخورد
- گام ۴ کف: بدون کف / هم‌مرکز با سوراخ مرکزی قابل‌تنظیم
- گام ۵ پرینتر: نازل 0.4/0.6/0.8+سفارشی، line width/layer height/سرعت/فیلامنت/ضریب جریان/E مطلق-نسبی/build volume/start-end کد؛ preset پیش‌فرض صادقانه `unverified`
- زنجیرهٔ منبع واحد: پروفایل→الگو→mesh(STL) و مسیر→بافت→**Toolpath IR واحد**→پیش‌نمایش و G-code یکسان (تطبیق مختصات در آزمون)
- خروجی STL باینری استاندارد (normals واحد، validated با خوانندهٔ مستقل در آزمون)
- پروژهٔ JSON نسخه‌دار با اعتبارسنجی schema (فایل خراب پروژهٔ جاری را خراب نمی‌کند)
- «ارسال به ویرایشگر G-code» با یک کلیک
- هشدارهای bounds/برخورد/clamp + تأیید قبل از خروجی بحرانی

## ۳. فایل‌ها/معماری مهم

| مسیر | نقش |
|---|---|
| `src-tauri/core/src/gcode/lexer.rs` | لغزندهٔ G-code |
| `src-tauri/core/src/gcode/interpreter.rs` | مفسر modal-state |
| `src-tauri/core/src/gcode/doc.rs` | سند خام/BOM/پایان‌خط |
| `src-tauri/core/src/toolpath/{mod,arcs}.rs` | IR مشترک + قوس‌ها |
| `src-tauri/core/src/designer/{mod,profile,pattern,texture,spiral}.rs` | موتور طراح |
| `src-tauri/core/src/export/mod.rs` | خروجی G-code/STL/پروژه |
| `src-tauri/core/src/ipc.rs` | منطق ۱۰ دستور IPC |
| `src-tauri/src/{lib,commands/mod}.rs` | پوستهٔ Tauri (wrapper نازک) |
| `src/features/gcode-editor/GcodeEditorPage.tsx` | صفحهٔ ویرایشگر |
| `src/features/spiral-designer/SpiralDesignerPage.tsx` | طراح ۵ گامی |
| `src/components/{GcodeEditor,Viewport}.tsx` | CodeMirror/Three.js |
| `Docs/{REFERENCE_AUDIT,ARCHITECTURE,USER_GUIDE_FA,SAFETY}.md` | مستندات |

## ۴. روش نصب و اجرا

```bash
# توسعه
npm install && npm run dev
# آزمون‌ها
npm test                       # ۱۴ آزمون Vitest — ✅ همه پاس
cd src-tauri && cargo test -p gcode-core   # ۱۰۸ آزمون — ✅ همه پاس
# build ویندوز (روی Windows یا CI)
npm run tauri build            # NSIS .exe
```

- CI: `.github/workflows/windows-build.yml` روی Windows runner آزمون‌ها را اجرا و installer NSIS را artefact می‌کند.
- نصب per-user بدون admin؛ WebView2 با `offlineInstaller` (بدون bootstrapper آنلاین).

## ۵. آزمون‌ها

| نام | تعداد | نتیجه |
|---|---|---|
| cargo test -p gcode-core (lexer) | ۱۳ | ✅ موفق |
| cargo test -p gcode-core (arcs) | ۱۳ | ✅ موفق |
| cargo test -p gcode-core (interpreter/fixtureها) | ۱۴ | ✅ موفق |
| cargo test -p gcode-core (doc/BOM/CRLF) | ۱۰ | ✅ موفق |
| cargo test -p gcode-core (designer) | ۲۹ | ✅ موفق |
| cargo test -p gcode-core (export/STL/checksum) | ۱۰ | ✅ موفق |
| cargo test -p gcode-core (ipc/Save As) | ۵ | ✅ موفق |
| cargo test -p gcode-core (بقیه) | ۱۴ | ✅ موفق |
| **جمع cargo** | **۱۰۸** | **همه موفق** |
| npm test (Vitest: پل IPC/i18n/fixture) | ۱۴ | ✅ موفق |
| tsc --noEmit + vite build | — | ✅ بدون خطا |
| Desktop E2E روی ویندوز | — | ❌ **اجرا‌نشده** (محیط Linux) |
| نصب آفلاین ویندوز با اینترنت قطع | — | ❌ **اجرا‌نشده** |
| Performance فایل ۱۰۰MB/۱M خط | — | ❌ اجرا‌نشده (طراحی آماده: parse در هسته + virtual scroll + payload فشرده) |

## ۶. پژوهش OGcode (خلاصه؛ کامل در `Docs/REFERENCE_AUDIT.md`)

- ✅ **بررسی شد:** صفحهٔ عمومی `ogcode.dabi.me` (چهار گام بازاریابی، FAQ شامل تمایز pattern/texture و قید تک‌دیواره، نازل‌ها، ۳۹ preset، وضعیت آزمایشی)؛ ویدئوی معرفی (توضیحات کامل + فصل‌های رسمی 00:00–08:10 از HTML خام یوتیوب)؛ کانال سازنده (از oEmbed).
- ❌ **در دسترس نبود:** اپ اصلی `ogcode.dabi.design` پشت دیوار کلید مجوز Lemon Squeezy — طبق پروتکل **توقف کردیم و عبور نکردیم**؛ help درون‌اپ و پنل‌های داخلی بررسی نشد.
- ⚠️ ترنسکریپت قاب‌به‌قاب ویدئو بررسی تصویری نشد؛ جزئیات پنل‌های 03:05–07:22 از یادداشت مقدماتی با برچسب «نیازمند تأیید تصویری» نقل شد.
- هیچ کد/لوگو/متن/دارایی مرجع کپی نشده؛ هویت مستقل است.

## ۷. محدودیت‌ها و ریسک‌ها (صادقانه)

1. **بیلد و تست دسکتاپ ویندوز در محیط توسعه انجام نشد** (Linux بدون sudo برای وابستگی‌های GUI). CI ویندوز آماده است؛ تا اجرای واقعی، installer «تست‌نشده» است.
2. **فایل بسیار بزرگ (۱۰۰MB/۱M خط):** طراحی آماده است (هستهٔ Rust خالص + payload فشردهٔ عددی + virtual scrolling CodeMirror) ولی benchmark واقعی اجرا نشده؛ سقف خواندن ۲۵۶MB محافظت فعال دارد.
3. **Dialectهای پشتیبانی‌نشده:** متغیر/شرط/ماکرو (`#`/`IF`/`WHILE`)، `G10/G11` (مقدار دقیق نامعلوم)، `G54–G59` (آفست مدل نمی‌شود) — همه با diagnostic صادقانه.
4. **حالت E پس از G28/پروفایل:** موقعیت نامعلوم با Certainty برچسب می‌خورد نه عدد جعلی.
5. **presets پرینتر:** همه `unverified`؛ اعداد باید با دستگاه واقعی صحت‌سنجی شوند.
6. **طراح فقط vase-mode تک‌دیواره است**؛ infill/multi-wall ادعا نمی‌شود.
7. **RTL/LTR در WebView2:** پیاده‌سازی استاندارد (dir + isolate)؛ تست روی ویندوز واقعی باقی است.
8. **زبان:** رابط فقط فارسی؛ ساختار i18n برای افزودن زبان آماده است.

## ۸. قبل از چاپ — هشدار ایمنی

> **این برنامه G-code را اجرا نمی‌کند و به هیچ پرینتری وصل نمی‌شود. صحت چاپ فیزیکی تضمین نمی‌شود.** نمایش سه‌بعدی «بازسازی مسیر نازل» است نه مدل اصلی. پیش از چاپ: فایل را با slicer/viewer معتبر بازبینی کنید، پروفایل واقعی خود را تنظیم کنید و **اولین چاپ را حتماً زیر نظر** داشته باشید. خروجی طراح «آزمایشی» است و طرح‌های تند/پیچ‌دار ممکن است چاپ‌ناپذیر باشند. جزئیات کامل: `Docs/SAFETY.md`.
