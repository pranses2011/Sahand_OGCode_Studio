/**
 * ============================================================================
 * features/spiral-designer/ProfileEditor.tsx — ادیتور پروفایل ۲بعدی
 * ============================================================================
 * فضای (شعاع-ارتفاع) با SVG: نقاط قابل افزودن/جابه‌جایی/حذف.
 *
 * نسخهٔ ۱.۵ (درخواست کاربر: «نقطه‌ها اسپیلاین باشند و با انتخاب هر نقطه
 * در دو سمت آن فلش‌ها ظاهر شوند برای تنظیم دقیق»):
 *   - منحنی نمایش = «همان ریاضی هستهٔ Rust»: اسپلاین مونوتون
 *     Fritsch–Carlson + هرمیت مکعبی؛ مماس صریح ProfilePoint.m (فلش‌ها)؛
 *     چه می‌بینید همان است که چاپ می‌شود (منبع واحد)؛
 *   - انتخاب نقطه (کلیک) → فلش‌های مماس «دو طرف» نقطه ظاهر می‌شوند؛
 *     کشیدن فلش = تنظیم شیب مماس (m = dr/dh) — مثل اپ‌های ویرایش منحنی؛
 *   - دابل‌کلیک روی فلش = بازگشت به مماس خودکار (m حذف می‌شود)؛
 *   - حالت Linear: خط چندضلعی (سازگاری ۱.۰/۱.۱) و بدون فلش.
 * ============================================================================
 */

import { useRef, useState } from "react";
import type { ProfilePoint, ProfileInterpolation } from "../../lib/tauri";
import { fa } from "../../i18n/fa";

export interface ProfileEditorProps {
  points: ProfilePoint[];
  height: number;
  maxR: number;
  interpolation?: ProfileInterpolation;
  onChange: (points: ProfilePoint[]) => void;
}

// ============================================================================
// ریاضی اسپلاین — آینهٔ TS از هستهٔ Rust (profile.rs)
// قابل‌آزمون (ProfileEditor.test.ts) و «همان» فرمول‌ها:
//   monotoneTangents = Fritsch–Carlson 1980
//   explicitOverride = cap ۶δ (effective_tangents)
//   hermite = درون‌یابی مکعبی هرمیتی
// ============================================================================

interface Knot {
  h: number;
  r: number;
}

