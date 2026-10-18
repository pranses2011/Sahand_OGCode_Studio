/**
 * ============================================================================
 * features/text-vase/contours.ts — ردیاب کانتور متن (نسخهٔ ۱٫۷ — درخواست ۱۱)
 * ============================================================================
 * «میخوام متن بنویسم و اون رو تبدیل کنم به حالت vase mode و خودش بتونه به
 *  بهترین نحو حروف جدا از هم رو به هم متصل بکنه بطوری که محل وصل شدنشون
 *  تو چشم نباشه … به همراه پشتیبانی از فونت های ویندوز و زبان فارسی که
 *  حروف جدا از هم و برعکس نشن»
 *
 * راه‌حل «هر فونتی + فارسی درست»:
 *   متن با Canvas رندر می‌شود — موتور رندر مرورگر خودش RTL و اتصال حروف
 *   فارسی را استاندارد انجام می‌دهد (برعکس/جداشدن ناممکن است) و «هر
 *   فونت نصب‌شده در ویندوز» + وزیرمتنِ باندل‌شده قابل انتخاب است.
 *
 * «اتصال نامرئی حروف»:
 *   fillText + strokeText با ضخامت قابل‌تنظیم ⇒ شکل حروف مجاور که فاصلهٔ
 *   کمی دارند در یک شکل یکپارچه «به‌هم می‌جوشند»؛ سپس ردیاب کانتور
 *   (marching squares) خروجیِ «یک کانتور پیوسته» می‌دهد ⇒ دیوارهٔ مارپیچی
 *   روی شکل یکپارچه چاپ می‌شود — محل اتصال هیچ‌جا دیده نمی‌شود چون
 *   اصلاً «درز» وجود ندارد. برای فاصله‌های بزرگ‌تر، نوارهای اتصال
 *   افقی بین لبۀ حروف رسم می‌شوند (شکاف‌های ریز پر می‌شود).
 *
 * پایپ‌لاین:
 *   متن → رندر بوم (رزولوشن بالا) → آستانهٔ آلفا → ماسک بولی →
 *   ردیابی Moore-neighbor → کانتورهای خام → ساده‌سازی Douglas-Peucker →
 *   تشخیص حفره (سیل از مرز) → مقیاس px→mm
 * ============================================================================
 */

/** گزینه‌های استخراج کانتور */
export interface ContourOptions {
  /** متن ورودی (فارسی/لاتین/اعداد) */
  text: string;
  /** خانوادهٔ فونت CSS — هر فونت ویندوزی یا "Vazirmatn" باندل‌شده */
  fontFamily: string;
  /** وزن فونت (۳۰۰..۹۰۰) */
  fontWeight: number;
  /** ارتفاع هدف حروف (mm در خروجی) */
  targetHeightMm: number;
  /** ضخامت هم‌جوشی stroke (٪ ارتفاع حرف — ۰ = خاموش) */
  fusionStrokePercent: number;
  /** ارتفاع نوارهای اتصال بین حروف دور (٪ ارتفاع؛ ۰ = خاموش) */
  connectorBarPercent: number;
  /** تلورانس ساده‌سازی Douglas-Peucker (px) */
  simplifyTolerancePx: number;
  /** رزولوشن رندر (px بر mm — بالاتر = دقیق‌تر) */
  pxPerMm: number;
}

export const DEFAULT_CONTOUR_OPTIONS: ContourOptions = {
  text: "Sahand",
  fontFamily: "Vazirmatn",
  fontWeight: 700,
  targetHeightMm: 30,
  fusionStrokePercent: 2.4,
  connectorBarPercent: 7,
  simplifyTolerancePx: 1.2,
  pxPerMm: 8,
};

/** یک کانتور بستهٔ ۲بعدی (mm — فضای مدل) */
export interface Contour {
  points: [number, number][];
  is_hole: boolean;
}

/**
 * استخراج کانتورهای متن — خروجی در mm حول مرکز جعبه (۰,۰)
 * (فضای مدل؛ هستهٔ Rust مرکز‌گذاری صفحه را اضافه می‌کند)
 */
