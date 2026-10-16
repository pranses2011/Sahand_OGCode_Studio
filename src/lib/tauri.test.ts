/**
 * ============================================================================
 * lib/tauri.test.ts — آزمون پل IPC در محیط غیر-Tauri
 * ============================================================================
 * در محیط آزمون (node، بدون __TAURI_INTERNALS__) همهٔ invokeها باید
 * «خطای صادقانه» بدهند نه دادهٔ جعلی (الزام: هیچ قابلیت نمایشی).
 * تست mapping helperها و رشته‌های i18n هم اینجاست.
 * ============================================================================
 */

import { describe, it, expect } from "vitest";
import type { DesignerProject } from "./tauri";
import {
  parseGcode, explainLine, saveAs, validateGcode,
  designerCompute, designerExportGcode, designerExportStl,
  saveProject, loadProject, readTextFile,
  openFileDialog, saveFileDialog, runningInDesktop,
} from "./tauri";
import { fa } from "../i18n/fa";
import { ALL_FIXTURES } from "../test-fixtures";

describe("پل IPC در محیط غیر-دسکتاپ (صداقت بدون دادهٔ جعلی)", () => {
  it("runningInDesktop باید false باشد در node", () => {
    expect(runningInDesktop()).toBe(false);
  });

  it("parseGcode باید خطای صریح بدهد نه دادهٔ ساختگی", async () => {
    await expect(parseGcode("G21\n", null)).rejects.toThrow(
      "دسکتاپ"
    );
  });

  it("explainLine باید خطا بدهد", async () => {
    await expect(explainLine("G21\n", 0)).rejects.toThrow();
  });

  it("saveAs باید خطا بدهد", async () => {
    await expect(
      saveAs({ content: "G21\n", original_content: "G21\n", path: "x.gcode", checksum_policy: "keep" })
    ).rejects.toThrow();
  });

  it("validateGcode باید خطا بدهد", async () => {
    await expect(validateGcode("G21\n", null)).rejects.toThrow();
  });

  it("دستورهای طراح همه خطای صریح می‌دهند", async () => {
    const project = {
      schema_version: 1, profile: [{ r: 10, h: 0 }, { r: 10, h: 50 }],
      form_height_mm: 50, form_bottom_diameter_mm: 20, form_top_diameter_mm: 20,
      taper_percent: 0, patterns: [], textures: [],
      floor: { enabled: false, center_hole_diameter_mm: 0, z_start_mm: 0, ring_gap_mm: 0.45 },
      printer: {
        name: "x", status: "unverified", build_volume_mm: [220, 220, 250] as [number, number, number],
        nozzle_diameter_mm: 0.4, filament_diameter_mm: 1.75, line_width_mm: 0.45,
        layer_height_mm: 0.25, print_speed_mm_min: 2400, travel_speed_mm_min: 6000,
        flow_multiplier: 1, absolute_e: true, start_gcode: "", end_gcode: "",
      },
      created_at_unix: 0, note: "",
    } satisfies DesignerProject;
    await expect(designerCompute(project)).rejects.toThrow();
    await expect(designerExportGcode(project)).rejects.toThrow();
    await expect(designerExportStl(project, "x.stl")).rejects.toThrow();
    await expect(saveProject(project, "x.json")).rejects.toThrow();
    await expect(loadProject("x.json")).rejects.toThrow();
  });

  it("readTextFile و دیالوگ‌ها خطای صریح می‌دهند", async () => {
    await expect(readTextFile("nope.gcode")).rejects.toThrow();
    await expect(openFileDialog()).rejects.toThrow();
    await expect(saveFileDialog("x.gcode", "gcode")).rejects.toThrow();
  });
});

describe("رشته‌های فارسی UI", () => {
  it("رشته‌های حیاتی غیرخالی‌اند", () => {
    for (const key of [
      "appTitle", "navEditor", "navDesigner", "openFile", "saveAs",
      "safetyNotice", "lineInspector", "designerTitle",
    ] as const) {
      expect(fa[key].length).toBeGreaterThan(3);
    }
  });

  it("پیام ایمنی متن الزام سند را دارد", () => {
    // متن پیشنهادی سند: تضمین نیست + بازبینی + اولین چاپ زیر نظر
    expect(fa.safetyNotice).toContain("تضمین");
    expect(fa.safetyNotice).toContain("بررسی");
    expect(fa.safetyNotice).toContain("زیر نظر");
  });

  it("هشدار placeholder/TODO در متن‌ها نیست", () => {
    // الزام: هیچ متن placeholder به‌عنوان قابلیت نهایی
    for (const [k, v] of Object.entries(fa)) {
      expect(String(v)).not.toContain("TODO");
      expect(String(v)).not.toContain("placeholder");
      expect(String(v)).not.toMatch(/لورم/);
      void k;
    }
  });
});

describe("fixtureهای سند TEST_GCODE_FIXTURES", () => {
  it("هر پنج fixture موجود و غیرخالی‌اند", () => {
    // fixture ۴ طبق سند TEST_GCODE_FIXTURES.md کامنت «only a comment» دارد
    // و برچسب TEST FIXTURE ONLY فقط در fixtureهای ۱، ۲، ۳، ۵ است
    for (const [name, text] of Object.entries(ALL_FIXTURES)) {
      expect(text.length > 20, `${name} خیلی کوتاه است`).toBe(true);
      const hasMarker =
        text.includes("TEST FIXTURE") ||
        text.includes("this is only a comment");
      expect(hasMarker, `${name} نشانگر آزمون‌بودن ندارد`).toBe(true);
    }
  });

  it("fixture ۴ دارای checksumهای معتبر سند است", () => {
    // چهار خط N...*cs از سند
    expect(ALL_FIXTURES.fixture4).toContain("N1 G21*27");
    expect(ALL_FIXTURES.fixture4).toContain("N4 G1 X10 Y10 E0.5 F1200*6");
  });

  it("fixture ۵ دارای XNaN و M900 است", () => {
    expect(ALL_FIXTURES.fixture5).toContain("XNaN");
    expect(ALL_FIXTURES.fixture5).toContain("M900");
  });

  it("fixture ۱ تمام فرمان‌های کلیدی حالت را دارد", () => {
    const f1 = ALL_FIXTURES.fixture1;
    for (const cmd of ["G21", "G90", "M82", "G28", "G92 E0", "M140", "M104",
      "M190", "M109", "G2", "G3", "M106", "M117", "M107", "M84"]) {
      expect(f1).toContain(cmd);
    }
  });
});
