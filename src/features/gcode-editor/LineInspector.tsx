/**
 * ============================================================================
 * features/gcode-editor/LineInspector.tsx — پنل توضیح و ویرایش خط
 * ============================================================================
 * الزامات Docs/PROMPT_02 §5:
 *   ۱) متن خام خط و شماره؛ ۲) فرمان‌ها و معنی؛ ۳) جدول پارامترها؛
 *   ۴) شرح قابل‌فهم «چه می‌شود»؛ ۵) نوع خط؛ ۶) وضعیت قبل/بعد؛
 *   ۷) ارتباط با segmentها؛ ۸) هشدارها؛ ۹) ویرایش همین خط.
 *   خط غیرحرکتی → پیام روشن (segment جعلی نه).
 * ============================================================================
 */

import { useState } from "react";
import type { LineDetail } from "../../lib/tauri";
import { fa } from "../../i18n/fa";

export interface LineInspectorProps {
  /** توضیح کامل خط (null = خطی انتخاب نشده) */
  detail: LineDetail | null;
  /** هشدارهای مربوط به همین خط */
  lineWarnings: string[];
  /** متن فعلی خط در ادیتور (برای ویرایش) */
  currentLineText: string;
  /** اجرای ویرایش خط */
  onApplyEdit: (newText: string) => void;
  /** بازگردانی متن قبلی خط */
  onRevertLine: () => void;
}

