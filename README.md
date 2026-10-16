# 🧰 Sahand OGCode Studio — استودیوی ویرایش و طراحی G-code

> **نرم‌افزار دسکتاپ ویندوز، کاملاً آفلاین، فارسی/RTL**
> طبق مشخصات کامل پوشهٔ `Docs/` ساخته شده است.

[![windows-build](https://github.com/pranses2011/Sahand_OGCode_Studio/actions/workflows/windows-build.yml/badge.svg)](https://github.com/pranses2011/Sahand_OGCode_Studio/actions/workflows/windows-build.yml)

---

## ⚡ معرفی

**G-code Studio** دو ماژول مستقل اما مرتبط دارد:

1. **ویرایشگر و تحلیل‌گر G-code** — بازکردن فایل محلی، متن کد و بازسازی سه‌بعدی مسیر نازل کنار هم، توضیح فارسی دقیق هر خط با پارامترها و وضعیت ماشین قبل/بعد، ویرایش زنده با به‌روزرسانی پیش‌نمایش، و «ذخیره با نام جدید» بدون بازنویسی فایل اصلی.
2. **طراح چاپ مارپیچی** — ساخت فرم چرخشی از منحنی مقطع با گردش‌کار ۵ گامی (پروفایل → الگوی هندسی → بافت مسیر → کف → تنظیمات پرینتر)، پیش‌نمایش زنده و خروجی STL/G-code.

هیچ سرور، حساب کاربری، آپلود، CDN، telemetry یا auto-update وجود ندارد؛ همهٔ پردازش‌ها محلی روی سیستم شما انجام می‌شود.

## 🧱 پشتهٔ فنی

| لایه | فناوری |
|---|---|
| پوستهٔ دسکتاپ | **Tauri 2** (WebView2 با `offlineInstaller`) |
| هستهٔ محاسباتی | **Rust** — parser/مفسر/طراح/خروجی‌ساز خالص |
| رابط کاربری | **React 18 + TypeScript + Vite** |
| ویرایشگر کد | **CodeMirror 6** |
| نمایش سه‌بعدی | **Three.js (WebGL2)** |
| آزمون | **cargo test** (۱۰۸ آزمون) + **Vitest** (۱۴ آزمون) |

## 📦 نصب

### دانلود installer

1. به تب **Actions** ریپازیتوری بروید → workflow **windows-build** → آخرین اجرای موفق → artefact **`gcode-studio-windows-x64`** را دانلود کنید.
2. فایل `G-code Studio_1.0.0_x64-setup.exe` (NSIS) را اجرا کنید.

### مشخصات نصب

- **هدف:** Windows 10 22H2 / Windows 11 x64
- **نصب per-user** (بدون نیاز به admin)
- **WebView2 با حالت `offlineInstaller`** داخل نصب‌کننده بسته‌بندی شده است → **نصب روی سیستم بدون اینترنت و بدون runtime از پیش نصب‌شده ممکن است** (بدون bootstrapper آنلاین).
- گزینهٔ `fixedRuntime` عمداً به‌کار نرفته: حجم نصب‌کننده را چند ده مگابایت بزرگ می‌کند و به‌روزرسانی امنیتی WebView2 را از کنترل کاربر خارج می‌کند. اگر این trade-off برایتان مهم است، در `src-tauri/tauri.conf.json` قابل تغییر است.
- حذف برنامه (uninstall) فایل‌های G-code و پروژه‌های شما را پاک **نمی‌کند**.

### build از سورس

```bash
# پیش‌نیاز: Node 20+ و Rust stable
npm install
npm test              # آزمون‌های فرانت‌اند (Vitest)
cd src-tauri
cargo test -p gcode-core   # ۱۰۸ آزمون هستهٔ Rust
cd ..
npm run tauri build   # روی ویندوز: NSIS .exe در src-tauri/target/release/bundle/nsis/
```

> **محدودیت صادقانهٔ محیط:** این ریپو روی سرور Linux توسعه یافته است؛ build پوستهٔ Tauri (که به WebView2/GTK وابسته است) روی Windows انجام می‌شود — CI گیت‌هاب همین کار را می‌کند. **تست نصب آفلاین روی ویندوز واقعی با اینترنت قطع، در محیط توسعه انجام نشده و «اجرا‌نشده» گزارش می‌شود.**

## 🗂️ ساختار پروژه

```text
Docs/                    # اسناد الزام + مستندات (REFERENCE_AUDIT، ARCHITECTURE، USER_GUIDE_FA، SAFETY)
src-tauri/
  core/src/              # هستهٔ Rust خالص (بدون وابستگی به Tauri)
    gcode/               #   lexer، doc (BOM/پایان‌خط)، interpreter (modal-state)
    toolpath/            #   IR مشترک + قوس‌های G2/G3
    designer/            #   پروفایل، الگو، بافت، اسپیرال، کف
    export/              #   G-code، STL باینری، پروژهٔ JSON، اعتبارسنجی Save
    ipc.rs               #   منطق ۱۰ دستور IPC (قابل‌آزمون)
  src/                   # پوستهٔ Tauri (wrapperهای نازک + دیالوگ native)
  capabilities/          # حداقل دسترسی (فقط core+dialog)
src/                     # رابط React فارسی RTL
  app/ components/ features/ i18n/ lib/ styles/ test-fixtures/
.github/workflows/       # CI build ویندوز
```

## ✅ آزمون‌های واقعاً اجراشده

| مجموعه | تعداد | وضعیت | محیط |
|---|---|---|---|
| `cargo test -p gcode-core` | ۱۰۸ | ✅ همه پاس | Linux (هسته خالص پلتفرم‌مستقل) |
| `npm test` (Vitest) | ۱۴ | ✅ همه پاس | Node 24 |
| `tsc --noEmit` + `vite build` | — | ✅ بدون خطا | Linux |
| build ویندوز NSIS | — | 🟡 از طریق CI | GitHub Actions Windows runner |
| **نصب/اجرا روی ویندوز با اینترنت قطع** | — | ❌ **اجرا‌نشده** | نیاز به سیستم ویندوز واقعی |

پوشش آزمون‌های هسته: lexer (فشرده/N/checksum/کامنت/M117/NaN)، مفسر modal (هر ۵ fixture سند + قوس R و IJ + bounds + spiral + macro)، قوس‌ها (سه صفحه/مارپیچ/خطاها)، doc (BOM/CRLF/roundtrip)، طراح (mesh/STL/الگو/بافت/کف/اعتبارسنجی)، خروجی (E مطلق-نسبی/STL binary/سیاست checksum)، IPC (payload/Save As).

## 🔒 امنیت و حریم خصوصی

- فقط ۱۰ دستور IPC دامنه‌ای؛ **هیچ shell/eval/شبکه‌ای وجود ندارد**.
- دسترسی فایل فقط از دیالوگ native (capability فقط `core:default` + `dialog:default`).
- CSP بدون هیچ remote source؛ همهٔ assetها داخل باندل.
- بدون telemetry/log محتوا؛ تنظیمات فقط در AppData.

## ⚠️ ایمنی چاپ

> **این برنامه صحت چاپ فیزیکی را تضمین نمی‌کند.** فایل را با ابزار/سازندهٔ معتبر بررسی کنید و اولین چاپ را زیر نظر داشته باشید. جزئیات در [`Docs/SAFETY.md`](Docs/SAFETY.md).

## 📚 مستندات

| سند | محتوا |
|---|---|
| [`Docs/REFERENCE_AUDIT.md`](Docs/REFERENCE_AUDIT.md) | پژوهش مرجع OGcode (۲۰۲۶-۱۰-۱۶) با منابع و محدودیت‌های دسترسی |
| [`Docs/ARCHITECTURE.md`](Docs/ARCHITECTURE.md) | معماری و تصمیم‌های ADR-001..008 |
| [`Docs/USER_GUIDE_FA.md`](Docs/USER_GUIDE_FA.md) | راهنمای کاربر فارسی |
| [`Docs/SAFETY.md`](Docs/SAFETY.md) | ایمنی چاپ و مرزهای نرم‌افزار |

## 📜 مجوز

مطابق فایل [LICENSE](LICENSE) ریپازیتوری.
