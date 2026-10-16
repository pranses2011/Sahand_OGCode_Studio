# 🏗️ معماری نرم‌افزار G-code Studio

> **نسخهٔ سند:** ۱.۰ — تاریخ: ۲۰۲۶-۱۰-۱۶
> **مرجع الزامات:** `Docs/PROMPT_04_ARCHITECTURE_UI.md` و سایر اسناد بستهٔ `Docs/`
> این سند «طرح فنی فاز ۱» پروژه است و تصمیم‌های معماری قطعی‌شده را ثبت می‌کند.

---

## ۱. تصمیم پشتهٔ فنی (تصمیم معماری شمارهٔ ADR-001)

### انتخاب قطعی

| لایه | فناوری | دلیل |
|---|---|---|
| پوستهٔ دسکتاپ | **Tauri 2** | نصب‌کنندهٔ بومی ویندوز، حجم کم، مرزبندی امنیتی capability، WebView2 |
| هستهٔ محاسباتی | **Rust (stable)** | parser قطعی و سریع، thread غیرمسدودکننده، تست واحد مستقل از UI |
| رابط کاربری | **React 18 + TypeScript + Vite** | بهره‌وری کامپوننتی، تایپ امن برای IR، باندل محلی |
| ویرایشگر کد | **CodeMirror 6** | اسکرول مجازی میلیون‌خطی، syntax highlighting سبک، API ترکیبی |
| viewport سه‌بعدی | **Three.js (WebGL2)** | رندر مسیر با BufferGeometry/Line2 و متریال لایه‌ای |
| آزمون | **cargo test + Vitest** | پوشش هسته و UI جدا از هم |

### رد گزینه‌ها

- **Electron + Chromium:** نصب‌کنندهٔ سنگین (۱۰۰+ مگابایت)، مصرف RAM بالاتر، ریسک امنیتی nodeIntegration. فقط اگر بسته‌بندی آفلاین WebView2 شکست بخورد مجاز بود؛ نشکست.
- **.NET 10 WPF:** تیم و مخزن موجود .NET ندارد؛ ادغام Three.js/CodeMirror در WPF هزینهٔ یکپارچه‌سازی بالا دارد. بدون دلیل فنی مهاجرت لازم نبود.

### استراتژی WebView2 برای نصب آفلاین (ADR-002)

- در `tauri.conf.json` مقدار `bundle.windows.webviewInstallMode` روی `{"mode": "offlineInstaller"}` تنظیم می‌شود تا نصب‌کنندهٔ WebView2 داخل NSIS `.exe` قرار گیرد و نصب روی سیستم بدون اینترنت و بدون runtime از پیش نصب‌شده ممکن شود.
- از `downloadBootstrapper` (نیازمند اینترنت هنگام نصب) استفاده نمی‌شود.
- `fixedRuntime` در v1 به‌کار نمی‌رود: حجم نصب‌کننده را به‌شدت بزرگ می‌کند و به‌روزرسانی امنیتی runtime را از کنترل کاربر خارج می‌کند؛ trade-off در README مستند شده است.
- **محدودیت صادقانهٔ محیط ساخت:** سرور ساخت این پروژه Linux است. تولید installer واقعی ویندوز باید روی Windows runner انجام شود؛ CI گیت‌هاب (workflow ساخت ویندوز) در `.github/workflows/` فراهم شده است. تا وقتی روی ویندوز build و نصب واقعی اجرا نشود، وضعیت آن «اجرا‌نشده» گزارش می‌شود.

## ۲. نمای کلی معماری و جریان داده

```text
کاربر ویندوز
   │  (دیالوگ native بازکردن/ذخیره — فقط مسیرهای انتخابی کاربر)
   ▼
دستورهای محدود Tauri (src-tauri/src/commands/)
   │  open_gcode | save_as | parse_gcode | designer_* | export_*
   ▼
هستهٔ Rust (src-tauri/src/core/)
   ├─ gcode/        ← lexer + parser + مفسر modal-state
   ├─ toolpath/     ← Toolpath IR مشترک + قوس‌ها + لایه‌بندی
   ├─ designer/     ← پروفایل، pattern، texture، floor، spiral
   ├─ export/       ← serializer G-code، STL باینری/ASCII، پروژهٔ JSON
   └─ diagnostics   ← خطا/هشدار/عدم‌قطعیت با کد و پیام فارسی
   │
   │  (IPC فشرده: آرایه‌های عددی + متادیتای JSON، نه JSON میلیونی)
   ▼
UI داخل WebView2 (src/)
   ├─ app shell فارسی RTL + ناوبری دو ماژول
   ├─ features/gcode-editor   ← CodeMirror 6 + همگام‌سازی خط↔segment
   ├─ features/spiral-designer← ۵ گام + پیش‌نمایش زنده
   ├─ viewport سه‌بعدی مشترک (Three.js)
   └─ هیچ درخواست شبکه‌ای در زمان اجرا
```