/** ردیف جدول وضعیت ماشین */
function StateTable({ title, state }: { title: string; state: Record<string, unknown> }) {
  const rows: [string, string][] = [];
  const add = (k: string, v: unknown, suffix = "") => {
    if (v === null || v === undefined) return;
    rows.push([k, String(v) + suffix]);
  };
  add("X", state.x_mm, " mm");
  add("Y", state.y_mm, " mm");
  add("Z", state.z_mm, " mm");
  add("E", state.e_mm, " mm");
  add("F", state.feedrate_mm_min, " mm/min");
  add("واحد", state.units_fa);
  add("مختصات", state.absolute_xyz ? "مطلق (G90)" : "نسبی (G91)");
  add("اکسترودر", state.absolute_e ? "E مطلق (M82)" : "E نسبی (M83)");
  add("صفحهٔ قوس", state.plane_fa);
  add("دمای نازل", state.nozzle_temp_c, " °C");
  add("دمای بستر", state.bed_temp_c, " °C");
  if (state.fan_255 !== null && state.fan_255 !== undefined) {
    add("فن", `${state.fan_255}/255 (${state.fan_percent ?? "?"}%)`);
  }
  add("ابزار", state.tool, " (T)");
  add("اطمینان موقعیت", state.position_certain_fa);

  return (
    <div className="section">
      <h4>{title}</h4>
      <table className="state">
        <tbody>
          {rows.map(([k, v]) => (
            <tr key={k}>
              <th>{k}</th>
              <td className="val">{v}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** badge رنگ نوع خط بر اساس متن فارسی */
function typeBadgeClass(lineTypeFa: string): string {
  if (lineTypeFa.includes("چاپی")) return "print";
  if (lineTypeFa.includes("خالی")) return "travel";
  if (lineTypeFa.includes("عقب") || lineTypeFa.includes("بازگرداندن")) return "retract";
  if (lineTypeFa.includes("قوسی")) return "arc";
  if (lineTypeFa.includes("نامعتبر")) return "error";
  return "setting";
}

export function LineInspector(props: LineInspectorProps) {
  const [editText, setEditText] = useState<string | null>(null);
  const d = props.detail;

  if (!d) {
    return (
      <div className="inspector">
        <h3>{fa.lineInspector}</h3>
        <p style={{ color: "var(--fg-2)" }}>
          یک خط را در ویرایشگر یا پیش‌نمایش سه‌بعدی انتخاب کنید تا توضیح کامل آن نمایش یابد.
        </p>
      </div>
    );
  }

  // حالت ویرایش: متن اولیه از متن فعلی ادیتور
  const editing = editText !== null;
  const editorText = editing ? editText : props.currentLineText;

  return (
    <div className="inspector">
      <h3>
        {fa.lineInspector} — {fa.lineNo} {d.line_no}
        <span className={`badge ${typeBadgeClass(d.line_type_fa)}`} style={{ marginInlineStart: 8 }}>
          {d.line_type_fa}
        </span>
        {d.n_word !== null && (
          <span className="badge" style={{ marginInlineStart: 6 }}>N{d.n_word}</span>
        )}
        {d.checksum_valid === false && (
          <span className="badge error" style={{ marginInlineStart: 6 }}>
            {fa.checksumInvalid}
          </span>
        )}
      </h3>

      {/* ---------- ۱. متن خام (LTR) ---------- */}
      <div className="section">
        <h4>{fa.rawText}</h4>
        <div className="ltr" style={{
          background: "var(--bg-0)", border: "1px solid var(--line)",
          borderRadius: 6, padding: "6px 10px", fontSize: 13,
        }}>
          {d.raw || "«خط خالی»"}
        </div>
        {d.comment && (
          <div style={{ color: "var(--fg-2)", fontSize: 12, marginTop: 4 }}>
            کامنت: {d.comment}
          </div>
        )}
      </div>

      {/* ---------- ۲-۴. فرمان‌ها ---------- */}
      {d.commands.length === 0 && (
        <div className="section">
          <p style={{ color: "var(--fg-1)" }}>{fa.noMovementLine}</p>
        </div>
      )}
      {d.commands.map((cmd, i) => (
        <div className="section" key={i}>
          <h4>
            {cmd.code}
            {cmd.unknown && (
              <span className="badge error" style={{ marginInlineStart: 6 }}>ناشناخته</span>
            )}
            {cmd.firmware_dependent && (
              <span className="badge" style={{ marginInlineStart: 6 }}>وابسته به firmware</span>
            )}
          </h4>
          <p style={{ color: "var(--fg-1)", margin: "2px 0 6px" }}>{cmd.summary_fa}</p>
          <p style={{ color: "var(--fg-0)", margin: "2px 0 6px" }}>
            <strong>{fa.whatHappens}:</strong> {cmd.action_fa}
          </p>

          {/* ---------- ۳. جدول پارامترها ---------- */}
          {cmd.params.length > 0 && (
            <table className="state">
              <thead>
                <tr>
                  <th>{fa.paramLetter}</th>
                  <th>{fa.paramValue}</th>
                  <th>{fa.paramMeaning}</th>
                </tr>
              </thead>
              <tbody>
                {cmd.params.map((p) => (
                  <tr key={p.letter}>
                    <td className="val">{p.letter}</td>
                    <td className="val">
                      {p.raw_value}
                      {p.normalized !== null && p.normalized !== undefined &&
                        p.raw_value !== String(p.normalized) && (
                          <span style={{ color: "var(--fg-2)" }}> ({p.normalized} mm)</span>
                        )}
                    </td>
                    <td>{p.meaning_fa}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ))}

      {/* ---------- ΔE ---------- */}
      {d.e_delta !== null && d.e_delta !== undefined && (
        <div className="section">
          <h4>تغییر فیلامنت این خط</h4>
          <div className="ltr" style={{ color: d.e_delta >= 0 ? "var(--print)" : "var(--retract)" }}>
            ΔE = {d.e_delta.toFixed(4)} mm
          </div>
        </div>
      )}

      {/* ---------- ۶. وضعیت قبل/بعد ---------- */}
      <StateTable title={fa.stateBefore} state={d.state_before} />
      <StateTable title={fa.stateAfter} state={d.state_after} />

      {/* ---------- ۷. segmentهای مرتبط ---------- */}
      <div className="section">
        <h4>{fa.relatedSegments}</h4>
        {d.segment_ids.length > 0 ? (
          <p style={{ color: "var(--fg-1)" }}>
            {d.segment_ids.length} قطعهٔ مسیر به این خط نگاشت شده است
            {d.segment_ids.length > 1 && " (قطعات قوس تقسیم‌شده)"}؛ در نمای سه‌بعدی نارنجی برجسته شده‌اند.
          </p>
        ) : (
          <p style={{ color: "var(--fg-2)" }}>{fa.noMovementLine}</p>
        )}
      </div>

      {/* ---------- ۸. هشدارها ---------- */}
      {props.lineWarnings.length > 0 && (
        <div className="section">
          <h4 style={{ color: "var(--warn)" }}>{fa.warnings}</h4>
          <ul style={{ paddingInlineStart: 18, color: "var(--fg-1)", fontSize: 12 }}>
            {props.lineWarnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}

      {/* ---------- ۹. ویرایش همین خط ---------- */}
      <div className="section">
        <h4>{fa.editThisLine}</h4>
        <div className="ltr" style={{ display: "flex", gap: 6, alignItems: "stretch" }}>
          <input
            type="text"
            value={editorText}
            onChange={(e) => setEditText(e.target.value)}
            aria-label={fa.editThisLine}
            style={{
              flex: 1, background: "var(--bg-0)", color: "var(--fg-0)",
              border: "1px solid var(--line)", borderRadius: 6,
              padding: "6px 8px", fontFamily: "var(--mono)", direction: "ltr",
            }}
          />
          <button
            className="btn primary small"
            disabled={!editing}
            onClick={() => {
              if (editText !== null) {
                props.onApplyEdit(editText);
                setEditText(null);
              }
            }}
          >
            {fa.applyEdit}
          </button>
          <button
            className="btn small"
            onClick={() => setEditText(null)}
            disabled={!editing}
          >
            {fa.cancelEdit}
          </button>
          <button className="btn small" onClick={props.onRevertLine}>
            {fa.revertLine}
          </button>
        </div>
      </div>
    </div>
  );
}
