/**
 * ============================================================================
 * features/spiral-designer/svgImport.ts — ایمپورت پروفایل از فایل SVG
 * ============================================================================
 * نسخهٔ ۱.۵ (درخواست کاربر: «برای پروفایل دکمهٔ ایمپورت از فایل SVG»).
 *
 * کاری که انجام می‌دهد — خالص و قابل‌آزمون (بدون DOM):
 *   ۱) استخراج مسیرهای `<path d>` و `<polyline>/<polygon points>`؛
 *   ۲) نمونه‌برداری منحنی‌ها (M/L/H/V/C/S/Q/T/A/Z) در گام‌های ریز؛
 *   ۳) تشخیص نیم‌رخِ راست: نقاطِ سمتِ راستِ محور قائم کانتور (برای فرم
 *      دورانی، x≥axis یعنی شعاع)؛
 *   ۴) نگاشت به (r, h): h = (پایین جعبه − y)، r = x − محور؛
 *   ۵) حذف تکراری‌ها + مرتب‌سازی + ساده‌سازی Douglas–Peucker.
 *
 * صداقت: SVG می‌تواند هر شکلی باشد؛ اگر نیم‌رخِ معنادار (≥۲ نقطه با
 * ارتفاع مفید) به‌دست نیاید، خطای فارسی صریح برمی‌گردد — پروفایل جعلی
 * ساخته نمی‌شود.
 * ============================================================================
 */

import type { ProfilePoint } from "../../lib/tauri";

/** یک نقطهٔ ۲بعدی SVG (واحد کاربر) */
interface Pt {
  x: number;
  y: number;
}

/** نتیجهٔ ایمپورت */
export interface SvgImportResult {
  points: ProfilePoint[];
  /** تعداد مسیرهای پارس‌شده (برای پیام وضعیت) */
  path_count: number;
  /** ارتفاع خام کانتور در واحد SVG (پیش از مقیاس) */
  raw_height: number;
}

// ============================================================================
// ۱) توکنایز دستورهای مسیر SVG
// ============================================================================

