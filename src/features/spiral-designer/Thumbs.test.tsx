// @vitest-environment jsdom
/**
 * ============================================================================
 * features/spiral-designer/Thumbs.test.tsx — آزمون کارت‌های تصویری ۱.۲
 * ============================================================================
 * آزمون‌های رندر بندانگشتی‌ها:
 *   - FloorThumb: هر ۵ الگوی کف باید SVG با عناصر هندسی واقعی بسازد؛
 *   - SurfaceThumb: بدون دادهٔ هسته → جایگزین صادقانه (نه بوم خالی)؛
 *   - SurfaceThumb: با دادهٔ شبکه → بوم با پیکسل‌های رنگی.
 * ============================================================================
 */

import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import {
  SurfaceThumb,
  FloorThumb,
  ProfileMiniThumb,
} from "./Thumbs";
import type { PreviewGrid } from "../../lib/tauri";

describe("FloorThumb — ۵ الگوی کف", () => {
  it("هر الگو SVG با عناصر هندسی می‌سازد", () => {
    const kinds = ["Rings", "Spiral", "Grid", "Radial", "ZigZag"] as const;
    for (const kind of kinds) {
      const { container } = render(<FloorThumb kind={kind} size={96} />);
      const svg = container.querySelector("svg");
      expect(svg, `${kind} باید SVG بسازد`).toBeTruthy();
      // عناصر الگو: circle برای حلقه‌ها یا path/line برای بقیه
      const elements = svg?.querySelectorAll("circle, path, line");
      expect(
        (elements?.length ?? 0) > 0,
        `${kind} باید عناصر هندسی داشته باشد`
      ).toBe(true);
    }
  });

  it("سوراخ مرکزی وقتی بزرگ است رسم می‌شود", () => {
    const { container } = render(<FloorThumb kind="Rings" hole={0.5} size={96} />);
    const circles = container.querySelectorAll("circle");
    // حداقل یک دایرهٔ سوراخ + چند حلقه
    expect(circles.length).toBeGreaterThanOrEqual(3);
  });
});

describe("SurfaceThumb — بندانگشتی سطح", () => {
  it("بدون دادهٔ هسته → جایگزین صادقانه با پیام", () => {
    const { container, getByText } = render(
      <SurfaceThumb grid={undefined} />
    );
    expect(container.querySelector("canvas")).toBeNull();
    expect(getByText("پیش‌نمایش زنده در دسترس نیست")).toBeTruthy();
  });

  it("با دادهٔ شبکه → بوم رندر می‌شود", () => {
    const grid: PreviewGrid = {
      rows: 4,
      cols: 8,
      values: new Array(32).fill(0).map((_, i) => Math.sin(i) * 0.5),
    };
    const { container } = render(<SurfaceThumb grid={grid} />);
    const canvas = container.querySelector("canvas");
    expect(canvas).toBeTruthy();
    expect(canvas?.getAttribute("role")).toBe("img");
  });
});

describe("ProfileMiniThumb — سیلوئت پروفایل", () => {
  it("مسیر آینه‌دار می‌سازد و با کمتر از ۲ نقطه خالی می‌ماند", () => {
    const points = [
      { r: 10, h: 0 },
      { r: 25, h: 50 },
      { r: 12, h: 100 },
    ];
    const { container } = render(<ProfileMiniThumb points={points} />);
    const path = container.querySelector("path");
    expect(path).toBeTruthy();
    // مسیر باید M و L داشته باشد (سمت راست + آینه)
    const d = path?.getAttribute("d") ?? "";
    expect(d.startsWith("M")).toBe(true);
    expect(d).toContain("L");

    const { container: empty } = render(
      <ProfileMiniThumb points={[{ r: 10, h: 0 }]} />
    );
    expect(empty.querySelector("svg")).toBeNull();
  });
});
