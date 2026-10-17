/**
 * ============================================================================
 * feedback.test.ts — آزمون ماژول بازخورد/گزارش خطا (نسخهٔ ۱٫۶)
 * ============================================================================
 * شبکه واقعی در آزمون ممنوع — sanitize و صف محلی (localStorage) خالص آزموده
 * می‌شوند؛ ارسال با mock fetch.
 * ============================================================================
 *
 * @vitest-environment jsdom — localStorage لازم است
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  AUTHOR,
  feedbackQueueSize,
  flushFeedbackQueue,
  sendFeedback,
} from "./feedback";

describe("بازخورد و گزارش خطا (درخواست کاربر ۹)", () => {
  beforeEach(() => {
    localStorage.clear();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("اطلاعات سازنده کامل است (دربارهٔ برنامه)", () => {
    expect(AUTHOR.name).toBe("سعید نجفی");
    expect(AUTHOR.phone).toBe("09144220430");
    expect(AUTHOR.email.toLowerCase()).toBe("pranses2011@gmail.com");
    expect(AUTHOR.website).toBe("EA-Fixer.ir");
    expect(AUTHOR.supportCard).toBe("6219861918934679");
    expect(AUTHOR.supportBank).toBe("بانک سامان");
  });

  it("sendFeedback موفق: nothing queued", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ success: true }), { status: 200 })
    );
    vi.stubGlobal("fetch", fetchMock);
    const r = await sendFeedback({ kind: "feedback", message: "سلام تست" });
    expect(r.ok).toBe(true);
    expect(r.queued).toBe(false);
    expect(feedbackQueueSize()).toBe(0);
    expect(fetchMock).toHaveBeenCalledOnce();
    // بدنهٔ درخواست: ایمیل مقصد formsubmit با _subject
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(String(url)).toContain("formsubmit.co/ajax/");
    const body = JSON.parse(String(init.body));
    expect(body["_subject"]).toContain("Sahand GCode Studio");
    expect(body["پیام"]).toContain("سلام تست");
  });

  it("شبکه قطع: پیام در صف محلی می‌ماند و بعداً ارسال می‌شود", async () => {
    // ۱) شکست شبکه
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
    const r = await sendFeedback({ kind: "error", message: "خطای آزمون" });
    expect(r.ok).toBe(false);
    expect(r.queued).toBe(true);
    expect(feedbackQueueSize()).toBe(1);
    // ۲) اتصال برمی‌گردد — flush صف
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", { status: 200 })
    );
    vi.stubGlobal("fetch", fetchMock);
    const flushed = await flushFeedbackQueue();
    expect(flushed).toBe(1);
    expect(feedbackQueueSize()).toBe(0);
    // بدنهٔ خطای صف‌شده همان پیام بود
    const body = JSON.parse(String((fetchMock.mock.calls[0][1] as RequestInit).body));
    expect(body["پیام"]).toContain("خطای آزمون");
  });

  it("حذف توکن/مسیر از متن خطا (حریم خصوصی)", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await sendFeedback({
      kind: "error",
      message: "خطا در /home/user/project/src/app/App.tsx با توکن ghp_abcdefghij12345",
    });
    const body = JSON.parse(String((fetchMock.mock.calls[0][1] as RequestInit).body));
    const msg = body["پیام"] as string;
    expect(msg).toContain("App.tsx");
    expect(msg).not.toContain("/home/user");
    expect(msg).not.toContain("ghp_");
  });
});
