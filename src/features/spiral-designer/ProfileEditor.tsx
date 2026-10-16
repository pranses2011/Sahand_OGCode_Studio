/**
 * ============================================================================
 * features/spiral-designer/ProfileEditor.tsx — گام ۱: ادیتور پروفایل ۲بعدی
 * ============================================================================
 * فضای (شعاع-ارتفاع) با SVG: نقاط قابل افزودن/جابه‌جایی/حذف.
 * الزام PROMPT_03 §4 گام ۱: pen/چندضلعی/پروفایل‌های آماده «قابل‌ویرایش».
 * ============================================================================
 */

import { useRef } from "react";
import type { ProfilePoint } from "../../lib/tauri";
import { fa } from "../../i18n/fa";

export interface ProfileEditorProps {
  points: ProfilePoint[];
  height: number;
  maxR: number;
  onChange: (points: ProfilePoint[]) => void;
}

/** تبدیل mm→SVG (محور h به بالا) */
function toSvg(p: ProfilePoint, W: number, H: number, height: number, maxR: number): [number, number] {
  const pad = 18;
  const sx = (W - 2 * pad) / (2 * maxR);
  const sy = (H - 2 * pad) / height;
  return [W / 2 + p.r * sx, H - pad - p.h * sy];
}

/** تبدیل SVG→mm */
function fromSvg(x: number, y: number, W: number, H: number, height: number, maxR: number): ProfilePoint {
  const pad = 18;
  const sx = (2 * maxR) / (W - 2 * pad);
  const sy = height / (H - 2 * pad);
  return {
    r: Math.max(0, (x - W / 2) * sx),
    h: Math.max(0, Math.min(height, (H - pad - y) * sy)),
  };
}

export function ProfileEditor(props: ProfileEditorProps) {
  const { points, height, maxR, onChange } = props;
  const W = 340, H = 300;
  const svgRef = useRef<SVGSVGElement>(null);
  const draggingIdx = useRef<number | null>(null);

  // مسیر نرم از نقاط (polyline — smoothing در هسته Rust اعمال می‌شود)
  const path = points
    .map((p, i) => {
      const [x, y] = toSvg(p, W, H, height, maxR);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  // انعکاس آینه‌ای برای نمایش حجمی‌مانند (فقط بصری)
  const mirrored = points
    .map((p) => {
      const [x, y] = toSvg(p, W, H, height, maxR);
      return `L${(W - x).toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  /** تبدیل رویداد ماوس/پوینتر به مختصات mm — با هر دو نوع رویداد کار می‌کند */
  const pt = (e: { clientX: number; clientY: number }) => {
    const svg = svgRef.current!;
    const rect = svg.getBoundingClientRect();
    return fromSvg(
      ((e.clientX - rect.left) / rect.width) * W,
      ((e.clientY - rect.top) / rect.height) * H,
      W, H, height, maxR
    );
  };

  return (
    <div>
      <svg
        ref={svgRef}
        className="profile-svg"
        viewBox={`0 0 ${W} ${H}`}
        onDoubleClick={(e) => {
          // افزودن نقطه با دابل‌کلیک
          const np = pt(e);
          const next = [...points, np].sort((a, b) => a.h - b.h);
          onChange(next);
        }}
        onPointerMove={(e) => {
          if (draggingIdx.current === null) return;
          const np = pt(e);
          const next = [...points];
          next[draggingIdx.current] = np;
          onChange(next);
        }}
        onPointerUp={() => { draggingIdx.current = null; }}
        onPointerLeave={() => { draggingIdx.current = null; }}
      >
        {/* خط محور */}
        <line x1={W / 2} y1={12} x2={W / 2} y2={H - 12} stroke="#343945" strokeDasharray="4 4" />
        {/* خط پایه */}
        <line x1={10} y1={H - 18} x2={W - 10} y2={H - 18} stroke="#343945" />

        {/* ناحیهٔ فرم (آینه‌ای) */}
        <path
          d={`M${W / 2},${H - 18} ${path.replace("M", "L")} ${mirrored} Z`}
          fill="#46c87822"
          stroke="none"
        />
        {/* خط پروفایل */}
        <path d={path} stroke="#ff7a28" strokeWidth={2} fill="none" />

        {/* نقاط قابل‌کشیدن */}
        {points.map((p, i) => {
          const [x, y] = toSvg(p, W, H, height, maxR);
          return (
            <g key={i}>
              <circle
                cx={x} cy={y} r={7}
                fill="#ff7a28" stroke="#14161c" strokeWidth={2}
                style={{ cursor: "grab" }}
                onPointerDown={(e) => {
                  e.stopPropagation();
                  draggingIdx.current = i;
                }}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  // حذف نقطه با دابل‌کلیک روی خودش (حداقل ۲ نقطه بماند)
                  if (points.length > 2) {
                    onChange(points.filter((_, j) => j !== i));
                  }
                }}
              />
              <text x={x + 10} y={y + 4} fontSize={9} fill="#7d8392" className="ltr">
                {p.r.toFixed(0)}×{p.h.toFixed(0)}
              </text>
            </g>
          );
        })}
        <text x={W / 2 + 6} y={22} fontSize={10} fill="#7d8392">r (mm)</text>
        <text x={W - 60} y={H - 24} fontSize={10} fill="#7d8392">h (mm)</text>
      </svg>
      <div style={{ fontSize: 11, color: "var(--fg-2)", marginTop: 4 }}>
        دابل‌کلیک روی فضای خالی: {fa.addPoint} — دابل‌کلیک روی نقطه: {fa.removePoint} — کشیدن: جابه‌جایی
      </div>
    </div>
  );
}
