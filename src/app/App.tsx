/**
 * ============================================================================
 * app/App.tsx — پوستهٔ برنامه: ناوبری دو ماژول + ارسال G-code طراح به ویرایشگر
 * ============================================================================
 * «دکمهٔ ارسال به ویرایشگر G-code» (الزام PROMPT_03 §6): خروجی طراح باید با
 * یک کلیک در ویرایشگر اصلی باز شود؛ همان G-code تولیدشده واقعاً تحلیل می‌شود.
 * نوار رنگ (ColorBar): انتخاب تم accent و رنگ رشتهٔ چاپ در پیش‌نمایش —
 * فقط نمایشی و ماندگار در localStorage (بدون ذخیرهٔ راه‌دور).
 * ============================================================================
 */

import { useCallback, useEffect, useState } from "react";
import { GcodeEditorPage } from "../features/gcode-editor/GcodeEditorPage";
import { SpiralDesignerPage } from "../features/spiral-designer/SpiralDesignerPage";
// ۱٫۷ (درخواست کاربر ۱۱): صفحهٔ «متن به گلدان» — کانتور متن → دیوارهٔ مارپیچی
import { TextVasePage } from "../features/text-vase/TextVasePage";
import { ColorBar, storedFilamentColor } from "../components/ColorBar";
import { AboutModal } from "../components/AboutModal";
import { fa } from "../i18n/fa";
// ۱٫۶ (درخواست کاربر ۹): گزارش خودکار خطا به ایمیل سازنده + صف محلی
import { flushFeedbackQueue, installGlobalErrorReporter } from "../lib/feedback";
// نسخهٔ برنامه از package.json — منبع واحد (همان که CI/رلیز می‌سازد).
// درخواست کاربر ۱.۵: «نسخه برنامه در عنوان باشه».
import pkg from "../../package.json";

type Tab = "editor" | "designer" | "textvase";

/** تحویل G-code طراح به ویرایشگر — seq تضمین می‌کند هر ارسال، remount تازه بگیرد */
interface DesignerHandoff {
  content: string;
  seq: number;
  /** نام پیشنهادی فایل تحویلی — ۱٫۷: متن به گلدان */
  note?: string;
}

/** پروفایل پرینتر پیش‌فرض برای صفحهٔ متن (کاربر در طراح تنظیمات را تنظیم می‌کند) */
const DEFAULT_PRINTER = {
  name: "Custom / Unverified",
  status: "unverified",
  build_volume_mm: [220, 220, 250] as [number, number, number],
  nozzle_diameter_mm: 0.4,
  filament_diameter_mm: 1.75,
  line_width_mm: 0.45,
  layer_height_mm: 0.25,
  print_speed_mm_min: 2400,
  travel_speed_mm_min: 6000,
  flow_multiplier: 1.0,
  absolute_e: true,
  nozzle_temp_c: 200,
  bed_temp_c: 60,
  fan_percent: 100,
  wait_for_temps: true,
  wall_speed_mm_s: 40,
  first_layer_speed_mm_s: 20,
  floor_speed_mm_s: 30,
  travel_speed_mm_s: 100,
  retract_mm: 0.8,
  retract_speed_mm_s: 40,
  start_gcode: "",
  end_gcode: "",
} as const;

export function App() {
  const [tab, setTab] = useState<Tab>("editor");
  // G-code تحویلی طراح — تا ارسال بعدی نگه داشته می‌شود تا key پایدار بماند
  // (باگ قبلی: پاک‌شدن فوری pending باعث remount بدون محتوا می‌شد)
  const [handoff, setHandoff] = useState<DesignerHandoff | null>(null);
  // رنگ رشتهٔ چاپ (اکستروژن) در پیش‌نمایش سه‌بعدی — صرفاً نمایشی
  const [filamentColor, setFilamentColor] = useState<string>(storedFilamentColor);
  // پنجرهٔ «درباره/بازخورد» — نسخهٔ ۱٫۶ (درخواست کاربر ۹)
  const [aboutOpen, setAboutOpen] = useState(false);

  // گزارشگر سراسری خطا (error + unhandledrejection) — نسخهٔ ۱٫۶:
  // خطاها به‌صورت خودکار برای سازنده ارسال می‌شوند تا اصلاح شوند؛
  // صف محلی در نبود اینترنت باقی می‌ماند و در بوت بعدی تلاش می‌شود.
  useEffect(() => {
    const cleanup = installGlobalErrorReporter();
    void flushFeedbackQueue();
    return cleanup;
  }, []);

  /** دریافت G-code از طراح و پرش به ویرایشگر */
  const handleSendToEditor = useCallback((gcode: string) => {
    setHandoff((h) => ({ content: gcode, seq: (h?.seq ?? 0) + 1 }));
    setTab("editor");
  }, []);

  /** دریافت G-code «متن به گلدان» و پرش به ویرایشگر — ۱٫۷ */
  const handleTextToEditor = useCallback((gcode: string, note: string) => {
    setHandoff((h) => ({ content: gcode, seq: (h?.seq ?? 0) + 1, note }));
    setTab("editor");
  }, []);

  return (
    <div className="app-shell">
      <div className="topbar">
        <span className="brand">
          <img
            src="brand-icon.png"
            alt="نشان Sahand GCode Studio"
            className="brand-icon"
            width={22}
            height={22}
          />
          Sahand GCode Studio
          <span className="brand-version ltr" title="نسخهٔ برنامه">v{pkg.version}</span>
        </span>
        <div className="tabs" role="tablist">
          <button
            className={`tab-btn ${tab === "editor" ? "active" : ""}`}
            role="tab"
            aria-selected={tab === "editor"}
            onClick={() => setTab("editor")}
          >
            {fa.navEditor}
          </button>
          <button
            className={`tab-btn ${tab === "designer" ? "active" : ""}`}
            role="tab"
            aria-selected={tab === "designer"}
            onClick={() => setTab("designer")}
          >
            {fa.navDesigner}
          </button>
          {/* متن به گلدان — نسخهٔ ۱٫۷ (درخواست کاربر ۱۱) */}
          <button
            className={`tab-btn ${tab === "textvase" ? "active" : ""}`}
            role="tab"
            aria-selected={tab === "textvase"}
            onClick={() => setTab("textvase")}
          >
            ✍️ متن به گلدان
          </button>
          {/* دربارهٔ برنامه + بازخورد مستقیم به سازنده — نسخهٔ ۱٫۶ (درخواست ۹) */}
          <button
            className="btn small about-btn"
            onClick={() => setAboutOpen(true)}
            title="دربارهٔ برنامه · تماس سازنده · نظرات و پیشنهادات"
          >
            ⓘ درباره
          </button>
        </div>
      </div>

      {/* پنجرهٔ درباره/بازخورد */}
      <AboutModal open={aboutOpen} onClose={() => setAboutOpen(false)} />

      <ColorBar filamentColor={filamentColor} onFilamentChange={setFilamentColor} />

      {tab === "editor" ? (
        <GcodeEditorPage
          key={handoff ? `handoff-${handoff.seq}` : "normal"}
          initialContent={handoff?.content ?? null}
          initialName={handoff ? (handoff.note ?? "text-vase-output.gcode") : null}
          extrusionColor={filamentColor}
        />
      ) : tab === "textvase" ? (
        <TextVasePage printer={DEFAULT_PRINTER} onGcode={handleTextToEditor} />
      ) : (
        <SpiralDesignerPage
          onSendToEditor={handleSendToEditor}
          extrusionColor={filamentColor}
        />
      )}
    </div>
  );
}