**قاعدهٔ حاکم:** مرجع معتبر parsing و تولید مسیر فقط Rust است؛ کامپوننت React هیچ‌گاه با regex خودش G-code را به هندسه تبدیل نمی‌کند.

## ۳. تصمیم‌های کلیدی Parser (ADR-003)

1. **دو لایهٔ جدا:** `Lexer` خط خام را به token (فرمان/پارامتر/comment/N/checksum) می‌شکند؛ `Interpreter` توکن‌ها را با وضعیت modal اجرا می‌کند. هیچ regex واحد و شکننده‌ای وجود ندارد.
2. **فرمت‌های پذیرفته‌شده:** فاصله‌دار (`G1 X10 Y20`)، فشرده (`G1X10Y20`)، با شمارهٔ خط و checksum (`N120 G1 X10*71`). `N` همیشه شمارهٔ خط است نه محور؛ payload متنی `M117` به پارامتر تبدیل نمی‌شود.
3. **commentها:** `;` و `(...)` هرگز تفسیر نمی‌شوند؛ متن داخل comment دست‌نخورده می‌ماند.
4. **وضعیت modal پیوسته:** واحد (G20/G21)، مختصات (G90/G91)، E (M82/M83/G92)، plane قوس (G17/18/19)، آخرین F، دما/فن/ابزار. موقعیت‌ها نشانگر «معلوم/تقریبی/نامعلوم» دارند.
5. **سینتکس پشتیبانی‌نشده:** متغیر/شرط/ماکرو (`#`, `IF`, `WHILE`, `<condition>`) اجرا نمی‌شوند؛ خط با diagnostic «نیازمند تفسیر firmware» علامت می‌خورد و متن خام حفظ می‌شود.
6. **عدد نامعتبر:** `XNaN` → diagnostic خطا؛ خط دور می‌زده می‌شود ولی متن خام در ادیتور باقی می‌ماند و آخرین preview معتبر با برچسب «قدیمی» نمایش داده می‌شود.

## ۴. همگام‌سازی خط↔segment (ADR-004)

- هر `ToolpathSegment` ایندکس خط مبدأ (`source_line_index`) دارد؛ چند segment (قوس‌های tessellate‌شده) می‌توانند به یک خط اشاره کنند.
- انتخاب خط در CodeMirror → فیلتر segmentهای آن خط در viewport → برجسته‌سازی با متریال emissive و تغییر رنگ.
- انتخاب segment در viewport (raycast روی خطوط با افزایش ضخامت pick) → jump-to-line در ادیتور با اسکرول.
- خط غیرحرکتی (دما/فن/comment) → پیام روشن در inspector + نمایش state قبل/بعد؛ segment جعلی ساخته نمی‌شود.
- scrubber شبیه‌سازی با خط انتخابی دوطرفه همگام است.

## ۵. حفظ متن خام و Save As (ADR-005)

- متن خام فایل بایت‌به‌بایت در `GcodeDocument` نگه می‌شود؛ خطوط به‌صورت spanهای قرضی تفکیک می‌شوند و چیزی بازنویسی نمی‌شود.
- BOM و نوع پایان‌خط (LF/CRLF) تشخیص و در خروجی بازتولید می‌شوند.
- ویرایش فقط خط(های) تغییرکرده را در آرایهٔ خطوط جایگزین می‌کند؛ Save As کل متن جدید را می‌نویسد و **هرگز** فایل مبدأ را بازنویسی نمی‌کند (نام پیش‌فرض: `originalname-edited.gcode`).
- فایل‌های دارای `N...*checksum`: سیاست پیش‌فرض «بازمحاسبهٔ checksum XOR» با پیش‌نمایش است؛ حذف N/checksum فقط با تأیید صریح کاربر.

## ۶. طراح مارپیچی — زنجیرهٔ منبع واحد (ADR-006)

```text
پروفایل 2D (نقاط r,h)
   → اعمال ارتفاع/قطر/taper + floor
   → patternهای هندسی: r(z,θ) = r_profile(z) + Σ pattern(z,θ)
   → mesh پایه (STL از همین زنجیره)
   → مسیر spiral پیوسته (θ پیوسته، Z صعودی با pitch)
   → textureهای toolpath (تعدیل شعاع/Z نقطه‌به‌نقطه)
   → محاسبهٔ E (حجم bead ≈ line width × layer height × طول) و F
   → افزودن start/end کد پروفایل
   → Toolpath IR واحد → همان داده برای 3D preview و G-code export
```

- **Toolpath IR مشترک** تضمین می‌کند چیزی که کاربر می‌بیند همان چیزی است که صادر می‌شود (قابل‌تطبیق مختصات در آزمون).
- Pattern روی r(z,θ) قبل از ساخت mesh اعمال می‌شود ⇒ در STL می‌ماند. Texture بعد از ساخت مسیر اعمال می‌شود ⇒ فقط در G-code/preview. این تمایز در UI صراحتاً توضیح داده می‌شود.
- E مطلق یا نسبی طبق پروفایل؛ حالت در میانهٔ فایل عوض نمی‌شود. correction factor جریان قابل‌تنظیم و مستند است.
- STL باینری با normals محاسبه‌شده؛ خروجی در آزمون با خوانندهٔ مستقل (خود تست) validate می‌شود.

