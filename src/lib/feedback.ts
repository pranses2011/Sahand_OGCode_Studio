/**
 * ============================================================================
 * lib/feedback.ts — بازخورد و گزارش خطا به ایمیل سازنده (نسخهٔ ۱٫۶)
 * ============================================================================
 * درخواست کاربر ۹:
 *   «هنگام خطا دادن، متن و توضیح خطا را به ایمیلم ارسال کن تا اصلاحش کنم؛
 *    و کاربران بتوانند نظرات و پیشنهاداتشان را از داخل خود برنامه بفرستند
 *    (نه با نرم‌افزار ایمیل).»
 *
 * سازوکار صادقانه و بدون نیاز به رمز SMTP:
 *   POST https://formsubmit.co/ajax/Pranses2011@GMail.com — سرویس فرم‌به‌ایمیل.
 *   * نخستین ارسال نیازمند تأیید یک‌بارهٔ مالک ایمیل است (ایمیل فعال‌سازی
 *     formsubmit به Pranses2011@GMail.com می‌آید) — پس از آن همه‌چیز خودکار است.
 *   * اگر ارسال شکست خورد (بدون اینترنت/فعال‌سازی)، گزارش در صف محلی
 *     (localStorage) می‌ماند و دفعات بعد دوباره تلاش می‌شود.
 *   * دامنهٔ HTTP در capabilities/tauri.conf به همین سرویس محدود است.
 *
 * انواع پیام: feedback (بازخورد دستی کاربر) · error (گزارش خودکار خطا)
 * ============================================================================
 */

import pkg from "../../package.json";

/** اطلاعات سازنده — «دربارهٔ برنامه» هم از همین منبع واحد می‌خواند */
export const AUTHOR = {
  name: "سعید نجفی",
  phone: "09144220430",
  email: "Pranses2011@GMail.com",
  website: "EA-Fixer.ir",
  supportCard: "6219861918934679",
  supportBank: "بانک سامان",
  role: "طراح و نویسنده",
} as const;

const ENDPOINT = "https://formsubmit.co/ajax/Pranses2011@GMail.com";
const QUEUE_KEY = "sahand-gcode-studio:feedback-outbox";

export type FeedbackKind = "feedback" | "error";

export interface FeedbackPayload {
  kind: FeedbackKind;
  /** پیام اصلی (متن بازخورد یا متن خطا) */
  message: string;
  /** نام فرستنده (اختیاری — برای بازخورد) */
  fromName?: string;
  /** ایمیل فرستنده برای پاسخ (اختیاری) */
  replyEmail?: string;
  /** خطای فنی همراه (stack/جزئیات) */
  detail?: string;
  /** چه بخشی از برنامه */
  context?: string;
}

interface QueuedItem {
  payload: FeedbackPayload;
  queuedAt: number;
}

/** فیلتر متن خطا — مسیر فایل‌های محلی و توکن‌ها را حذف می‌کند (حریم خصوصی) */
function sanitize(text: string): string {
  return text
    // مسیرهای مطلق فایل/دیسک → فقط نام فایل
    .replace(/(?:file|\/)[^\s]*\/([A-Za-z0-9_.-]+\.(?:ts|tsx|rs|js|json))/g, "$1")
    // توکن‌های احتمالی
    .replace(/\b(?:ghp|gho|github_pat)[A-Za-z0-9_]{10,}\b/g, "[توکن-حذف‌شده]")
    .slice(0, 4000);
}

/** محیط اجرا برای گزارش */
function environmentLine(): string {
  const ua = typeof navigator !== "undefined" ? navigator.userAgent : "unknown";
  return `Sahand GCode Studio v${pkg.version} · ${ua}`;
}

