/**
 * ============================================================================
 * lib/feedback.ts — بازخورد و گزارش خطا به ایمیل سازنده (نسخهٔ ۱٫۷)
 * ============================================================================
 * رفع باگ ۱٫۶ (درخواست کاربر ۱۰):
 *   «ایمیل ارسال نمیشود (پیام موفقیت آمیز بودن رو میگه ولی هیچی به ایمیلم
 *    نمیاد)» — علت: سرویس formsubmit «پاسخ موفق» می‌داد ولی تا فعال‌سازی
 *    یک‌بارهٔ مالک ایمیل، هیچ ایمیلی تحویل نمی‌شد.
 *
 * زنجیرهٔ صادقانهٔ ۱٫۷ — هر کانال واقعاً چه کرد گزارش می‌شود:
 *   ۱) SMTP مستقیم (قابل‌اتکا): تنظیمات در «تنظیمات ایمیل» ذخیره می‌شود و
 *      ارسال مستقیماً از سرور خود سازنده انجام می‌شود (crate gcode-mail)؛
 *   ۲) FormSubmit (پیش‌فرض بدون تنظیمات): تحویل به سرویس فرم — برای قطعی
 *      شدن دریافت، سازنده باید یک‌بار ایمیل فعال‌سازی را تأیید کند؛
 *   ۳) mailto: پشتیبان دکمه‌ای — برنامهٔ ایمیل خود کاربر باز می‌شود؛
 *   ۴) صف محلی: در نبود اینترنت گزارش می‌ماند و بوت بعدی دوباره تلاش می‌کند.
 *
 * «موفق» فقط وقتی گزارش می‌شود که کانال واقعاً تأیید کند؛ queued ≠ موفق.
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

/** کانال ارسال — برای گزارش صادقانه در UI */
export type SendChannel = "smtp" | "formsubmit" | "queued";

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

/** نتیجهٔ صادقانهٔ ارسال */
export interface SendResult {
  ok: boolean;
  channel: SendChannel;
  /** پیام فارسی وضعیت — دقیقاً همان‌چه اتفاق افتاد */
  messageFa: string;
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

/** آیا در پوستهٔ دسکتاپ هستیم؟ */
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// ============================================================================
// کانال ۱: SMTP مستقیم — از تنظیمات ذخیره‌شدهٔ برنامه (crate gcode-mail)
// ============================================================================

async function sendViaSmtp(payload: FeedbackPayload): Promise<SendResult | null> {
  if (!isTauri()) return null; // فقط دسکتاپ
  try {
    const { smtpSendFeedback } = await import("./tauri");
    const report = await smtpSendFeedback({
      kind: payload.kind,
      message: sanitize(payload.message),
      from_name: payload.fromName,
      reply_email: payload.replyEmail,
      context: payload.context,
      detail: payload.detail ? sanitize(payload.detail) : undefined,
      app_version: pkg.version,
    });
    if (report.ok) {
      return {
        ok: true,
        channel: "smtp",
        messageFa: "ایمیل مستقیماً از سرور SMTP شما ارسال و تحویل داده شد ✅",
      };
    }
    return null; // خطای SMTP → کانال بعدی
  } catch {
    return null; // SMTP تنظیم نشده/خطا → کانال بعدی
  }
}

// ============================================================================
// کانال ۲: FormSubmit — پشتیبان بدون تنظیمات (نیازمند فعال‌سازی یک‌باره)
// ============================================================================

async function sendViaFormSubmit(payload: FeedbackPayload): Promise<SendResult> {
  const body = {
    _subject:
      payload.kind === "error"
        ? `گزارش خودکار خطا — Sahand GCode Studio v${pkg.version}`
        : `بازخورد کاربر — Sahand GCode Studio v${pkg.version}`,
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
    let res: Response;
    if (isTauri()) {
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
    if (res.ok) {
      // صادقانه: تحویل به سرویس فرم انجام شد؛ دریافت نهایی به فعال‌سازی
      // یک‌بارهٔ سازنده وابسته است — این را شفاف می‌گوییم (رفع باگ ۱٫۶).
      return {
        ok: true,
        channel: "formsubmit",
        messageFa:
          "به سرویس فرم تحویل شد ✅ — دریافت قطعی نیازمند تأیید یک‌بارهٔ ایمیل فعال‌سازی است (اگر هنوز تأیید نکرده‌اید، پوشهٔ اسپم Pranses2011@GMail.com را ببینید)؛ برای ارسال همیشه-قطعی «تنظیمات ایمیل ← SMTP» را کامل کنید",
      };
    }
    return {
      ok: false,
      channel: "formsubmit",
      messageFa: `سرویس فرم پاسخ ${res.status} داد`,
    };
  } catch (e) {
    return {
      ok: false,
      channel: "formsubmit",
      messageFa: `ارتباط برقرار نشد: ${String(e)}`,
    };
  }
}

// ============================================================================
// صف محلی — نبود اینترنت/خطا ⇒ گزارش می‌ماند و بعداً تلاش می‌شود
// ============================================================================

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

/** ارسال بازخورد/گزارش — زنجیرهٔ صادقانه: SMTP → FormSubmit → صف */
export async function sendFeedback(payload: FeedbackPayload): Promise<SendResult> {
  // ۱) SMTP مستقیم (اگر تنظیم شده)
  const smtp = await sendViaSmtp(payload);
  if (smtp) return smtp;

  // ۲) FormSubmit
  const fs = await sendViaFormSubmit(payload);
  if (fs.ok) return fs;

  // ۳) صف محلی — честно: ارسال نشد
  const q = readQueue();
  q.push({ payload, queuedAt: Date.now() });
  writeQueue(q);
  return {
    ok: false,
    channel: "queued",
    messageFa: `${fs.messageFa} — گزارش در صف محلی ذخیره شد و با اتصال اینترنت دوباره تلاش می‌شود`,
  };
}

/** تلاش دوباره برای صف محلی — هنگام بوت برنامه صدا زده می‌شود */
export async function flushFeedbackQueue(): Promise<number> {
  const q = readQueue();
  if (q.length === 0) return 0;
  const remaining: QueuedItem[] = [];
  for (const item of q) {
    const smtp = await sendViaSmtp(item.payload);
    if (smtp?.ok) continue;
    const fs = await sendViaFormSubmit(item.payload);
    if (!fs.ok) remaining.push(item);
  }
  writeQueue(remaining);
  return q.length - remaining.length;
}

/** تعداد موارد در انتظار (نمایش وضعیت در UI) */
export function feedbackQueueSize(): number {
  return readQueue().length;
}

/** ساخت لینک mailto: پشتیبان — «ارسال با برنامهٔ ایمیل خودم» */
export function buildMailtoLink(payload: FeedbackPayload): string {
  const subject =
    payload.kind === "error"
      ? `گزارش خودکار خطا — Sahand GCode Studio v${pkg.version}`
      : `بازخورد کاربر — Sahand GCode Studio v${pkg.version}`;
  const lines = [
    payload.message,
    "",
    "— — —",
    payload.fromName ? `از: ${payload.fromName}` : "",
    payload.replyEmail ? `ایمیل پاسخ: ${payload.replyEmail}` : "",
    payload.context ? `بخش برنامه: ${payload.context}` : "",
    environmentLine(),
    payload.detail ? `جزئیات فنی:\n${sanitize(payload.detail)}` : "",
  ].filter(Boolean);
  const body = encodeURIComponent(lines.join("\n"));
  return `mailto:${AUTHOR.email}?subject=${encodeURIComponent(subject)}&body=${body}`;
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