export function extractTextContours(opts: ContourOptions): {
  contours: Contour[];
  boundsMm: { w: number; h: number };
  rasterSize: { w: number; h: number };
} {
  const text = opts.text.trim();
  if (!text) {
    return { contours: [], boundsMm: { w: 0, h: 0 }, rasterSize: { w: 0, h: 0 } };
  }

  // ---------- ۱) رندر روی بوم با رزولوشن بالا ----------
  const targetPx = Math.max(64, Math.round(opts.targetHeightMm * opts.pxPerMm));
  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) {
    return { contours: [], boundsMm: { w: 0, h: 0 }, rasterSize: { w: 0, h: 0 } };
  }

  const fontSpec = `${opts.fontWeight} ${targetPx}px "${opts.fontFamily}", "Vazirmatn", sans-serif`;
  ctx.font = fontSpec;
  ctx.direction = "rtl"; // فارسی RTL؛ bidi مرورگر متن مخلوط را درست می‌چیند
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";

  const measure = ctx.measureText(text);
  const glyphAscent = Math.abs(measure.actualBoundingBoxAscent ?? targetPx * 0.8);
  const glyphDescent = Math.abs(measure.actualBoundingBoxDescent ?? targetPx * 0.2);
  const inkH = Math.max(glyphAscent + glyphDescent, targetPx * 0.4);
  const scale = targetPx / inkH;

  const pad = Math.ceil(targetPx * 0.35);
  const W = Math.ceil(measure.width * scale + pad * 2);
  const H = Math.ceil(targetPx + pad * 2);
  canvas.width = Math.min(Math.max(W, 32), 8192);
  canvas.height = Math.min(Math.max(H, 32), 8192);

  const cx = canvas.width / 2;
  const cy = canvas.height / 2;

  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.font = fontSpec;
  ctx.direction = "rtl";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scale, scale);

  // «هم‌جوشی نامرئی»: fill + stroke هم‌رنگ — حروف نزدیک به‌هم جوش می‌دهند
  const fusion = (opts.fusionStrokePercent / 100) * targetPx;
  if (fusion > 0.01) {
    ctx.lineWidth = fusion;
    ctx.lineJoin = "round";
    ctx.strokeStyle = "#fff";
    ctx.strokeText(text, 0, 0);
  }
  ctx.fillStyle = "#fff";
  ctx.fillText(text, 0, 0);
  ctx.restore();

  // ---------- ۲) نوارهای اتصال بین حروف دور ----------
  if (opts.connectorBarPercent > 0) {
    const barH = (opts.connectorBarPercent / 100) * targetPx;
    const img0 = ctx.getImageData(0, 0, canvas.width, canvas.height);
    let minX = Infinity;
    let maxX = -Infinity;
    for (let y = 0; y < img0.height; y++) {
      for (let x = 0; x < img0.width; x++) {
        if (img0.data[(y * img0.width + x) * 4 + 3] > 96) {
          if (x < minX) minX = x;
          if (x > maxX) maxX = x;
        }
      }
    }
    if (minX < maxX) {
      ctx.fillStyle = "#fff";
      const yBase = Math.round(cy + targetPx * 0.3);
      const yMid = Math.round(cy + targetPx * 0.02);
      for (const yb of [yBase, yMid]) {
        ctx.fillRect(minX, yb - barH / 2, maxX - minX, barH);
      }
    }
  }

  // ---------- ۳) ماسک بولی ----------
  const img = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const w = img.width;
  const h = img.height;
  const solid = new Uint8Array(w * h);
  for (let i = 0; i < w * h; i++) {
    solid[i] = img.data[i * 4 + 3] > 110 ? 1 : 0;
  }

  // ---------- ۴) ردیابی کانتورها (بیرونی + حفره‌ها) ----------
  const raw = traceAllContours(solid, w, h);

  // ---------- ۵) ساده‌سازی + مقیاس + جهت ----------
  const mmPerPx = 1 / (opts.pxPerMm * scale); // px رندر → mm
  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
  for (const c of raw) {
    for (const [x, y] of c) {
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      if (y < minY) minY = y;
      if (y > maxY) maxY = y;
    }
  }
  if (!raw.length || minX === Infinity) {
    return { contours: [], boundsMm: { w: 0, h: 0 }, rasterSize: { w, h } };
  }
  const cxM = (minX + maxX) / 2;
  const cyM = (minY + maxY) / 2;

  const contours: Contour[] = [];
  for (const c of raw) {
    if (c.length < 8) continue; // نویز ریز
    const simplified = douglasPeuckerClosed(c, opts.simplifyTolerancePx);
    if (simplified.length < 4) continue;
    const pts: [number, number][] = simplified.map(([x, y]) => [
      +(((x - cxM) * mmPerPx).toFixed(3)),
      +((-(y - cyM) * mmPerPx).toFixed(3)), // y بوم پایین‌رونده → مدل بالا‌رونده
    ]);
    const area = signedArea(pts);
    if (Math.abs(area) < 1.0) continue; // خیلی ریز
    // CCW (مساحت مثبت) = بیرونی؛ CW = حفره
    contours.push({ points: pts, is_hole: area < 0 });
  }

  const boundsMm = {
    w: +((maxX - minX) * mmPerPx).toFixed(2),
    h: +((maxY - minY) * mmPerPx).toFixed(2),
  };
  return { contours, boundsMm, rasterSize: { w, h } };
}