async function postForm(payload: FeedbackPayload): Promise<{ ok: boolean; reason?: string }> {
  const subject =
    payload.kind === "error"
      ? `گزارش خودکار خطا — Sahand GCode Studio v${pkg.version}`
      : `بازخورد کاربر — Sahand GCode Studio v${pkg.version}`;
  const body = {
    _subject: subject,
    _template: "table",
    نوع: payload.kind === "error" ? "گزارش خطای خودکار" : "بازخورد کاربر",
    از: payload.fromName || "کاربر ناشناس",
    "ایمیل پاسخ": payload.replyEmail || "—",
    "بخش برنامه": payload.context || "—",
    پیام: sanitize(payload.message),
    "جزئیات فنی": payload.detail ? sanitize(payload.detail) : "—",
    محیط: environmentLine(),
  };
  try {
    // در پوستهٔ دسکتاپ: fetch پلاگین HTTP با دامنهٔ مجاز؛ در dev: fetch مرورگر
    let res: Response;
    const isTauri =
      typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    if (isTauri) {
      const { fetch: tauriFetch } = await import("@tauri-apps/plugin-http");
      res = await tauriFetch(ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(body),
      });
    } else {
      res = await fetch(ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(body),
      });
    }
    if (!res.ok) {
      return { ok: false, reason: `پاسخ سرویس: ${res.status}` };
    }
    return { ok: true };
  } catch (e) {
    return { ok: false, reason: String(e) };
  }
}

function readQueue(): QueuedItem[] {
  try {
    const raw = localStorage.getItem(QUEUE_KEY);
    return raw ? (JSON.parse(raw) as QueuedItem[]) : [];
  } catch {
    return [];
  }
}

function writeQueue(q: QueuedItem[]) {
  try {
    localStorage.setItem(QUEUE_KEY, JSON.stringify(q.slice(-20))); // حداکثر ۲۰ مورد
  } catch {
    /* فضای پر — بی‌خیال صف */
  }
}

/** ارسال بازخورد/گزارش — در شکست، به صف محلی می‌رود */
export async function sendFeedback(payload: FeedbackPayload): Promise<{ ok: boolean; queued: boolean; reason?: string }> {
  const r = await postForm(payload);
  if (r.ok) return { ok: true, queued: false };
  const q = readQueue();
  q.push({ payload, queuedAt: Date.now() });
  writeQueue(q);
  return { ok: false, queued: true, reason: r.reason };
}

/** تلاش دوباره برای صف محلی — هنگام بوت برنامه صدا زده می‌شود */
export async function flushFeedbackQueue(): Promise<number> {
  const q = readQueue();
  if (q.length === 0) return 0;
  const remaining: QueuedItem[] = [];
  for (const item of q) {
    const r = await postForm(item.payload);
    if (!r.ok) remaining.push(item);
  }
  writeQueue(remaining);
  return q.length - remaining.length;
}

/** تعداد موارد در انتظار (نمایش وضعیت در UI) */
export function feedbackQueueSize(): number {
  return readQueue().length;
}

/** شنود سراسری خطاهای JS — گزارش خودکار (درخواست کاربر ۹) */
export function installGlobalErrorReporter(onCaptured?: (message: string) => void): () => void {
  const onError = (ev: ErrorEvent) => {
    const msg = ev.message || "خطای ناشناخته";
    void sendFeedback({
      kind: "error",
      message: msg,
      detail: ev.error instanceof Error ? ev.error.stack || String(ev.error) : undefined,
      context: "پنجره",
    });
    onCaptured?.(msg);
  };
  const onRejection = (ev: PromiseRejectionEvent) => {
    const msg = String(ev.reason ?? "رد شدن Promise بدون پیام");
    void sendFeedback({
      kind: "error",
      message: msg,
      detail: ev.reason instanceof Error ? ev.reason.stack || String(ev.reason) : undefined,
      context: "Promise",
    });
    onCaptured?.(msg);
  };
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
  return () => {
    window.removeEventListener("error", onError);
    window.removeEventListener("unhandledrejection", onRejection);
  };
}
