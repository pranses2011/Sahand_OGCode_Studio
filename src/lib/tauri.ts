/**
 * ============================================================================
 * lib/tauri.ts — پل IPC تایپ‌دار به دستورهای Rust
 * ============================================================================
 * این فایل «تنها» لایهٔ ارتباط فرانت‌اند با هستهٔ Rust است.
 * قواعد (الزام Docs/PROMPT_04 §2):
 *   ۱) کامپوننت‌ها هرگز خودشان G-code را تفسیر نمی‌کنند — همه‌چیز از
 *      همین invokeها می‌آید؛
 *   ۲) انواع خروجی دقیقاً معادل structهای Rust در core/src/ipc.rs هستند؛
 *   ۳) در محیط dev مرورگر (بدون Tauri) fallback آزمایشی روشن می‌شود تا
 *      UI بدون پوستهٔ دسکتاپ هم قابل توسعه/تست باشد — این fallback
 *      «دادهٔ ساختگی نمایشی» نیست؛ همان parser TS کوچکی نیست بلکه خطا
 *      صادقانه برمی‌گرداند (الزام: هیچ قابلیت جعلی).
 * ============================================================================
 */

import { invoke } from "@tauri-apps/api/core";

// ============================================================================
// انواع معادل structهای Rust (core/src/ipc.rs) — تک‌منبعی با هسته
// ============================================================================

export interface SegmentsPayload {
  revision: number;
  positions: number[];
  kinds: number[]; // 0=Extrusion 1=Travel 2=Retract 3=Unretract 4=Arc 5=Unknown
  source_lines: number[];
  ids: number[];
  certainties: number[]; // 0=Known 1=Approximate 2=Unknown
  feedrates: number[];
}

export interface LayerPayload {
  index: number;
  z_mm: number | null;
  first_segment: number;
  last_segment: number;
  source_line_index: number;
}

export interface DiagPayload {
  severity_fa: string;
  code: string;
  message_fa: string;
  line_index: number | null;
  uncertain: boolean;
}

export interface StatsPayload {
  segment_count: number;
  extrusion_segments: number;
  travel_segments: number;
  retract_count: number;
  extrusion_length_mm: number;
  travel_length_mm: number;
  total_e_mm: number;
}

export interface ParseResult {
  revision: number;
  raw_lines: string[];
  n_words: number[];
  line_types_fa: string[];
  line_segment_offsets: number[];
  segments: SegmentsPayload;
  layers: LayerPayload[];
  diagnostics: DiagPayload[];
  stats: StatsPayload;
  spiral_mode: boolean;
  has_bom: boolean;
  line_ending_fa: string;
  default_save_as: string;
}

export interface ParamDetail {
  letter: string;
  raw_value: string;
  normalized: number | null;
  meaning_fa: string;
}

export interface CommandDetail {
  code: string;
  summary_fa: string;
  action_fa: string;
  params: ParamDetail[];
  firmware_dependent: boolean;
  unknown: boolean;
}

export interface LineDetail {
  raw: string;
  line_index: number;
  line_no: number;
  line_type_fa: string;
  commands: CommandDetail[];
  segment_ids: number[];
  state_before: Record<string, unknown>;
  state_after: Record<string, unknown>;
  e_delta: number | null;
  comment: string | null;
  n_word: number | null;
  checksum_valid: boolean | null;
}

export interface SaveAsRequest {
  content: string;
  original_content: string;
  path: string;
  checksum_policy: "recompute" | "strip" | "keep";
}

export interface SaveAsResult {
  bytes_written: number;
  checksum_policy_fa: string;
}

export interface SaveValidation {
  syntax_errors: number;
  unknown_commands: number;
  checksum_warnings: number;
  problem_lines: number[];
  messages_fa: string[];
}

/** نوع پروفایل پرینتر — معادل Rust */
export interface PrinterProfile {
  name: string;
  status: string;
  build_volume_mm: [number, number, number];
  nozzle_diameter_mm: number;
  filament_diameter_mm: number;
  line_width_mm: number;
  layer_height_mm: number;
  print_speed_mm_min: number;
  travel_speed_mm_min: number;
  flow_multiplier: number;
  absolute_e: boolean;
  start_gcode: string;
  end_gcode: string;
}

export interface PatternParams {
  kind: "Wave" | "Petal" | "Twist" | "Flute";
  amplitude_mm: number;
  count: number;
  frequency: number;
  phase_deg: number;
  z_start_mm: number;
  z_end_mm: number | null;
}

export interface TextureParams {
  kind: "Ripple" | "Weave" | "Stitch";
  amplitude_mm: number;
  period_mm: number;
  radial: boolean;
  phase_deg: number;
  fade_in_percent: number;
  fade_out_percent: number;
}

export interface ProfilePoint {
  r: number;
  h: number;
}

export interface FloorSettings {
  enabled: boolean;
  center_hole_diameter_mm: number;
  z_start_mm: number;
  ring_gap_mm: number;
}

