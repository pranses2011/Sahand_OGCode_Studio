/**
 * ============================================================================
 * features/text-vase/TextVasePage.tsx — صفحهٔ «متن به گلدان» (نسخهٔ ۱٫۷)
 * ============================================================================
 * درخواست کاربر ۱۱: نوشتن متن → تبدیل به حالت vase-mode با اتصال نامرئی
 * حروف، پشتیبانی فونت‌های ویندوز + فارسی (RTL، حروف متصل و غیربرعکس)
 * و تنظیمات کامل و حرفه‌ای.
 *
 * ساختار صفحه:
 *   * فرم متن (متن/فونت/وزن/ارتفاع حروف/هم‌جوشی/نوار اتصال/کیفیت)؛
 *   * پیش‌نمایش ۲بعدی زنده (بوم رندر + کانتورهای استخراج‌شده)؛
 *   * پیش‌نمایش ۳بعدی (همان Viewport برنامه با segments واقعی هسته)؛
 *   * تنظیمات چاپ (ارتفاع دیواره/کف/سرعت) + پروفایل پرینتر جاری؛
 *   * خروجی G-code (کیفیت انتخابی) + ذخیرهٔ مستقیم.
 * ============================================================================
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  text2vaseCompute,
  text2vaseExportGcode,
  saveFileDialog,
  writeTextFile,
  coreAvailable,
  type PrinterProfile,
  type SegmentsPayload,
  type TextVaseParams,
} from "../../lib/tauri";
import { Viewport } from "../../components/Viewport";
import {
  FONT_SUGGESTIONS,
  extractTextContours,
  type Contour,
} from "./contours";

/** حالت فرم — منبع واحد صفحه */
interface TextForm {
  text: string;
  fontFamily: string;
  fontWeight: number;
  targetHeightMm: number;
  fusionStrokePercent: number;
  connectorBarPercent: number;
  simplifyTolerancePx: number;
  pxPerMm: number;
  // تنظیمات چاپ
  height_mm: number;
  floor_layers: number;
  speed_scale: number;
  quality: number; // کیفیت G-code (۰٫۵..۳)
}

const DEFAULT_FORM: TextForm = {
  text: "Sahand",
  fontFamily: "Vazirmatn",
  fontWeight: 700,
  targetHeightMm: 30,
  fusionStrokePercent: 2.4,
  connectorBarPercent: 7,
  simplifyTolerancePx: 1.2,
  pxPerMm: 8,
  height_mm: 40,
  floor_layers: 2,
  speed_scale: 0.8,
  quality: 1,
};

