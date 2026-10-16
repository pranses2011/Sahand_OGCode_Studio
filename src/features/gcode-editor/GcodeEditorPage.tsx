/**
 * ============================================================================
 * features/gcode-editor/GcodeEditorPage.tsx — صفحهٔ اصلی ویرایشگر G-code
 * ============================================================================
 * گردش‌کار کامل (Docs/PROMPT_02):
 *   بازکردن فایل (دیالوگ native) → تفسیر Rust → ادیتور + viewport
 *   → انتخاب خط↔segment دوطرفه → ویرایش زنده (debounce) → re-parse
 *   → Save As با اعتبارسنجی + سیاست checksum.
 *
 * undo/redo: تاریخچهٔ متن در سطح صفحه (snapshotها) — بازگردانی متن کامل.
 * ============================================================================
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { GcodeEditor } from "../../components/GcodeEditor";
import type { LineMarker } from "../../components/GcodeEditor";
import { Viewport } from "../../components/Viewport";
import { LineInspector } from "./LineInspector";
import { fa } from "../../i18n/fa";
import type { ParseResult, LineDetail, SaveValidation } from "../../lib/tauri";
import {
  openFileDialog, saveFileDialog, readTextFile,
  parseGcode, explainLine, saveAs, validateGcode, runningInDesktop, coreAvailable,
} from "../../lib/tauri";

/** debounce ویرایش تایپ (الزام §6: کوتاه، با وضعیت روشن) */
const EDIT_DEBOUNCE_MS = 400;

export interface GcodeEditorPageProps {
  /** محتوای اولیه از طراح مارپیچی (ارسال به ویرایشگر) */
  initialContent?: string | null;
  /** نام نمایشی فایل اولیه */
  initialName?: string | null;
  /** اطلاع از مصرف محتوای اولیه (تا دوباره اعمال نشود) */
  onConsumed?: () => void;
  /** رنگ رشتهٔ چاپ در پیش‌نمایش (نمایشی — از ColorBar) */
  extrusionColor?: string;
}

