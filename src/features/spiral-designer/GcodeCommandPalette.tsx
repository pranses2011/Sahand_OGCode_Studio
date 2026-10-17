/**
 * ============================================================================
 * features/spiral-designer/GcodeCommandPalette.tsx — پالت دستورات شروع/پایان
 * ============================================================================
 * درخواست کاربر ۳ (نسخهٔ ۱٫۶): فهرست دستورات مرتبط با توضیح فارسی؛
 * افزودن با یک کلیک به کادر «کد شروع» یا «کد پایان» گام ۵.
 * ============================================================================
 */

import { useMemo, useState } from "react";
import { COMMAND_CATEGORIES, filterCommands, type CommandSlot } from "./gcodeCommands";

export interface GcodeCommandPaletteProps {
  /** کدام کادر: شروع یا پایان */
  slot: CommandSlot;
  /** افزودن قطعه به کادر هدف */
  onInsert: (snippet: string) => void;
}

const SLOT_LABEL: Record<CommandSlot, string> = {
  start: "دستورات شروع",
  end: "دستورات پایان",
  both: "شروع و پایان",
};

export function GcodeCommandPalette(props: GcodeCommandPaletteProps) {
  const { slot, onInsert } = props;
  const [query, setQuery] = useState("");
  const [cat, setCat] = useState<string>("");

  const list = useMemo(() => {
    const filtered = filterCommands(query, slot);
    return cat ? filtered.filter((c) => c.category === cat) : filtered;
  }, [query, slot, cat]);

  return (
    <div className="cmd-palette">
      <div className="cmd-palette-head">
        <span className="cmd-title">📚 {SLOT_LABEL[slot]}</span>
        <input
          className="cmd-search"
          type="search"
          placeholder="جست‌وجوی دستور… (مثلاً M104 یا دما)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      <div className="cmd-cats" role="tablist" aria-label="دسته‌بندی دستورات">
        <button
          className={`cmd-cat ${cat === "" ? "active" : ""}`}
          role="tab"
          aria-selected={cat === ""}
          onClick={() => setCat("")}
        >
          همه
        </button>
        {COMMAND_CATEGORIES.map((c) => (
          <button
            key={c}
            className={`cmd-cat ${cat === c ? "active" : ""}`}
            role="tab"
            aria-selected={cat === c}
            onClick={() => setCat(c === cat ? "" : c)}
          >
            {c}
          </button>
        ))}
      </div>
      <div className="cmd-list" role="listbox" aria-label="فهرست دستورات">
        {list.length === 0 && (
          <div className="cmd-empty">دستوری یافت نشد — عبارت دیگری بیازمایید.</div>
        )}
        {list.map((c) => (
          <div key={c.code + c.name} className="cmd-item" role="option" aria-selected={false}>
            <div className="cmd-meta">
              <code className="cmd-code ltr">{c.code}</code>
              <span className="cmd-name">{c.name}</span>
              <span className="cmd-slot-chip">{SLOT_LABEL[c.slot]}</span>
            </div>
            <div className="cmd-desc">{c.desc}</div>
            <button
              className="btn small primary cmd-add"
              onClick={() => onInsert(c.snippet)}
              title="افزودن به انتهای کادر"
            >
              + افزودن
            </button>
          </div>
        ))}
      </div>
      <span className="hint-text">
        قطعه با یک کلیک به «انتهای» کادر افزوده می‌شود؛ اعداد را مطابق پرینتر خود
        ویرایش کنید. توضیحات بر پایهٔ Marlin/RepRap است — فریم‌ورهای خاص ممکن
        است تفاوت داشته باشند.
      </span>
    </div>
  );
}
