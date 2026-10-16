import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

/**
 * پیکربندی Vite برای G-code Studio
 * =================================
 * چرا این فایل این‌قدر ساده است؟
 * - این برنامه یک اپ دسکتاپ Tauri است نه وب‌سایت؛ بنابراین:
 *   ۱) هیچ dev proxy یا server راه‌دور تعریف نمی‌شود؛
 *   ۲) base نسبت به tauri.conf.json تنظیم می‌شود (RelativeAssetUrls حالت پیش‌فرض
 *      تauri با frontendDist کار می‌کند)؛
 *   ۳) در build نهایی همهٔ assetها (JS/CSS/فونت) داخل باندل محلی قرار می‌گیرند.
 * - Vitest از همین فایل پیکربندی استفاده می‌کند (environment jsdom در
 *   vitest.config.ts جداگانه تنظیم شده تا وابستگی test با build قاطی نشود).
 *
 * پل توسعهٔ هستهٔ Rust (فقط dev):
 *   دستورهای IPC در پوستهٔ دسکتاپ با invoke کار می‌کنند. برای توسعه/گرفتن
 *   اسکرین‌شات در مرورگر، پلاگین زیر POST /api/ipc/<cmd> را به باینری
 *   gcode-cli (همان هستهٔ gcode-core) وصل می‌کند — محاسبات «واقعی»‌اند،
 *   نه دادهٔ ساختگی. این پلاگین در build نهایی و در بستهٔ نصب حضور ندارد.
 */

/** مسیر باینری CLI هسته (build debug کافی است) — نسبت به این فایل */
const CLI_BIN = fileURLToPath(
  new URL("./src-tauri/target/debug/gcode-cli", import.meta.url)
);

/** پلاگین پل توسعه: POST /api/ipc/:cmd → spawn gcode-cli */
function rustCoreDevBridge(): Plugin {
  return {
    name: "gcode-rust-core-dev-bridge",
    apply: "serve", // فقط سرور توسعه — در build نهایی وجود ندارد
    configureServer(server) {
      server.middlewares.use("/api/ipc", (req, res, next) => {
        if (!req.url || req.method !== "POST") {
          next();
          return;
        }
        // نام دستور از مسیر: /api/ipc/<cmd>
        const cmd = decodeURIComponent(req.url.replace(/^\//, ""));
        if (!cmd) {
          next();
          return;
        }
        let body = "";
        req.setEncoding("utf8");
        req.on("data", (chunk: string) => (body += chunk));
        req.on("end", () => {
          if (!existsSync(CLI_BIN)) {
            res.writeHead(500, { "Content-Type": "application/json; charset=utf-8" });
            res.end(
              JSON.stringify({
                __error:
                  "باینری gcode-cli ساخته نشده است — اجرا کنید: cargo build -p gcode-cli",
              })
            );
            return;
          }
          let args: unknown = {};
          try {
            args = body ? JSON.parse(body) : {};
          } catch {
            res.writeHead(400, { "Content-Type": "application/json; charset=utf-8" });
            res.end(JSON.stringify({ __error: "بدنهٔ درخواست JSON نامعتبر است" }));
            return;
          }
          const child = spawn(CLI_BIN, [], { stdio: ["pipe", "pipe", "pipe"] });
          let out = "";
          let errOut = "";
          child.stdout.on("data", (d: Buffer) => (out += d.toString("utf8")));
          child.stderr.on("data", (d: Buffer) => (errOut += d.toString("utf8")));
          child.on("close", () => {
            if (out.trim()) {
              res.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
              res.end(out);
            } else {
              res.writeHead(500, { "Content-Type": "application/json; charset=utf-8" });
              res.end(
                JSON.stringify({
                  __error: `هستهٔ Rust پاسخ نداد: ${errOut || "خروجی خالی"}`,
                })
              );
            }
          });
          child.on("error", (e) => {
            res.writeHead(500, { "Content-Type": "application/json; charset=utf-8" });
            res.end(JSON.stringify({ __error: `اجرای gcode-cli ناموفق بود: ${e.message}` }));
          });
          const payload = JSON.stringify({ cmd, args });
          child.stdin.write(payload);
          child.stdin.end();
        });
      });
    },
  };
}

export default defineConfig({
  // بدون فونت/اسکریپت خارجی؛ همه‌چیز از src بارگذاری می‌شود
  plugins: [react(), rustCoreDevBridge()],
  // خروجی build به پوشهٔ dist می‌رود که tauri.conf.json به آن اشاره می‌کند
  build: {
    // sourcemap برای build نهایی لازم نیست؛ حجم باندل دسکتاپ را کم نگه می‌دارد
    sourcemap: false,
    // هشدار حجم chunk برای اپ دسکتاپ آفلاین اهمیت عملی ندارد؛
    // three.js و codemirror به‌طور طبیعی بزرگ‌اند و split برای فایل محلی بی‌معناست
    chunkSizeWarningLimit: 4096,
    // نام‌های پایدار برای کش داخلی WebView2 و جلوگیری از hash تصادفی در تست‌ها
    rollupOptions: {
      output: {
        // یک chunk واحد: بارگذاری محلی از دیسک است؛ تعداد درخواست HTTP اهمیتی ندارد
        entryFileNames: "assets/[name].[hash].js",
        assetFileNames: "assets/[name].[hash].[ext]",
      },
    },
  },
});
