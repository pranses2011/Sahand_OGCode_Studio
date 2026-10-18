/**
 * ============================================================================
 * features/spiral-designer/Thumbs.tsx — کارت‌های تصویری الگو/بافت/کف
 * ============================================================================
 * درخواست کاربر (نسخهٔ ۱.۲): «دکمه‌ها بزرگ‌تر و دارای تصویری باشند که
 * بیانگر نوعشون باشه و با دیدن اون بفهمیم که تقریبا چطور هست».
 *
 * قاعدهٔ صداقت: تصویر هر کارت از «همان دادهٔ ارزیابی‌شدهٔ هستهٔ Rust»
 * (designer_preview_data) کشیده می‌شود — نه آیکون تزئینی. اگر داده در
 * دسترس نباشد (مرورگر بدون هسته) جای کارت یک نما جایگزین ساده می‌نشیند
 * و برچسب «بدون پیش‌نمایش زنده» می‌گیرد.
 *
 * سه نوع بندانگشتی:
 *   ۱) SurfaceThumb — استوانهٔ شبه‌سه‌بعدی سایه‌دار از شبکهٔ الگو/بافت؛
 *   ۲) FloorThumb — نمای بالا الگوهای کف (SVG تحلیلی)؛
 *   ۳) ProfileMiniThumb — سیلوئت کوچک پروفایل‌های آماده.
 * ============================================================================
 */

import { useEffect, useRef } from "react";
import type { PreviewGrid } from "../../lib/tauri";

/** رنگ پایهٔ سطح (RGB) — تم صنعتی برنامه */
const BASE = { r: 84, g: 196, b: 140 }; // سبز رشتهٔ چاپ
const BASE_DIM = { r: 108, g: 140, b: 190 }; // آبی کمرنگ برای بافت‌ها

// ============================================================================
// ۱) بندانگشتی سطح (الگو/بافت) — canvas شبه‌سه‌بعدی
// ============================================================================

export interface SurfaceThumbProps {
  /** شبکهٔ داده از هسته — undefined = بدون پیش‌نمایش زنده */
  grid: PreviewGrid | undefined;
  /** عرض/ارتفاع CSS کارت (px) */
  width?: number;
  height?: number;
  /** تم رنگی: patterns سبز، textures آبی */
  variant?: "pattern" | "texture";
}

