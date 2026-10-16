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

import { useCallback, useState } from "react";
import { GcodeEditorPage } from "../features/gcode-editor/GcodeEditorPage";
import { SpiralDesignerPage } from "../features/spiral-designer/SpiralDesignerPage";
import { ColorBar, storedFilamentColor } from "../components/ColorBar";
import { fa } from "../i18n/fa";

type Tab = "editor" | "designer";

/** تحویل G-code طراح به ویرایشگر — seq تضمین می‌کند هر ارسال، remount تازه بگیرد */
interface DesignerHandoff {
  content: string;
  seq: number;
}

export function App() {
  const [tab, setTab] = useState<Tab>("editor");
  // G-code تحویلی طراح — تا ارسال بعدی نگه داشته می‌شود تا key پایدار بماند
  // (باگ قبلی: پاک‌شدن فوری pending باعث remount بدون محتوا می‌شد)
  const [handoff, setHandoff] = useState<DesignerHandoff | null>(null);
  // رنگ رشتهٔ چاپ (اکستروژن) در پیش‌نمایش سه‌بعدی — صرفاً نمایشی
  const [filamentColor, setFilamentColor] = useState<string>(storedFilamentColor);

  /** دریافت G-code از طراح و پرش به ویرایشگر */
  const handleSendToEditor = useCallback((gcode: string) => {
    setHandoff((h) => ({ content: gcode, seq: (h?.seq ?? 0) + 1 }));
    setTab("editor");
  }, []);

  return (
    <div className="app-shell">
      <div className="topbar">
        <span className="brand">G-code Studio</span>
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
        </div>
      </div>

      <ColorBar filamentColor={filamentColor} onFilamentChange={setFilamentColor} />

      {tab === "editor" ? (
        <GcodeEditorPage
          key={handoff ? `handoff-${handoff.seq}` : "normal"}
          initialContent={handoff?.content ?? null}
          initialName={handoff ? "spiral-designer-output.gcode" : null}
          extrusionColor={filamentColor}
        />
      ) : (
        <SpiralDesignerPage
          onSendToEditor={handleSendToEditor}
          extrusionColor={filamentColor}
        />
      )}
    </div>
  );
}
