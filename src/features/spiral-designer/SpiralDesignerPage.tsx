/**
 * ============================================================================
 * features/spiral-designer/SpiralDesignerPage.tsx — طراح مارپیچی ۵ گامی
 * ============================================================================
 * گردش‌کار (Docs/PROMPT_03): ۱. پروفایل ۲. الگو ۳. بافت ۴. کف ۵. پرینتر
 * + پیش‌نمایش زنده سه‌بعدی + خروجی G-code/STL/پروژه + ارسال به ویرایشگر.
 *
 * تمایز حیاتی در UI (تأیید از پژوهش مرجع):
 *   الگو → هندسه (در STL می‌ماند)؛ بافت → مسیر نازل (فقط در G-code).
 *
 * نسخهٔ ۱.۲ (درخواست‌های کاربر):
 *   - کارت‌های بزرگ تصویری برای ۱۷ الگو / ۱۲ بافت / ۵ الگوی کف —
 *     تصویرها از «همان ریاضی هستهٔ Rust» رندر می‌شوند (designer_preview_data)؛
 *   - اسپات قابل‌درگ روی نمای 3D: بازهٔ z الگو/بافت و قطر سوراخ کف
 *     با کشیدن حلقه‌ها مستقیماً جابه‌جا می‌شود؛
 *   - مرکز‌گذاری طرح روی صفحهٔ چاپ (پیش‌فرض روشن — رفع باگ 0,0)؛
 *   - درون‌یابی اسپلاین مونوتون پروفایل (پیش‌فرض — بدون لبهٔ تیز)؛
 *   - واگرد/ازنوی کامل طراحی + ۶ استایل آمادهٔ قابل‌ویرایش.
 * ============================================================================
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ProfileEditor } from "./ProfileEditor";
import { SurfaceThumb, FloorThumb, ProfileMiniThumb } from "./Thumbs";
import { Viewport } from "../../components/Viewport";
import type { SpotHandle, SpotChange } from "../../components/Viewport";
import { fa } from "../../i18n/fa";
import type {
  DesignerProject, PatternParams, TextureParams, DesignerOutput,
  SegmentsPayload, ProfilePoint, PrinterProfile, PreviewData,
  FloorPatternKind, MeshData,
} from "../../lib/tauri";
import {
  designerCompute, designerExportGcode, designerExportStl,
  saveProject, loadProject, saveFileDialog, openFileDialog,
  coreAvailable, printerPresets, designerPreviewData,
} from "../../lib/tauri";

// ============================================================================
// فهرست الگوها/بافت‌ها/کف‌ها — ۱۷ الگو + ۱۲ بافت + ۵ کف (نسخهٔ ۱.۲)
// ============================================================================

const PATTERNS: { kind: PatternParams["kind"]; name: string; hint: string }[] = [
  { kind: "Wave", name: "موج محوری", hint: "برجستگی سینوسی که با ارتفاع می‌چرخد" },
  { kind: "Petal", name: "گلبرگ", hint: "گلبرگ‌های نرم دور فرم" },
  { kind: "Twist", name: "پیچش", hint: "موج تاب‌خورده" },
  { kind: "Flute", name: "شیار عمودی", hint: "کانال‌های عمودی ستون کلاسیک" },
  { kind: "Diamond", name: "لوزی", hint: "شبکهٔ لوزی برجسته" },
  { kind: "Chevron", name: "مهری", hint: "زیگزاگ مورب چین‌دار" },
  { kind: "Rings", name: "حلقهٔ افقی", hint: "برجستگی‌های دور فرم مستقل از زاویه" },
  { kind: "Scale", name: "فلس", hint: "فلس‌های مورب هم‌پوشان" },
  { kind: "Facet", name: "تراش‌خورده", hint: "مقطع چندضلعی تراش‌خورده" },
  { kind: "Braid", name: "رشته‌ای (طناب)", hint: "تارهای درهم‌بافته طناب" },
  { kind: "Pineapple", name: "آناناسی", hint: "شبکهٔ لوزی تیز پوستهٔ آناناس" },
  { kind: "Lotus", name: "نیلوفر (آبشاری)", hint: "ردیف گلبرگ‌های هم‌پوشان" },
  { kind: "TwistedFlute", name: "شیار پیچان", hint: "شیارهایی که با ارتفاع می‌پیچند" },
  { kind: "Pearl", name: "مروارید", hint: "دانه‌های گرد درهم" },
  { kind: "Meander", name: "یونانی", hint: "بلوک‌های مربعی نقش یونانی" },
  { kind: "Brick", name: "آجری", hint: "ردیف‌های نیم‌جابه‌جا دیوار آجری" },
  { kind: "Spikes", name: "خاردار", hint: "خارهای مثلثی تیز" },
];

const TEXTURES: { kind: TextureParams["kind"]; name: string; hint: string }[] = [
  { kind: "Ripple", name: "موجک (ریپل)", hint: "موج نرم روی طول مسیر" },
  { kind: "Weave", name: "تور بافته", hint: "بالا/پایین متناوب تار و پود" },
  { kind: "Stitch", name: "بخیه‌ای", hint: "پرش‌های گسسته مثل دوخت" },
  { kind: "SpiralGroove", name: "شیار مارپیچی", hint: "کانال مورب دور دیواره" },
  { kind: "Hammered", name: "چکشی", hint: "فرورفتگی‌های شبه‌تصادفی قطعی" },
  { kind: "Basket", name: "سبدی", hint: "شبکهٔ متقاطع دو جهته" },
  { kind: "WoodGrain", name: "رگهٔ چوب", hint: "الیاف کشیدهٔ چوب" },
  { kind: "Leather", name: "چرم", hint: "دانه‌های ریز پوست/چرم" },
  { kind: "Knurl", name: "آج رزوه", hint: "شیارهای متقاطع دستگیرهٔ آجدار" },
  { kind: "Ocean", name: "موج دریا", hint: "موج‌های روان بی‌تکرار" },
  { kind: "Bubbles", name: "حباب", hint: "فرورفتگی‌های گرد پراکنده" },
  { kind: "Rows", name: "تخته‌ای", hint: "شیارهای افقی ردیف تخته‌ها" },
];

const FLOORS: { kind: FloorPatternKind; name: string; hint: string }[] = [
  { kind: "Rings", name: "حلقه‌های هم‌مرکز", hint: "حلقه‌های تودرتو از بیرون به داخل" },
  { kind: "Spiral", name: "مارپیچ پیوسته", hint: "یک مسیر پیوسته بدون travel — مناسب vase" },
  { kind: "Grid", name: "شبکهٔ متقاطع", hint: "خطوط X و Y روی دیسک" },
  { kind: "Radial", name: "پرتوهای شعاعی", hint: "پرتو از سوراخ مرکزی تا لبه" },
  { kind: "ZigZag", name: "رفت‌وبرگشتی", hint: "ردیف‌های مارپیچی بدون درز" },
];

// ============================================================================
// ۶ استایل آماده — ترکیب پروفایل + الگو + بافت (کاملاً قابل‌ویرایش)
// ============================================================================

interface StylePreset {
  key: string;
  name: string;
  desc: string;
  profile: ProfilePoint[];
  height: number;
  bottomD: number;
  topD: number;
  patterns: PatternParams[];
  textures: TextureParams[];
  floorPattern: FloorPatternKind;
  floorHole: number;
}

const STYLE_PRESETS: StylePreset[] = [
  {
    key: "column",
    name: "ستون کلاسیک",
    desc: "شیارهای عمودی ۱۲تایی + کف حلقه‌ای",
    profile: [{ r: 18, h: 0 }, { r: 19, h: 50 }, { r: 17, h: 95 }, { r: 21, h: 100 }],
    height: 100, bottomD: 38, topD: 42,
    patterns: [{ kind: "Flute", amplitude_mm: 1.8, count: 12, frequency: 0, phase_deg: 0, z_start_mm: 5, z_end_mm: 90 }],
    textures: [],
    floorPattern: "Rings", floorHole: 0,
  },
  {
    key: "pineapple",
    name: "آناناس",
    desc: "شبکهٔ لوزی آناناسی + کف مارپیچ",
    profile: [{ r: 20, h: 0 }, { r: 24, h: 30 }, { r: 22, h: 80 }, { r: 24, h: 100 }],
    height: 100, bottomD: 42, topD: 46,
    patterns: [{ kind: "Pineapple", amplitude_mm: 2.2, count: 10, frequency: 3, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Spiral", floorHole: 8,
  },
  {
    key: "rope",
    name: "طناب بافته",
    desc: "رشته‌های درهم‌بافته + کف شبکه",
    profile: [{ r: 16, h: 0 }, { r: 16, h: 100 }],
    height: 90, bottomD: 34, topD: 34,
    patterns: [{ kind: "Braid", amplitude_mm: 2.5, count: 6, frequency: 2, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Grid", floorHole: 0,
  },
  {
    key: "lotus",
    name: "نیلوفر آبی",
    desc: "ردیف گلبرگ‌های آبشاری",
    profile: [{ r: 12, h: 0 }, { r: 20, h: 20 }, { r: 28, h: 60 }, { r: 26, h: 100 }],
    height: 100, bottomD: 26, topD: 52,
    patterns: [{ kind: "Lotus", amplitude_mm: 2.0, count: 10, frequency: 2, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Radial", floorHole: 10,
  },
  {
    key: "hammered",
    name: "مس چکشی",
    desc: "فرورفتگی‌های چکشی قطعی روی فرم ساده",
    profile: [{ r: 22, h: 0 }, { r: 26, h: 40 }, { r: 20, h: 100 }],
    height: 95, bottomD: 46, topD: 40,
    patterns: [],
    textures: [{ kind: "Hammered", amplitude_mm: 0.5, period_mm: 3.5, radial: true, phase_deg: 0, fade_in_percent: 5, fade_out_percent: 5 }],
    floorPattern: "Rings", floorHole: 0,
  },
  {
    key: "wood",
    name: "چوب کهنه",
    desc: "رگه‌های چوب + پیچش ملایم",
    profile: [{ r: 18, h: 0 }, { r: 21, h: 60 }, { r: 17, h: 100 }],
    height: 110, bottomD: 38, topD: 34,
    patterns: [{ kind: "Twist", amplitude_mm: 1.2, count: 5, frequency: 1, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [{ kind: "WoodGrain", amplitude_mm: 0.45, period_mm: 5, radial: true, phase_deg: 0, fade_in_percent: 0, fade_out_percent: 0 }],
    floorPattern: "ZigZag", floorHole: 0,
  },
];

/** پروژهٔ پیش‌فرض — گلدان آماده (قابل‌ویرایش) با تنظیمات ۱.۲ */
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
    profile_interpolation: "Spline",
    center_on_bed: true,
    phase_start_deg: 0,
    patterns: [],
    textures: [],
    floor: { enabled: false, pattern: "Rings", center_hole_diameter_mm: 0, z_start_mm: 0, ring_gap_mm: 0.45, spokes: 12 },
    printer: {
      name: "Custom / Unverified", status: "unverified",
      build_volume_mm: [220, 220, 250],
      nozzle_diameter_mm: 0.4, filament_diameter_mm: 1.75,
      line_width_mm: 0.45, layer_height_mm: 0.25,
      print_speed_mm_min: 2400, travel_speed_mm_min: 6000,
      flow_multiplier: 1.0, absolute_e: true,
      // ---- نسخهٔ ۱.۳: دما/فن (پیش‌فرض PLA عمومی) ----
      nozzle_temp_c: 200, bed_temp_c: 60, fan_percent: 100, wait_for_temps: true,
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
    positions[i * 6 + 0] = start[0];
    positions[i * 6 + 1] = start[1];
    positions[i * 6 + 2] = start[2];
    positions[i * 6 + 3] = end[0];
    positions[i * 6 + 4] = end[1];
    positions[i * 6 + 5] = end[2];
    kinds[i] = KIND_NUM[String(s.kind)] ?? 5;
    source_lines[i] = s.source_line_index;
    ids[i] = s.id;
    certainties[i] = CERTAINTY_NUM[String(s.certainty)] ?? 2;
    feedrates[i] = s.feedrate_mm_min ?? NaN;
  }
  return { revision: 1, positions, kinds, source_lines, ids, certainties, feedrates };
}