const NUM = /[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?/g;

/** تفسیر `d` یک path به دنبالهٔ نقاط نمونه‌برداری‌شده */
function samplePathD(d: string, step = 6): Pt[] {
  const out: Pt[] = [];
  // تجزیهٔ فرمان‌ها: حرف + اعدادش
  const re = /([MLHVCSQTAZmlhvcsqtaz])((?:[^MLHVCSQTAZmlhvcsqtaz]*)?)/g;
  let m: RegExpExecArray | null;
  // وضعیت جاری (آبشاری مطلق)
  let cx = 0, cy = 0;
  let sx = 0, sy = 0; // نقطهٔ شروع subpath
  let prevCmd = "";
  let prevCx2 = 0, prevCy2 = 0; // کنترل قبلی برای S/T
  const nums = (s: string): number[] => {
    const a: number[] = [];
    NUM.lastIndex = 0;
    let n: RegExpExecArray | null;
    while ((n = NUM.exec(s))) a.push(parseFloat(n[1]));
    return a;
  };
  const push = (x: number, y: number) => {
    const last = out[out.length - 1];
    if (!last || Math.hypot(last.x - x, last.y - y) > 1e-9) out.push({ x, y });
  };
  /** نمونه‌برداری بزیه مکعبی با step گام */
  const cubic = (p0: Pt, p1: Pt, p2: Pt, p3: Pt) => {
    for (let i = 1; i <= step; i++) {
      const t = i / step, u = 1 - t;
      push(
        u * u * u * p0.x + 3 * u * u * t * p1.x + 3 * u * t * t * p2.x + t * t * t * p3.x,
        u * u * u * p0.y + 3 * u * u * t * p1.y + 3 * u * t * t * p2.y + t * t * t * p3.y
      );
    }
  };
  /** نمونه‌برداری بزیه درجه ۲ */
  const quad = (p0: Pt, p1: Pt, p2: Pt) => {
    for (let i = 1; i <= step; i++) {
      const t = i / step, u = 1 - t;
      push(
        u * u * p0.x + 2 * u * t * p1.x + t * t * p2.x,
        u * u * p0.y + 2 * u * t * p1.y + t * t * p2.y
      );
    }
  };
  /** نمونه‌برداری قوس بیضوی A (تقریب با چندگانه بزیه یا گام زاویه‌ای مستقیم) */
  const arc = (x0: number, y0: number, a: number[]) => {
    const [rx, ry, rotDeg, largeArc, sweep, x1, y1] = a;
    if (rx <= 0 || ry <= 0) {
      push(x1, y1);
      return;
    }
    const phi = (rotDeg * Math.PI) / 180;
    const cosPhi = Math.cos(phi), sinPhi = Math.sin(phi);
    // تبدیل به پارامتری (W3C F.6)
    const dx = (x0 - x1) / 2, dy = (y0 - y1) / 2;
    const x1p = cosPhi * dx + sinPhi * dy;
    const y1p = -sinPhi * dx + cosPhi * dy;
    const lam = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    const rx2 = lam > 1 ? rx * Math.sqrt(lam) : rx;
    const ry2 = lam > 1 ? ry * Math.sqrt(lam) : ry;
    const sign = largeArc !== sweep ? 1 : -1;
    const num = rx2 * rx2 * ry2 * ry2 - rx2 * rx2 * y1p * y1p - ry2 * ry2 * x1p * x1p;
    const den = rx2 * rx2 * y1p * y1p + ry2 * ry2 * x1p * x1p;
    const co = sign * Math.sqrt(Math.max(0, num / (den || 1e-12)));
    const cxp = (co * rx2 * y1p) / ry2;
    const cyp = (-co * ry2 * x1p) / rx2;
    const cx0 = cosPhi * cxp - sinPhi * cyp + (x0 + x1) / 2;
    const cy0 = sinPhi * cxp + cosPhi * cyp + (y0 + y1) / 2;
    // زوایای شروع/پایان
    const ang = (ux: number, uy: number, vx: number, vy: number) => {
      const dot = ux * vx + uy * vy;
      const len = Math.hypot(ux, uy) * Math.hypot(vx, vy) || 1e-12;
      let a = Math.acos(Math.max(-1, Math.min(1, dot / len)));
      if (ux * vy - uy * vx < 0) a = -a;
      return a;
    };
    const th1 = ang(1, 0, (x1p - cxp) / rx2, (y1p - cyp) / ry2);
    let dth = ang((x1p - cxp) / rx2, (y1p - cyp) / ry2, (-x1p - cxp) / rx2, (-y1p - cyp) / ry2);
    if (!sweep && dth > 0) dth -= 2 * Math.PI;
    if (sweep && dth < 0) dth += 2 * Math.PI;
    const n = Math.max(2, Math.ceil((Math.abs(dth) / (Math.PI / 2)) * step));
    for (let i = 1; i <= n; i++) {
      const t = th1 + (dth * i) / n;
      push(
        cx0 + rx2 * Math.cos(t) * cosPhi - ry2 * Math.sin(t) * sinPhi,
        cy0 + rx2 * Math.cos(t) * sinPhi + ry2 * Math.sin(t) * cosPhi
      );
    }
  };

  while ((m = re.exec(d))) {
    const cmd = m[1];
    const args = nums(m[2] || "");
    const rel = cmd === cmd.toLowerCase();
    const C = cmd.toUpperCase();
    const dxr = rel ? cx : 0;
    const dyr = rel ? cy : 0;
    let i = 0;
    switch (C) {
      case "M":
        for (; i + 1 < args.length; i += 2) {
          cx = args[i] + dxr; cy = args[i + 1] + dyr;
          if (i === 0) { push(cx, cy); sx = cx; sy = cy; }
          else push(cx, cy); // M های بعدی = LineTo
        }
        break;
      case "L":
        for (; i + 1 < args.length; i += 2) {
          cx = args[i] + dxr; cy = args[i + 1] + dyr;
          push(cx, cy);
        }
        break;
      case "H":
        for (; i < args.length; i++) {
          cx = args[i] + dxr;
          push(cx, cy);
        }
        break;
      case "V":
        for (; i < args.length; i++) {
          cy = args[i] + dyr;
          push(cx, cy);
        }
        break;
      case "C":
        for (; i + 5 < args.length; i += 6) {
          const x1 = args[i] + dxr, y1 = args[i + 1] + dyr;
          const x2 = args[i + 2] + dxr, y2 = args[i + 3] + dyr;
          const x3 = args[i + 4] + dxr, y3 = args[i + 5] + dyr;
          cubic({ x: cx, y: cy }, { x: x1, y: y1 }, { x: x2, y: y2 }, { x: x3, y: y3 });
          prevCx2 = x2; prevCy2 = y2;
          cx = x3; cy = y3;
        }
        break;
      case "S":
        for (; i + 3 < args.length; i += 4) {
          const x2 = args[i] + dxr, y2 = args[i + 1] + dyr;
          const x3 = args[i + 2] + dxr, y3 = args[i + 3] + dyr;
          // بازتاب کنترل قبلی
          const x1 = prevCmd === "C" || prevCmd === "S" ? 2 * cx - prevCx2 : cx;
          const y1 = prevCmd === "C" || prevCmd === "S" ? 2 * cy - prevCy2 : cy;
          cubic({ x: cx, y: cy }, { x: x1, y: y1 }, { x: x2, y: y2 }, { x: x3, y: y3 });
          prevCx2 = x2; prevCy2 = y2;
          cx = x3; cy = y3;
        }
        break;
      case "Q":
        for (; i + 3 < args.length; i += 4) {
          const x1 = args[i] + dxr, y1 = args[i + 1] + dyr;
          const x2 = args[i + 2] + dxr, y2 = args[i + 3] + dyr;
          quad({ x: cx, y: cy }, { x: x1, y: y1 }, { x: x2, y: y2 });
          prevCx2 = x1; prevCy2 = y1;
          cx = x2; cy = y2;
        }
        break;
      case "T":
        for (; i + 1 < args.length; i += 2) {
          const x2 = args[i] + dxr, y2 = args[i + 1] + dyr;
          const x1 = prevCmd === "Q" || prevCmd === "T" ? 2 * cx - prevCx2 : cx;
          const y1 = prevCmd === "Q" || prevCmd === "T" ? 2 * cy - prevCy2 : cy;
          quad({ x: cx, y: cy }, { x: x1, y: y1 }, { x: x2, y: y2 });
          prevCx2 = x1; prevCy2 = y1;
          cx = x2; cy = y2;
        }
        break;
      case "A":
        for (; i + 6 < args.length; i += 7) {
          const abs = args.slice(i + 5, i + 7).map((v, j) => (rel && j === 0 ? v + cx : j === 1 ? v + cy : v));
          arc(cx, cy, [args[i], args[i + 1], args[i + 2], args[i + 3], args[i + 4], abs[0], abs[1]]);
          cx = abs[0]; cy = abs[1];
        }
        break;
      case "Z":
        push(sx, sy);
        cx = sx; cy = sy;
        break;
    }
    if (cmd) prevCmd = C;
  }
  return out;
}

// ============================================================================
// ۲) استخراج همهٔ نقاط از متن SVG
// ============================================================================

/** نقاط polyline/polygon: «x1,y1 x2,y2 ...» */
function parsePointsAttr(s: string): Pt[] {
  const nums = (s.match(NUM) || []).map(Number);
  const out: Pt[] = [];
  for (let i = 0; i + 1 < nums.length; i += 2) out.push({ x: nums[i], y: nums[i + 1] });
  return out;
}

/** حذف transformهای سادهٔ translate/scale از عناصر — با اعمال روی نقاط.
 *  فقط translate و scale پشتیبانی می‌شود (رایج‌ترین‌ها در SVG پروفایل). */
function applyTransform(pts: Pt[], transform: string | null): Pt[] {
  if (!transform) return pts;
  const ops: { kind: "t" | "s"; args: number[] }[] = [];
  const re = /(translate|scale|matrix)\s*\(([^)]*)\)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(transform))) {
    const args = (m[2].match(NUM) || []).map(Number);
    if (m[1] === "translate") ops.push({ kind: "t", args });
    else if (m[1] === "scale") ops.push({ kind: "s", args });
    else if (m[1] === "matrix" && args.length >= 4) {
      // matrix(a,b,c,d,e,f) ≈ scale + translate ساده اگر b=c=0
      if (Math.abs(args[1]) < 1e-9 && Math.abs(args[2]) < 1e-9) {
        ops.push({ kind: "s", args: [args[0], args[3]] });
        ops.push({ kind: "t", args: [args[4], args[5]] });
      }
    }
  }
  // اعمال از آخر به اول (ترتیب DOM معکوس)
  for (let k = ops.length - 1; k >= 0; k--) {
    const o = ops[k];
    if (o.kind === "t") {
      const tx = o.args[0] ?? 0, ty = o.args[1] ?? 0;
      pts = pts.map((p) => ({ x: p.x + tx, y: p.y + ty }));
    } else {
      const sx = o.args[0] ?? 1, sy = o.args.length > 1 ? o.args[1] : sx;
      pts = pts.map((p) => ({ x: p.x * sx, y: p.y * sy }));
    }
  }
  return pts;
}