export interface DesignerProject {
  schema_version: number;
  profile: ProfilePoint[];
  form_height_mm: number;
  form_bottom_diameter_mm: number;
  form_top_diameter_mm: number;
  taper_percent: number;
  patterns: PatternParams[];
  textures: TextureParams[];
  floor: FloorSettings;
  printer: PrinterProfile;
  created_at_unix: number;
  note: string;
}

/** خروجی موتور طراح — معادل DesignerOutput در Rust */
export interface ToolpathSegmentLite {
  id: number;
  source_line_index: number;
  kind: number;
  start_mm: number[] | null;
  end_mm: number[] | null;
  e_delta_mm: number | null;
  feedrate_mm_min: number | null;
  layer_index: number | null;
  certainty: number;
}

export interface MeshData {
  vertices: number[][];
  triangles: number[][];
}

export interface DiagLite {
  severity: string;
  code: string;
  message_fa: string;
  line_index: number | null;
  uncertain: boolean;
}

export interface DesignerOutput {
  segments: ToolpathSegmentLite[];
  line_count: number;
  mesh: MeshData;
  stats: StatsPayload;
  diagnostics: DiagLite[];
  layers: LayerPayload[];
  summary_fa: string;
}

export interface StlExportResult {
  byte_count: number;
  path: string;
  triangle_count: number;
}

// ============================================================================
// تشخیص محیط: Tauri واقعی یا مرورگر dev
// ============================================================================
const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** خطای استاندارد «قابلیت بدون پوستهٔ دسکتاپ» — صداقت به‌جای دادهٔ جعلی */
function notInDesktop(): never {
  throw new Error(
    "این قابلیت فقط در برنامهٔ دسکتاپ (Tauri) کار می‌کند؛ در حال حاضر در مرورگر توسعه اجرا می‌شود."
  );
}

// ============================================================================
// دستورهای IPC — نگاشت یک‌به‌یک به commands/mod.rs
// ============================================================================

/** تفسیر کامل فایل G-code */
export async function parseGcode(
  content: string,
  fileName: string | null
): Promise<ParseResult> {
  if (!isTauri) notInDesktop();
  return invoke("parse_gcode", { content, fileName });
}

/** توضیح کامل یک خط برای inspector */
export async function explainLine(
  content: string,
  lineIndex: number
): Promise<LineDetail | null> {
  if (!isTauri) notInDesktop();
  return invoke("explain_line", { content, lineIndex });
}

/** ذخیره با نام جدید */
export async function saveAs(req: SaveAsRequest): Promise<SaveAsResult> {
  if (!isTauri) notInDesktop();
  return invoke("save_as", { req });
}

/** اعتبارسنجی پیش از ذخیره */
export async function validateGcode(
  content: string,
  buildVolume: [number, number, number] | null
): Promise<SaveValidation> {
  if (!isTauri) notInDesktop();
  return invoke("validate_gcode", { content, buildVolume });
}

/** محاسبهٔ طراح مارپیچی */
export async function designerCompute(
  project: DesignerProject
): Promise<DesignerOutput> {
  if (!isTauri) notInDesktop();
  return invoke("designer_compute", { project });
}

/** خروجی G-code از طراح */
export async function designerExportGcode(
  project: DesignerProject
): Promise<string> {
  if (!isTauri) notInDesktop();
  return invoke("designer_export_gcode", { project });
}

/** خروجی STL در مسیر انتخابی */
export async function designerExportStl(
  project: DesignerProject,
  path: string
): Promise<StlExportResult> {
  if (!isTauri) notInDesktop();
  return invoke("designer_export_stl", { project, path });
}

/** ذخیرهٔ پروژهٔ طراح */
export async function saveProject(
  project: DesignerProject,
  path: string
): Promise<number> {
  if (!isTauri) notInDesktop();
  return invoke("save_project", { project, path });
}

/** بارگذاری پروژهٔ طراح */
export async function loadProject(path: string): Promise<DesignerProject> {
  if (!isTauri) notInDesktop();
  return invoke("load_project", { path });
}

/** خواندن فایل متنی از مسیر دیالوگ */
export async function readTextFile(path: string): Promise<string> {
  if (!isTauri) notInDesktop();
  return invoke("read_text_file", { path });
}

// ============================================================================
// دیالوگ‌های native — تنها plugin فعال (dialog)
// ============================================================================

/** دیالوگ بازکردن فایل G-code */
export async function openFileDialog(): Promise<string | null> {
  if (!isTauri) notInDesktop();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    title: "بازکردن فایل G-code",
    filters: [
      {
        name: "G-code",
        extensions: ["gcode", "gco", "g", "gc", "ngc", "txt"],
      },
    ],
    multiple: false,
    directory: false,
  });
  return typeof selected === "string" ? selected : null;
}

/** دیالوگ ذخیره با نام جدید */
export async function saveFileDialog(
  defaultName: string,
  ext: string
): Promise<string | null> {
  if (!isTauri) notInDesktop();
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save({
    title: "ذخیره با نام جدید",
    defaultPath: defaultName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
}

/** آیا در پوستهٔ دسکتاپ اجرا می‌شویم؟ (برای نمایش وضعیت صادقانه) */
export function runningInDesktop(): boolean {
  return isTauri;
}
