/**
 * ============================================================================
 * components/GcodeEditor.tsx — ویرایشگر G-code با CodeMirror 6
 * ============================================================================
 * الزامات Docs/PROMPT_02 §2 و §6:
 *   - شمارهٔ خط، syntax highlighting، اسکرول مجازی (فایل میلیون‌خطی)؛
 *   - جست‌وجو/goto line (از پکیج search)؛
 *   - نشانگر خطا/هشدار در حاشیه + خط انتخاب‌شده برجسته؛
 *   - ویرایش زنده با debounce کوتاه؛ متن نامعتبر حذف نمی‌شود؛
 *   - undo/redo استاندارد.
 *
 * جهت: ادیتور چپ‌به‌راست است (کد) ولی داخل shell فارسی RTL.
 * ============================================================================
 */

import { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import type { Extension, Range } from "@codemirror/state";
import {
  EditorView, keymap, lineNumbers, highlightActiveLine,
  highlightActiveLineGutter, Decoration,
  ViewPlugin, ViewUpdate,
} from "@codemirror/view";
import type { DecorationSet } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { StreamLanguage, LanguageSupport } from "@codemirror/language";

/** مارکر خطوط دارای خطا/هشدار — از diagnostics payload */
export interface LineMarker {
  lineIndex: number;
  severity: "error" | "warning" | "info";
}

export interface GcodeEditorProps {
  /** متن کامل (خطوط با \n) */
  doc: string;
  /** شمارهٔ خط انتخاب‌شده (۰مبنا) — برای highlight و اسکرول */
  selectedLine: number | null;
  /** مارکرها */
  markers: LineMarker[];
  /** disable در حالت parse */
  readOnly?: boolean;
  /** رویداد تغییر متن (debounce در caller) */
  onTextChange: (text: string) => void;
  /** رویداد انتخاب خط با کلیک/حرکت مکان‌نما */
  onLineSelect: (lineIndex: number) => void;
}

// ============================================================================
// زبان G-code — StreamLanguage سبک (lexer نمایشی؛ تفسیر واقعی در Rust است!)
// ============================================================================

/** واژه‌های کلیدی: G/M/T + N + پارامترها + کامنت */
const gcodeStream = StreamLanguage.define<{ inComment: boolean }>({
  name: "gcode",
  startState() { return { inComment: false }; },
  token(stream, state) {
    // کامنت ; تا پایان خط
    if (stream.eat(";")) {
      stream.skipToEnd();
      return "comment";
    }
    // کامنت پرانتزی
    if (stream.eat("(")) {
      state.inComment = true;
      return "comment";
    }
    if (state.inComment) {
      if (stream.eat(")")) {
        state.inComment = false;
        return "comment";
      }
      stream.next();
      return "comment";
    }
    // فاصله
    if (stream.eatSpace()) return null;
    // شمارهٔ خط N
    if (stream.match(/N\d+/i)) return "keyword";
    // فرمان G/M/T
    if (stream.match(/[GMTgmt]\d+(\.\d+)?/)) return "keyword";
    // پارامتر + عدد
    if (stream.match(/[XYZEFSRIJKPUBCATWHxyzefsrijkpubcatwh][-+]?\d*\.?\d+/)) {
      return "variableName";
    }
    // checksum
    if (stream.eat("*")) return "operator";
    // عدد تنها
    if (stream.match(/[-+]?\d*\.?\d+/)) return "number";
    // هرچیز دیگر
    stream.next();
    return null;
  },
  languageData: {
    commentTokens: { line: ";", block: { open: "(", close: ")" } },
  },
});

const gcodeLanguage = new LanguageSupport(gcodeStream);

// ============================================================================
// تم تیرهٔ هماهنگ با global.css
// ============================================================================
const theme = EditorView.theme(
  {
    "&": { backgroundColor: "#14161c", color: "#e8eaf0" },
    ".cm-content": { caretColor: "#ff7a28" },
    "&.cm-focused .cm-cursor": { borderLeftColor: "#ff7a28" },
    ".cm-gutters": { backgroundColor: "#1a1d24", color: "#7d8392", border: "none" },
    ".cm-activeLine": { backgroundColor: "#ffffff08" },
    ".cm-activeLineGutter": { backgroundColor: "#ffffff10" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "#ff7a2833" },
    ".cm-searchMatch": { backgroundColor: "#e0b34133" },
    ".cm-searchMatch-selected": { backgroundColor: "#e0b34166" },
    ".cm-tooltip": { backgroundColor: "#22262f", border: "1px solid #343945" },
    // توکن‌ها
    ".cm-keyword": { color: "#ff7a28", fontWeight: "600" },
    ".cm-comment": { color: "#6a7080", fontStyle: "italic" },
    ".cm-variableName": { color: "#46c878" },
    ".cm-number": { color: "#e0b341" },
    ".cm-operator": { color: "#d65a5a" },
    // مارکرها
    ".cm-line-error": { background: "#e0525222" },
    ".cm-line-warning": { background: "#e0b34122" },
    ".cm-line-info": { background: "#ffffff10" },
    // خط انتخاب‌شده (از خارج)
    ".cm-line-selected": { background: "#ff7a2826" },
  },
  { dark: true }
);

// ============================================================================
// افکت‌های خط: مارکر خطا/هشدار + خط انتخابی
// ============================================================================

function lineDeco(view: EditorView, markers: LineMarker[], selected: number | null): DecorationSet {
  const decos: Range<Decoration>[] = [];
  for (const m of markers) {
    if (m.lineIndex < view.state.doc.lines) {
      const cls = m.severity === "error" ? "cm-line-error"
        : m.severity === "warning" ? "cm-line-warning" : "cm-line-info";
      decos.push(
        Decoration.line({ class: cls }).range(view.state.doc.line(m.lineIndex + 1).from)
      );
    }
  }
  if (selected !== null && selected < view.state.doc.lines) {
    decos.push(
      Decoration.line({ class: "cm-line-selected" }).range(
        view.state.doc.line(selected + 1).from
      )
    );
  }
  return Decoration.set(decos, true);
}

// ============================================================================
// کامپوننت
// ============================================================================
export function GcodeEditor(props: GcodeEditorProps) {
  const holderRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const cbRef = useRef(props);
  cbRef.current = props;

  // ------------------------------------------------------------------------
  // ساخت view یک‌بار
  // ------------------------------------------------------------------------
  useEffect(() => {
    const holder = holderRef.current;
    if (!holder) return;

    const extensions: Extension[] = [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      history(),
      keymap.of([
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap,
        // Ctrl+G → goto line از طریق پرسش prompt ساده (بدون وابستگی بیرونی)
        { key: "Mod-g", run: () => {
            const input = window.prompt("شمارهٔ خط:");
            const n = parseInt(input || "", 10);
            if (!isNaN(n) && viewRef.current) {
              const view = viewRef.current;
              const line = Math.max(1, Math.min(view.state.doc.lines, n));
              view.dispatch({
                selection: { anchor: view.state.doc.line(line).from },
                effects: EditorView.scrollIntoView(view.state.doc.line(line).from, { y: "center" }),
              });
              cbRef.current.onLineSelect(line - 1);
            }
            return true;
        }},
      ]),
      searchKeymap.length ? highlightSelectionMatches() : [],
      gcodeLanguage,
      theme,
      EditorView.lineWrapping,
      EditorState.readOnly.of(!!props.readOnly),
      // رویدادها
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          cbRef.current.onTextChange(update.state.doc.toString());
        }
        if (update.selectionSet) {
          const head = update.state.selection.main.head;
          const line = update.state.doc.lineAt(head).number - 1;
          cbRef.current.onLineSelect(line);
        }
      }),
      // decorationها با state field
      ViewPlugin.fromClass(
        class {
          decos: DecorationSet;
          constructor(view: EditorView) {
            this.decos = lineDeco(view, cbRef.current.markers, cbRef.current.selectedLine);
          }
          update(update: ViewUpdate) {
            if (update.docChanged || update.selectionSet || update.viewportChanged) {
              this.decos = lineDeco(update.view, cbRef.current.markers, cbRef.current.selectedLine);
            } else {
              this.decos = this.decos.map(update.changes);
            }
          }
        },
        { decorations: (v) => v.decos }
      ),
    ];

    const state = EditorState.create({ doc: props.doc, extensions });
    const view = new EditorView({ state, parent: holder });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // doc فقط در ساخت اولیه؛ تغییرات بعدی از مسیر دیگری می‌آیند
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ------------------------------------------------------------------------
  // جایگزینی کل متن وقتی سند جدید از بیرون می‌آید (بازکردن فایل)
  // ------------------------------------------------------------------------
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (view.state.doc.toString() !== props.doc) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: props.doc },
      });
    }
  }, [props.doc]);

  // ------------------------------------------------------------------------
  // اسکرول به خط انتخاب‌شده از بیرون (کلیک روی segment)
  // ------------------------------------------------------------------------
  useEffect(() => {
    const view = viewRef.current;
    if (!view || props.selectedLine === null) return;
    const lineNo = props.selectedLine + 1;
    if (lineNo <= view.state.doc.lines) {
      const pos = view.state.doc.line(lineNo).from;
      view.dispatch({
        effects: EditorView.scrollIntoView(pos, { y: "center" }),
      });
    }
  }, [props.selectedLine]);

  // مارکرها/انتخاب با re-render decoration بالا اعمال می‌شوند (cbRef)

  return (
    <div className="cm-holder">
      <div ref={holderRef} className="ltr" />
    </div>
  );
}