/** استخراج همهٔ کانتورهای SVG (path + polyline + polygon + rect + circle/ellipse) */
export function extractSvgPoints(svgText: string): { pts: Pt[]; path_count: number } {
  const all: Pt[] = [];
  let pathCount = 0;
  // --- path ---
  const pathRe = /<path\b([^>]*)\/?>/g;
  let m: RegExpExecArray | null;
  while ((m = pathRe.exec(svgText))) {
    const attrs = m[1];
    const dMatch = attrs.match(/\bd\s*=\s*"([^"]*)"|\bd\s*=\s*'([^']*)'/);
    if (!dMatch) continue;
    const d = dMatch[1] ?? dMatch[2] ?? "";
    if (!d.trim()) continue;
    const trMatch = attrs.match(/\btransform\s*=\s*"([^"]*)"|\btransform\s*=\s*'([^']*)'/);
    let pts = samplePathD(d);
    pts = applyTransform(pts, trMatch ? (trMatch[1] ?? trMatch[2] ?? null) : null);
    if (pts.length > 1) {
      all.push(...pts);
      pathCount++;
    }
  }
  // --- polyline/polygon ---
  const polyRe = /<(polyline|polygon)\b([^>]*)\/?>/g;
  while ((m = polyRe.exec(svgText))) {
    const attrs = m[2];
    const pMatch = attrs.match(/\bpoints\s*=\s*"([^"]*)"|\bpoints\s*=\s*'([^']*)'/);
    if (!pMatch) continue;
    let pts = parsePointsAttr(pMatch[1] ?? pMatch[2] ?? "");
    const trMatch = attrs.match(/\btransform\s*=\s*"([^"]*)"|\btransform\s*=\s*'([^']*)'/);
    pts = applyTransform(pts, trMatch ? (trMatch[1] ?? trMatch[2] ?? null) : null);
    if (pts.length > 1) {
      all.push(...pts);
      pathCount++;
    }
  }
  // --- rect (کانتور چهارگوش — نیم‌رخش مستطیل می‌شود) ---
  const rectRe = /<rect\b([^>]*)\/?>/g;
  while ((m = rectRe.exec(svgText))) {
    const attrs = m[1];
    const num = (name: string, def = 0) => {
      const r = attrs.match(new RegExp(`\\b${name}\\s*=\\s*"?([\\d.\\-e]+)"?`));
      return r ? parseFloat(r[1]) : def;
    };
    const x = num("x"), y = num("y"), w = num("width"), h = num("height");
    if (w > 0 && h > 0) {
      all.push({ x, y }, { x: x + w, y }, { x: x + w, y: y + h }, { x, y: y + h });
      pathCount++;
    }
  }
  // --- circle/ellipse (نیم‌رخ = نیم‌بیضی) ---
  const ellRe = /<(circle|ellipse)\b([^>]*)\/?>/g;
  while ((m = ellRe.exec(svgText))) {
    const attrs = m[2];
    const num = (name: string, def = 0) => {
      const r = attrs.match(new RegExp(`\\b${name}\\s*=\\s*"?([\\d.\\-e]+)"?`));
      return r ? parseFloat(r[1]) : def;
    };
    const cx = num("cx"), cy = num("cy");
    const rx = attrs.includes("r=") ? num("r") : num("rx");
    const ry = attrs.includes("r=") ? num("r") : num("ry");
    if (rx > 0 && ry > 0) {
      // نیم‌رخ راست بیضی: از پایین تا بالا
      for (let i = 0; i <= 24; i++) {
        const t = Math.PI / 2 - (Math.PI * i) / 24; // از θ=90° تا −90°
        all.push({ x: cx + rx * Math.cos(t), y: cy + ry * Math.sin(t) });
      }
      pathCount++;
    }
  }
  return { pts: all, path_count: pathCount };
}

