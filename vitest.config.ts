import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

/**
 * پیکربندی Vitest — جدا از vite.config.ts تا build و test قاطی نشوند
 * environment: node برای آزمون‌های منطق خالص (بدون DOM لازم)
 */
export default defineConfig({
  // cast برای سازگاری تایپ plugin بین نسخه‌های vite/vitest
  plugins: [react() as never],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