// ============================================================================
// ردیابی کانتور — Moore-neighbor tracing روی ماسک بولی
// ============================================================================

/** ردیابی همهٔ کانتورهای «پیکسل روشن» + حفره‌های داخل آن‌ها */
function traceAllContours(mask: Uint8Array, w: number, h: number): [number, number][][] {
  const at = (x: number, y: number): number => {
    if (x < 0 || y < 0 || x >= w || y >= h) return 0;
    return mask[y * w + x];
  };
  const DIRS: [number, number][] = [
    [1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1],
  ];
  const trace = (region: (x: number, y: number) => number): [number, number][][] => {
    const visited = new Uint8Array(w * h);
    const out: [number, number][][] = [];
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        if (!region(x, y) || visited[y * w + x]) continue;
        // باید «مرز» باشد (همسایهٔ غیر-عضو)
        let edge = false;
        for (const [dx, dy] of DIRS) {
          if (!region(x + dx, y + dy)) {
            edge = true;
            break;
          }
        }
        if (!edge) continue;
        const contour: [number, number][] = [];
        let cx = x;
        let cy = y;
        let dir = 7;
        let guard = 0;
        const maxGuard = 8 * (w + h) + 10000;
        do {
          contour.push([cx, cy]);
          visited[cy * w + cx] = 1;
          let found = false;
          for (let k = 0; k < 8; k++) {
            const d = (dir + 5 + k) % 8;
            const nx = cx + DIRS[d][0];
            const ny = cy + DIRS[d][1];
            if (region(nx, ny) === 1) {
              dir = d;
              cx = nx;
              cy = ny;
              found = true;
              break;
            }
          }
          if (!found) break;
          guard++;
        } while ((cx !== x || cy !== y) && guard < maxGuard);
        if (contour.length >= 8) out.push(contour);
      }
    }
    return out;
  };

  // ۱) کانتورهای بیرونی (پیکسل روشن)
  const outer = trace((x, y) => at(x, y));

  // ۲) حفره‌ها: پیکسل‌های «خاموش» محصور — سیل از مرز بوم
  const outside = new Uint8Array(w * h);
  const queue: number[] = [];
  for (let x = 0; x < w; x++) {
    for (const y of [0, h - 1]) {
      const i = y * w + x;
      if (!mask[i] && !outside[i]) {
        outside[i] = 1;
        queue.push(i);
      }
    }
  }
  for (let y = 0; y < h; y++) {
    for (const x of [0, w - 1]) {
      const i = y * w + x;
      if (!mask[i] && !outside[i]) {
        outside[i] = 1;
        queue.push(i);
      }
    }
  }
  let qi = 0;
  while (qi < queue.length) {
    const i = queue[qi++];
    const x = i % w;
    const y = (i - x) / w;
    for (const [dx, dy] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as const) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
      const ni = ny * w + nx;
      if (!mask[ni] && !outside[ni]) {
        outside[ni] = 1;
        queue.push(ni);
      }
    }
  }
  const holeRegion = (x: number, y: number): number => {
    if (x < 0 || y < 0 || x >= w || y >= h) return 0;
    const i = y * w + x;
    return !mask[i] && !outside[i] ? 1 : 0;
  };
  const holes = trace(holeRegion);

  return [...outer, ...holes];
}