/** شیب‌های هرمیتی مونوتون (Fritsch–Carlson) — همان پیاده‌سازی Rust */
export function monotoneTangents(pts: Knot[]): number[] {
  const n = pts.length;
  if (n < 2) return new Array(n).fill(0);
  const delta: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    const dh = pts[i + 1].h - pts[i].h;
    delta.push(Math.abs(dh) < 1e-12 ? 0 : (pts[i + 1].r - pts[i].r) / dh);
  }
  const m: number[] = new Array(n).fill(0);
  m[0] = delta[0];
  m[n - 1] = delta[n - 2];
  for (let i = 1; i < n - 1; i++) {
    if (delta[i - 1] * delta[i] <= 0) {
      m[i] = 0; // بیشینه/کمینهٔ محلی → مماس افقی
    } else {
      const w1 = 2 * (pts[i + 1].h - pts[i].h) + (pts[i].h - pts[i - 1].h);
      const w2 = (pts[i + 1].h - pts[i].h) + 2 * (pts[i].h - pts[i - 1].h);
      m[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
    }
  }
  for (let i = 0; i < n - 1; i++) {
    if (delta[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
    } else {
      const limit = 3 * delta[i];
      if ((delta[i] > 0 && m[i] > limit) || (delta[i] < 0 && m[i] < limit)) m[i] = limit;
      if ((delta[i] > 0 && m[i + 1] > limit) || (delta[i] < 0 && m[i + 1] < limit)) m[i + 1] = limit;
      if (delta[i] * m[i] < 0) m[i] = 0;
      if (delta[i] * m[i + 1] < 0) m[i + 1] = 0;
    }
  }
  return m;
}

/** مماس‌های مؤثر: خودکار + هم‌پوشانی مماس صریح m با سقف ۶δ — مثل Rust */
export function effectiveTangents(pts: Knot[], explicit: (number | undefined)[]): number[] {
  const t = monotoneTangents(pts);
  const n = pts.length;
  const deltaAt = (i: number): number => {
    let best = 0;
    if (i > 0) {
      const dh = pts[i].h - pts[i - 1].h;
      if (Math.abs(dh) > 1e-12) best = Math.max(best, Math.abs((pts[i].r - pts[i - 1].r) / dh));
    }
    if (i + 1 < n) {
      const dh = pts[i + 1].h - pts[i].h;
      if (Math.abs(dh) > 1e-12) best = Math.max(best, Math.abs((pts[i + 1].r - pts[i].r) / dh));
    }
    return best;
  };
  for (let i = 0; i < n && i < explicit.length; i++) {
    const mEx = explicit[i];
    if (mEx === undefined || !Number.isFinite(mEx)) continue;
    const cap = Math.max(6 * deltaAt(i), 0.05);
    t[i] = Math.max(-cap, Math.min(cap, mEx));
  }
  return t;
}

/** ارزیابی هرمیت مکعبی در بازهٔ [i, i+1] — همان فرمول Rust */
export function hermiteAt(pts: Knot[], m: number[], i: number, t: number): number {
  const { h: h0, r: r0 } = pts[i];
  const { h: h1, r: r1 } = pts[i + 1];
  const dh = h1 - h0;
  if (Math.abs(dh) < 1e-12) return r0;
  const t2 = t * t, t3 = t2 * t;
  const h00 = 2 * t3 - 3 * t2 + 1;
  const h10 = t3 - 2 * t2 + t;
  const h01 = -2 * t3 + 3 * t2;
  const h11 = t3 - t2;
  return h00 * r0 + h10 * dh * m[i] + h01 * r1 + h11 * dh * m[i + 1];
}

/** مسیر SVG نرم از نقاط — اسپلاین یا خطی (تطابق با هسته) */
export function buildCurvePath(
  pts: Knot[],
  explicit: (number | undefined)[],
  interpolation: ProfileInterpolation,
  toXY: (p: Knot) => [number, number],
  samplesPerInterval = 14
): string {
  if (pts.length < 2) return "";
  if (interpolation === "Linear") {
    return pts.map((p, i) => {
      const [x, y] = toXY(p);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
  }
  const m = effectiveTangents(pts, explicit);
  let d = "";
  for (let i = 0; i < pts.length - 1; i++) {
    for (let s = 0; s <= samplesPerInterval; s++) {
      const t = s / samplesPerInterval;
      const r = hermiteAt(pts, m, i, t);
      const h = pts[i].h + (pts[i + 1].h - pts[i].h) * t;
      const [x, y] = toXY({ h, r });
      d += `${s === 0 && i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)} `;
    }
  }
  return d.trim();
}

// ============================================================================
// ادیتور
// ============================================================================

const W = 340, H = 300, PAD = 18;

export function ProfileEditor(props: ProfileEditorProps) {
  const { points, height, maxR, interpolation = "Spline", onChange } = props;
  const svgRef = useRef<SVGSVGElement>(null);
  const draggingIdx = useRef<number | null>(null);
  const draggingHandle = useRef<number | null>(null); // ایندکس نقطهٔ فلش
  const [selected, setSelected] = useState<number | null>(null);

  const knots: Knot[] = points.map((p) => ({ h: p.h, r: p.r }));
  const explicit: (number | undefined)[] = points.map((p) => p.m);

  const toXY = (p: Knot): [number, number] => {
    const sx = (W - 2 * PAD) / (2 * maxR);
    const sy = (H - 2 * PAD) / Math.max(height, 1);
    return [W / 2 + p.r * sx, H - PAD - p.h * sy];
  };
  const fromSvg = (x: number, y: number): ProfilePoint => {
    const sx = (2 * maxR) / (W - 2 * PAD);
    const sy = height / (H - 2 * PAD);
    return {
      r: Math.max(0, (x - W / 2) * sx),
      h: Math.max(0, Math.min(height, (H - PAD - y) * sy)),
    };
  };

  /** رویداد → مختصات mm */
  const pt = (e: { clientX: number; clientY: number }) => {
    const svg = svgRef.current!;
    const rect = svg.getBoundingClientRect();
    return fromSvg(
      ((e.clientX - rect.left) / rect.width) * W,
      ((e.clientY - rect.top) / rect.height) * H
    );
  };

  /** mm → صفحهٔ SVG (برای محاسبهٔ مماس از درگ) */
  const svgOf = (p: ProfilePoint): [number, number] => {
    const sx = (W - 2 * PAD) / (2 * maxR);
    const sy = (H - 2 * PAD) / Math.max(height, 1);
    return [W / 2 + p.r * sx, H - PAD - p.h * sy];
  };

  // مسیر نرم — «همان ریاضی هسته» (اسپلاین/خطی)
  const path = buildCurvePath(knots, explicit, interpolation, toXY);
  // مسیر آینه‌ای (نمایش حجمی‌مانند — صرفاً بصری)
  const mirrored = path
    .replace(/([ML])([\d.]+),([\d.]+)/g, (_mm, cmd: string, x: string, y: string) =>
      `${cmd}${(W - parseFloat(x)).toFixed(1)},${y}`)
    .replace(/^M/, "L");

  // محاسبهٔ جهت فلش مماس نقطهٔ انتخاب‌شده — در فضای صفحه
  const tangentInfo = (() => {
    if (selected === null || selected >= points.length || interpolation !== "Spline") return null;
    const m = effectiveTangents(knots, explicit);
    const slope = m[selected]; // dr/dh در mm
    // جهت در صفحه: r→x مثبت، h→y منفی ⇒ بردار (slope, −۱) نرمال‌شده
    const len = Math.hypot(slope, 1) || 1;
    const ux = slope / len, uy = -1 / len;
    const L = 42; // طول فلش (px)
    const [px, py] = svgOf(points[selected]);
    return {
      px, py, ux, uy, L,
      mx: px + ux * L, my: py + uy * L,
      nx: px - ux * L, ny: py - uy * L,
      slope,
    };
  })();

  /** درگ فلش مماس → m جدید (سقف ۶δ مثل هسته) */
  const setTangentFromPointer = (e: { clientX: number; clientY: number }, idx: number) => {
    const svg = svgRef.current!;
    const rect = svg.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * W;
    const y = ((e.clientY - rect.top) / rect.height) * H;
    const [px, py] = svgOf(points[idx]);
    const sx = (2 * maxR) / (W - 2 * PAD); // px→mm در r
    const sy = Math.max(height, 1) / (H - 2 * PAD); // px→mm در h
    // mm در فضا: Δr = (x−px)·sx؛ Δh = (py−y)·sy
    const dr = (x - px) * sx;
    const dh = (py - y) * sy;
    // سقف دامنه مثل هسته: ۶×بیشینهٔ شیب سکانت مجاور
    const n = knots.length;
    const deltaAt = (i: number): number => {
      let best = 0;
      if (i > 0) {
        const d = knots[i].h - knots[i - 1].h;
        if (Math.abs(d) > 1e-12) best = Math.max(best, Math.abs((knots[i].r - knots[i - 1].r) / d));
      }
      if (i + 1 < n) {
        const d = knots[i + 1].h - knots[i].h;
        if (Math.abs(d) > 1e-12) best = Math.max(best, Math.abs((knots[i + 1].r - knots[i].r) / d));
      }
      return best;
    };
    const cap = Math.max(6 * deltaAt(idx), 0.05);
    let m: number;
    if (Math.abs(dh) < 1e-6) {
      m = dr >= 0 ? cap : -cap; // قائم کامل → سقف
    } else {
      m = Math.max(-cap, Math.min(cap, dr / dh));
    }
    const next = [...points];
    next[idx] = { ...next[idx], m: Math.round(m * 100) / 100 };
    onChange(next);
  };

  /** بازگشت مماس به خودکار — دابل‌کلیک روی فلش */
  const resetTangent = (idx: number) => {
    const next = [...points];
    const { r, h } = next[idx];
    next[idx] = { r, h }; // m حذف
    onChange(next);
  };

  const handleLen = 7;

  return (
    <div>
      <svg
        ref={svgRef}
        className="profile-svg"
        viewBox={`0 0 ${W} ${H}`}
        onDoubleClick={(e) => {
          const np = pt(e);
          const next = [...points, { r: Math.round(np.r * 10) / 10, h: Math.round(np.h * 10) / 10 }]
            .sort((a, b) => a.h - b.h);
          onChange(next);
        }}
        onPointerMove={(e) => {
          if (draggingHandle.current !== null) {
            setTangentFromPointer(e, draggingHandle.current);
            return;
          }
          if (draggingIdx.current === null) return;
          const np = pt(e);
          const next = [...points];
          next[draggingIdx.current] = {
            r: Math.max(0, np.r),
            h: np.h,
            m: next[draggingIdx.current]?.m,
          };
          // مرتب‌سازی مجدد هنگام عبور از همسایه — ایندکس انتخاب/درگ اصلاح
          const dragged = next[draggingIdx.current];
          const sorted = [...next].sort((a, b) => a.h - b.h);
          const newIdx = sorted.findIndex((p) => p === dragged);
          draggingIdx.current = newIdx;
          setSelected(newIdx);
          onChange(sorted);
        }}
        onPointerUp={() => {
          draggingIdx.current = null;
          draggingHandle.current = null;
        }}
        onPointerLeave={() => {
          draggingIdx.current = null;
          draggingHandle.current = null;
        }}
      >
        {/* خط محور و پایه */}
        <line x1={W / 2} y1={12} x2={W / 2} y2={H - 12} stroke="#343945" strokeDasharray="4 4" />
        <line x1={10} y1={H - 18} x2={W - 10} y2={H - 18} stroke="#343945" />

        {/* ناحیهٔ فرم (آینه‌ای) */}
        <path
          d={`M${W / 2},${H - 18} ${path.replace(/^M/, "L")} ${mirrored} Z`}
          fill="#46c87822"
          stroke="none"
        />
        {/* منحنی پروفایل — اسپلاین هسته */}
        <path d={path} stroke="#ff7a28" strokeWidth={2} fill="none" />

        {/* ---- فلش‌های مماس نقطهٔ انتخاب‌شده (نسخهٔ ۱.۵) ---- */}
        {tangentInfo && (() => {
          const ti = tangentInfo;
          const arrow = (tipX: number, tipY: number, dirX: number, dirY: number) => {
            // پیکان در نوک فلش (رو به بیرون از نقطه)
            const nx = -dirY, ny = dirX;
            const w = 4.2;
            return (
              <polygon
                points={`${tipX},${tipY} ${tipX - dirX * handleLen + nx * w},${tipY - dirY * handleLen + ny * w} ${tipX - dirX * handleLen - nx * w},${tipY - dirY * handleLen - ny * w}`}
                fill="#ffd23f"
                stroke="#14161c"
                strokeWidth={0.8}
              />
            );
          };
          return (
            <g className="tangent-g">
              {/* خط مماس عبوری از نقطه */}
              <line
                x1={ti.nx} y1={ti.ny} x2={ti.mx} y2={ti.my}
                stroke="#ffd23f" strokeWidth={1.6} strokeDasharray="5 3" opacity={0.9}
              />
              {/* فلش سمت مثبت */}
              {arrow(ti.mx, ti.my, ti.ux, ti.uy)}
              {/* فلش سمت منفی */}
              {arrow(ti.nx, ti.ny, -ti.ux, -ti.uy)}
              {/* دسته‌های قابل‌درگ (دو سر) */}
              <circle
                cx={ti.mx} cy={ti.my} r={6}
                fill="#ffd23f" stroke="#14161c" strokeWidth={1.5}
                style={{ cursor: "crosshair" }}
                onPointerDown={(e) => {
                  e.stopPropagation();
                  draggingHandle.current = selected;
                }}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  if (selected !== null) resetTangent(selected);
                }}
              />
              <circle
                cx={ti.nx} cy={ti.ny} r={6}
                fill="#ffd23f" stroke="#14161c" strokeWidth={1.5}
                style={{ cursor: "crosshair" }}
                onPointerDown={(e) => {
                  e.stopPropagation();
                  draggingHandle.current = selected;
                }}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  if (selected !== null) resetTangent(selected);
                }}
              />
              {/* برچسب شیب مماس */}
              <text
                x={ti.px + 10} y={ti.py - 8} fontSize={9.5} fill="#ffd23f" className="ltr"
                style={{ fontWeight: 600 }}
              >
                m={points[selected!].m !== undefined
                  ? points[selected!].m!.toFixed(2)
                  : `${ti.slope.toFixed(2)} (خودکار)`}
              </text>
            </g>
          );
        })()}

        {/* نقاط قابل‌کشیدن */}
        {points.map((p, i) => {
          const [x, y] = svgOf(p);
          const isSel = selected === i;
          return (
            <g key={i}>
              {isSel && (
                <circle cx={x} cy={y} r={11} fill="#ff7a2833" stroke="none" />
              )}
              <circle
                cx={x} cy={y} r={isSel ? 8 : 7}
                fill={isSel ? "#ffb27a" : "#ff7a28"}
                stroke={isSel ? "#ffd23f" : "#14161c"}
                strokeWidth={isSel ? 2.5 : 2}
                style={{ cursor: "grab" }}
                onPointerDown={(e) => {
                  e.stopPropagation();
                  draggingIdx.current = i;
                  setSelected(i);
                }}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  if (points.length > 2) {
                    onChange(points.filter((_, j) => j !== i));
                    setSelected(null);
                  }
                }}
              />
              <text x={x + 10} y={y + 4} fontSize={9} fill="#7d8392" className="ltr">
                {p.r.toFixed(0)}×{p.h.toFixed(0)}
                {p.m !== undefined ? "◆" : ""}
              </text>
            </g>
          );
        })}
        <text x={W / 2 + 6} y={22} fontSize={10} fill="#7d8392">r (mm)</text>
        <text x={W - 60} y={H - 24} fontSize={10} fill="#7d8392">h (mm)</text>
      </svg>
      <div style={{ fontSize: 11, color: "var(--fg-2)", marginTop: 4 }}>
        {fa.addPointHint} — {fa.tangentHint}
      </div>
    </div>
  );
}
