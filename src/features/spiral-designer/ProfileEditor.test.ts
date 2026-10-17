/**
 * ============================================================================
 * ProfileEditor.test.ts — آزمون ریاضی اسپلاین نسخهٔ ۱.۵
 * ============================================================================
 * آینهٔ TS از هستهٔ Rust (profile.rs) باید همان رفتار را بدهد:
 *   - monotoneTangents: Fritsch–Carlson بدون فراجهش
 *   - effectiveTangents: مماس صریح با سقف ۶δ
 *   - hermiteAt: عبور دقیق از گره‌ها
 * ============================================================================
 */
import { describe, expect, it } from "vitest";
import {
  monotoneTangents,
  effectiveTangents,
  hermiteAt,
  buildCurvePath,
} from "./ProfileEditor";

const K = (arr: [number, number][]) => arr.map(([h, r]) => ({ h, r }));

describe("monotoneTangents — Fritsch–Carlson", () => {
  it("دادهٔ یکنوا → شیب‌های مثبت و بدون فراجهش", () => {
    const pts = K([[0, 5], [20, 12], [45, 19], [80, 25], [100, 28]]);
    const m = monotoneTangents(pts);
    expect(m.length).toBe(5);
    // همه مثبت (یکنوایی صعودی)
    for (const v of m) expect(v).toBeGreaterThanOrEqual(0);
  });

  it("قلهٔ محلی → مماس صفر در قله", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    const m = monotoneTangents(pts);
    expect(m[1]).toBe(0); // قله → افقی
  });

  it("دو نقطه → شیب سکانت", () => {
    const m = monotoneTangents(K([[0, 10], [100, 30]]));
    expect(m[0]).toBeCloseTo(0.2, 9);
    expect(m[1]).toBeCloseTo(0.2, 9);
  });
});

describe("effectiveTangents — مماس صریح (فلش‌های ۱.۵)", () => {
  it("undefined → همان خودکار", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    expect(effectiveTangents(pts, [undefined, undefined, undefined]))
      .toEqual(monotoneTangents(pts));
  });

  it("مماس صریح صفر در قله → شکل صاف‌تر", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    const auto = monotoneTangents(pts);
    // قله خودکار = ۰؛ صریح منفی → سمت پایین باز می‌شود (S-curve)
    const withM = effectiveTangents(pts, [undefined, -0.3, undefined]);
    expect(withM[1]).toBeCloseTo(-0.3, 9);
    expect(auto[1]).toBe(0);
  });

  it("سقف دامنه ۶δ — مماس بی‌نهایت مهار می‌شود", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    // δ ≈ ۰٫۴ هر بازه؛ سقف ≈ ۲٫۴ → مماس ۹۹۹ کلمپ می‌شود
    const m = effectiveTangents(pts, [999, undefined, undefined]);
    expect(m[0]).toBeLessThanOrEqual(2.5);
    expect(m[0]).toBeGreaterThanOrEqual(-2.5);
  });

  it("NaN → نادیده (fallback خودکار)", () => {
    const pts = K([[0, 10], [100, 30]]);
    expect(effectiveTangents(pts, [NaN])).toEqual(monotoneTangents(pts));
  });
});

describe("hermiteAt — درون‌یابی مکعبی", () => {
  it("از هر دو گره دقیقاً می‌گذرد", () => {
    const pts = K([[0, 10], [100, 30]]);
    const m = monotoneTangents(pts);
    expect(hermiteAt(pts, m, 0, 0)).toBeCloseTo(10, 9);
    expect(hermiteAt(pts, m, 0, 1)).toBeCloseTo(30, 9);
  });

  it("خطی دادهٔ یکنوا → وسط = میانگین", () => {
    const pts = K([[0, 10], [100, 30]]);
    const m = [0.2, 0.2];
    expect(hermiteAt(pts, m, 0, 0.5)).toBeCloseTo(20, 9);
  });

  it("مماس صریح شکل بین گره‌ها را عوض می‌کند", () => {
    const pts = K([[0, 10], [100, 30]]);
    // مماس نامتقارن: گرهٔ اول افقی، دومی تند → S-curve واقعی
    const explicit = [0.0, 0.8];
    const auto = monotoneTangents(pts); // [0.2, 0.2] — خطی
    const midExp = hermiteAt(pts, explicit, 0, 0.5);
    const midAuto = hermiteAt(pts, auto, 0, 0.5);
    expect(Math.abs(midExp - midAuto)).toBeGreaterThan(0.5);
  });
});

describe("buildCurvePath — مسیر SVG", () => {
  const toXY = (p: { h: number; r: number }): [number, number] => [p.r, p.h];
  it("خطی: تعداد گره‌ها = تعداب L", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    const d = buildCurvePath(pts, [undefined, undefined, undefined], "Linear", toXY);
    expect(d.startsWith("M")).toBe(true);
    expect((d.match(/L/g) || []).length).toBe(2);
  });
  it("اسپلاین: مسیر نرم با نمونه‌های زیاد", () => {
    const pts = K([[0, 10], [50, 30], [100, 10]]);
    const d = buildCurvePath(pts, [undefined, undefined, undefined], "Spline", toXY, 10);
    expect((d.match(/L/g) || []).length).toBeGreaterThan(20); // ۲ بازه × ۱۱ نمونه
  });
});
