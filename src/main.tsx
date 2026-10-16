/**
 * ============================================================================
 * main.tsx — نقطهٔ ورود React
 * ============================================================================
 * بارگذاری استایل سراسری و رندر پوستهٔ برنامه.
 * فونت وزیرمتن از asset محلی باندل می‌شود (نه CDN) — الزام آفلاین.
 * ============================================================================
 */

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app/App";
import "./styles/global.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