/** شعاع واقعی فرم در ارتفاع z — از mesh هسته (هندسهٔ صادقانه) */
function meshRadiusAt(mesh: MeshData | undefined, z: number, tol = 3.0): number {
  if (!mesh || mesh.vertices.length === 0) return 20;
  let max = 0;
  for (const v of mesh.vertices) {
    if (Math.abs(v[2] - z) <= tol) {
      const r = Math.hypot(v[0], v[1]);
      if (r > max) max = r;
    }
  }
  return max || 20;
}

/** انتخاب اسپات (برای درگ روی 3D) */
type SpotSelection =
  | { type: "pattern"; index: number }
  | { type: "texture"; index: number }
  | { type: "floor" }
  | { type: "phase" }
  | null;

/** پریست‌های ماده — «شروع مناسب رایج» نه عدد قطعی (قاعدهٔ صداقت) */
const MATERIALS: { name: string; nozzle: number; bed: number; fan: number }[] = [
  { name: "PLA — 200/60", nozzle: 200, bed: 60, fan: 100 },
  { name: "PETG — 240/80", nozzle: 240, bed: 80, fan: 30 },
  { name: "ABS — 250/100", nozzle: 250, bed: 100, fan: 0 },
  { name: "TPU — 225/45", nozzle: 225, bed: 45, fan: 50 },
  { name: "PLA ابریشمی — 215/65", nozzle: 215, bed: 65, fan: 100 },
];

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
  // دادهٔ پیش‌نمایش کارت‌ها (۱۷ الگو + ۱۲ بافت) از ریاضی واقعی هسته
  const [previewData, setPreviewData] = useState<PreviewData | null>(null);
  // پرسش تأیید خروجی‌های دارای هشدار بحرانی (الزام §8)
  const [confirmExport, setConfirmExport] = useState<null | (() => void)>(null);
  // اسپات انتخاب‌شده برای درگ روی 3D
  const [spotSel, setSpotSel] = useState<SpotSelection>(null);
  const debounceRef = useRef<number | null>(null);

  // ---------------- واگرد/ازنو (نسخهٔ ۱.۲) ----------------
  const historyRef = useRef<DesignerProject[]>([]);
  const futureRef = useRef<DesignerProject[]>([]);
  const [histLen, setHistLen] = useState(0);
  const [futureLen, setFutureLen] = useState(0);
  // درگ اسپات: یک قدم تاریخ در هر ژست (نه هر پیکسل)
  const spotGestureRef = useRef<number | null>(null);
  // نسخهٔ همیشه‌به‌روز پروژه برای جهش‌های تاریخ خارج از updater
  // (StrictMode updaterها را دوبار اجرا می‌کند — جهش ref داخل updater ممنوع)
  const projectRef = useRef(project);
  projectRef.current = project;

  /** ثبت تغییر پروژه با قدم تاریخ */
  const commitProject = useCallback((next: DesignerProject) => {
    historyRef.current.push(projectRef.current);
    if (historyRef.current.length > 80) historyRef.current.shift();
    futureRef.current = [];
    setProject(next);
    setHistLen(historyRef.current.length);
    setFutureLen(0);
  }, []);

  const undo = useCallback(() => {
    const prev = historyRef.current.pop();
    if (!prev) return;
    futureRef.current.push(projectRef.current);
    setProject(prev);
    setHistLen(historyRef.current.length);
    setFutureLen(futureRef.current.length);
  }, []);

  const redo = useCallback(() => {
    const next = futureRef.current.pop();
    if (!next) return;
    historyRef.current.push(projectRef.current);
    setProject(next);
    setHistLen(historyRef.current.length);
    setFutureLen(futureRef.current.length);
  }, []);

  // میان‌بر کیبورد واگرد/ازنو (فقط در صفحهٔ طراح)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        undo();
      } else if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === "y" || (e.shiftKey && e.key.toLowerCase() === "z"))) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo, redo]);

  // بارگذاری پریست‌های پرینتر و دادهٔ پیش‌نمایش از هسته
  useEffect(() => {
    if (!coreAvailable()) return;
    printerPresets()
      .then((list) => setPresets(list))
      .catch(() => setPresets([]));
    designerPreviewData()
      .then((d) => setPreviewData(d))
      .catch(() => setPreviewData(null)); // بدون هسته → کارت‌های جایگزین صادقانه
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
  const setP = (patch: Partial<DesignerProject>) => commitProject({ ...project, ...patch });
  const setPrinter = (patch: Partial<DesignerProject["printer"]>) =>
    commitProject({ ...project, printer: { ...project.printer, ...patch } });
  const setFloor = (patch: Partial<DesignerProject["floor"]>) =>
    commitProject({ ...project, floor: { ...project.floor, ...patch } });

  const addPattern = (kind: PatternParams["kind"]) =>
    commitProject({
      ...project,
      patterns: [...project.patterns, {
        kind, amplitude_mm: 1.5, count: 8, frequency: 1,
        phase_deg: 0, z_start_mm: 0, z_end_mm: null,
      }],
    });
  const updatePattern = (i: number, patch: Partial<PatternParams>) =>
    commitProject({
      ...project,
      patterns: project.patterns.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    });
  const removePattern = (i: number) =>
    commitProject({ ...project, patterns: project.patterns.filter((_, j) => j !== i) });

  const addTexture = (kind: TextureParams["kind"]) =>
    commitProject({
      ...project,
      textures: [...project.textures, {
        kind, amplitude_mm: 0.4, period_mm: 4, radial: true,
        phase_deg: 0, fade_in_percent: 0, fade_out_percent: 0,
      }],
    });
  const updateTexture = (i: number, patch: Partial<TextureParams>) =>
    commitProject({
      ...project,
      textures: project.textures.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    });
  const removeTexture = (i: number) =>
    commitProject({ ...project, textures: project.textures.filter((_, j) => j !== i) });

  /** اعمال استایل آماده — کاملاً قابل‌ویرایش بعد از اعمال */
  const applyStyle = (sp: StylePreset) =>
    commitProject({
      ...project,
      profile: sp.profile.map((p) => ({ ...p })),
      form_height_mm: sp.height,
      form_bottom_diameter_mm: sp.bottomD,
      form_top_diameter_mm: sp.topD,
      patterns: sp.patterns.map((p) => ({ ...p })),
      textures: sp.textures.map((t) => ({ ...t })),
      floor: {
        ...project.floor,
        enabled: true,
        pattern: sp.floorPattern,
        center_hole_diameter_mm: sp.floorHole,
      },
    });

  const patternName = useMemo(() => {
    const m: Record<string, string> = {};
    PATTERNS.forEach((p) => (m[p.kind] = p.name));
    return m;
  }, []);
  const textureName = useMemo(() => {
    const m: Record<string, string> = {};
    TEXTURES.forEach((t) => (m[t.kind] = t.name));
    return m;
  }, []);

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
      // ادغام با پیش‌فرض‌ها: پروژه‌های ۱.۲/۱.۱ فاز و دما ندارند →
      // مقادیر پیش‌فرض ۱.۳ برایشان می‌نشیند (بدون از دست رفتن داده)
      commitProject({
        ...defaultProject(),
        ...p,
        printer: { ...defaultProject().printer, ...p.printer },
        floor: { ...defaultProject().floor, ...p.floor },
      });
      setNote("پروژه بارگذاری شد.");
    } catch (e) {
      // schema خراب → پروژهٔ جاری دست‌نخورده می‌ماند (الزام §7)
      setNote(`بارگذاری ناموفق: ${e} — پروژهٔ جاری تغییر نکرد.`);
    }
  };

  const segPayload = output ? toSegmentsPayload(output) : null;
  const warningsFa = output?.diagnostics.map((d) => `${d.severity === "Error" ? "خطا" : "هشدار"} [${d.code}]: ${d.message_fa}`) ?? [];

  // ---------------- اسپات: اتصال انتخاب به هندل‌های 3D ----------------
  // (useMemo — آرایهٔ پایدار تا حلقه‌ها در هر رندر بازسازی نشوند)
  const bedOffset = useMemo<[number, number]>(() =>
    project.center_on_bed !== false
      ? [project.printer.build_volume_mm[0] / 2, project.printer.build_volume_mm[1] / 2]
      : [0, 0],
    [project.center_on_bed, project.printer.build_volume_mm]
  );

  const spot: SpotHandle | null = useMemo(() => {
    if (!spotSel) return null;
    const height = Math.max(project.form_height_mm, 1);
    if (spotSel.type === "phase") {
      // ---- دیال چرخش فاز θ (نسخهٔ ۱.۳) ----
      // دیال در ارتفاع کم (~۸٪ ارتفاع، بین ۳ تا ۱۲mm) روی شعاع واقعی فرم
      const zDial = Math.min(Math.max(height * 0.08, 3), 12);
      const r = meshRadiusAt(output?.mesh, zDial);
      return {
        kind: "phaseRotation",
        zStart: zDial,
        zEnd: null,
        radius: Math.max(r, 8),
        height,
        bedOffset,
        phaseDeg: project.phase_start_deg ?? 0,
      };
    }
    if (spotSel.type === "pattern") {
      const p = project.patterns[spotSel.index];
      if (!p) return null;
      const r = meshRadiusAt(output?.mesh, p.z_start_mm);
      return {
        kind: "patternRange",
        zStart: p.z_start_mm,
        zEnd: p.z_end_mm,
        radius: Math.max(r, 6),
        height,
        bedOffset,
      };
    }
    if (spotSel.type === "texture") {
      const t = project.textures[spotSel.index];
      if (!t) return null;
      const zStart = (t.fade_in_percent / 100) * height;
      const zEnd = height - (t.fade_out_percent / 100) * height;
      const r = meshRadiusAt(output?.mesh, zStart + 2);
      return {
        kind: "textureFade",
        zStart: Math.max(zStart, 0.5),
        zEnd: zEnd > zStart + 1 ? zEnd : null,
        radius: Math.max(r, 6),
        height,
        bedOffset,
      };
    }
    // کف — سوراخ مرکزی
    if (!project.floor.enabled || project.floor.center_hole_diameter_mm < 1) return null;
    const rOuter = meshRadiusAt(output?.mesh, 0.5);
    return {
      kind: "floorHole",
      zStart: 0.5,
      zEnd: null,
      radius: Math.min(project.floor.center_hole_diameter_mm / 2, rOuter - 1),
      height,
      bedOffset,
    };
  }, [spotSel, project, output, bedOffset]);

  /** تغییر اسپات با درگ — نگاشت به فیلدهای پروژه + قدم تاریخ در هر ژست
   *  (جهش ref خارج از updater — سازگار با StrictMode) */
  const onSpotChange = useCallback((change: SpotChange) => {
    const cur = projectRef.current;
    // یک قدم تاریخ در آغاز ژست (۱۵۰۰ms بی‌حرکتی = پایان ژست)
    if (spotGestureRef.current === null) {
      historyRef.current.push(cur);
      if (historyRef.current.length > 80) historyRef.current.shift();
      futureRef.current = [];
      setHistLen(historyRef.current.length);
      setFutureLen(0);
    } else {
      clearTimeout(spotGestureRef.current);
    }
    spotGestureRef.current = window.setTimeout(() => {
      spotGestureRef.current = null;
    }, 1500);

    const height = Math.max(cur.form_height_mm, 1);
    let next: DesignerProject | null = null;
    if (change.phaseDeg !== undefined) {
      // ---- چرخش فاز θ با اسپات (نسخهٔ ۱.۳) ----
      next = { ...cur, phase_start_deg: change.phaseDeg };
    } else if (spotSel?.type === "pattern" && change.zStart !== undefined) {
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index ? {
          ...x,
          z_start_mm: change.zStart!,
          z_end_mm: x.z_end_mm !== null ? Math.max(x.z_end_mm, change.zStart! + 1) : null,
        } : x
      );
      next = { ...cur, patterns };
    } else if (spotSel?.type === "pattern" && change.zEnd !== undefined) {
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index ? { ...x, z_end_mm: change.zEnd ?? null } : x
      );
      next = { ...cur, patterns };
    } else if (spotSel?.type === "texture" && change.zStart !== undefined) {
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index ? { ...x, fade_in_percent: (change.zStart! / height) * 100 } : x
      );
      next = { ...cur, textures };
    } else if (spotSel?.type === "texture" && change.zEnd !== undefined && change.zEnd !== null) {
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index ? { ...x, fade_out_percent: ((height - change.zEnd!) / height) * 100 } : x
      );
      next = { ...cur, textures };
    } else if (spotSel?.type === "floor" && change.holeDiameter !== undefined) {
      next = { ...cur, floor: { ...cur.floor, center_hole_diameter_mm: change.holeDiameter } };
    }
    if (next) setProject(next);
  }, [spotSel]);

  // شمارش فعال بودن هر الگو/بافت روی کارت‌ها
  const patternCounts = useMemo(() => {
    const m: Record<string, number> = {};
    project.patterns.forEach((p) => {
      m[p.kind] = (m[p.kind] ?? 0) + 1;
    });
    return m;
  }, [project.patterns]);
  const textureCounts = useMemo(() => {
    const m: Record<string, number> = {};
    project.textures.forEach((t) => {
      m[t.kind] = (m[t.kind] ?? 0) + 1;
    });
    return m;
  }, [project.textures]);

  // ------------------------------------------------------------ رندر
  return (
    <div className="designer-layout">
      {/* ---------- پنل کناری ۵ گام ---------- */}
      <div className="designer-side">
        <div className="designer-head">
          <h3 style={{ marginTop: 0, color: "var(--accent)", marginBottom: 0 }}>{fa.designerTitle}</h3>
          <div className="history-btns">
            <button className="btn small" disabled={histLen === 0} onClick={undo}
              title="Ctrl+Z">{fa.designerUndo} ↶</button>
            <button className="btn small" disabled={futureLen === 0} onClick={redo}
              title="Ctrl+Y">{fa.designerRedo} ↷</button>
          </div>
        </div>

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
              <div className="preset-row">
                <button className="btn preset-btn" onClick={() => setP({ profile: [
                  { r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 },
                  { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 },
                ] })}>
                  <ProfileMiniThumb points={[{ r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 }, { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 }]} width={44} height={56} />
                  <span>{fa.presetVase}</span>
                </button>
                <button className="btn preset-btn" onClick={() => setP({ profile: [
                  { r: 8, h: 0 }, { r: 15, h: 15 }, { r: 35, h: 90 }, { r: 40, h: 100 },
                ] })}>
                  <ProfileMiniThumb points={[{ r: 8, h: 0 }, { r: 15, h: 15 }, { r: 35, h: 90 }, { r: 40, h: 100 }]} width={44} height={56} />
                  <span>{fa.presetLamp}</span>
                </button>
                <button className="btn preset-btn" onClick={() => setP({ profile: [
                  { r: 15, h: 0 }, { r: 18, h: 5 }, { r: 30, h: 60 }, { r: 32, h: 65 },
                ] })}>
                  <ProfileMiniThumb points={[{ r: 15, h: 0 }, { r: 18, h: 5 }, { r: 30, h: 60 }, { r: 32, h: 65 }]} width={44} height={56} />
                  <span>{fa.presetPlanter}</span>
                </button>
              </div>
            </div>
            {/* درون‌یابی: اسپلاین مونوتون (نسخهٔ ۱.۲) */}
            <div className="field">
              <label>{fa.interpolationMode}:</label>
              <div className="radio-row" role="radiogroup" aria-label={fa.interpolationMode}>
                <button
                  className={`btn small ${project.profile_interpolation !== "Linear" ? "primary" : ""}`}
                  role="radio" aria-checked={project.profile_interpolation !== "Linear"}
                  onClick={() => setP({ profile_interpolation: "Spline" })}
                >{fa.interpolationSpline}</button>
                <button
                  className={`btn small ${project.profile_interpolation === "Linear" ? "primary" : ""}`}
                  role="radio" aria-checked={project.profile_interpolation === "Linear"}
                  onClick={() => setP({ profile_interpolation: "Linear" })}
                >{fa.interpolationLinear}</button>
              </div>
              <span className="hint-text">{fa.interpolationHint}</span>
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

        {/* ================= استایل‌های آماده ================= */}
        {step === 1 && (
          <div className="field" style={{ marginTop: 10 }}>
            <label>{fa.stylePresets}:</label>
            <span className="hint-text">{fa.stylePresetHint}</span>
            <div className="style-grid">
              {STYLE_PRESETS.map((sp) => (
                <button key={sp.key} className="style-card" onClick={() => applyStyle(sp)}>
                  <ProfileMiniThumb points={sp.profile} width={40} height={52} />
                  <div className="style-meta">
                    <strong>{sp.name}</strong>
                    <span>{sp.desc}</span>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* ================= گام ۲: الگو (کارت‌های تصویری بزرگ) ================= */}
        {step === 2 && (
          <div>
            {/* ---- چرخش فاز θ — نسخهٔ ۱.۳ (درخواست کاربر) ---- */}
            <div className="field">
              <label>{fa.phaseRotate}</label>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                <input type="number" step="15" min="-360" max="360" style={{ width: 90 }}
                  value={Math.round((project.phase_start_deg ?? 0) * 10) / 10}
                  onChange={(e) => {
                    const v = +e.target.value || 0;
                    setP({ phase_start_deg: ((v % 360) + 360) % 360 });
                  }} />
                <span className="ltr" style={{ fontSize: 12, color: "var(--fg-2)" }}>°</span>
                <button
                  className={`btn small ${spotSel?.type === "phase" ? "primary" : ""}`}
                  title={fa.spotPhaseRotate}
                  onClick={() => setSpotSel(spotSel?.type === "phase" ? null : { type: "phase" })}
                >🔄</button>
              </div>
              <span className="hint-text">{fa.phaseRotateHint}</span>
              <span className="hint-text">{fa.phaseRotateSeam}</span>
              {spotSel?.type === "phase" && (
                <div className="spot-note">🔄 {fa.spotPhaseRotate} — Shift = چفت ۱۵°</div>
              )}
            </div>
            <p className="step-note">
              الگو <strong>هندسهٔ مدل</strong> را تغییر می‌دهد؛ اثر آن در <strong>STL</strong> هم باقی می‌ماند.
              روی کارت کلیک کنید تا افزوده شود؛ سپس با دکمهٔ 🎯 آن را روی نمای 3D جابه‌جا کنید.
            </p>
            <div className="card-grid">
              {PATTERNS.map((p) => {
                const count = patternCounts[p.kind] ?? 0;
                return (
                  <button
                    key={p.kind}
                    className={`asset-card ${count > 0 ? "active" : ""}`}
                    onClick={() => addPattern(p.kind)}
                    title={p.hint}
                  >
                    <SurfaceThumb grid={previewData?.patterns[p.kind]} variant="pattern" width={104} height={76} />
                    <span className="asset-name">{p.name}</span>
                    <span className="asset-badge">
                      {count > 0 ? `${count}× ${fa.cardActive}` : `+ ${fa.cardAdd}`}
                    </span>
                  </button>
                );
              })}
            </div>
            {project.patterns.map((pat, i) => (
              <div className={`pattern-item ${spotSel?.type === "pattern" && spotSel.index === i ? "spot-selected" : ""}`} key={i}>
                <div className="head">
                  <span className="name">{patternName[pat.kind]}</span>
                  <div style={{ display: "flex", gap: 4 }}>
                    <button
                      className={`btn small ${spotSel?.type === "pattern" && spotSel.index === i ? "primary" : ""}`}
                      title={fa.spotPatternRange}
                      onClick={() => setSpotSel(spotSel?.type === "pattern" && spotSel.index === i ? null : { type: "pattern", index: i })}
                    >🎯</button>
                    <button className="btn small danger" onClick={() => removePattern(i)}>×</button>
                  </div>
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
                {spotSel?.type === "pattern" && spotSel.index === i && (
                  <div className="spot-note">🎯 {fa.spotHint}</div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۳: بافت (کارت‌های تصویری بزرگ) ================= */}
        {step === 3 && (
          <div>
            <p className="step-note">
              بافت <strong>مسیر حرکت نازل</strong> را تعدیل می‌کند و در STL ذخیره <strong>نمی‌شود</strong>؛
              فقط در G-code و پیش‌نمایش مسیر دیده می‌شود. با 🎯 ناحیهٔ اعمال را روی 3D جابه‌جا کنید.
            </p>
            <div className="card-grid">
              {TEXTURES.map((t) => {
                const count = textureCounts[t.kind] ?? 0;
                return (
                  <button
                    key={t.kind}
                    className={`asset-card texture ${count > 0 ? "active" : ""}`}
                    onClick={() => addTexture(t.kind)}
                    title={t.hint}
                  >
                    <SurfaceThumb grid={previewData?.textures[t.kind]} variant="texture" width={104} height={76} />
                    <span className="asset-name">{t.name}</span>
                    <span className="asset-badge">
                      {count > 0 ? `${count}× ${fa.cardActive}` : `+ ${fa.cardAdd}`}
                    </span>
                  </button>
                );
              })}
            </div>
            {project.textures.map((tex, i) => (
              <div className={`texture-item ${spotSel?.type === "texture" && spotSel.index === i ? "spot-selected" : ""}`} key={i}>
                <div className="head">
                  <span className="name">{textureName[tex.kind]}</span>
                  <div style={{ display: "flex", gap: 4 }}>
                    <button
                      className={`btn small ${spotSel?.type === "texture" && spotSel.index === i ? "primary" : ""}`}
                      title={fa.spotTextureFade}
                      onClick={() => setSpotSel(spotSel?.type === "texture" && spotSel.index === i ? null : { type: "texture", index: i })}
                    >🎯</button>
                    <button className="btn small danger" onClick={() => removeTexture(i)}>×</button>
                  </div>
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
                {spotSel?.type === "texture" && spotSel.index === i && (
                  <div className="spot-note">🎯 {fa.spotHint}</div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۴: کف (۵ الگوی تصویری) ================= */}
        {step === 4 && (
          <div>
            <p className="step-note">
              پنج الگوی چاپ کف — همه مسیر واقعی تولید می‌کنند. نمای بالا هر الگو را نشان می‌دهد.
            </p>
            <label className="check-row">
              <input type="checkbox" checked={project.floor.enabled}
                onChange={(e) => setFloor({ enabled: e.target.checked })} />
              {fa.floorEnabled}
            </label>
            <div className="field">
              <label>{fa.floorPattern}:</label>
              <div className="card-grid floor">
                {FLOORS.map((f) => (
                  <button
                    key={f.kind}
                    className={`asset-card floor ${(project.floor.pattern ?? "Rings") === f.kind && project.floor.enabled ? "active" : ""}`}
                    onClick={() => setFloor({ pattern: f.kind, enabled: true })}
                    title={f.hint}
                    disabled={!project.floor.enabled}
                  >
                    <FloorThumb kind={f.kind} hole={f.kind === "Radial" ? 0.22 : 0.16} size={84} />
                    <span className="asset-name">{f.name}</span>
                    {(project.floor.pattern ?? "Rings") === f.kind && project.floor.enabled && (
                      <span className="asset-badge">{fa.cardActive}</span>
                    )}
                  </button>
                ))}
              </div>
            </div>
            <div className="field"><label>{fa.floorHole}</label>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                <input type="number" step="1" min="0" disabled={!project.floor.enabled}
                  value={project.floor.center_hole_diameter_mm}
                  onChange={(e) => setFloor({ center_hole_diameter_mm: +e.target.value })} />
                <button
                  className={`btn small ${spotSel?.type === "floor" ? "primary" : ""}`}
                  disabled={!project.floor.enabled || project.floor.center_hole_diameter_mm < 1}
                  title={fa.spotFloorHole}
                  onClick={() => setSpotSel(spotSel?.type === "floor" ? null : { type: "floor" })}
                >🎯</button>
              </div>
              {spotSel?.type === "floor" && (
                <span className="hint-text">🎯 قطر سوراخ را با کشیدن حلقه در نمای 3D تغییر دهید</span>
              )}
            </div>
            <div className="field"><label>{fa.floorGap}</label>
              <input type="number" step="0.05" min="0.2" disabled={!project.floor.enabled}
                value={project.floor.ring_gap_mm}
                onChange={(e) => setFloor({ ring_gap_mm: +e.target.value })} /></div>
            {(project.floor.pattern ?? "Rings") === "Radial" && (
              <div className="field"><label>{fa.floorSpokes}</label>
                <input type="number" step="1" min="3" max="96" disabled={!project.floor.enabled}
                  value={project.floor.spokes ?? 12}
                  onChange={(e) => setFloor({ spokes: Math.round(+e.target.value) || 12 })} /></div>
            )}
          </div>
        )}

        {/* ================= گام ۵: پرینتر ================= */}
        {step === 5 && (
          <div>
            <p className="step-note">
              {fa.printerProfileStatus}: <strong className="ltr">{project.printer.status}</strong> —
              presetها دادهٔ ثابت نیستند؛ قبل از چاپ صحت‌سنجی کنید.
            </p>
            {/* مرکز‌گذاری روی صفحه (نسخهٔ ۱.۲ — رفع باگ 0,0) */}
            <label className="check-row">
              <input type="checkbox" checked={project.center_on_bed !== false}
                onChange={(e) => setP({ center_on_bed: e.target.checked })} />
              {fa.centerOnBed}
            </label>
            <span className="hint-text">{fa.centerOnBedHint}</span>
            {/* انتخاب از ۲۰ پریست پرینتر واقعی (وضعیت community) */}
            <div className="field">
              <label>{fa.printerPresets}</label>
              <select
                className="ltr"
                value={presets.some((x) => x.name === project.printer.name) ? project.printer.name : ""}
                onChange={(e) => {
                  const p = presets.find((x) => x.name === e.target.value);
                  if (p) commitProject({ ...project, printer: { ...p } });
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
                <span className="hint-text">
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
            {/* ---- تنظیمات دما و خنک‌کاری — نسخهٔ ۱.۳ (درخواست کاربر) ---- */}
            <div className="field" style={{ marginTop: 10 }}>
              <label>{fa.tempsTitle}</label>
              <span className="hint-text">{fa.materialHint}</span>
              <div className="preset-row">
                {MATERIALS.map((m) => {
                  const active = project.printer.nozzle_temp_c === m.nozzle
                    && project.printer.bed_temp_c === m.bed
                    && (project.printer.fan_percent ?? 100) === m.fan;
                  return (
                    <button key={m.name}
                      className={`btn preset-btn small ${active ? "primary" : ""}`}
                      onClick={() => setPrinter({
                        nozzle_temp_c: m.nozzle,
                        bed_temp_c: m.bed,
                        fan_percent: m.fan,
                      })}
                    >
                      <span>{m.name}</span>
                    </button>
                  );
                })}
              </div>
            </div>
            <div className="field"><label>{fa.nozzleTemp}</label>
              <input type="number" step="5" min="0" max="500" style={{ width: 100 }}
                value={project.printer.nozzle_temp_c ?? 200}
                onChange={(e) => setPrinter({ nozzle_temp_c: +e.target.value })} /></div>
            <div className="field"><label>{fa.bedTemp}</label>
              <input type="number" step="5" min="0" max="150" style={{ width: 100 }}
                value={project.printer.bed_temp_c ?? 60}
                onChange={(e) => setPrinter({ bed_temp_c: +e.target.value })} /></div>
            <div className="field"><label>{fa.fanSpeed}</label>
              <input type="number" step="5" min="0" max="100" style={{ width: 100 }}
                value={project.printer.fan_percent ?? 100}
                onChange={(e) => setPrinter({ fan_percent: +e.target.value })} /></div>
            <label className="check-row">
              <input type="checkbox" checked={project.printer.wait_for_temps !== false}
                onChange={(e) => setPrinter({ wait_for_temps: e.target.checked })} />
              {fa.waitTemps}
            </label>
            <span className="hint-text">{fa.waitTempsHint}</span>
            <span className="hint-text">{fa.tempsZeroNote}</span>
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
          buildVolume={project.center_on_bed !== false || project.floor.enabled || step === 5
            ? [project.printer.build_volume_mm[0], project.printer.build_volume_mm[1], project.printer.build_volume_mm[2]]
            : null}
          spot={spot}
          onSpotChange={onSpotChange}
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
