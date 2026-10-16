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
  kind: "Rings" | "Spiral" | "Grid" | "Radial" | "ZigZag";
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
