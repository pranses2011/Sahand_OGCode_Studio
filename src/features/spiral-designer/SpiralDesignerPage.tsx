/**
 * ============================================================================
 * features/spiral-designer/SpiralDesignerPage.tsx — طراح مارپیچی ۵ گامی
 * ============================================================================
 * گردش‌کار (Docs/PROMPT_03): ۱. پروفایل ۲. الگو ۳. بافت ۴. کف ۵. پرینتر
 * + پیش‌نمایش زنده سه‌بعدی + خروجی G-code/STL/پروژه + ارسال به ویرایشگر.
 *
 * تمایز حیاتی در UI (تأیید از پژوهش مرجع):
 *   الگو → هندسه (در STL می‌ماند)؛ بافت → مسیر نازل (فقط در G-code).
 *   این تمایز در متن پنل‌ها صراحتاً توضیح داده می‌شود (الزام §3 گام ۳).
 * ============================================================================
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ProfileEditor } from "./ProfileEditor";
import { Viewport } from "../../components/Viewport";
import { fa } from "../../i18n/fa";
import type {
  DesignerProject, PatternParams, TextureParams, DesignerOutput,
  SegmentsPayload, ProfilePoint, PrinterProfile,
} from "../../lib/tauri";
import {
  designerCompute, designerExportGcode, designerExportStl,
  saveProject, loadProject, saveFileDialog, openFileDialog,
  coreAvailable, printerPresets,
} from "../../lib/tauri";

/** پروژهٔ پیش‌فرض — گلدان آماده (قابل‌ویرایش) */
function defaultProject(): DesignerProject {
  return {
    schema_version: 1,
    profile: [
      { r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 },
      { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 },
    ],
    form_height_mm: 100,
    form_bottom_diameter_mm: 44,
    form_top_diameter_mm: 42,
    taper_percent: 0,
    patterns: [],
    textures: [],
    floor: { enabled: false, center_hole_diameter_mm: 0, z_start_mm: 0, ring_gap_mm: 0.45 },
    printer: {
      name: "Custom / Unverified", status: "unverified",
      build_volume_mm: [220, 220, 250],
      nozzle_diameter_mm: 0.4, filament_diameter_mm: 1.75,
      line_width_mm: 0.45, layer_height_mm: 0.25,
      print_speed_mm_min: 2400, travel_speed_mm_min: 6000,
      flow_multiplier: 1.0, absolute_e: true,
      start_gcode: "", end_gcode: "",
    },
    created_at_unix: 0,
    note: "",
  };
}

/** تبدیل خروجی طراح به payload viewport — همان IR واحد
 *  توجه: SegmentKind/Certainty در Rust به‌صورت رشتهٔ نام variant سریال می‌شوند
 *  (enum بدون داده)؛ اینجا به کد عددی viewport نگاشت می‌شوند. */
const KIND_NUM: Record<string, number> = {
  Extrusion: 0, Travel: 1, Retract: 2, Unretract: 3, Arc: 4, Unknown: 5,
};
const CERTAINTY_NUM: Record<string, number> = {
  Known: 0, Approximate: 1, Unknown: 2,
};

function toSegmentsPayload(out: DesignerOutput): SegmentsPayload {
  const n = out.segments.length;
  const positions: number[] = new Array(n * 6);
  const kinds: number[] = new Array(n);
  const source_lines: number[] = new Array(n);
  const ids: number[] = new Array(n);
  const certainties: number[] = new Array(n);
  const feedrates: number[] = new Array(n);
  for (let i = 0; i < n; i++) {
    const s = out.segments[i];
    const start = s.start_mm ?? [0, 0, 0];
    const end = s.end_mm ?? start;
    positions.splice(i * 6, 6, ...start, ...end);
    kinds[i] = KIND_NUM[String(s.kind)] ?? 5;
    source_lines[i] = s.source_line_index;
    ids[i] = s.id;
    certainties[i] = CERTAINTY_NUM[String(s.certainty)] ?? 2;
    feedrates[i] = s.feedrate_mm_min ?? NaN;
  }
  return { revision: 1, positions, kinds, source_lines, ids, certainties, feedrates };
}