// ============================================================================
// ۳) نیم‌رخ راست + نگاشت به (r, h) + ساده‌سازی
// ============================================================================

/** ساده‌سازی Douglas–Peucker روی منحنی (r,h) */
export function simplifyRdp(pts: ProfilePoint[], eps: number): ProfilePoint[] {
  if (pts.length <= 2) return pts;
  const keep = new Array<boolean>(pts.length).fill(false);
  keep[0] = keep[pts.length - 1] = true;
  const stack: [number, number][] = [[0, pts.length - 1]];
  while (stack.length) {
    const [a, b] = stack.pop()!;
    if (b - a < 2) continue;
    // بیشترین فاصله عمودی از پاره‌خط a..b (در فضای h↦r)
    const h0 = pts[a].h, h1 = pts[b].h;
    const r0 = pts[a].r, r1 = pts[b].r;
    let maxD = -1, maxI = -1;
    for (let i = a + 1; i < b; i++) {
      const t = h1 > h0 ? (pts[i].h - h0) / (h1 - h0) : 0;
      const lin = r0 + (r1 - r0) * t;
      const d = Math.abs(pts[i].r - lin);
      if (d > maxD) { maxD = d; maxI = i; }
    }
    if (maxD > eps && maxI > 0) {
      keep[maxI] = true;
      stack.push([a, maxI], [maxI, b]);
    }
  }
  return pts.filter((_, i) => keep[i]);
}