export function GcodeEditorPage(props: GcodeEditorPageProps) {
  // ---------------------------------------------------------------- state
  const [fileName, setFileName] = useState<string | null>(props.initialName ?? null);
  const [originalText, setOriginalText] = useState<string>(props.initialContent ?? "");
  const [currentText, setCurrentText] = useState<string>(props.initialContent ?? "");
  const [parse, setParse] = useState<ParseResult | null>(null);
  const [detail, setDetail] = useState<LineDetail | null>(null);
  const [selectedLine, setSelectedLine] = useState<number | null>(null);
  const [status, setStatus] = useState<"empty" | "parsing" | "ready" | "updating" | "error">("empty");
  const [errorMsg, setErrorMsg] = useState<string>("");
  const [validation, setValidation] = useState<SaveValidation | null>(null);
  const [showValidation, setShowValidation] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [simLine, setSimLine] = useState<number>(0);
  const [filterMode, setFilterMode] = useState<"all" | "upTo" | "onlyHighlight">("all");

  // undo/redo متن (سطح صفحه)
  const undoStack = useRef<string[]>([]);
  const redoStack = useRef<string[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);

  // debounce timer
  const debounceRef = useRef<number | null>(null);
  // جلوگیری از push بی‌متن به undo وقتی بازکردن فایل است
  const loadingRef = useRef(false);

  const pushUndo = useCallback((text: string) => {
    undoStack.current.push(text);
    if (undoStack.current.length > 200) undoStack.current.shift();
    redoStack.current = [];
    setCanUndo(true);
    setCanRedo(false);
  }, []);

  /** تفسیر مجدد متن — پس از بازکردن یا ویرایش */
  const reparse = useCallback(async (text: string, name: string | null) => {
    if (!coreAvailable()) {
      setStatus("error");
      setErrorMsg("تفسیر فقط در برنامهٔ دسکتاپ (Tauri) انجام می‌شود.");
      return;
    }
    setStatus(text === originalText ? "parsing" : "updating");
    try {
      const result = await parseGcode(text, name);
      setParse(result);
      setStatus("ready");
      setErrorMsg("");
    } catch (e) {
      // متن نامعتبر → preview قبلی می‌ماند (الزام §6)
      setStatus("error");
      setErrorMsg(String(e));
    }
  }, [originalText]);

  // محاسبهٔ اولیه — شامل محتوای ارسالی از طراح مارپیچی
  // (الزام PROMPT_03 §6: خروجی طراح با یک کلیک در ویرایشگر باز شود)
  useEffect(() => {
    if (props.initialContent) {
      const text = props.initialContent;
      const name = props.initialName ?? "spiral-designer-output.gcode";
      loadingRef.current = true;
      reparse(text, name);
      loadingRef.current = false;
      props.onConsumed?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ------------------------------------------------------------ بازکردن فایل
  const handleOpen = useCallback(async () => {
    try {
      const path = await openFileDialog();
      if (!path) return;
      const content = await readTextFile(path);
      const name = path.split(/[\\/]/).pop() || "file.gcode";
      loadingRef.current = true;
      undoStack.current = [];
      redoStack.current = [];
      setCanUndo(false); setCanRedo(false);
      setOriginalText(content);
      setCurrentText(content);
      setFileName(name);
      setDirty(false);
      setSelectedLine(null);
      setDetail(null);
      setValidation(null);
      await reparse(content, name);
      loadingRef.current = false;
    } catch (e) {
      loadingRef.current = false;
      setStatus("error");
      setErrorMsg(String(e));
    }
  }, [reparse]);

  // ------------------------------------------------------------ ویرایش زنده
  const handleTextChange = useCallback((text: string) => {
    if (loadingRef.current) return;
    setCurrentText(text);
    if (text !== originalText && !dirty) setDirty(true);
    // debounce
    if (debounceRef.current !== null) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      pushUndo(originalText.length ? currentTextRef.current : text);
      reparse(text, fileName);
    }, EDIT_DEBOUNCE_MS);
  }, [fileName, originalText, pushUndo, reparse]);

  // ref برای متن فعلی در debounce (بی‌نیاز از closure قدیمی)
  const currentTextRef = useRef(currentText);
  currentTextRef.current = currentText;

  // ------------------------------------------------------------ انتخاب خط
  const handleLineSelect = useCallback(async (lineIndex: number) => {
    setSelectedLine(lineIndex);
    setSimLine(lineIndex);
    if (!coreAvailable() || !currentText) return;
    try {
      const d = await explainLine(currentText, lineIndex);
      setDetail(d);
    } catch {
      setDetail(null);
    }
  }, [currentText]);

  // انتخاب segment در viewport → خط مبدأ
  const handleSegmentClick = useCallback((segmentId: number) => {
    if (!parse) return;
    const idx = parse.segments.ids.indexOf(segmentId);
    if (idx >= 0) {
      const lineIdx = parse.segments.source_lines[idx];
      handleLineSelect(lineIdx);
    }
  }, [parse, handleLineSelect]);

  // detail متن خام خط از متن فعلی (برای ویرایش)
  const currentLineText = selectedLine !== null
    ? (currentText.split("\n")[selectedLine] ?? "")
    : "";

  // segmentهای برجستهٔ خط انتخابی
  const highlightIds = useCallback(() => {
    const s = new Set<number>();
    if (parse && selectedLine !== null) {
      const start = parse.line_segment_offsets[selectedLine] ?? 0;
      const end = parse.line_segment_offsets[selectedLine + 1] ?? start;
      for (let i = start; i < end; i++) {
        if (i < parse.segments.ids.length) s.add(parse.segments.ids[i]);
      }
    }
    return s;
  }, [parse, selectedLine])();

  // ------------------------------------------------------------ Save As
  const handleSaveAs = useCallback(async () => {
    if (!currentText) return;
    // اعتبارسنجی پیش از ذخیره (الزام §8)
    let v: SaveValidation | null = null;
    if (coreAvailable()) {
      try {
        v = await validateGcode(currentText, null);
        setValidation(v);
      } catch { /* بدون اعتبارسنجی ادامه */ }
    }
    setShowValidation(true);
  }, [currentText]);

  const confirmSave = useCallback(async (policy: "recompute" | "strip" | "keep") => {
    setShowValidation(false);
    if (!runningInDesktop()) return;
    const defaultName = parse?.default_save_as || "edited.gcode";
    const path = await saveFileDialog(defaultName, "gcode");
    if (!path) return;
    try {
      const result = await saveAs({
        content: currentText,
        original_content: originalText,
        path,
        checksum_policy: policy,
      });
      setDirty(false);
      setStatus("ready");
      // اطلاع کوتاه
      setErrorMsg(`ذخیره شد: ${path} (${result.bytes_written} بایت) — ${result.checksum_policy_fa}`);
    } catch (e) {
      setStatus("error");
      setErrorMsg(String(e));
    }
  }, [currentText, originalText, parse]);

  // ------------------------------------------------------------ undo/redo
  const doUndo = useCallback(() => {
    if (undoStack.current.length === 0) return;
    const prev = undoStack.current.pop()!;
    redoStack.current.push(currentText);
    setCurrentText(prev);
    setCanUndo(undoStack.current.length > 0);
    setCanRedo(true);
    reparse(prev, fileName);
  }, [currentText, fileName, reparse]);

  const doRedo = useCallback(() => {
    if (redoStack.current.length === 0) return;
    const next = redoStack.current.pop()!;
    undoStack.current.push(currentText);
    setCurrentText(next);
    setCanRedo(redoStack.current.length > 0);
    setCanUndo(true);
    reparse(next, fileName);
  }, [currentText, fileName, reparse]);

  // ------------------------------------------------------------ میانبرها
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.key === "o") { e.preventDefault(); handleOpen(); }
      else if (e.key === "s") { e.preventDefault(); handleSaveAs(); }
      else if (e.key === "z") { e.preventDefault(); doUndo(); }
      else if (e.key === "y") { e.preventDefault(); doRedo(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleOpen, handleSaveAs, doUndo, doRedo]);

  // ------------------------------------------------------------ مارکرها
  const markers: LineMarker[] = (parse?.diagnostics ?? [])
    .filter((d) => d.line_index !== null)
    .map((d) => ({
      lineIndex: d.line_index as number,
      severity: d.severity_fa === "خطا" ? "error"
        : d.severity_fa === "هشدار" ? "warning" : "info",
    }));

  const lineWarnings = (parse?.diagnostics ?? [])
    .filter((d) => d.line_index === selectedLine)
    .map((d) => `${d.severity_fa} [${d.code}]: ${d.message_fa}`);

  // ------------------------------------------------------ شبیه‌سازی ساده
  // اسلایدر خط: خط فعلی → segmentهای تا آن خط «تا این خط» نمایش می‌یابند
  const simSegmentId = useCallback(() => {
    if (!parse) return 0;
    const start = parse.line_segment_offsets[simLine] ?? 0;
    const id = start < parse.segments.ids.length ? parse.segments.ids[start] : 0;
    return id;
  }, [parse, simLine])();

  const totalLines = parse?.raw_lines.length ?? 0;

  // ------------------------------------------------------------ رندر
  return (
    <div className="editor-layout">
      {/* ---------- نوار ابزار ---------- */}
      <div className="topbar">
        <button className="btn primary" onClick={handleOpen}>{fa.openFile}</button>
        <button className="btn" disabled={!currentText} onClick={handleSaveAs}>{fa.saveAs}</button>
        <button className="btn" disabled={!canUndo} onClick={doUndo}>{fa.undo}</button>
        <button className="btn" disabled={!canRedo} onClick={doRedo}>{fa.redo}</button>
        <span className="file-name ltr">{fileName ?? fa.noFile}</span>
        {dirty && <span className="dirty">● {fa.unsavedChanges}</span>}
        <span className="badge">
          {status === "empty" ? fa.statusEmpty
            : status === "parsing" ? fa.statusParsing
            : status === "updating" ? fa.statusUpdating
            : status === "ready" ? fa.statusReady
            : fa.statusError}
        </span>
        {status === "updating" && <span className="badge">{fa.oldPreview}</span>}
      </div>

      <div className="work">
        {/* ---------- ستون کد ---------- */}
        <div className="col" style={{ flex: 1 }}>
          <GcodeEditor
            doc={currentText}
            selectedLine={selectedLine}
            markers={markers}
            onTextChange={handleTextChange}
            onLineSelect={handleLineSelect}
          />
        </div>

        <div className="splitter" />

        {/* ---------- ستون سه‌بعدی ---------- */}
        <div className="col" style={{ flex: 1 }}>
          <Viewport
            segments={parse?.segments ?? null}
            highlightIds={highlightIds}
            filter={{ mode: filterMode, upToId: simSegmentId }}
            buildVolume={null}
            emptyMessage={
              status === "empty" ? fa.openHint
                : status === "error" ? `${fa.statusError}: ${errorMsg || "تفسیر ناموفق"}`
                : "مسیری برای نمایش نیست — فایل شامل حرکت قابل‌تفسیر نیست."
            }
            webglMessage={fa.webglMissing}
            extrusionColor={props.extrusionColor}
            onSegmentClick={handleSegmentClick}
          />
        </div>
      </div>

      {/* ---------- نوار شبیه‌سازی ---------- */}
      {totalLines > 0 && (
        <div className="simbar">
          <span className="sim-label">{fa.simulation}:</span>
          <button className="btn small" onClick={() => setSimLine(0)} aria-label={fa.toStart}>⏮</button>
          <button className="btn small" onClick={() => setSimLine(Math.max(0, simLine - 1))} aria-label={fa.stepBack}>◀</button>
          <button className="btn small" onClick={() => setSimLine(Math.min(totalLines - 1, simLine + 1))} aria-label={fa.stepForward}>▶</button>
          <button className="btn small" onClick={() => setSimLine(totalLines - 1)} aria-label={fa.toEnd}>⏭</button>
          <input
            type="range" min={0} max={totalLines - 1} value={simLine}
            onChange={(e) => setSimLine(parseInt(e.target.value, 10))}
            aria-label={fa.progress}
          />
          <span className="sim-label ltr">{simLine + 1} / {totalLines}</span>
          <select
            value={filterMode}
            onChange={(e) => setFilterMode(e.target.value as typeof filterMode)}
            aria-label="فیلتر نمایش"
            className="btn small"
          >
            <option value="all">{fa.showAll}</option>
            <option value="upTo">{fa.showUpToLine}</option>
            <option value="onlyHighlight">{fa.showOnlyLine}</option>
          </select>
          {parse?.spiral_mode && <span className="badge accent">{fa.spiralMode}</span>}
        </div>
      )}

      {/* ---------- پنل توضیح ---------- */}
      <LineInspector
        detail={detail}
        lineWarnings={lineWarnings}
        currentLineText={currentLineText}
        onApplyEdit={(newText) => {
          // جایگزینی همان خط در متن کلی (الزام: فقط خط هدف تغییر کند)
          if (selectedLine === null) return;
          const lines = currentText.split("\n");
          if (selectedLine < lines.length) {
            pushUndo(currentText);
            lines[selectedLine] = newText;
            const updated = lines.join("\n");
            setCurrentText(updated);
            setDirty(true);
            reparse(updated, fileName);
          }
        }}
        onRevertLine={() => {
          if (selectedLine === null) return;
          const lines = originalText.split("\n");
          if (selectedLine < lines.length) {
            pushUndo(currentText);
            const cur = currentText.split("\n");
            cur[selectedLine] = lines[selectedLine];
            const updated = cur.join("\n");
            setCurrentText(updated);
            reparse(updated, fileName);
          }
        }}
      />

      {/* ---------- نوار وضعیت ---------- */}
      <div className="statusbar">
        <span className="chip">{fa.lineCount}: {totalLines}</span>
        <span className="chip">{fa.segmentCount}: {parse?.stats.segment_count ?? 0}</span>
        <span className="chip">{fa.layerCount}: {parse?.layers.length ?? 0}</span>
        {parse && <span className="chip ltr">{parse.line_ending_fa}{parse.has_bom ? " +BOM" : ""}</span>}
        <span className="chip ltr">
          E≈{(parse?.stats.total_e_mm ?? 0).toFixed(1)}mm
        </span>
        <span className="chip ltr">L≈{(parse?.stats.extrusion_length_mm ?? 0 / 10).toFixed(0)}cm</span>
        <span className="safety">{fa.safetyNotice}</span>
        {errorMsg && status !== "updating" && (
          <span className="chip" style={{ color: "var(--error)" }}>{errorMsg}</span>
        )}
      </div>

      {/* ---------- مودال اعتبارسنجی Save As ---------- */}
      {showValidation && (
        <div className="modal-back" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>{fa.validationTitle}</h3>
            {validation ? (
              <ul>
                <li>{fa.syntaxErrors}: {validation.syntax_errors}</li>
                <li>{fa.unknownCommands}: {validation.unknown_commands}</li>
                <li>{fa.checksumWarnings}: {validation.checksum_warnings}</li>
                {validation.messages_fa.slice(0, 12).map((m, i) => (
                  <li key={i} style={{ color: "var(--fg-2)" }}>• {m}</li>
                ))}
              </ul>
            ) : (
              <p style={{ color: "var(--fg-1)" }}>{fa.noProblems}</p>
            )}
            <p style={{ color: "var(--warn)", fontSize: 12 }}>{fa.safetyNotice}</p>
            <div className="actions">
              <button className="btn primary" onClick={() => confirmSave("recompute")}>
                ذخیره (بازمحاسبهٔ N/checksum)
              </button>
              <button className="btn" onClick={() => confirmSave("keep")}>
                ذخیره (حفظ ساختار فعلی)
              </button>
              <button className="btn danger" onClick={() => confirmSave("strip")}>
                ذخیره (حذف N/checksum)
              </button>
              <button className="btn" onClick={() => setShowValidation(false)}>انصراف</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
