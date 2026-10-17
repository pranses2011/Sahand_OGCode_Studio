/**
 * ============================================================================
 * svgImport.test.ts — آزمون پارسر SVG → پروفایل (نسخهٔ ۱.۵)
 * ============================================================================
 * سناریوها: path ساده / منحنی بزیه / قوس / polyline / rect / circle /
 * نیم‌رخ راست از کانتور کامل / خطاهای صریح.
 * ============================================================================
 */
import { describe, expect, it } from "vitest";
import {
  extractSvgPoints,
  importProfileFromSvg,
  simplifyRdp,
} from "./svgImport";

describe("extractSvgPoints", () => {
  it("path خطی ساده پارس می‌شود", () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0 100 L 20 100 L 20 0 L 0 0 Z"/></svg>`;
    const { pts, path_count } = extractSvgPoints(svg);
    expect(path_count).toBe(1);
    expect(pts.length).toBeGreaterThanOrEqual(4);
  });

  it("منحنی بزیه مکعبی نمونه‌برداری می‌شود", () => {
    const svg = `<svg><path d="M 0 100 C 10 80, 30 20, 40 0"/></svg>`;
    const { pts } = extractSvgPoints(svg);
    // ۶ گام × هر گام یک نقطه + شروع → نقاط میانی واقعی
    expect(pts.length).toBeGreaterThan(5);
    // yها بین ۰ و ۱۰۰ نزولی‌اند
    expect(pts[0].y).toBeCloseTo(100, 0);
    expect(pts[pts.length - 1].y).toBeCloseTo(0, 0);
  });

  it("قوس بیضوی (A) نقطه‌های میانی می‌دهد", () => {
    const svg = `<svg><path d="M 0 50 A 50 50 0 0 1 100 50"/></svg>`;
    const { pts } = extractSvgPoints(svg);
    expect(pts.length).toBeGreaterThan(3);
    // قوس نیم‌دایره: بیشینهٔ y وسط باید بالاتر از سرها باشد (SVG y به پایین)
    const ys = pts.map((p) => p.y);
    expect(Math.max(...ys)).toBeGreaterThan(Math.min(...ys) + 5);
  });

  it("polyline پارس می‌شود", () => {
    const svg = `<svg><polyline points="0,100 20,100 20,0"/></svg>`;
    const { pts, path_count } = extractSvgPoints(svg);
    expect(path_count).toBe(1);
    expect(pts.length).toBe(3);
  });

  it("rect و circle پشتیبانی می‌شوند", () => {
    const svg = `<svg><rect x="0" y="0" width="10" height="50"/><circle cx="0" cy="25" r="20"/></svg>`;
    const { path_count } = extractSvgPoints(svg);
    expect(path_count).toBe(2);
  });

  it("بدون مسیر → صفر", () => {
    const svg = `<svg><text>hello</text></svg>`;
    const { pts, path_count } = extractSvgPoints(svg);
    expect(path_count).toBe(0);
    expect(pts.length).toBe(0);
  });
});

describe("importProfileFromSvg", () => {
  /** SVG گلدان متقارن: نیم‌رخ کامل دو طرف محور x=25 */
  const VASE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 100">
    <path d="M 25 100 L 18 100 C 12 90, 12 60, 18 45 C 22 36, 20 20, 25 0
             L 25 0 C 30 20, 28 36, 32 45 C 38 60, 38 90, 32 100 L 25 100 Z"/>
  </svg>`;

  it("نیم‌رخ راست کانتور متقارن → پروفایل معتبر", () => {
    const res = importProfileFromSvg(VASE_SVG, 100, 45);
    expect(res.points.length).toBeGreaterThanOrEqual(3);
    expect(res.points.length).toBeLessThanOrEqual(30);
    // مرتب‌سازی صعودی h
    for (let i = 1; i < res.points.length; i++) {
      expect(res.points[i].h).toBeGreaterThan(res.points[i - 1].h - 1e-9);
    }
    // همهٔ شعاع‌ها ≥ ۰ و ≤ maxRadius
    for (const p of res.points) {
      expect(p.r).toBeGreaterThanOrEqual(0);
      expect(p.r).toBeLessThanOrEqual(45);
    }
    // h در بازهٔ هدف
    const hMax = res.points[res.points.length - 1].h;
    expect(hMax).toBeGreaterThan(90);
    expect(hMax).toBeLessThanOrEqual(100.01);
    // نقطهٔ آخر کف: r مثبت (پایهٔ چسبندگی)
    expect(res.points[0].r).toBeGreaterThan(0);
  });

  it("مقیاس به ارتفاع مقصد اعمال می‌شود", () => {
    const a = importProfileFromSvg(VASE_SVG, 100, 45);
    const b = importProfileFromSvg(VASE_SVG, 200, 45);
    expect(b.points[b.points.length - 1].h).toBeCloseTo(
      a.points[a.points.length - 1].h * 2, 0
    );
  });

  it("SVG بدون مسیر → خطای فارسی صریح", () => {
    expect(() => importProfileFromSvg("<svg></svg>", 100, 45)).toThrow();
    expect(() => importProfileFromSvg("not svg at all", 100, 45)).toThrow();
  });

  it("path تنها نیم‌رخ راست (بدون چپ) هم کار می‌کند", () => {
    // فقط سمت راست محور — ابزارهای وکتور معمولاً نیم‌رخ می‌کشند
    const half = `<svg><path d="M 0 100 L 8 100 C 14 90, 14 60, 8 45 C 4 36, 6 20, 0 0"/></svg>`;
    const res = importProfileFromSvg(half, 100, 45);
    expect(res.points.length).toBeGreaterThanOrEqual(3);
    // محور = مرکز جعبهٔ خودش؛ همهٔ r ≥ ۰
    for (const p of res.points) expect(p.r).toBeGreaterThanOrEqual(0);
  });
});

describe("simplifyRdp", () => {
  it("نقاط روی خط مستقیم → فقط دو سر", () => {
    const pts = Array.from({ length: 21 }, (_, i) => ({ r: 10 + i, h: i * 5 }));
    const out = simplifyRdp(pts, 0.5);
    expect(out.length).toBe(2);
  });
  it("قلهٔ واقعی حفظ می‌شود", () => {
    const pts = Array.from({ length: 41 }, (_, i) => {
      const h = i * 2.5;
      const r = 20 - Math.abs(i - 20) * 0.9; // قله در i=20
      return { r, h };
    });
    const out = simplifyRdp(pts, 0.3);
    // قله (h=50) باید بماند
    expect(out.some((p) => Math.abs(p.h - 50) < 1e-9)).toBe(true);
  });
});