// ============================================================================
// ساده‌سازی Douglas-Peucker (حلقهٔ بسته)
// ============================================================================

function perpendicularDist(
  p: [number, number],
  a: [number, number],
  b: [number, number]
): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len = Math.hypot(dx, dy);
  if (len < 1e-9) return Math.hypot(p[0] - a[0], p[1] - a[1]);
  return Math.abs(dy * (p[0] - a[0]) - dx * (p[1] - a[1])) / len;
}

function dpSegment(
  pts: [number, number][],
  first: number,
  last: number,
  tol: number,
  keep: boolean[]
): void {
  if (last <= first + 1) return;
  let maxD = -1;
  let idx = -1;
  for (let i = first + 1; i < last; i++) {
    const d = perpendicularDist(pts[i], pts[first], pts[last]);
    if (d > maxD) {
      maxD = d;
      idx = i;
    }
  }
  if (maxD > tol && idx > 0) {
    keep[idx] = true;
    dpSegment(pts, first, idx, tol, keep);
    dpSegment(pts, idx, last, tol, keep);
  }
}

/** ساده‌سازی حلقهٔ بسته با Douglas-Peucker — نقطهٔ شکست دورترین از نقطهٔ ۰ */
export function douglasPeuckerClosed(
  pts: [number, number][],
  tol: number
): [number, number][] {
  if (pts.length <= 4) return pts;
  let far = 0;
  let maxD = -1;
  for (let i = 1; i < pts.length; i++) {
    const d = Math.hypot(pts[i][0] - pts[0][0], pts[i][1] - pts[0][1]);
    if (d > maxD) {
      maxD = d;
      far = i;
    }
  }
  const keep = new Array<boolean>(pts.length).fill(false);
  keep[0] = true;
  keep[far] = true;
  dpSegment(pts, 0, far, tol, keep);
  dpSegment(pts, far, pts.length - 1, tol, keep);
  const out = pts.filter((_, i) => keep[i]);
  if (out.length >= 3) {
    const first = out[0];
    const lastC = out[out.length - 1];
    if (Math.hypot(first[0] - lastC[0], first[1] - lastC[1]) > 1e-9) {
      out.push([first[0], first[1]]);
    }
  }
  return out;
}

/** مساحت علامت‌دار (قانون کفش‌بندی) */
export function signedArea(pts: [number, number][]): number {
  let s = 0;
  for (let i = 0; i < pts.length; i++) {
    const a = pts[i];
    const b = pts[(i + 1) % pts.length];
    s += a[0] * b[1] - b[0] * a[1];
  }
  return s / 2;
}

/** فهرست خانوادهٔ فونت‌های پیشنهادی (ویندوز + باندل) */
export const FONT_SUGGESTIONS: { family: string; label: string }[] = [
  { family: "Vazirmatn", label: "وزیرمتن (باندل برنامه)" },
  { family: "Segoe UI", label: "Segoe UI" },
  { family: "Arial", label: "Arial" },
  { family: "Times New Roman", label: "Times New Roman" },
  { family: "Tahoma", label: "Tahoma" },
  { family: "Calibri", label: "Calibri" },
  { family: "Cascadia Code", label: "Cascadia Code" },
  { family: "Consolas", label: "Consolas" },
  { family: "Comic Sans MS", label: "Comic Sans MS" },
  { family: "Impact", label: "Impact" },
  { family: "Georgia", label: "Georgia" },
  { family: "Trebuchet MS", label: "Trebuchet MS" },
];