export function TextVasePage(props: {
  printer: PrinterProfile;
  onGcode?: (gcode: string, note: string) => void;
}) {
  const { printer } = props;
  const [form, setForm] = useState<TextForm>(DEFAULT_FORM);
  const [contours, setContours] = useState<Contour[]>([]);
  const [bounds, setBounds] = useState<{ w: number; h: number }>({ w: 0, h: 0 });
  const [params, setParams] = useState<TextVaseParams | null>(null);
  const [viewport, setViewport] = useState<{
    positions: number[];
    kinds: number[];
    feedrates: number[];
    summary_fa: string;
    stats: { extrusion_length_mm: number; total_e_mm: number } | null;
    diagnostics: { severity_fa: string; code: string; message_fa: string }[];
    bbox: [number, number, number, number];
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const debounceRef = useRef<number | null>(null);
  // رفع ۱٫۷٫۱: ترتیب پاسخ‌های IPC — پاسخ کهنهٔ محاسبهٔ قبلی نباید پیش‌نمایش
  // تازه را بازنویسی کند (تایپ سریع ⇒ چند compute هم‌زمان در پرواز)
  const computeSeqRef = useRef(0);

  const set = <K extends keyof TextForm>(k: K, v: TextForm[K]) =>
    setForm((f) => ({ ...f, [k]: v }));

  // ---------- استخراج کانتور (debounce) ----------
  // رفع ۱٫۷٫۱: قبل از رندر بوم، فونت انتخابی با document.fonts.load آماده
  // می‌شود — وگرنه در اولین استخراجِ وزیرمتنِ باندل‌شده، @font-face هنوز
  // بارگذاری نشده و کانتور از فونت جایگزین گرفته می‌شد (پیش‌نمایش ناهمگون).
  useEffect(() => {
    if (debounceRef.current !== null) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(async () => {
      try {
        const spec = `${form.fontWeight} 64px "${form.fontFamily}", "Vazirmatn"`;
        try {
          await document.fonts.load(spec, form.text || "Sahand");
        } catch {
          /* فونت سیستم/نامعتبر — رندر با جایگزین، اشکالی ندارد */
        }
        const r = extractTextContours({
          text: form.text,
          fontFamily: form.fontFamily,
          fontWeight: form.fontWeight,
          targetHeightMm: form.targetHeightMm,
          fusionStrokePercent: form.fusionStrokePercent,
          connectorBarPercent: form.connectorBarPercent,
          simplifyTolerancePx: form.simplifyTolerancePx,
          pxPerMm: form.pxPerMm,
        });
        setContours(r.contours);
        setBounds(r.boundsMm);
      } catch (e) {
        setStatus(`خطای استخراج کانتور: ${String(e)}`);
      }
    }, 240);
    return () => {
      if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    };
  }, [
    form.text,
    form.fontFamily,
    form.fontWeight,
    form.targetHeightMm,
    form.fusionStrokePercent,
    form.connectorBarPercent,
    form.simplifyTolerancePx,
    form.pxPerMm,
  ]);

  // ---------- محاسبه در هسته (رفع ۱٫۷٫۱) ----------
  // قبلاً گیت runningInDesktop() بود و تابع هم invoke خام می‌زد ⇒ در dev
  // مرورگر پیش‌نمایش هیچ‌وقت ساخته نمی‌شد. حالا گیت coreAvailable() است
  // (دسکتاپ یا پل توسعه) و مسیر سه‌گانهٔ callCore رعایت می‌شود.
  useEffect(() => {
    if (contours.length === 0) {
      setParams(null);
      setViewport(null);
      return;
    }
    const p: TextVaseParams = {
      contours,
      height_mm: form.height_mm,
      floor_layers: form.floor_layers,
      quality_step_mm: 1.1,
      center_on_bed: true,
      speed_scale: form.speed_scale,
      printer,
    };
    setParams(p);
    if (!coreAvailable()) {
      // محیط تولیدی مرورگر — بدون هستهٔ Rust محاسبه نمی‌شود (صداقت)
      setViewport(null);
      return;
    }
    let alive = true;
    const seq = ++computeSeqRef.current;
    text2vaseCompute(p)
      .then((v) => {
        if (!alive || seq !== computeSeqRef.current) return; // پاسخ کهنه
        setViewport({
          positions: v.positions,
          kinds: v.kinds,
          feedrates: v.feedrates,
          summary_fa: v.summary_fa,
          stats: v.stats,
          diagnostics: v.diagnostics,
          bbox: v.bbox,
        });
      })
      .catch((e) => {
        if (!alive) return;
        setStatus(`خطای محاسبه: ${String(e)}`);
      });
    return () => {
      alive = false;
    };
  }, [contours, form.height_mm, form.floor_layers, form.speed_scale, printer]);

  // ---------- پیش‌نمایش ۲بعدی ----------
  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    const W = cv.width;
    const H = cv.height;
    ctx.clearRect(0, 0, W, H);
    if (contours.length === 0) {
      ctx.fillStyle = "#7d8392";
      ctx.font = "13px Vazirmatn, sans-serif";
      ctx.direction = "rtl";
      ctx.textAlign = "center";
      ctx.fillText("متنی بنویسید…", W / 2, H / 2);
      return;
    }
    // مقیاس برازش
    const spanX = bounds.w || 1;
    const spanY = bounds.h || 1;
    const sc = Math.min((W - 24) / spanX, (H - 24) / spanY);
    const ox = W / 2;
    const oy = H / 2;
    // پرکردن شکل (even-odd)
    ctx.save();
    ctx.beginPath();
    for (const c of contours) {
      c.points.forEach(([x, y], i) => {
        const px = ox + x * sc;
        const py = oy - y * sc;
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      });
      ctx.closePath();
    }
    ctx.fillStyle = "rgba(70, 200, 120, 0.16)";
    ctx.fill("evenodd");
    ctx.restore();
    // خطوط کانتور: بیرونی پررنگ، حفره نقطه‌چین
    for (const c of contours) {
      ctx.beginPath();
      c.points.forEach(([x, y], i) => {
        const px = ox + x * sc;
        const py = oy - y * sc;
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      });
      ctx.closePath();
      ctx.strokeStyle = c.is_hole ? "#5a87d6" : "#46c878";
      ctx.lineWidth = c.is_hole ? 1 : 1.6;
      if (c.is_hole) ctx.setLineDash([4, 3]);
      ctx.stroke();
      ctx.setLineDash([]);
    }
  }, [contours, bounds]);

  // ---------- خروجی G-code ----------
  const exportGcode = useCallback(async () => {
    if (!params || contours.length === 0) {
      setStatus("کانتوری برای خروجی نیست");
      return;
    }
    setBusy(true);
    setStatus("");
    try {
      const gcode = await text2vaseExportGcode(params, form.quality);
      const path = await saveFileDialog(
        `text-vase-${form.text.trim().slice(0, 16) || "text"}.gcode`,
        "gcode"
      );
      if (!path) {
        setBusy(false);
        return;
      }
      const n = await writeTextFile(path, gcode);
      setStatus(`ذخیره شد: ${path} (${n.toLocaleString("fa-IR")} بایت)`);
    } catch (e) {
      setStatus(`خطای خروجی: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }, [params, contours.length, form.quality, form.text]);

  // ---------- ارسال به ویرایشگر ----------
  const sendToEditor = useCallback(async () => {
    if (!params || contours.length === 0) return;
    setBusy(true);
    try {
      const gcode = await text2vaseExportGcode(params, form.quality);
      props.onGcode?.(gcode, `متن: ${form.text.trim().slice(0, 24)}`);
      setStatus("به ویرایشگر G-code ارسال شد");
    } catch (e) {
      setStatus(`خطا: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }, [params, contours.length, form.quality, form.text, props]);

  // ---------- payload viewport ----------
  const segPayload: SegmentsPayload | null = useMemo(() => {
    if (!viewport || viewport.positions.length === 0) return null;
    return {
      revision: 1,
      positions: viewport.positions,
      kinds: viewport.kinds,
      source_lines: [],
      ids: [],
      certainties: [],
      feedrates: viewport.feedrates,
    };
  }, [viewport]);

  const nHoles = contours.filter((c) => c.is_hole).length;

  return (
    <div className="designer-page">
      {/* ================= پنل تنظیمات =================
          رفع ۱٫۷٫۱: ستون‌ها از CSS (.designer-page) می‌آیند — قبلاً
          inline gridTemplateColumns بود و چون کلاسی در CSS وجود نداشت،
          ارتفاع گرید نامحدود می‌شد: پنل اسکرول نمی‌شد و پیش‌نمایش ۳بعدی
          (flex:1) به ارتفاع صفر فرو می‌افتاد. */}
      <div className="designer-side">
        <div className="designer-head">
          <h3 style={{ marginTop: 0, color: "var(--accent)", marginBottom: 0 }}>
            ✍️ متن به گلدان (vase-mode)
          </h3>
        </div>

        <div className="field">
          <label>متن (فارسی/لاتین)</label>
          <textarea
            rows={2}
            dir="rtl"
            value={form.text}
            onChange={(e) => set("text", e.target.value)}
            placeholder="مثلاً: Sahand یا سعید"
            style={{ fontFamily: "Vazirmatn", fontSize: 16 }}
          />
          <span className="hint-text">
            فارسی خودکار RTL و متصل رندر می‌شود — حروف جدا/برعکس نمی‌شوند
          </span>
        </div>

        <div className="field">
          <label>فونت (هر فونت ویندوزی)</label>
          <select
            className="ltr"
            value={form.fontFamily}
            onChange={(e) => set("fontFamily", e.target.value)}
          >
            {FONT_SUGGESTIONS.map((f) => (
              <option key={f.family} value={f.family}>
                {f.label}
              </option>
            ))}
          </select>
          <input
            type="text"
            className="ltr"
            placeholder="یا نام دقیق فونت نصب‌شده…"
            style={{ marginTop: 4 }}
            value={FONT_SUGGESTIONS.some((f) => f.family === form.fontFamily) ? "" : form.fontFamily}
            onChange={(e) => set("fontFamily", e.target.value || "Vazirmatn")}
          />
        </div>

        <div className="field">
          <label>وزن فونت</label>
          <select value={form.fontWeight} onChange={(e) => set("fontWeight", +e.target.value)}>
            <option value={300}>نازک ۳۰۰</option>
            <option value={400}>معمولی ۴۰۰</option>
            <option value={500}>متوسط ۵۰۰</option>
            <option value={700}>ضخیم ۷۰۰</option>
            <option value={800}>خیلی ضخیم ۸۰۰</option>
            <option value={900}>سیاه ۹۰۰</option>
          </select>
          <span className="hint-text">ضخیم‌تر = حروف مقاوم‌تر و جوش بهتر</span>
        </div>

        <div className="field">
          <label>ارتفاع حروف (mm)</label>
          <input type="number" step="1" min="8" max="120"
            value={form.targetHeightMm}
            onChange={(e) => set("targetHeightMm", Math.max(8, +e.target.value || 30))} />
        </div>

        <div className="field">
          <label>هم‌جوشی نامرئی حروف (٪ ارتفاع)</label>
          <input type="number" step="0.2" min="0" max="12"
            value={form.fusionStrokePercent}
            onChange={(e) => set("fusionStrokePercent", Math.min(12, Math.max(0, +e.target.value || 0)))} />
          <span className="hint-text">
            ضخامت جوش لبه‌ها — حروف کنار هم در یک شکل یکپارچه ذوب می‌شوند و
            جای اتصال دیده نمی‌شود (۰ = خاموش)
          </span>
        </div>

        <div className="field">
          <label>نوارهای اتصال حروف دور (٪ ارتفاع)</label>
          <input type="number" step="0.5" min="0" max="30"
            value={form.connectorBarPercent}
            onChange={(e) => set("connectorBarPercent", Math.min(30, Math.max(0, +e.target.value || 0)))} />
          <span className="hint-text">شکاف‌های ریز بین حروف را پر می‌کند (۰ = خاموش)</span>
        </div>

        <div className="field">
          <label>کیفیت کانتور (px بر mm)</label>
          <input type="number" step="1" min="4" max="16"
            value={form.pxPerMm}
            onChange={(e) => set("pxPerMm", Math.min(16, Math.max(4, +e.target.value || 8)))} />
          <span className="hint-text">بیشتر = لبه‌های نرم‌تر (کمی کندتر)</span>
        </div>

        <div className="field">
          <label>ساده‌سازی لبه (px)</label>
          <input type="number" step="0.1" min="0.4" max="4"
            value={form.simplifyTolerancePx}
            onChange={(e) => set("simplifyTolerancePx", Math.max(0.4, +e.target.value || 1.2))} />
          <span className="hint-text">بیشتر = نقاط کمتر و فایل سبک‌تر</span>
        </div>

        <div className="field">
          <label>قد دیواره‌ها (mm)</label>
          <input type="number" step="1" min="4" max="200"
            value={form.height_mm}
            onChange={(e) => set("height_mm", Math.max(4, +e.target.value || 40))} />
        </div>

        <div className="field">
          <label>لایه‌های کف</label>
          <input type="number" step="1" min="0" max="8"
            value={form.floor_layers}
            onChange={(e) => set("floor_layers", Math.min(8, Math.max(0, Math.round(+e.target.value) || 0)))} />
          <span className="hint-text">۲ = استاندارد (کف حروف روی میز می‌نشیند)</span>
        </div>

        <div className="field">
          <label>سرعت نسبی چاپ متن (۰٫۳..۱)</label>
          <input type="number" step="0.05" min="0.3" max="1"
            value={form.speed_scale}
            onChange={(e) => set("speed_scale", Math.min(1, Math.max(0.3, +e.target.value || 0.8)))} />
          <span className="hint-text">حروف ریز = آرام‌تر برای دقت</span>
        </div>

        <div className="field">
          <label>کیفیت G-code</label>
          <select value={form.quality} onChange={(e) => set("quality", +e.target.value)}>
            <option value={0.5}>زبر (فایل کوچک)</option>
            <option value={1}>استاندارد</option>
            <option value={2}>ریز</option>
            <option value={3}>بسیار ریز</option>
          </select>
        </div>

        {/* نوار اقدام چسبان — رفع ۱٫۷٫۱: همیشه در دید، حتی وقتی پنل
            تنظیمات دراز است و کاربر پایین اسکرول نکرده (باگ «دکمه
            بیرون از دسترس») */}
        <div
          className="textvase-actions"
          style={{
            position: "sticky",
            bottom: 0,
            display: "flex",
            gap: 6,
            marginTop: 10,
            padding: "10px 0 2px",
            background: "var(--bg-1)",
            borderTop: "1px solid var(--line)",
            zIndex: 2,
          }}
        >
          <button className="btn primary" disabled={busy || contours.length === 0} onClick={exportGcode}>
            💾 ذخیرهٔ G-code
          </button>
          {/* رفع ۱٫۷٫۱: ارسال به ویرایشگر فقط به هسته نیاز دارد، نه پوسته —
              در dev با پل CLI هم کار می‌کند */}
          <button className="btn" disabled={busy || contours.length === 0 || !coreAvailable()} onClick={sendToEditor}>
            ➡️ ارسال به ویرایشگر
          </button>
        </div>
        {status && <p className="hint-text" style={{ marginTop: 8 }}>{status}</p>}
      </div>

      {/* ================= پیش‌نمایش ================= */}
      <div className="designer-main">
        <div style={{ display: "flex", flexDirection: "column", gap: 10, height: "100%" }}>
          <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
            <span className="hint-text">
              {contours.length > 0
                ? `${contours.length.toLocaleString("fa-IR")} کانتور (${nHoles.toLocaleString("fa-IR")} حفره) · متن ${bounds.w.toFixed(0)}×${bounds.h.toFixed(0)} mm`
                : "بدون متن"}
            </span>
            {viewport?.summary_fa && (
              <span className="hint-text" style={{ color: "var(--accent)" }}>
                {viewport.summary_fa}
              </span>
            )}
          </div>
          {viewport?.diagnostics && viewport.diagnostics.length > 0 && (
            <div className="diag-list">
              {viewport.diagnostics.slice(0, 4).map((d, i) => (
                <div key={i} className={`diag ${d.severity_fa}`}>
                  {d.message_fa}
                </div>
              ))}
            </div>
          )}
          <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
            <Viewport
              segments={segPayload}
              highlightIds={new Set<number>()}
              filter={{ mode: "all", upToId: 0 }}
              buildVolume={printer.build_volume_mm}
              emptyMessage={
                coreAvailable()
                  ? "متن را بنویسید تا پیش‌نمایش سه‌بعدی ساخته شود"
                  : "پیش‌نمایش هستهٔ Rust فقط در برنامهٔ دسکتاپ یا حالت توسعه فعال است"
              }
              webglMessage="مرورگر WebGL را پشتیبانی نمی‌کند"
            />
          </div>
          <canvas
            ref={canvasRef}
            width={520}
            height={180}
            style={{
              width: "100%",
              height: 180,
              background: "var(--bg-1)",
              border: "1px solid var(--line)",
              borderRadius: "var(--radius)",
            }}
            aria-label="پیش‌نمایش دوبعدی کانتور متن"
          />
        </div>
      </div>
    </div>
  );
}