## ۷. مرزبندی امنیتی (ADR-007)

- فقط دستورهای دامنه‌ای IPC وجود دارد: `open_gcode`, `parse_gcode`, `save_as`, `validate_gcode`, `designer_compute`, `designer_export_gcode`, `designer_export_stl`, `save_project`, `load_project`.
- هیچ دستور اجرای shell/eval arbitrary وجود ندارد؛ plugin `shell`، `updater`، `http` و دسترسی remote فعال نیستند.
- دسترسی فایل فقط از مسیرهای برگشتی دیالوگ native (plugin `dialog`)؛ capability `fs` محدود به scope انتخابی.
- CSP بدون remote source؛ همهٔ assetها (فونت وزیرمتن، آیکون‌ها) باندل محلی.
- محتوای G-code با textContent رندر می‌شود، هرگز innerHTML نمی‌گیرد.
- تنظیمات فقط در `AppData`، بدون telemetry/log محتوا.

## ۸. طرح آزمون (ADR-008)

| سطح | ابزار | پوشش |
|---|---|---|
| هسته Rust | `cargo test` | lexer (فشرده/comment/N/checksum)، modal (G90/91، M82/83، G20/21، G92)، قوس‌ها، fixtureهای ۵گانهٔ `TEST_GCODE_FIXTURES.md`، spiral/pattern/texture، STL، serializer |
| فرانت‌اند | `Vitest` | مدل state، همگام‌سازی خط↔segment، undo/redo، اعتبارسنجی فرم‌های طراح |
| یکپارچه | `cargo test` + Vitest | تطبیق IR طراح با G-code تولیدی، تطبیق STL با mesh |
| E2E دسکتاپ | اجرا‌نشده روی ویندوز | CI workflow آماده است؛ گزارش صادقانه |

## ۹. ریسک‌های فنی ثبت‌شده

1. **build ویندوز در محیط Linux ممکن نیست** → CI گیت‌هاب فراهم می‌شود؛ تا اجرای واقعی، installer «تست‌نشده» است.
2. **فایل ۱۰۰MB/۱M خط** → parse در thread Rust با progress/cancel؛ رندر CodeMirror با virtual scroll؛ IPC با آرایه‌های فشرده float32-friendly.
3. **قوس‌های نامعتبر (R منفی، شعاع ناهمخوان)** → diagnostic هشدار + segment نامطمئن، نه crash.
4. **G-code پیچیده firmware (Marlin macros و غیره)** → علامت «پشتیبانی‌نشده» صادقانه.
5. **از دست رفتن WebGL** → پیام راهنما + امکانات متنی (inspector/search) همچنان فعال.

## ۱۰. ساختار نهایی مخزن

```text
Sahand_OGCode_Studio/
├── Docs/                     # اسناد الزام و مستندات پروژه (این پوشه از قبل موجود است)
│   ├── PROMPT_01..05_*.md    # مشخصات الزام
│   ├── OGCODE_REFERENCE_RESEARCH.md
│   ├── TEST_GCODE_FIXTURES.md
│   ├── REFERENCE_AUDIT.md    # پژوهش مرجع روز اجرا
│   ├── ARCHITECTURE.md       # همین سند
│   ├── USER_GUIDE_FA.md      # راهنمای کاربر فارسی
│   └── SAFETY.md             # ایمنی چاپ
├── src-tauri/                # هستهٔ Rust + دستورات Tauri
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   └── src/
│       ├── main.rs
│       ├── commands/         # دستورهای IPC محدود
│       └── core/
│           ├── mod.rs
│           ├── gcode/        # lexer, parser, interpreter, doc
│           ├── toolpath/     # IR, arcs, layers
│           ├── designer/     # profile, pattern, texture, floor, spiral
│           └── export/       # gcode_out, stl, project
├── src/                      # رابط React فارسی RTL
│   ├── app/                  # shell و ناوبری
│   ├── components/           # کامپوننت‌های مشترک
│   ├── features/
│   │   ├── gcode-editor/     # ویرایشگر + inspector + شبیه‌سازی
│   │   └── spiral-designer/  # ۵ گام طراح
│   ├── i18n/fa.ts            # رشته‌های فارسی
│   ├── lib/tauri.ts          # پل IPC تایپ‌دار
│   ├── styles/               # RTL، تم تیره صنعتی
│   └── test-fixtures/        # fixtureهای مشترک با Rust
├── .github/workflows/        # CI build ویندوز
├── package.json, vite.config.ts, tsconfig.json, index.html
└── README.md
```

> **یادداشت:** پوشهٔ مستندات در مخزن از ابتدا `Docs/` (با D بزرگ) است؛ برای جلوگیری از تداخل case-sensitivity روی ویندوز، همهٔ مستندات جدید نیز در همان پوشهٔ `Docs/` قرار می‌گیرند، هرچند اسناد الزام مسیر `docs/` را نوشته‌اند.
