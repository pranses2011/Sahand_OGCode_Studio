/**
 * ============================================================================
 * components/ColorBar.tsx — نوار انتخاب رنگ: تم accent + رنگ رشتهٔ چاپ
 * ============================================================================
 * دو انتخاب مستقل:
 *   ۱) «رنگ تم»: رنگ accent رابط (۸ پالت) — با data-accent روی :root اعمال
 *      می‌شود و همهٔ کنترل‌های accent دار دنبالش می‌روند؛
 *   ۲) «رنگ رشتهٔ چاپ»: رنگ segmentهای اکستروژن در پیش‌نمایش سه‌بعدی
 *      (۱۰ رنگ فیلامنت) — صرفاً نمایشی و روی دادهٔ G-code اثر ندارد.
 *
 * ماندگاری: localStorage (بدون هیچ ذخیرهٔ راه‌دور — الزام آفلاین).
 * دسترس‌پذیری: هر swatch دکمهٔ واقعی با aria-pressed و title فارسی است؛
 * رنگ تنها راه اطلاع نیست (برچسب متنی هر گروه موجود است).
 * ============================================================================
 */

import { useEffect, useState } from "react";

/** پالت‌های رنگ تم — id باید با CSS data-accent یکی باشد */
export const ACCENT_THEMES: { id: string; fa: string; color: string }[] = [
  { id: "orange", fa: "نارنجی صنعتی", color: "#ff7a28" },
  { id: "cyan", fa: "سایان", color: "#22d3ee" },
  { id: "green", fa: "سبز", color: "#34d399" },
  { id: "purple", fa: "بنفش", color: "#a78bfa" },
  { id: "amber", fa: "کهربایی", color: "#fbbf24" },
  { id: "pink", fa: "سرخابی", color: "#f472b6" },
  { id: "blue", fa: "آبی", color: "#60a5fa" },
  { id: "red", fa: "قرمز", color: "#f87171" },
];

/** رنگ‌های رشتهٔ چاپ (اکستروژن پیش‌نمایش) — نمایشی، تأثیری بر G-code ندارد */
export const FILAMENT_COLORS: { fa: string; color: string }[] = [
  { fa: "سبز", color: "#46c878" },
  { fa: "نارنجی", color: "#ffa04d" },
  { fa: "فیروزه‌ای", color: "#4dd0c4" },
  { fa: "بنفش", color: "#a78bfa" },
  { fa: "زرد", color: "#e6d444" },
  { fa: "صورتی", color: "#f472b6" },
  { fa: "آبی یخی", color: "#9cc7e8" },
  { fa: "لیمویی", color: "#b8d94e" },
  { fa: "طوسی", color: "#c9cdd7" },
  { fa: "سفید", color: "#f2f4f8" },
];

const LS_ACCENT = "gs-accent";
const LS_FILAMENT = "gs-filament";

/** مقدار اولیهٔ امن — فقط مقادیر معتبر پذیرفته می‌شوند */
function initialAccent(): string {
  try {
    const v = localStorage.getItem(LS_ACCENT);
    if (v && ACCENT_THEMES.some((t) => t.id === v)) return v;
  } catch { /* localStorage در دسترس نیست → پیش‌فرض */ }
  return "orange";
}

/** بازیابی امن رنگ رشتهٔ ذخیره‌شده (برای App) */
export function storedFilamentColor(): string {
  try {
    const v = localStorage.getItem(LS_FILAMENT);
    if (v && FILAMENT_COLORS.some((t) => t.color === v)) return v;
  } catch { /* localStorage در دسترس نیست → پیش‌فرض */ }
  return "#46c878";
}

export interface ColorBarProps {
  /** رنگ اکستروژن فعلی (هگز) — به Viewportها داده می‌شود */
  filamentColor: string;
  /** تغییر رنگ رشتهٔ چاپ */
  onFilamentChange: (color: string) => void;
}

export function ColorBar(props: ColorBarProps) {
  const [accent, setAccent] = useState<string>(initialAccent);

  // اعمال تم روی ریشهٔ سند (CSS متغیرهای --accent/--accent-dim)
  useEffect(() => {
    document.documentElement.setAttribute("data-accent", accent);
    try {
      localStorage.setItem(LS_ACCENT, accent);
    } catch { /* ذخیره نشود — بی‌صدا رد می‌شود */ }
  }, [accent]);

  const setFilament = (color: string) => {
    props.onFilamentChange(color);
    try {
      localStorage.setItem(LS_FILAMENT, color);
    } catch { /* ذخیره نشود — بی‌صدا رد می‌شود */ }
  };

  return (
    <div className="color-bar" role="toolbar" aria-label="انتخاب رنگ نمایش">
      <div className="swatches" role="group" aria-label="رنگ تم">
        <span className="lbl">رنگ تم:</span>
        {ACCENT_THEMES.map((t) => (
          <button
            key={t.id}
            className="sw-btn"
            title={t.fa}
            aria-label={`تم ${t.fa}`}
            aria-pressed={accent === t.id}
            style={{ background: t.color }}
            onClick={() => setAccent(t.id)}
          />
        ))}
      </div>
      <span className="divider" aria-hidden="true" />
      <div className="swatches" role="group" aria-label="رنگ رشتهٔ چاپ">
        <span className="lbl">رنگ رشتهٔ چاپ (نمایشی):</span>
        {FILAMENT_COLORS.map((t) => (
          <button
            key={t.color}
            className="sw-btn"
            title={t.fa}
            aria-label={`رشتهٔ ${t.fa}`}
            aria-pressed={props.filamentColor === t.color}
            style={{ background: t.color }}
            onClick={() => setFilament(t.color)}
          />
        ))}
      </div>
    </div>
  );
}
