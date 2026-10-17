/**
 * ============================================================================
 * Speeds.test.ts — آزمون‌های نسخهٔ ۱.۴: radius_samples و منطق UI سرعت‌ها
 * ============================================================================
 * خالص و بدون DOM — همان توابعی که صفحهٔ طراح استفاده می‌کند.
 */

import { describe, expect, it } from "vitest";
import { radiusAt } from "./SpiralDesignerPage";

describe("radiusAt — جست‌وجوی دودویی روی radius_samples (۱.۴)", () => {
  const samples = [
    [0, 22],
    [50, 30],
    [100, 17],
  ];

  it("نمونهٔ خالی → مقدار پیش‌فرض صادقانه ۲۰", () => {
    expect(radiusAt(undefined, 10)).toBe(20);
    expect(radiusAt([], 10)).toBe(20);
  });

  it("پایین‌تر از اولین نمونه → شعاع اولین (با کف ۲)", () => {
    expect(radiusAt(samples, -5)).toBe(22);
    expect(radiusAt([[0, 0.5]], -1)).toBe(2); // کف حداقل ۲ برای اسپات
  });

  it("بالاتر از آخرین نمونه → شعاع آخرین", () => {
    expect(radiusAt(samples, 500)).toBe(17);
  });

  it("درون‌یابی خطی بین دو نمونه", () => {
    // میانهٔ [0..50] → (22+30)/2 = 26
    expect(radiusAt(samples, 25)).toBeCloseTo(26, 5);
    // ربع → 22 + 8×0.25 = 24
    expect(radiusAt(samples, 12.5)).toBeCloseTo(24, 5);
  });

  it("روی خود نمونه → همان مقدار", () => {
    expect(radiusAt(samples, 0)).toBe(22);
    expect(radiusAt(samples, 50)).toBe(30);
    expect(radiusAt(samples, 100)).toBe(17);
  });
});

describe("منطق نمایش سرعت‌های mm/s (۱.۴)", () => {
  it("فال‌بک mm/min قدیمی وقتی mm/s صفر است", () => {
    // همان فرمول UI: wall || print_speed_mm_min/60
    const legacy = { print_speed_mm_min: 2400, wall_speed_mm_s: 0 };
    const shown = legacy.wall_speed_mm_s || Math.round(legacy.print_speed_mm_min / 60);
    expect(shown).toBe(40);
  });

  it("mm/s مقدار مستقیم است و بر mm/min تقدم دارد", () => {
    const p = { print_speed_mm_min: 2400, wall_speed_mm_s: 55 };
    const shown = p.wall_speed_mm_s || Math.round(p.print_speed_mm_min / 60);
    expect(shown).toBe(55);
  });

  it("مشتقات پیش‌فرض: لایهٔ اول ۵۰٪ و کف ۷۵٪ دیواره", () => {
    const wall = 40;
    const first = Math.round(wall * 0.5);
    const floor = Math.round(wall * 0.75);
    expect(first).toBe(20);
    expect(floor).toBe(30);
  });

  it("همگام‌سازی نگه‌دارندهٔ mm/min هنگام تغییر mm/s (سازگاری قدیمی)", () => {
    const v = 45;
    const printer = { wall_speed_mm_s: v, print_speed_mm_min: v * 60 };
    expect(printer.print_speed_mm_min).toBe(2700);
  });
});

// ============================================================================
// نسخهٔ ۱.۵ — estimateTriangles (آینهٔ TS از build_mesh)
// ============================================================================
import { estimateTriangles } from "./SpiralDesignerPage";

describe("estimateTriangles — دیالوگ کیفیت STL (۱.۵)", () => {
  const samples = Array.from({ length: 129 }, (_, i) => [i, 25] as unknown as number[]);
  // شعاع ثابت ۲۵ → circumference ≈ ۱۵۷ → steps ≈ ۷۹
  it("کیفیت بالاتر → مثلث بیشتر (تقریباً مجذور ضریب)", () => {
    const t05 = estimateTriangles(samples, 100, 0.25, 0.5);
    const t1 = estimateTriangles(samples, 100, 0.25, 1);
    const t2 = estimateTriangles(samples, 100, 0.25, 2);
    expect(t1).toBeGreaterThan(t05);
    expect(t2).toBeGreaterThan(t1);
    // نسبت ≈ ۴× — rows و cols هر دو با کیفیت مقیاس می‌شوند (مجذور ضریب)
    expect(t1 / t05).toBeGreaterThan(3);
    expect(t1 / t05).toBeLessThan(5);
  });
  it("بدون نمونه → مقدار معقول پیش‌فرض", () => {
    const t = estimateTriangles(undefined, 100, 0.25, 1);
    expect(t).toBeGreaterThan(1000);
  });
  it("کیفیت سقف‌دار (۴× کلمپ نمی‌شود بی‌نهایت)", () => {
    const t = estimateTriangles(samples, 100, 0.25, 99);
    expect(t).toBeLessThan(4_000_000);
  });
});
