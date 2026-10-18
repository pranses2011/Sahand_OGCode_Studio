/**
 * ============================================================================
 * components/AboutModal.tsx — دربارهٔ برنامه + فرم بازخورد (نسخهٔ ۱٫۶)
 * ============================================================================
 * درخواست کاربر ۹:
 *   اطلاعات طراح/نویسنده (سعید نجفی — تماس/ایمیل/سایت/کارت حمایت) +
 *   فرم «نظرات و پیشنهادات» که از داخل برنامه (بدون نرم‌افزار ایمیل)
 *   به ایمیل سازنده ارسال می‌شود + وضعیت گزارش‌های خطای خودکار.
 * ============================================================================
 */

import { useEffect, useState } from "react";
import pkg from "../../package.json";
import {
  AUTHOR,
  buildMailtoLink,
  feedbackQueueSize,
  flushFeedbackQueue,
  sendFeedback,
} from "../lib/feedback";

export interface AboutModalProps {
  open: boolean;
  onClose: () => void;
}

type SendState =
  | { phase: "idle" }
  | { phase: "sending" }
  | { phase: "sent"; messageFa?: string; channel?: string }
  | { phase: "queued"; reason?: string }
  | { phase: "error"; reason?: string };

export function AboutModal(props: AboutModalProps) {
  const { open, onClose } = props;
  const [tab, setTab] = useState<"about" | "feedback" | "smtp">("about");
  // فرم بازخورد
  const [fromName, setFromName] = useState("");
  const [replyEmail, setReplyEmail] = useState("");
  const [message, setMessage] = useState("");
  const [sendState, setSendState] = useState<SendState>({ phase: "idle" });
  const [queued, setQueued] = useState(0);
  // ۱٫۷ (درخواست ۱۰): تنظیمات SMTP — ارسال قطعی ایمیل از سرور خود سازنده
  const [smtpCfg, setSmtpCfg] = useState<{
    host: string;
    port: number;
    username: string;
    from_name: string;
    to_email: string;
    has_password: boolean;
    ready: boolean;
  } | null>(null);
  const [smtpForm, setSmtpForm] = useState({
    host: "smtp.gmail.com",
    port: 465,
    username: "",
    password: "",
    to_email: "Pranses2011@GMail.com",
  });
  const [smtpBusy, setSmtpBusy] = useState(false);
  const [smtpNote, setSmtpNote] = useState("");

  // شمار صف هنگام باز شدن پنجره + تلاش برای تخلیهٔ آن + بارگذاری SMTP
  useEffect(() => {
    if (!open) return;
    setQueued(feedbackQueueSize());
    void flushFeedbackQueue().then(() => setQueued(feedbackQueueSize()));
    // ۱٫۷: وضعیت SMTP (بدون رمز) برای نمایش
    void import("../lib/tauri").then(({ smtpGetConfig, runningInDesktop }) => {
      if (!runningInDesktop()) return;
      smtpGetConfig()
        .then((c) => {
          setSmtpCfg(c);
          setSmtpForm((f) => ({
            ...f,
            host: c.host,
            port: c.port,
            username: c.username,
            to_email: c.to_email,
          }));
        })
        .catch(() => setSmtpCfg(null));
    }).catch(() => setSmtpCfg(null));
  }, [open]);

  if (!open) return null;

  const submitFeedback = async () => {
    if (message.trim().length < 3) {
      setSendState({ phase: "error", reason: "متن بازخورد خیلی کوتاه است." });
      return;
    }
    setSendState({ phase: "sending" });
    // زنجیرهٔ صادقانهٔ ۱٫۷: SMTP مستقیم → FormSubmit → صف — هر کانال واقعاً
    // چه کرد گزارش می‌شود (رفع باگ «موفق می‌گوید ولی نمی‌رسد»)
    const r = await sendFeedback({
      kind: "feedback",
      message: message.trim(),
      fromName: fromName.trim() || undefined,
      replyEmail: replyEmail.trim() || undefined,
      context: "فرم بازخورد",
    });
    if (r.ok) {
      setSendState({ phase: "sent", messageFa: r.messageFa, channel: r.channel });
      setMessage("");
    } else {
      setSendState({ phase: "queued", reason: r.messageFa });
      setQueued(feedbackQueueSize());
    }
  };

  /** ذخیرهٔ تنظیمات SMTP (۱٫۷ — درخواست ۱۰) */
  const saveSmtp = async () => {
    setSmtpBusy(true);
    setSmtpNote("");
    try {
      const { smtpSaveConfig } = await import("../lib/tauri");
      const c = await smtpSaveConfig({
        host: smtpForm.host.trim() || "smtp.gmail.com",
        port: Math.round(+smtpForm.port || 465),
        username: smtpForm.username.trim(),
        password: smtpForm.password,
        to_email: smtpForm.to_email.trim() || "Pranses2011@GMail.com",
      });
      setSmtpCfg(c);
      setSmtpNote("تنظیمات ذخیره شد ✅ — از این پس همهٔ بازخوردها مستقیماً از سرور شما ارسال می‌شود");
    } catch (e) {
      setSmtpNote(`ذخیره ناموفق: ${String(e)}`);
    } finally {
      setSmtpBusy(false);
    }
  };

  /** ارسال ایمیل آزمایشی SMTP */
  const testSmtp = async () => {
    setSmtpBusy(true);
    setSmtpNote("");
    try {
      const { smtpTest } = await import("../lib/tauri");
      const r = await smtpTest();
      setSmtpNote(r.ok ? "ایمیل آزمایشی ارسال شد ✅ — صندوق خود را چک کنید" : `ارسال ناموفق: ${r.message}`);
    } catch (e) {
      setSmtpNote(`ارسال ناموفق: ${String(e)}`);
    } finally {
      setSmtpBusy(false);
    }
  };

  return (
    <div className="modal-back" role="dialog" aria-modal="true">
      <div className="modal about-modal">
        <div className="about-tabs" role="tablist">
          <button
            className={`btn small ${tab === "about" ? "primary" : ""}`}
            role="tab"
            aria-selected={tab === "about"}
            onClick={() => setTab("about")}
          >
            دربارهٔ برنامه
          </button>
          <button
            className={`btn small ${tab === "feedback" ? "primary" : ""}`}
            role="tab"
            aria-selected={tab === "feedback"}
            onClick={() => setTab("feedback")}
          >
            نظرات و پیشنهادات
          </button>
          {/* ۱٫۷ (درخواست ۱۰): تنظیمات ایمیل SMTP — ارسال قطعی */}
          <button
            className={`btn small ${tab === "smtp" ? "primary" : ""}`}
            role="tab"
            aria-selected={tab === "smtp"}
            onClick={() => setTab("smtp")}
          >
            ⚙️ تنظیمات ایمیل
          </button>
        </div>

        {tab === "about" && (
          <div className="about-body">
            <div className="about-brand">
              <div className="about-logo" aria-hidden="true">
                <img src="brand-icon.png" alt="" width={64} height={64} />
              </div>
              <div>
                <h2 className="about-title">Sahand GCode Studio</h2>
                <div className="about-version ltr">نسخهٔ {pkg.version}</div>
                <div className="about-tagline">
                  استودیوی ویرایش و طراحی G-code — طراح مارپیچی، الگو و بافت
                </div>
              </div>
            </div>

            <div className="about-author">
              <h3>طراح و نویسنده</h3>
              <table className="about-table">
                <tbody>
                  <tr>
                    <th>نام</th>
                    <td>{AUTHOR.name}</td>
                  </tr>
                  <tr>
                    <th>تلفن</th>
                    <td className="ltr">{AUTHOR.phone}</td>
                  </tr>
                  <tr>
                    <th>ایمیل</th>
                    <td className="ltr">{AUTHOR.email}</td>
                  </tr>
                  <tr>
                    <th>وب‌سایت</th>
                    <td className="ltr">
                      <a href={`https://${AUTHOR.website}`} target="_blank" rel="noreferrer">
                        {AUTHOR.website}
                      </a>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div className="about-support">
              <h3>حمایت از توسعهٔ برنامه</h3>
              <p>
                اگر این برنامه برایتان مفید بود، می‌توانید با کارت زیر از
                توسعهٔ آن حمایت کنید:
              </p>
              <div className="support-card ltr" title="کپی با کلیک">
                <button
                  className="card-copy"
                  onClick={(e) => {
                    void navigator.clipboard?.writeText(AUTHOR.supportCard);
                    e.currentTarget.textContent = "کپی شد ✓";
                    setTimeout(() => {
                      e.currentTarget.textContent = AUTHOR.supportCard;
                    }, 1600);
                  }}
                >
                  {AUTHOR.supportCard}
                </button>
              </div>
              <div className="support-bank">{AUTHOR.supportBank}</div>
            </div>

            <div className="about-notes">
              <p>
                🛡️ گزارش خطاهای برنامه به‌صورت خودکار برای سازنده ارسال می‌شود
                تا سریع‌تر اصلاح شوند (نیازمند اینترنت؛ در نبود اینترنت در صف
                محلی می‌ماند و بعداً ارسال می‌شود).
              </p>
              <p>
                📮 نظرها و پیشنهادهای شما از تب «نظرات و پیشنهادات» مستقیم به
                ایمیل سازنده می‌رسد — بدون نیاز به نرم‌افزار ایمیل.
              </p>
              {queued > 0 && (
                <p className="queued-note">
                  ⏳ {queued} گزارش در صف ارسال است — با اتصال اینترنت دوباره
                  تلاش می‌شود.
                </p>
              )}
            </div>
          </div>
        )}

        {tab === "feedback" && (
          <div className="about-body">
            <p className="feedback-intro">
              نظر، پیشنهاد یا گزارش مشکل خود را بنویسید — مستقیم به
              <strong className="ltr"> {AUTHOR.email} </strong>
              ارسال می‌شود (از داخل برنامه، بدون نرم‌افزار ایمیل).
            </p>
            <div className="field">
              <label>نام شما (اختیاری)</label>
              <input
                type="text"
                value={fromName}
                onChange={(e) => setFromName(e.target.value)}
                placeholder="مثلاً: علی"
              />
            </div>
            <div className="field">
              <label>ایمیل برای پاسخ (اختیاری)</label>
              <input
                type="email"
                dir="ltr"
                value={replyEmail}
                onChange={(e) => setReplyEmail(e.target.value)}
                placeholder="you@example.com"
              />
            </div>
            <div className="field">
              <label>پیام شما</label>
              <textarea
                rows={5}
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder="چه چیزی را دوست دارید؟ چه چیزی باید بهتر شود؟"
              />
            </div>
            {sendState.phase === "sending" && <p className="send-note">در حال ارسال…</p>}
            {sendState.phase === "sent" && (
              <div className="send-note ok">
                <p style={{ margin: 0 }}>
                  با سپاس — {sendState.messageFa ?? "بازخورد شما ارسال شد ✅"}
                </p>
                {sendState.channel === "formsubmit" && (
                  <p style={{ margin: "6px 0 0", fontSize: 12 }}>
                    💡 برای «همیشه قطعی» تب «⚙️ تنظیمات ایمیل» را کامل کنید —
                    ارسال مستقیم از سرور SMTP خودتان بدون واسطه.
                  </p>
                )}
              </div>
            )}
            {sendState.phase === "queued" && (
              <div className="send-note warn">
                <p style={{ margin: 0 }}>
                  {sendState.reason ?? "اینترنت در دسترس نیست — پیام ذخیره شد و پس از اتصال دوباره ارسال می‌شود ⏳"}
                </p>
                <button
                  className="btn small"
                  style={{ marginTop: 6 }}
                  onClick={() => {
                    const link = buildMailtoLink({
                      kind: "feedback",
                      message: message.trim() || "(خالی)",
                      fromName: fromName.trim() || undefined,
                      replyEmail: replyEmail.trim() || undefined,
                      context: "فرم بازخورد",
                    });
                    window.open(link, "_self");
                  }}
                >
                  📧 ارسال با برنامهٔ ایمیل خودم (mailto)
                </button>
              </div>
            )}
            {sendState.phase === "error" && (
              <p className="send-note warn">{sendState.reason ?? "ارسال ناموفق بود."}</p>
            )}
            <div className="actions">
              <button
                className="btn primary"
                disabled={sendState.phase === "sending"}
                onClick={submitFeedback}
              >
                ✉️ ارسال بازخورد
              </button>
            </div>
            <p className="feedback-privacy">
              🔒 فقط متن این فرم و نسخهٔ برنامه ارسال می‌شود — هیچ فایل یا
              دادهٔ دیگری از سیستم شما ارسال نمی‌گردد.
            </p>
          </div>
        )}

        {tab === "smtp" && (
          <div className="about-body">
            <p className="feedback-intro">
              ⚙️ <strong>تنظیمات ایمیل SMTP</strong> — رفع قطعی مشکل «ایمیل
              نمی‌رسد»: برنامه با این تنظیمات <strong>مستقیماً</strong> از
              سرور ایمیل خودِ شما (مثل Gmail) ارسال می‌کند؛ بدون هیچ سرویس
              واسط و بدون نیاز به فعال‌سازی.
            </p>

            {smtpCfg?.ready ? (
              <p className="send-note ok" style={{ marginTop: 0 }}>
                ✅ SMTP فعال است ({smtpCfg.username} → {smtpCfg.to_email}) —
                همهٔ بازخوردها و گزارش‌های خطا مستقیم ارسال می‌شوند
              </p>
            ) : (
              <p className="send-note warn" style={{ marginTop: 0 }}>
                ⚠️ SMTP تنظیم نشده — در حال حاضر از سرویس فرم واسط استفاده
                می‌شود که نیازمند «تأیید یک‌بارهٔ ایمیل فعال‌سازی» است
                (ایمیل فعال‌سازی بعد از اولین ارسال به
                <span className="ltr"> {smtpCfg?.to_email ?? AUTHOR.email} </span>
                می‌آید — پوشهٔ اسپم را هم ببینید)
              </p>
            )}

            <div className="field">
              <label>میزبان SMTP</label>
              <input
                type="text"
                dir="ltr"
                value={smtpForm.host}
                onChange={(e) => setSmtpForm((f) => ({ ...f, host: e.target.value }))}
                placeholder="smtp.gmail.com"
              />
            </div>
            <div className="field">
              <label>پورت</label>
              <select
                dir="ltr"
                value={smtpForm.port}
                onChange={(e) => setSmtpForm((f) => ({ ...f, port: +e.target.value }))}
              >
                <option value={465}>465 — TLS (پیشنهادی Gmail)</option>
                <option value={587}>587 — STARTTLS</option>
              </select>
            </div>
            <div className="field">
              <label>نام کاربری (آدرس کامل ایمیل فرستنده)</label>
              <input
                type="email"
                dir="ltr"
                value={smtpForm.username}
                onChange={(e) => setSmtpForm((f) => ({ ...f, username: e.target.value }))}
                placeholder="you@gmail.com"
              />
            </div>
            <div className="field">
              <label>رمز / App Password</label>
              <input
                type="password"
                dir="ltr"
                value={smtpForm.password}
                onChange={(e) => setSmtpForm((f) => ({ ...f, password: e.target.value }))}
                placeholder={smtpCfg?.has_password ? "•••••••••••••• (ذخیره شده — برای تغییر بنویسید)" : "۱۶ کاراکتر App Password"}
              />
              <span className="hint-text">
                برای Gmail: حساب Google ← Security ← 2-Step Verification ←
                App passwords — رمز ۱۶رقمی بسازید و اینجا بگذارید. رمز فقط
                روی همین کامپیوتر (app_data برنامه) ذخیره می‌شود.
              </span>
            </div>
            <div className="field">
              <label>گیرنده (پیش‌فرض ایمیل سازنده)</label>
              <input
                type="email"
                dir="ltr"
                value={smtpForm.to_email}
                onChange={(e) => setSmtpForm((f) => ({ ...f, to_email: e.target.value }))}
              />
            </div>
            <div className="actions" style={{ display: "flex", gap: 6 }}>
              <button className="btn primary" disabled={smtpBusy} onClick={saveSmtp}>
                💾 ذخیرهٔ تنظیمات
              </button>
              <button className="btn" disabled={smtpBusy || !smtpCfg?.ready} onClick={testSmtp}>
                🧪 ارسال ایمیل آزمایشی
              </button>
            </div>
            {smtpNote && <p className="send-note" style={{ marginTop: 8 }}>{smtpNote}</p>}
            <p className="feedback-privacy" style={{ marginTop: 10 }}>
              🔒 رمز SMTP هرگز به فایل خروجی/لاگ کپی نمی‌شود و فقط محلی در
              دادهٔ برنامه ذخیره می‌ماند — مثل هر نرم‌افزار ایمیل دسکتاپ.
            </p>
          </div>
        )}

        <div className="actions">
          <button className="btn" onClick={onClose}>
            بستن
          </button>
        </div>
      </div>
    </div>
  );
}
