/**
 * ============================================================================
 * components/PhaseDial.test.ts — آزمون ریاضی دیال چرخش فاز θ (نسخهٔ ۱.۳)
 * ============================================================================
 * توابع خالص dialAngleDeg/snapDeg همان تبدیلی هستند که درگ اسپات θ در
 * Viewport استفاده می‌کند — بدون نیاز به WebGL قابل آزمون‌اند.
 *
 * رابطهٔ مختصات: G-code (x,y) → Three (x, z=−y)؛ پس θ_gc = atan2(−z, x).
 * این آزمون تضمین می‌کند نشانگر دیال دقیقاً همان زاویه‌ای را نشان دهد
 * که هستهٔ Rust در مارپیچ می‌نویسد (فاز درجه = زاویهٔ atan2 استاندارد).
 * ============================================================================
 */

import { describe, it, expect } from "vitest";
import { dialAngleDeg, snapDeg } from "./Viewport";
import { fa } from "../i18n/fa";

describe("دیال چرخش فاز θ — تبدیل مختصات Three→G-code", () => {
  it("سمت +X (راست) → ۰ درجه", () => {
    expect(dialAngleDeg(1, 0)).toBeCloseTo(0, 6);
  });

  it("سمت +Y گcode (z منفی در Three) → ۹۰ درجه", () => {
    // θ=90 در G-code: (x,y)=(0,1) → Three: (x,z)=(0,−1)
    expect(dialAngleDeg(0, -1)).toBeCloseTo(90, 6);
  });

  it("سمت −X → ۱۸۰ درجه", () => {
    expect(dialAngleDeg(-1, 0)).toBeCloseTo(180, 6);
  });

  it("سمت −Y گcode (z مثبت) → ۲۷۰ درجه", () => {
    expect(dialAngleDeg(0, 1)).toBeCloseTo(270, 6);
  });

  it("زاویهٔ مثبت گانی را نرمال می‌کند به [0,360)", () => {
    // ۴۵ درجه در ربع اول
    expect(dialAngleDeg(1, -1)).toBeCloseTo(45, 6);
    // −۴۵ درجه (z مثبت) → ۳۱۵
    expect(dialAngleDeg(1, 1)).toBeCloseTo(315, 6);
  });

  it("شعاع روی زاویه بی‌اثر است (درگ مطلق)", () => {
    expect(dialAngleDeg(10, -10)).toBeCloseTo(dialAngleDeg(3, -3), 6);
  });
});

describe("چفت ۱۵ درجه‌ای (Shift هنگام درگ)", () => {
  it("۷ → ۰ و ۸ → ۱۵", () => {
    expect(snapDeg(7, 15)).toBe(0);
    expect(snapDeg(8, 15)).toBe(15);
  });

  it("۳۵۸ → ۰ (چرخش دور کامل)", () => {
    expect(snapDeg(358, 15)).toBe(0);
  });

  it("گام صفر یا منفی → بدون تغییر", () => {
    expect(snapDeg(97, 0)).toBe(97);
    expect(snapDeg(97, -5)).toBe(97);
  });
});

describe("رشته‌های UI نسخهٔ ۱.۳ (فاز + دما)", () => {
  it("همهٔ کلیدهای فاز/دما غیرخالی هستند", () => {
    const keys = [
      "phaseRotate", "phaseRotateHint", "phaseRotateSeam", "spotPhaseRotate",
      "tempsTitle", "materialPresets", "materialHint",
      "nozzleTemp", "bedTemp", "fanSpeed", "waitTemps", "waitTempsHint",
      "matPla", "matPetg", "matAbs", "matTpu", "matSilk", "tempsZeroNote",
    ] as const;
    for (const k of keys) {
      expect(fa[k].length).toBeGreaterThan(3);
    }
  });

  it("پریست‌های ماده الگوی «نام — نازل/بستر» دارند", () => {
    expect(fa.matPla).toMatch(/200\/60/);
    expect(fa.matPetg).toMatch(/240\/80/);
    expect(fa.matAbs).toMatch(/250\/100/);
    expect(fa.matTpu).toMatch(/225\/45/);
  });
});
