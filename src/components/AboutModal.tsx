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
  | { phase: "sent" }
  | { phase: "queued"; reason?: string }
  | { phase: "error"; reason?: string };

export function AboutModal(props: AboutModalProps) {
  const { open, onClose } = props;
  const [tab, setTab] = useState<"about" | "feedback">("about");
  // فرم بازخورد
  const [fromName, setFromName] = useState("");
  const [replyEmail, setReplyEmail] = useState("");
  const [message, setMessage] = useState("");
  const [sendState, setSendState] = useState<SendState>({ phase: "idle" });
  const [queued, setQueued] = useState(0);

  // شمار صف هنگام باز شدن پنجره + تلاش برای تخلیهٔ آن
  useEffect(() => {
    if (!open) return;
    setQueued(feedbackQueueSize());
    void flushFeedbackQueue().then(() => setQueued(feedbackQueueSize()));
  }, [open]);

  if (!open) return null;

  const submitFeedback = async () => {
    if (message.trim().length < 3) {
      setSendState({ phase: "error", reason: "متن بازخورد خیلی کوتاه است." });
      return;
    }
    setSendState({ phase: "sending" });
    const r = await sendFeedback({
      kind: "feedback",
      message: message.trim(),
      fromName: fromName.trim() || undefined,
      replyEmail: replyEmail.trim() || undefined,
      context: "فرم بازخورد",
    });
    if (r.ok) {
      setSendState({ phase: "sent" });
      setMessage("");
    } else {
      setSendState({ phase: "queued", reason: r.reason });
      setQueued(feedbackQueueSize());
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
              <p className="send-note ok">
                با سپاس — بازخورد شما ارسال شد ✅ (اگر پاسخ خواستید، ایمیل
                خود را نوشته باشید)
              </p>
            )}
            {sendState.phase === "queued" && (
              <p className="send-note warn">
                اینترنت در دسترس نیست — پیام شما ذخیره شد و پس از اتصال دوباره
                ارسال می‌شود ⏳
                {sendState.reason ? ` (${sendState.reason})` : ""}
              </p>
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

        <div className="actions">
          <button className="btn" onClick={onClose}>
            بستن
          </button>
        </div>
      </div>
    </div>
  );
}