export function SpiralDesignerPage(props: {
  onSendToEditor: (gcode: string) => void;
  /** رنگ رشتهٔ چاپ در پیش‌نمایش (نمایشی — از ColorBar) */
  extrusionColor?: string;
}) {
  const [project, setProject] = useState<DesignerProject>(defaultProject);
  const [output, setOutput] = useState<DesignerOutput | null>(null);
  const [step, setStep] = useState(1);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");
  // پریست‌های ۲۰ پرینتر واقعی — از هستهٔ Rust بارگذاری می‌شود
  const [presets, setPresets] = useState<PrinterProfile[]>([]);
  // پرسش تأیید خروجی‌های دارای هشدار بحرانی (الزام §8)
  const [confirmExport, setConfirmExport] = useState<null | (() => void)>(null);
  const debounceRef = useRef<number | null>(null);

  // بارگذاری پریست‌های پرینتر از هسته (دسکتاپ یا پل dev)
  useEffect(() => {
    if (!coreAvailable()) return;
    printerPresets()
      .then((list) => setPresets(list))
      .catch(() => setPresets([])); // خطا → فهرست خالی و پیام گام ۵ همچنان صادقانه
  }, []);

  // ------------------------------------------------------------ محاسبهٔ زنده
  const recompute = useCallback(async (p: DesignerProject) => {
    if (!coreAvailable()) {
      setNote("محاسبه فقط در برنامهٔ دسکتاپ انجام می‌شود.");
      return;
    }
    try {
      const out = await designerCompute(p);
      setOutput(out);
      setNote(out.summary_fa);
    } catch (e) {
      setNote(`خطا: ${e}`);
    }
  }, []);

  useEffect(() => {
    // debounce پارامترها (الزام §6: «هر تغییر، نما و IR را همگام به‌روز کند»)
    if (debounceRef.current !== null) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => recompute(project), 300);
    return () => { if (debounceRef.current !== null) clearTimeout(debounceRef.current); };
  }, [project, recompute]);

  // محاسبهٔ اولیه
  useEffect(() => { recompute(project); /* eslint-disable-next-line */ }, []);

  // ------------------------------------------------------------ helpers
  const setP = (patch: Partial<DesignerProject>) => setProject((p) => ({ ...p, ...patch }));
  const setPrinter = (patch: Partial<DesignerProject["printer"]>) =>
    setProject((p) => ({ ...p, printer: { ...p.printer, ...patch } }));
  const setFloor = (patch: Partial<DesignerProject["floor"]>) =>
    setProject((p) => ({ ...p, floor: { ...p.floor, ...patch } }));

  const addPattern = (kind: PatternParams["kind"]) =>
    setProject((p) => ({
      ...p,
      patterns: [...p.patterns, {
        kind, amplitude_mm: 1.5, count: 8, frequency: 1,
        phase_deg: 0, z_start_mm: 0, z_end_mm: null,
      }],
    }));
  const updatePattern = (i: number, patch: Partial<PatternParams>) =>
    setProject((p) => ({
      ...p,
      patterns: p.patterns.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    }));
  const removePattern = (i: number) =>
    setProject((p) => ({ ...p, patterns: p.patterns.filter((_, j) => j !== i) }));

  const addTexture = (kind: TextureParams["kind"]) =>
    setProject((p) => ({
      ...p,
      textures: [...p.textures, {
        kind, amplitude_mm: 0.4, period_mm: 4, radial: true,
        phase_deg: 0, fade_in_percent: 0, fade_out_percent: 0,
      }],
    }));
  const updateTexture = (i: number, patch: Partial<TextureParams>) =>
    setProject((p) => ({
      ...p,
      textures: p.textures.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    }));
  const removeTexture = (i: number) =>
    setProject((p) => ({ ...p, textures: p.textures.filter((_, j) => j !== i) }));

  const patternName: Record<PatternParams["kind"], string> = {
    Wave: "موج محوری", Petal: "گلبرگ", Twist: "پیچش", Flute: "شیار عمودی",
    Diamond: "لوزی", Chevron: "مهری (زیگزاگ)", Rings: "حلقهٔ افقی",
    Scale: "فلس", Facet: "تراش‌خورده",
  };
  const textureName: Record<TextureParams["kind"], string> = {
    Ripple: "موجک", Weave: "تور بافته", Stitch: "بخیه‌ای",
    SpiralGroove: "شیار مارپیچی", Hammered: "چکشی", Basket: "سبدی",
  };

  /** نازل‌های رایج (پیشنهادی نه دادهٔ ثابت — از پژوهش مرجع) */
  const nozzlePresets = [0.4, 0.6, 0.8];

  const maxR = Math.max(...project.profile.map((p) => p.r), 10) * 1.2;

  // ------------------------------------------------------------ خروجی‌ها
  const exportGcode = async () => {
    setBusy(true);
    try {
      const gcode = await designerExportGcode(project);
      props.onSendToEditor(gcode);
      setNote("G-code تولید و به ویرایشگر ارسال شد.");
    } catch (e) {
      setNote(`خطا در خروجی: ${e}`);
    }
    setBusy(false);
  };

  const exportStl = async () => {
    const path = await saveFileDialog("spiral-form.stl", "stl");
    if (!path) return;
    // الزام §8: تأیید اگر هشدار بحرانی
    const hasCritical = output?.diagnostics.some(
      (d) => d.severity === "Error" || d.code.startsWith("W-DESIGN")
    );
    const doExport = async () => {
      setConfirmExport(null);
      setBusy(true);
      try {
        const r = await designerExportStl(project, path);
        setNote(`STL ذخیره شد: ${r.path} (${r.triangle_count} مثلث)`);
      } catch (e) {
        setNote(`خطا: ${e}`);
      }
      setBusy(false);
    };
    if (hasCritical) {
      setConfirmExport(() => doExport);
    } else {
      await doExport();
    }
  };

  const doSaveProject = async () => {
    const path = await saveFileDialog("project.gcsproj.json", "gcsproj.json");
    if (!path) return;
    try {
      await saveProject({ ...project, created_at_unix: Math.floor(Date.now() / 1000) }, path);
      setNote(`پروژه ذخیره شد: ${path}`);
    } catch (e) {
      setNote(`خطا: ${e}`);
    }
  };

  const doLoadProject = async () => {
    const path = await openFileDialog();
    if (!path) return;
    try {
      const p = await loadProject(path);
      setProject(p);
      setNote("پروژه بارگذاری شد.");
    } catch (e) {
      // schema خراب → پروژهٔ جاری دست‌نخورده می‌ماند (الزام §7)
      setNote(`بارگذاری ناموفق: ${e} — پروژهٔ جاری تغییر نکرد.`);
    }
  };

  const segPayload = output ? toSegmentsPayload(output) : null;
  const warningsFa = output?.diagnostics.map((d) => `${d.severity === "Error" ? "خطا" : "هشدار"} [${d.code}]: ${d.message_fa}`) ?? [];

  // ------------------------------------------------------------ رندر
  return (
    <div className="designer-layout">
      {/* ---------- پنل کناری ۵ گام ---------- */}
      <div className="designer-side">
        <h3 style={{ marginTop: 0, color: "var(--accent)" }}>{fa.designerTitle}</h3>

        <div className="steps">
          {[1, 2, 3, 4, 5].map((s) => (
            <button
              key={s}
              className={`step-btn ${step === s ? "active" : ""}`}
              onClick={() => setStep(s)}
            >
              {s === 1 ? fa.step1Profile : s === 2 ? fa.step2Pattern
                : s === 3 ? fa.step3Texture : s === 4 ? fa.step4Floor : fa.step5Printer}
            </button>
          ))}
        </div>

        {/* ================= گام ۱: پروفایل ================= */}
        {step === 1 && (
          <div>
            <ProfileEditor
              points={project.profile}
              height={project.form_height_mm}
              maxR={maxR}
              onChange={(pts: ProfilePoint[]) => setP({ profile: pts })}
            />
            <div className="field">
              <label>{fa.presets}:</label>
              <div style={{ display: "flex", gap: 6 }}>
                <button className="btn small" onClick={() => setP({ profile: [
                  { r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 },
                  { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 },
                ] })}>{fa.presetVase}</button>
                <button className="btn small" onClick={() => setP({ profile: [
                  { r: 8, h: 0 }, { r: 15, h: 15 }, { r: 35, h: 90 }, { r: 40, h: 100 },
                ] })}>{fa.presetLamp}</button>
                <button className="btn small" onClick={() => setP({ profile: [
                  { r: 15, h: 0 }, { r: 18, h: 5 }, { r: 30, h: 60 }, { r: 32, h: 65 },
                ] })}>{fa.presetPlanter}</button>
              </div>
            </div>
            <div className="field">
              <label>{fa.height}</label>
              <input type="number" step="1" min="5" max="400"
                value={project.form_height_mm}
                onChange={(e) => setP({ form_height_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.bottomDiameter}</label>
              <input type="number" step="1" min="2"
                value={project.form_bottom_diameter_mm}
                onChange={(e) => setP({ form_bottom_diameter_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.topDiameter}</label>
              <input type="number" step="1" min="2"
                value={project.form_top_diameter_mm}
                onChange={(e) => setP({ form_top_diameter_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.taper}</label>
              <input type="number" step="1" min="0" max="95"
                value={project.taper_percent}
                onChange={(e) => setP({ taper_percent: +e.target.value || 0 })} />
            </div>
          </div>
        )}

        {/* ================= گام ۲: الگو ================= */}
        {step === 2 && (
          <div>
            <p style={{ color: "var(--fg-1)", fontSize: 12 }}>
              الگو <strong>هندسهٔ مدل</strong> را تغییر می‌دهد؛ اثر آن در <strong>STL</strong> هم باقی می‌ماند.
            </p>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {(Object.keys(patternName) as PatternParams["kind"][]).map((k) => (
                <button key={k} className="btn small" onClick={() => addPattern(k)}>
                  + {patternName[k]}
                </button>
              ))}
            </div>
            {project.patterns.map((pat, i) => (
              <div className="pattern-item" key={i}>
                <div className="head">
                  <span className="name">{patternName[pat.kind]}</span>
                  <button className="btn small danger" onClick={() => removePattern(i)}>×</button>
                </div>
                <div className="field"><label>{fa.patternAmplitude}</label>
                  <input type="number" step="0.1" min="0" value={pat.amplitude_mm}
                    onChange={(e) => updatePattern(i, { amplitude_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.patternCount}</label>
                  <input type="number" step="1" min="1" max="64" value={pat.count}
                    onChange={(e) => updatePattern(i, { count: Math.round(+e.target.value) || 1 })} /></div>
                <div className="field"><label>{fa.patternFrequency}</label>
                  <input type="number" step="0.5" min="0" value={pat.frequency}
                    onChange={(e) => updatePattern(i, { frequency: +e.target.value })} /></div>
                <div className="field"><label>{fa.patternPhase}</label>
                  <input type="number" step="15" value={pat.phase_deg}
                    onChange={(e) => updatePattern(i, { phase_deg: +e.target.value })} /></div>
                <div className="field"><label>شروع بازه (mm)</label>
                  <input type="number" step="5" min="0" value={pat.z_start_mm}
                    onChange={(e) => updatePattern(i, { z_start_mm: +e.target.value })} /></div>
                <div className="field"><label>پایان بازه (mm؛ خالی = تا بالا)</label>
                  <input type="number" step="5" value={pat.z_end_mm ?? ""}
                    onChange={(e) => updatePattern(i, {
                      z_end_mm: e.target.value === "" ? null : +e.target.value,
                    })} /></div>
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۳: بافت ================= */}
        {step === 3 && (
          <div>
            <p style={{ color: "var(--fg-1)", fontSize: 12 }}>
              بافت <strong>مسیر حرکت نازل</strong> را تعدیل می‌کند و در STL ذخیره <strong>نمی‌شود</strong>؛
              فقط در G-code و پیش‌نمایش مسیر دیده می‌شود.
            </p>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {(Object.keys(textureName) as TextureParams["kind"][]).map((k) => (
                <button key={k} className="btn small" onClick={() => addTexture(k)}>
                  + {textureName[k]}
                </button>
              ))}
            </div>
            {project.textures.map((tex, i) => (
              <div className="texture-item" key={i}>
                <div className="head">
                  <span className="name">{textureName[tex.kind]}</span>
                  <button className="btn small danger" onClick={() => removeTexture(i)}>×</button>
                </div>
                <div className="field"><label>{fa.textureAmplitude}</label>
                  <input type="number" step="0.05" min="0" value={tex.amplitude_mm}
                    onChange={(e) => updateTexture(i, { amplitude_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.texturePeriod}</label>
                  <input type="number" step="0.5" min="0.2" value={tex.period_mm}
                    onChange={(e) => updateTexture(i, { period_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.textureDirection}</label>
                  <select value={tex.radial ? "r" : "z"}
                    onChange={(e) => updateTexture(i, { radial: e.target.value === "r" })}>
                    <option value="r">{fa.textureRadial}</option>
                    <option value="z">{fa.textureVertical}</option>
                  </select></div>
                <div className="field"><label>{fa.fade} ↓%</label>
                  <input type="number" step="5" min="0" max="100" value={tex.fade_in_percent}
                    onChange={(e) => updateTexture(i, { fade_in_percent: +e.target.value })} /></div>
                <div className="field"><label>{fa.fade} ↑%</label>
                  <input type="number" step="5" min="0" max="100" value={tex.fade_out_percent}
                    onChange={(e) => updateTexture(i, { fade_out_percent: +e.target.value })} /></div>
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۴: کف ================= */}
        {step === 4 && (
          <div>
            <p style={{ color: "var(--fg-1)", fontSize: 12 }}>
              حداقل دو گزینه: «بدون کف» و «کف هم‌مرکز با سوراخ مرکزی قابل‌تنظیم».
            </p>
            <label style={{ display: "flex", gap: 8, alignItems: "center", margin: "10px 0" }}>
              <input type="checkbox" checked={project.floor.enabled}
                onChange={(e) => setFloor({ enabled: e.target.checked })} />
              {fa.floorEnabled}
            </label>
            <div className="field"><label>{fa.floorHole}</label>
              <input type="number" step="1" min="0" disabled={!project.floor.enabled}
                value={project.floor.center_hole_diameter_mm}
                onChange={(e) => setFloor({ center_hole_diameter_mm: +e.target.value })} /></div>
            <div className="field"><label>فاصلهٔ حلقه‌های کف (mm)</label>
              <input type="number" step="0.05" min="0.2" disabled={!project.floor.enabled}
                value={project.floor.ring_gap_mm}
                onChange={(e) => setFloor({ ring_gap_mm: +e.target.value })} /></div>
          </div>
        )}

        {/* ================= گام ۵: پرینتر ================= */}
        {step === 5 && (
          <div>
            <p style={{ color: "var(--fg-2)", fontSize: 11 }}>
              {fa.printerProfileStatus}: <strong className="ltr">{project.printer.status}</strong> —
              presetها دادهٔ ثابت نیستند؛ قبل از چاپ صحت‌سنجی کنید.
            </p>
            {/* انتخاب از ۲۰ پریست پرینتر واقعی (وضعیت community) */}
            <div className="field">
              <label>{fa.printerPresets}</label>
              <select
                className="ltr"
                value={presets.some((x) => x.name === project.printer.name) ? project.printer.name : ""}
                onChange={(e) => {
                  const p = presets.find((x) => x.name === e.target.value);
                  if (p) setProject((prev) => ({ ...prev, printer: { ...p } }));
                }}
              >
                <option value="">— انتخاب از فهرست —</option>
                {presets.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.name} — {p.build_volume_mm[0]}×{p.build_volume_mm[1]}×{p.build_volume_mm[2]} mm
                  </option>
                ))}
              </select>
              {presets.length > 0 && (
                <span style={{ color: "var(--fg-2)", fontSize: 11 }}>
                  {presets.length} مدل — مشخصات از صفحات رسمی سازنده‌ها؛
                  وضعیت همه community است (تست دستی نداده‌ایم).
                </span>
              )}
            </div>
            {/* نمایش نام/برند پرینتر جاری وقتی از فهرست انتخاب شده */}
            {presets.some((x) => x.name === project.printer.name) && (
              <div className="printer-badge">
                <span className="dot" aria-hidden="true" /> {project.printer.name}
              </div>
            )}
            <div className="field"><label>{fa.nozzleDiameter}</label>
              <div style={{ display: "flex", gap: 4 }}>
                {nozzlePresets.map((n) => (
                  <button key={n} className="btn small ltr"
                    style={{ borderColor: project.printer.nozzle_diameter_mm === n ? "var(--accent)" : undefined }}
                    onClick={() => setPrinter({ nozzle_diameter_mm: n })}>{n}</button>
                ))}
                <input type="number" step="0.05" min="0.1" style={{ width: 70 }}
                  value={project.printer.nozzle_diameter_mm}
                  onChange={(e) => setPrinter({ nozzle_diameter_mm: +e.target.value })} />
              </div></div>
            <div className="field"><label>{fa.lineWidth}</label>
              <input type="number" step="0.05" value={project.printer.line_width_mm}
                onChange={(e) => setPrinter({ line_width_mm: +e.target.value })} /></div>
            <div className="field"><label>{fa.layerHeight}</label>
              <input type="number" step="0.05" min="0.05" value={project.printer.layer_height_mm}
                onChange={(e) => setPrinter({ layer_height_mm: +e.target.value })} /></div>
            <div className="field"><label>{fa.printSpeed}</label>
              <input type="number" step="100" value={project.printer.print_speed_mm_min}
                onChange={(e) => setPrinter({ print_speed_mm_min: +e.target.value })} /></div>
            <div className="field"><label>{fa.filamentDiameter}</label>
              <input type="number" step="0.05" value={project.printer.filament_diameter_mm}
                onChange={(e) => setPrinter({ filament_diameter_mm: +e.target.value })} /></div>
            <div className="field"><label>{fa.flowMultiplier}</label>
              <input type="number" step="0.05" min="0.5" max="2" value={project.printer.flow_multiplier}
                onChange={(e) => setPrinter({ flow_multiplier: +e.target.value })} /></div>
            <div className="field"><label>{fa.buildVolume}</label>
              <div className="ltr" style={{ display: "flex", gap: 4 }}>
                <input type="number" value={project.printer.build_volume_mm[0]}
                  onChange={(e) => setPrinter({ build_volume_mm: [+e.target.value, project.printer.build_volume_mm[1], project.printer.build_volume_mm[2]] })} />
                <input type="number" value={project.printer.build_volume_mm[1]}
                  onChange={(e) => setPrinter({ build_volume_mm: [project.printer.build_volume_mm[0], +e.target.value, project.printer.build_volume_mm[2]] })} />
                <input type="number" value={project.printer.build_volume_mm[2]}
                  onChange={(e) => setPrinter({ build_volume_mm: [project.printer.build_volume_mm[0], project.printer.build_volume_mm[1], +e.target.value] })} />
              </div></div>
            <div className="field"><label>{fa.absoluteE} / {fa.relativeE}</label>
              <select value={project.printer.absolute_e ? "abs" : "rel"}
                onChange={(e) => setPrinter({ absolute_e: e.target.value === "abs" })}>
                <option value="abs">M82 — {fa.absoluteE}</option>
                <option value="rel">M83 — {fa.relativeE}</option>
              </select></div>
            <div className="field"><label>{fa.startGcode} (پیش‌نمایش در خروجی)</label>
              <textarea rows={3} value={project.printer.start_gcode}
                onChange={(e) => setPrinter({ start_gcode: e.target.value })} /></div>
            <div className="field"><label>{fa.endGcode}</label>
              <textarea rows={3} value={project.printer.end_gcode}
                onChange={(e) => setPrinter({ end_gcode: e.target.value })} /></div>
          </div>
        )}

        {/* ---------- دکمه‌های پروژه ---------- */}
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 12 }}>
          <button className="btn small" onClick={doSaveProject}>{fa.saveProject}</button>
          <button className="btn small" onClick={doLoadProject}>{fa.loadProject}</button>
        </div>
      </div>

      {/* ---------- ناحیهٔ اصلی: پیش‌نمایش + خروجی ---------- */}
      <div className="designer-main">
        <Viewport
          segments={segPayload}
          highlightIds={new Set()}
          filter={{ mode: "all", upToId: 0 }}
          buildVolume={project.floor.enabled || step === 5
            ? [project.printer.build_volume_mm[0], project.printer.build_volume_mm[1], project.printer.build_volume_mm[2]]
            : null}
          emptyMessage={busy ? "در حال محاسبه…" : fa.openHint}
          webglMessage={fa.webglMissing}
          extrusionColor={props.extrusionColor}
        />

        {/* خلاصه + هشدارها + خروجی */}
        <div style={{ padding: "10px 14px", borderTop: "1px solid var(--line)", overflowY: "auto", maxHeight: "34vh" }}>
          <div className="export-summary">
            <strong>{fa.exportSummary}:</strong>{" "}
            {note || "در حال محاسبه…"}
            <div className="note">⚠ {fa.exportExperimental} — {fa.safetyNotice}</div>
            <div className="note">{fa.stlNote}</div>
          </div>
          {warningsFa.length > 0 && (
            <ul style={{ paddingInlineStart: 18, color: "var(--warn)", fontSize: 12, margin: "4px 0" }}>
              {warningsFa.map((w, i) => <li key={i}>{w}</li>)}
            </ul>
          )}
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
            <button className="btn primary" disabled={busy || !output} onClick={exportGcode}>
              {fa.sendToEditor} / {fa.exportGcode}
            </button>
            <button className="btn" disabled={busy || !output} onClick={exportStl}>
              {fa.exportStl}
            </button>
          </div>
        </div>
      </div>

      {/* ---------- تأیید خروجی با هشدار بحرانی ---------- */}
      {confirmExport && (
        <div className="modal-back" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>هشدار پیش از خروجی</h3>
            <p style={{ color: "var(--fg-1)" }}>
              این طرح هشدار بحرانی دارد (خارج از bounds یا خطای هندسه).
              خروجی تولید شود؟
            </p>
            <p style={{ color: "var(--warn)", fontSize: 12 }}>{fa.safetyNotice}</p>
            <div className="actions">
              <button className="btn primary" onClick={() => confirmExport && confirmExport()}>
                بله، تولید کن
              </button>
              <button className="btn" onClick={() => setConfirmExport(null)}>انصراف</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
