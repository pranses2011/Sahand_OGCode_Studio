import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

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
 */
export default defineConfig({
  // بدون فونت/اسکریپت خارجی؛ همه‌چیز از src بارگذاری می‌شود
  plugins: [react()],
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
