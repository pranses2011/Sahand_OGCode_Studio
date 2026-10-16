import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

/**
 * پیکربندی Vitest — جدا از vite.config.ts تا build و test قاطی نشوند
 * environment: node برای آزمون‌های منطق خالص (بدون DOM لازم)
 */
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
