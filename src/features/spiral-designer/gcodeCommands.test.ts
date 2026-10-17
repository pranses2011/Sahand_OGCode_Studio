/**
 * ============================================================================
 * gcodeCommands.test.ts — آزمون کتابخانهٔ دستورات شروع/پایان (نسخهٔ ۱٫۶)
 * ============================================================================
 */

import { describe, expect, it } from "vitest";
import {
  COMMAND_CATEGORIES,
  GCODE_COMMANDS,
  filterCommands,
  type CommandSlot,
} from "./gcodeCommands";

describe("کتابخانهٔ دستورات G-code (درخواست کاربر ۳)", () => {
  it("حداقل ۳۰ دستور با توضیح فارسی دارد", () => {
    expect(GCODE_COMMANDS.length).toBeGreaterThanOrEqual(30);
    // همه: کد + نام + توضیح فارسی + قطعه + دستهٔ معتبر
    for (const c of GCODE_COMMANDS) {
      expect(c.code.length).toBeGreaterThan(0);
      expect(c.name.length).toBeGreaterThan(0);
      expect(c.desc.length).toBeGreaterThan(10);
      expect(c.snippet.length).toBeGreaterThan(0);
      expect(COMMAND_CATEGORIES).toContain(c.category as never);
      expect(["start", "end", "both"]).toContain(c.slot);
    }
  });

  it("دستورات کلیدی شروع و پایان موجودند", () => {
    const codes = GCODE_COMMANDS.map((c) => c.code);
    expect(codes).toContain("G28");
    expect(codes).toContain("M104 S{t}");
    expect(codes).toContain("M109 S{t}");
    expect(codes).toContain("M140 S{t}");
    expect(codes).toContain("M190 S{t}");
    expect(codes).toContain("M106 S{0..255}");
    expect(codes).toContain("M107");
    expect(codes).toContain("G1 E{e} F{f}");
    expect(codes).toContain("M84");
    expect(codes).toContain("G28 X");
  });

  it("فیلتر اسلات: شروع فقط شروع‌پسند؛ پایان فقط پایان‌پسند", () => {
    const starts = filterCommands("", "start");
    const ends = filterCommands("", "end");
    // شروع: M190 (انتظار میز) هست؛ M84 نیست
    expect(starts.some((c) => c.code === "M190 S{t}")).toBe(true);
    expect(starts.some((c) => c.code === "M84")).toBe(false);
    // پایان: M84 هست؛ G29 (کالیبراسیون کند) نیست
    expect(ends.some((c) => c.code === "M84")).toBe(true);
    expect(ends.some((c) => c.code === "G29")).toBe(false);
    // «هر دو» در هر دو فهرست
    expect(starts.some((c) => c.slot === "both")).toBe(true);
    expect(ends.some((c) => c.slot === "both")).toBe(true);
  });

  it("جست‌وجو با کد و متن فارسی کار می‌کند", () => {
    const byCode = filterCommands("M104", "start");
    // در اسلات شروع فقط نسخهٔ «تنظیم» می‌آید (خاموشی S0 پایان است)
    expect(byCode).toHaveLength(1);
    expect(byCode[0].code).toBe("M104 S{t}");
    // در اسلات پایان هر دو حضور دارند (خاموشی + بستهٔ کامل)
    const byCodeEnd = filterCommands("M104", "end");
    expect(byCodeEnd.length).toBeGreaterThanOrEqual(2);
    const byFa = filterCommands("هوم", "start");
    expect(byFa.some((c) => c.code === "G28")).toBe(true);
    // بدون نتیجه
    expect(filterCommands("xyz123", "start")).toHaveLength(0);
  });

  it("بستهٔ پایان استاندارد ترتیب درخواست کاربر ۲ را دارد", () => {
    const pack = GCODE_COMMANDS.find((c) => c.name === "پایان کامل استاندارد");
    expect(pack).toBeDefined();
    const s = pack!.snippet;
    const iRetract = s.indexOf("; retract");
    const iBed = s.indexOf("M140 S0");
    const iNozzle = s.indexOf("M104 S0");
    const iFan = s.indexOf("M107");
    const iLift = s.indexOf("G1 Z1 F600");
    const iHomeX = s.indexOf("G28 X");
    const iM84 = s.indexOf("M84");
    expect(iRetract).toBeGreaterThanOrEqual(0);
    expect(iRetract).toBeLessThan(iBed);
    expect(iBed).toBeLessThan(iNozzle);
    expect(iNozzle).toBeLessThan(iFan);
    expect(iFan).toBeLessThan(iLift);
    expect(iLift).toBeLessThan(iHomeX);
    expect(iHomeX).toBeLessThan(iM84);
  });
});