/** رندر شبکهٔ هسته به‌صورت استوانهٔ سایه‌دار — یک‌بار per data */
function paintSurface(
  canvas: HTMLCanvasElement,
  grid: PreviewGrid,
  variant: "pattern" | "texture"
) {
  const W = 132; // عرض داخلی
  const H = 96; // ارتفاع داخلی
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const img = ctx.createImageData(W, H);
  const base = variant === "pattern" ? BASE : BASE_DIM;
  const { rows, cols, values } = grid;

  for (let py = 0; py < H; py++) {
    // z نرمال: بالا = ۱، پایین = ۰ (ردیف ۰ پایین است در دادهٔ هسته)
    const zNorm = 1 - py / (H - 1);
    const row = Math.min(rows - 1, Math.max(0, Math.round(zNorm * (rows - 1))));
    for (let px = 0; px < W; px++) {
      // نقشهٔ قوس جلویی استوانه: px → θ ∈ [−0.46π, +0.46π]
      const t = px / (W - 1); // 0..1 چپ→راست
      const theta = (t - 0.5) * Math.PI * 0.92;
      // θ داده در [0, 2π) — قرینهٔ سمت چپ را از آینهٔ زاویه می‌گیریم
      const thetaData = theta < 0 ? theta + 2 * Math.PI : theta;
      const col = Math.min(
        cols - 1,
        Math.max(0, Math.round((thetaData / (2 * Math.PI)) * (cols - 1)))
      );
      const d = values[row * cols + col] ?? 0; // ∈ [−1, 1]

      // سایهٔ لامبرت روی انحنای استوانه (مرکز روشن، لبه‌ها تیره)
      const lambert = Math.cos(theta / 0.5) * 0.5 + 0.5; // 0..1
      // برجستگی روشن‌تر، گودی تیره‌تر
      const bump = (d + 1) / 2; // 0..1
      // ترکیب: ۶۰٪ الگو + ۴۰٪ انحنا + ته‌رنگ ارتفاعی ملایم
      const v = bump * 0.62 + lambert * 0.38;
      const i = (py * W + px) * 4;
      // کمی ته‌رنگ گرم بالا و سرد پایین برای عمق
      img.data[i] = Math.min(
        255,
        Math.round(base.r * v + (py < H * 0.25 ? 14 * v : 0))
      );
      img.data[i + 1] = Math.min(255, Math.round(base.g * v));
      img.data[i + 2] = Math.min(
        255,
        Math.round(base.b * v + (py > H * 0.7 ? 18 * (1 - v) : 0))
      );
      img.data[i + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
}

/** کارت تصویری الگو/بافت — استوانهٔ شبه‌3D از دادهٔ واقعی هسته */
export function SurfaceThumb(props: SurfaceThumbProps) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  const { grid, width = 132, height = 96, variant = "pattern" } = props;

  useEffect(() => {
    if (ref.current && grid) {
      paintSurface(ref.current, grid, variant);
    }
  }, [grid, variant]);

  if (!grid) {
    // بدون هسته → نما جایگزین صادقانه (نه آیکون جعلی)
    return (
      <div
        className="thumb-fallback"
        style={{ width, height }}
        aria-label="پیش‌نمایش زنده در دسترس نیست"
      >
        پیش‌نمایش زنده در دسترس نیست
      </div>
    );
  }
  return (
    <canvas
      ref={ref}
      width={width}
      height={height}
      style={{ width, height, display: "block" }}
      role="img"
    />
  );
}

// ============================================================================
// ۲) بندانگشتی الگوهای کف — SVG نمای بالا (تحلیلی)
// ============================================================================

export interface FloorThumbProps {
  kind:
    | "Rings" | "Spiral" | "Grid" | "Radial" | "ZigZag"
    // ---- نسخهٔ ۱٫۶: ۵ الگوی جدید ----
    | "Honeycomb" | "Wave" | "Star" | "Snowflake" | "Octagram"
    // ---- نسخهٔ ۱٫۷: الگوهای کف لامپ ----
    | "LampWeb" | "Sunburst";
  /** قطر سوراخ مرکزی نسبی (0..1 از شعاع) */
  hole?: number;
  size?: number;
}

/** نمای بالا الگوهای کف — همان هندسهٔ build_floor در هسته */
export function FloorThumb(props: FloorThumbProps) {
  const { kind, hole = 0.18, size = 96 } = props;
  const R = size / 2 - 3;
  const cx = size / 2;
  const cy = size / 2;
  const rh = R * hole;
  const stroke = "var(--extrusion, #46c878)";
  const thin = Math.max(1, R * 0.045);

  let body: React.ReactNode = null;
  if (kind === "Rings") {
    const circles: JSX.Element[] = [];
    for (let r = R; r > rh; r -= R / 6) {
      circles.push(
        <circle
          key={r}
          cx={cx}
          cy={cy}
          r={r}
          fill="none"
          stroke={stroke}
          strokeWidth={thin}
          opacity={0.9}
        />
      );
    }
    body = circles;
  } else if (kind === "Spiral") {
    // مارپیچ آرشیمدسی: r = a + b·θ
    const b = (R - rh) / (6 * 2 * Math.PI);
    const a = rh;
    let d = "";
    for (let th = 0; th <= 6 * 2 * Math.PI; th += 0.12) {
      const r = a + b * th;
      const x = cx + r * Math.cos(th - Math.PI / 2);
      const y = cy + r * Math.sin(th - Math.PI / 2);
      d += (d === "" ? "M" : "L") + x.toFixed(1) + " " + y.toFixed(1);
    }
    body = <path d={d} fill="none" stroke={stroke} strokeWidth={thin} />;
  } else if (kind === "Grid") {
    const lines: JSX.Element[] = [];
    const step = R / 3.2;
    for (let off = -R; off <= R + 0.01; off += step) {
      const half = Math.sqrt(Math.max(0, R * R - off * off));
      if (half < R * 0.12) continue;
      // افقی
      lines.push(
        <line
          key={"h" + off.toFixed(1)}
          x1={cx - half}
          y1={cy + off}
          x2={cx + half}
          y2={cy + off}
          stroke={stroke}
          strokeWidth={thin}
          opacity={0.85}
        />
      );
      // عمودی
      lines.push(
        <line
          key={"v" + off.toFixed(1)}
          x1={cx + off}
          y1={cy - half}
          x2={cx + off}
          y2={cy + half}
          stroke={stroke}
          strokeWidth={thin}
          opacity={0.85}
        />
      );
    }
    body = lines;
  } else if (kind === "Radial") {
    const spokes: JSX.Element[] = [];
    const n = 12;
    for (let i = 0; i < n; i++) {
      const a = (i / n) * 2 * Math.PI;
      spokes.push(
        <line
          key={i}
          x1={cx + rh * Math.cos(a)}
          y1={cy + rh * Math.sin(a)}
          x2={cx + R * Math.cos(a)}
          y2={cy + R * Math.sin(a)}
          stroke={stroke}
          strokeWidth={thin}
          opacity={0.9}
        />
      );
    }
    body = spokes;
  } else if (kind === "Honeycomb") {
    // شانه‌زنبوری — شش‌ضلعی‌های داخل دایره
    const hexes: JSX.Element[] = [];
    const s = R / 3.4;
    const dy = s * 0.866;
    const nRows = Math.ceil(R / dy);
    for (let row = -nRows; row <= nRows; row++) {
      const yv = row * dy;
      if (Math.abs(yv) > R) continue;
      const xoff = Math.abs(row % 2) === 1 ? s * 0.5 : 0;
      const nCols = Math.ceil((R + s) / s);
      for (let col = -nCols; col <= nCols; col++) {
        const hx = col * s + xoff;
        if (Math.hypot(hx, yv) + s > R + 1) continue;
        const pts: string[] = [];
        for (let vi = 0; vi < 6; vi++) {
          const va = ((30 + 60 * vi) * Math.PI) / 180;
          pts.push(
            `${(cx + hx + s * Math.cos(va)).toFixed(1)},${(cy + yv + s * Math.sin(va)).toFixed(1)}`
          );
        }
        hexes.push(
          <polygon
            key={`${row}:${col}`}
            points={pts.join(" ")}
            fill="none"
            stroke={stroke}
            strokeWidth={thin}
            opacity={0.9}
          />
        );
      }
    }
    body = hexes;
  } else if (kind === "Wave") {
    // موج دایره‌ای — مارپیچ با موج شعاعی
    let d = "";
    const b = (R - rh) / (5 * 2 * Math.PI);
    const amp = Math.min(R * 0.12, 7);
    for (let th = 0; th <= 5 * 2 * Math.PI; th += 0.1) {
      const r = rh + b * th + amp * Math.sin(6 * th);
      const x = cx + r * Math.cos(th - Math.PI / 2);
      const y = cy + r * Math.sin(th - Math.PI / 2);
      d += (d === "" ? "M" : "L") + x.toFixed(1) + " " + y.toFixed(1);
    }
    body = <path d={d} fill="none" stroke={stroke} strokeWidth={thin} />;
  } else if (kind === "Star") {
    // ستاره ۶پر + حلقهٔ مرکزی
    const n = 12;
    const rIn = Math.max(rh, R * 0.45);
    const pts: string[] = [];
    for (let i = 0; i < n; i++) {
      const th = (i / n) * 2 * Math.PI - Math.PI / 2;
      const r = i % 2 === 0 ? R : rIn;
      pts.push(`${(cx + r * Math.cos(th)).toFixed(1)},${(cy + r * Math.sin(th)).toFixed(1)}`);
    }
    body = (
      <>
        <polygon points={pts.join(" ")} fill="none" stroke={stroke} strokeWidth={thin} />
        <circle cx={cx} cy={cy} r={rIn * 0.55} fill="none" stroke={stroke} strokeWidth={thin} opacity={0.7} />
      </>
    );
  } else if (kind === "Snowflake") {
    // دانه‌برفی — ۶ شاخه + شاخه‌های ۴۵°
    const els: JSX.Element[] = [];
    for (let bIdx = 0; bIdx < 6; bIdx++) {
      const a = (bIdx / 6) * 2 * Math.PI;
      els.push(
        <line
          key={"m" + bIdx}
          x1={cx + rh * Math.cos(a)}
          y1={cy + rh * Math.sin(a)}
          x2={cx + R * Math.cos(a)}
          y2={cy + R * Math.sin(a)}
          stroke={stroke}
          strokeWidth={thin}
        />
      );
      for (const fr of [0.45, 0.7]) {
        const rAt = rh + (R - rh) * fr;
        const px = cx + rAt * Math.cos(a);
        const py = cy + rAt * Math.sin(a);
        const side = (R - rh) * 0.22;
        for (const sgn of [-1, 1]) {
          const sa = a + sgn * Math.PI / 4;
          els.push(
            <line
              key={`${bIdx}:${fr}:${sgn}`}
              x1={px}
              y1={py}
              x2={px + side * Math.cos(sa)}
              y2={py + side * Math.sin(sa)}
              stroke={stroke}
              strokeWidth={thin}
              opacity={0.9}
            />
          );
        }
      }
    }
    els.push(
      <circle key="c" cx={cx} cy={cy} r={rh} fill="none" stroke={stroke} strokeWidth={thin} opacity={0.8} />
    );
    body = els;
  } else if (kind === "Octagram") {
    // گره هشت‌پر — دو مربع چرخیده + هشت‌ضلعی داخلی
    const els: JSX.Element[] = [];
    for (let sq = 0; sq < 2; sq++) {
      const a0 = (sq * Math.PI) / 4;
      const pts: string[] = [];
      for (let vi = 0; vi < 4; vi++) {
        const va = a0 + (vi * Math.PI) / 2;
        pts.push(`${(cx + R * Math.cos(va)).toFixed(1)},${(cy + R * Math.sin(va)).toFixed(1)}`);
      }
      els.push(
        <polygon key={"sq" + sq} points={pts.join(" ")} fill="none" stroke={stroke} strokeWidth={thin} />
      );
    }
    const rMid = R * 0.414;
    if (rMid > rh + 2) {
      const pts: string[] = [];
      for (let i = 0; i < 8; i++) {
        const th = (i / 8) * 2 * Math.PI + Math.PI / 8;
        pts.push(`${(cx + rMid * Math.cos(th)).toFixed(1)},${(cy + rMid * Math.sin(th)).toFixed(1)}`);
      }
      els.push(
        <polygon key="oct" points={pts.join(" ")} fill="none" stroke={stroke} strokeWidth={thin} opacity={0.9} />
      );
    }
    body = els;
  } else if (kind === "LampWeb") {
    // وب لامپ — حلقه‌های درشت + پره‌های شعاعی (شبکهٔ نور)
    const els: JSX.Element[] = [];
    for (let r = R; r > rh; r -= R / 3.5) {
      els.push(
        <circle key={"lw" + r} cx={cx} cy={cy} r={r} fill="none" stroke={stroke} strokeWidth={thin} opacity={0.85} />
      );
    }
    for (let k = 0; k < 6; k++) {
      const a = (k * Math.PI) / 3;
      els.push(
        <line
          key={"lws" + k}
          x1={cx + rh * Math.cos(a)}
          y1={cy + rh * Math.sin(a)}
          x2={cx + R * Math.cos(a)}
          y2={cy + R * Math.sin(a)}
          stroke={stroke}
          strokeWidth={thin * 1.4}
          opacity={0.9}
        />
      );
    }
    body = <>{els}</>;
  } else if (kind === "Sunburst") {
    // خورشیدشکاف — پرتوهای پهن واگرا از سوکت تا لبه
    const els: JSX.Element[] = [];
    for (let k = 0; k < 8; k++) {
      const a = (k * Math.PI) / 4;
      const h0 = thin * 1.2;
      const h1 = (Math.PI * R) / 8 * 0.42;
      const ux = Math.cos(a);
      const uy = Math.sin(a);
      const px = -uy;
      const py = ux;
      els.push(
        <g key={"sb" + k}>
          <line
            x1={cx + rh * ux - h0 * px} y1={cy + rh * uy - h0 * py}
            x2={cx + R * ux - h1 * px} y2={cy + R * uy - h1 * py}
            stroke={stroke} strokeWidth={thin} opacity={0.9}
          />
          <line
            x1={cx + rh * ux + h0 * px} y1={cy + rh * uy + h0 * py}
            x2={cx + R * ux + h1 * px} y2={cy + R * uy + h1 * py}
            stroke={stroke} strokeWidth={thin} opacity={0.9}
          />
          <line
            x1={cx + rh * ux} y1={cy + rh * uy}
            x2={cx + R * ux} y2={cy + R * uy}
            stroke={stroke} strokeWidth={thin} opacity={0.6}
          />
        </g>
      );
    }
    body = <>{els}</>;
  } else {
    // ZigZag — رفت‌وبرگشتی
    let d = "";
    let dir = 1;
    const step = R / 4.2;
    let rowIdx = 0;
    for (let y = -R + step * 0.5; y <= R; y += step) {
      const half = Math.sqrt(Math.max(0, R * R - y * y));
      if (half < R * 0.1) {
        rowIdx++;
        continue;
      }
      const x1 = dir > 0 ? -half : half;
      const x2 = dir > 0 ? half : -half;
      d +=
        (d === "" ? "M" : "L") +
        (cx + x1).toFixed(1) +
        " " +
        (cy + y).toFixed(1);
      d += "L" + (cx + x2).toFixed(1) + " " + (cy + y).toFixed(1);
      dir *= -1;
      rowIdx++;
      if (rowIdx > 24) break;
    }
    body = <path d={d} fill="none" stroke={stroke} strokeWidth={thin} />;
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role="img"
      style={{ display: "block" }}
    >
      <circle
        cx={cx}
        cy={cy}
        r={R + 1.5}
        fill="none"
        stroke="var(--line, #3a3f4a)"
        strokeWidth={1.4}
      />
      {hole > 0.02 && (
        <circle
          cx={cx}
          cy={cy}
          r={rh}
          fill="var(--bg-1, #14161c)"
          stroke="var(--line, #3a3f4a)"
          strokeWidth={1}
        />
      )}
      {body}
    </svg>
  );
}

// ============================================================================
// ۳) بندانگشتی پروفایل‌های آماده — سیلوئت کوچک
// ============================================================================

export interface ProfileMiniThumbProps {
  points: { r: number; h: number }[];
  width?: number;
  height?: number;
}

/** سیلوئت پروفایل — منحنی r(h) آینه‌دار (هر دو طرف محور) */
export function ProfileMiniThumb(props: ProfileMiniThumbProps) {
  const { points, width = 72, height = 88 } = props;
  if (points.length < 2) return null;
  const maxR = Math.max(...points.map((p) => p.r), 1);
  const maxH = Math.max(...points.map((p) => p.h), 1);
  const pad = 4;
  const usableW = width / 2 - pad;
  const usableH = height - pad * 2;
  const x = (r: number) => width / 2 + (r / maxR) * usableW;
  const y = (h: number) => height - pad - (h / maxH) * usableH;
  const right = points.map((p, i) => `${i === 0 ? "M" : "L"}${x(p.r).toFixed(1)} ${y(p.h).toFixed(1)}`).join(" ");
  const left = [...points]
    .reverse()
    .map((p) => `L${(width - x(p.r)).toFixed(1)} ${y(p.h).toFixed(1)}`)
    .join(" ");
  return (
    <svg width={width} height={height} role="img" style={{ display: "block" }}>
      <path
        d={`${right} ${left} Z`}
        fill="var(--accent, #f0883e)"
        opacity={0.25}
        stroke="var(--accent, #f0883e)"
        strokeWidth={1.6}
      />
    </svg>
  );
}