/**
 * ایمپورت کامل: متن SVG → نقاط پروفایل (r, h) چاپ‌پسند.
 *
 * @param svgText محتوای خام SVG
 * @param targetHeight ارتفاع مقصد فرم (mm) — پروفایل به این مقیاس می‌شود
 * @param maxRadius شعاع مجاز حداکثری (mm) — کلمپ ایمنی
 */
export function importProfileFromSvg(
  svgText: string,
  targetHeight: number,
  maxRadius: number
): SvgImportResult {
  const { pts, path_count } = extractSvgPoints(svgText);
  if (pts.length < 3) {
    throw new Error("در SVG مسیر قابل‌استفاده‌ای پیدا نشد (حداقل ۳ نقطه لازم است).");
  }
  // جعبهٔ محیطی
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of pts) {
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.y > maxY) maxY = p.y;
  }
  const rawH = maxY - minY;
  if (rawH < 1e-6) {
    throw new Error("کانتور SVG ارتفاع صفر دارد — پروفایل ساخته نمی‌شود.");
  }
  // محور قائم = مرکز x جعبه (فرم دورانی متقارن فرض می‌شود)
  const axis = (minX + maxX) / 2;
  // نیم‌رخ راست: نقاط با x ≥ محور (با تلورانس ۲٪ عرض)
  const tol = (maxX - minX) * 0.02;
  const right = pts.filter((p) => p.x >= axis - tol);
  if (right.length < 2) {
    throw new Error("نیم‌رخ راست کانتور خالی است — شکل متقارن دورانی نیست.");
  }
  // نگاشت: h = (maxY − y) × scale؛ r = x − axis (≥ ۰)
  const scale = targetHeight / rawH;
  const mapped: ProfilePoint[] = right.map((p) => ({
    r: Math.max(0, (p.x - axis) * scale),
    h: (maxY - p.y) * scale,
  }));
  // حذف تکراری h: برای هر h یکتا، بیشینهٔ r (بیرونی‌ترین سطح)
  const byH = new Map<number, number>();
  for (const p of mapped) {
    const key = Math.round(p.h * 100) / 100;
    const cur = byH.get(key);
    if (cur === undefined || p.r > cur) byH.set(key, p.r);
  }
  const sorted: ProfilePoint[] = [...byH.entries()]
    .map(([h, r]) => ({ r: Math.min(r, maxRadius), h, m: undefined }))
    .sort((a, b) => a.h - b.h);
  if (sorted.length < 2) {
    throw new Error("پس از پاک‌سازی، نقاط کافی برای پروفایل باقی نماند.");
  }
  // ساده‌سازی — eps نسبی ۱٫۵٪ ارتفاع (نقاط بیهوده حذف؛ خم هندسه می‌ماند)
  const simplified = simplifyRdp(sorted, targetHeight * 0.015);
  // گارد: ۳۰ نقطهٔ حداکثر (ادیتور شلوغ نشود) — اگر بیشتر شد eps بالا
  let result = simplified;
  let eps = targetHeight * 0.015;
  while (result.length > 30) {
    eps *= 1.6;
    result = simplifyRdp(sorted, eps);
  }
  // حذف مماس‌های صریح (m) — ایمپورت فقط هندسه می‌آورد؛ اسپلاین خودکار نرم می‌کند
  result = result.map((p) => ({ r: Math.round(p.r * 10) / 10, h: Math.round(p.h * 10) / 10 }));
  return { points: result, path_count, raw_height: rawH };
}
