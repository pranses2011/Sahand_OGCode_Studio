/**
 * ============================================================================
 * features/spiral-designer/SpiralDesignerPage.tsx — طراح مارپیچی ۵ گامی
 * ============================================================================
 * گردش‌کار (Docs/PROMPT_03): ۱. پروفایل ۲. الگو ۳. بافت ۴. کف ۵. پرینتر
 * + پیش‌نمایش زنده سه‌بعدی + خروجی G-code/STL/پروژه + ارسال به ویرایشگر.
 *
 * تمایز حیاتی در UI (تأیید از پژوهش مرجع):
 *   الگو → هندسه (در STL می‌ماند)؛ بافت → مسیر نازل (فقط در G-code).
 *
 * نسخهٔ ۱.۲ (درخواست‌های کاربر):
 *   - کارت‌های بزرگ تصویری برای ۱۷ الگو / ۱۲ بافت / ۵ الگوی کف —
 *     تصویرها از «همان ریاضی هستهٔ Rust» رندر می‌شوند (designer_preview_data)؛
 *   - اسپات قابل‌درگ روی نمای 3D: بازهٔ z الگو/بافت و قطر سوراخ کف
 *     با کشیدن حلقه‌ها مستقیماً جابه‌جا می‌شود؛
 *   - مرکز‌گذاری طرح روی صفحهٔ چاپ (پیش‌فرض روشن — رفع باگ 0,0)؛
 *   - درون‌یابی اسپلاین مونوتون پروفایل (پیش‌فرض — بدون لبهٔ تیز)؛
 *   - واگرد/ازنوی کامل طراحی + ۶ استایل آمادهٔ قابل‌ویرایش.
 *
 * نسخهٔ ۱.۴ (درخواست‌های کاربر):
 *   - پیش‌نمایش با پیلود تخت سریع (designer_viewport) — بدون mesh و بدون
 *     JSON شیءمحور؛ شعاع اسپات از radius_samples — رفع لگ/وقفهٔ نمایشگر؛
 *   - کف چندلایه (انتخاب تعداد لایه) — سپس ادامهٔ vase؛
 *   - تنظیمات سرعت به mm/s (لایهٔ اول/کف/دیواره/travel) + رتراکشن؛
 *   - اسپات الگو/بافت روی سطح مدل: درگ عمودی = ارتفاع، افقی = زاویهٔ فاز.
 * ============================================================================
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ProfileEditor } from "./ProfileEditor";
import { SurfaceThumb, FloorThumb, ProfileMiniThumb } from "./Thumbs";
import { Viewport } from "../../components/Viewport";
import type { SpotHandle, SpotChange } from "../../components/Viewport";
import { fa } from "../../i18n/fa";
import type {
  DesignerProject, PatternParams, TextureParams,
  SegmentsPayload, ProfilePoint, PrinterProfile, PreviewData,
  FloorPatternKind, FastViewport,
} from "../../lib/tauri";
import {
  designerViewport, designerExportGcode, designerExportStl,
  designerRandomProfile, writeTextFile, readTextFile,
  saveProject, loadProject, saveFileDialog, openFileDialog, openSvgDialog,
  coreAvailable, printerPresets, designerPreviewData,
  savePrinterProfile, listPrinterProfiles, deletePrinterProfile,
} from "../../lib/tauri";
import { importProfileFromSvg } from "./svgImport";
import { GcodeCommandPalette } from "./GcodeCommandPalette";

// ============================================================================
// فهرست الگوها/بافت‌ها/کف‌ها — ۱۷ الگو + ۱۲ بافت + ۵ کف (نسخهٔ ۱.۲)
// ============================================================================

const PATTERNS: { kind: PatternParams["kind"]; name: string; hint: string }[] = [
  { kind: "Wave", name: "موج محوری", hint: "برجستگی سینوسی که با ارتفاع می‌چرخد" },
  { kind: "Petal", name: "گلبرگ", hint: "گلبرگ‌های نرم دور فرم" },
  { kind: "Twist", name: "پیچش", hint: "موج تاب‌خورده" },
  { kind: "Flute", name: "شیار عمودی", hint: "کانال‌های عمودی ستون کلاسیک" },
  { kind: "Diamond", name: "لوزی", hint: "شبکهٔ لوزی برجسته" },
  { kind: "Chevron", name: "مهری", hint: "زیگزاگ مورب چین‌دار" },
  { kind: "Rings", name: "حلقهٔ افقی", hint: "برجستگی‌های دور فرم مستقل از زاویه" },
  { kind: "Scale", name: "فلس", hint: "فلس‌های مورب هم‌پوشان" },
  { kind: "Facet", name: "تراش‌خورده", hint: "مقطع چندضلعی تراش‌خورده" },
  { kind: "Braid", name: "رشته‌ای (طناب)", hint: "تارهای درهم‌بافته طناب" },
  { kind: "Pineapple", name: "آناناسی", hint: "شبکهٔ لوزی تیز پوستهٔ آناناس" },
  { kind: "Lotus", name: "نیلوفر (آبشاری)", hint: "ردیف گلبرگ‌های هم‌پوشان" },
  { kind: "TwistedFlute", name: "شیار پیچان", hint: "شیارهایی که با ارتفاع می‌پیچند" },
  { kind: "Pearl", name: "مروارید", hint: "دانه‌های گرد درهم" },
  { kind: "Meander", name: "یونانی", hint: "بلوک‌های مربعی نقش یونانی" },
  { kind: "Brick", name: "آجری", hint: "ردیف‌های نیم‌جابه‌جا دیوار آجری" },
  { kind: "Spikes", name: "خاردار", hint: "خارهای مثلثی تیز" },
  // ---- نسخهٔ ۱٫۷ (درخواست کاربر ۵: الگوهای بیشتر) ----
  { kind: "Gothic", name: "گوتیک", hint: "قوس‌های تیزه‌دار عمودی کلیسایی" },
  { kind: "Moorish", name: "موریسکی", hint: "طاق‌های هلالی نعل‌اسبی اسلامی" },
  { kind: "Bamboo", name: "بامبو", hint: "گره‌های بامبو با گردن تیز" },
  { kind: "Colosseum", name: "کولوسئومی", hint: "نوارهای افقی مربعی آمفی‌تئاتر" },
  { kind: "RopeSpiral", name: "طناب مارپیچ", hint: "تار ضخیم پیچیده دور فرم" },
  { kind: "Checker", name: "شطرنجی", hint: "بلوک‌های متناوب ریز" },
  { kind: "Sunray", name: "پرتو خورشید", hint: "اشعه‌های عمودی پهن و نرم" },
  { kind: "Cartouche", name: "کارتوش", hint: "قرص‌های کشیده نقش ایرانی" },
];

const TEXTURES: { kind: TextureParams["kind"]; name: string; hint: string }[] = [
  { kind: "Ripple", name: "موجک (ریپل)", hint: "موج نرم روی طول مسیر" },
  { kind: "Weave", name: "تور بافته", hint: "بالا/پایین متناوب تار و پود" },
  { kind: "Stitch", name: "بخیه‌ای", hint: "پرش‌های گسسته مثل دوخت" },
  { kind: "SpiralGroove", name: "شیار مارپیچی", hint: "کانال مورب دور دیواره" },
  { kind: "Hammered", name: "چکشی", hint: "فرورفتگی‌های شبه‌تصادفی قطعی" },
  { kind: "Basket", name: "سبدی", hint: "شبکهٔ متقاطع دو جهته" },
  { kind: "WoodGrain", name: "رگهٔ چوب", hint: "الیاف کشیدهٔ چوب" },
  { kind: "Leather", name: "چرم", hint: "دانه‌های ریز پوست/چرم" },
  { kind: "Knurl", name: "آج رزوه", hint: "شیارهای متقاطع دستگیرهٔ آجدار" },
  { kind: "Ocean", name: "موج دریا", hint: "موج‌های روان بی‌تکرار" },
  { kind: "Bubbles", name: "حباب", hint: "فرورفتگی‌های گرد پراکنده" },
  { kind: "Rows", name: "تخته‌ای", hint: "شیارهای افقی ردیف تخته‌ها" },
  // ---- نسخهٔ ۱٫۷ (درخواست کاربر ۶: بافت‌های بیشتر) ----
  { kind: "Carbon", name: "کربنی (تویل)", hint: "بافت متقاطع ریز پنل کربن" },
  { kind: "Herringbone", name: "ست‌فقرات ماهی", hint: "ستون‌های زیگزاگ متناوب جهت" },
  { kind: "Cobblestone", name: "سنگ‌فرش", hint: "سنگ‌های گرد درهم" },
  { kind: "Crackle", name: "کراکله", hint: "شبکهٔ ترک‌های ریز سرامیکی" },
  { kind: "FishScale", name: "فلس ماهی", hint: "کاسه‌های هم‌پوشان با آفست" },
  { kind: "Sand", name: "شنی", hint: "نویز ریز ملایم سطح شن" },
  { kind: "Ridges", name: "شیار عمودی ریز", hint: "رگه‌های عمودی ظریف پارچه" },
  { kind: "Pores", name: "منافذ اسفنجی", hint: "گودی‌های ریز پراکنده متخلخل" },
];

const FLOORS: { kind: FloorPatternKind; name: string; hint: string }[] = [
  { kind: "Rings", name: "حلقه‌های هم‌مرکز", hint: "حلقه‌های تودرتو از بیرون به داخل" },
  { kind: "Spiral", name: "مارپیچ پیوسته", hint: "یک مسیر پیوسته بدون travel — مناسب vase" },
  { kind: "Grid", name: "شبکهٔ متقاطع", hint: "خطوط X و Y روی دیسک" },
  { kind: "Radial", name: "پرتوهای شعاعی", hint: "پرتو از سوراخ مرکزی تا لبه" },
  { kind: "ZigZag", name: "رفت‌وبرگشتی", hint: "ردیف‌های مارپیچی بدون درز" },
  // ---- نسخهٔ ۱٫۶ (درخواست کاربر ۷): ۵ الگوی جدید ----
  { kind: "Honeycomb", name: "شانه‌زنبوری", hint: "شبکهٔ شش‌گوش — استحکام بالا با فیلامنت کم" },
  { kind: "Wave", name: "موج دایره‌ای", hint: "مارپیچ با موج شعاعی — کف تزئینی" },
  { kind: "Star", name: "ستاره‌ای", hint: "ستارهٔ چندپر پیوسته — نمایشی" },
  { kind: "Snowflake", name: "دانه‌برفی", hint: "۶ شاخه با شاخه‌های فرعی ۴۵°" },
  { kind: "Octagram", name: "گره هندسی", hint: "دو مربع چرخیده + هشت‌ضلعی — نقش هندسی" },
  // ---- نسخهٔ ۱٫۷ (درخواست کاربر ۸: کف لامپ) ----
  { kind: "LampWeb", name: "وب لامپ", hint: "حلقه + پره = شبکهٔ نور عبوری" },
  { kind: "Sunburst", name: "خورشیدشکاف", hint: "پرتوهای پهن از سوکت تا لبه" },
];

// ============================================================================
// کتابخانهٔ پروفایل‌ها — ۱۸ شکل آمادهٔ قابل‌ویرایش (نسخهٔ ۱.۵)
// ۳ پریست اصلی + ۱۵ پریست جدید هستهٔ Rust (ProfilePreset)
// ============================================================================
const PROFILE_LIBRARY: {
  key: string;
  name: string;
  profile: ProfilePoint[];
}[] = [
  { key: "vase", name: fa.presetVase, profile: [
    { r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 },
    { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 }] },
  { key: "lamp", name: fa.presetLamp, profile: [
    { r: 8, h: 0 }, { r: 15, h: 15 }, { r: 35, h: 90 }, { r: 40, h: 100 }] },
  { key: "planter", name: fa.presetPlanter, profile: [
    { r: 15, h: 0 }, { r: 18, h: 5 }, { r: 30, h: 60 }, { r: 32, h: 65 }] },
  { key: "cup", name: fa.presetCup, profile: [
    { r: 20, h: 0 }, { r: 21, h: 8 }, { r: 24, h: 80 }, { r: 25, h: 90 }] },
  { key: "bottle", name: fa.presetBottle, profile: [
    { r: 18, h: 0 }, { r: 24, h: 15 }, { r: 25, h: 45 }, { r: 18, h: 75 },
    { r: 8, h: 90 }, { r: 8, h: 110 }, { r: 9.5, h: 115 }] },
  { key: "bowl", name: fa.presetBowl, profile: [
    { r: 12, h: 0 }, { r: 16, h: 4 }, { r: 24, h: 16 }, { r: 29, h: 34 }, { r: 31, h: 55 }] },
  { key: "hourglass", name: fa.presetHourglass, profile: [
    { r: 22, h: 0 }, { r: 20, h: 10 }, { r: 12, h: 35 }, { r: 7, h: 50 },
    { r: 12, h: 65 }, { r: 20, h: 90 }, { r: 22, h: 100 }] },
  { key: "goblet", name: fa.presetGoblet, profile: [
    { r: 16, h: 0 }, { r: 14, h: 5 }, { r: 5, h: 20 }, { r: 4.5, h: 55 },
    { r: 6, h: 62 }, { r: 16, h: 80 }, { r: 20, h: 95 }] },
  { key: "lantern", name: fa.presetLantern, profile: [
    { r: 14, h: 0 }, { r: 17, h: 10 }, { r: 18.5, h: 40 }, { r: 16.5, h: 55 },
    { r: 18, h: 75 }, { r: 15, h: 90 }, { r: 16.5, h: 100 }] },
  { key: "amphora", name: fa.presetAmphora, profile: [
    { r: 10, h: 0 }, { r: 15, h: 6 }, { r: 24, h: 30 }, { r: 25.5, h: 55 },
    { r: 20, h: 80 }, { r: 11, h: 95 }, { r: 12.5, h: 105 }] },
  { key: "pencil", name: fa.presetPencilHolder, profile: [
    { r: 19, h: 0 }, { r: 17.5, h: 5 }, { r: 19.5, h: 12 }, { r: 20, h: 95 }] },
  { key: "stacked", name: fa.presetStackedPlanter, profile: [
    { r: 14, h: 0 }, { r: 18, h: 6 }, { r: 17, h: 40 }, { r: 20, h: 45 },
    { r: 23, h: 48 }, { r: 22, h: 80 }, { r: 24, h: 85 }] },
  { key: "eggcup", name: fa.presetEggCup, profile: [
    { r: 12, h: 0 }, { r: 10, h: 4 }, { r: 5, h: 15 }, { r: 7, h: 22 },
    { r: 15, h: 30 }, { r: 16.5, h: 40 }] },
  { key: "barrel", name: fa.presetBarrel, profile: [
    { r: 18, h: 0 }, { r: 22, h: 15 }, { r: 24, h: 30 }, { r: 24, h: 45 },
    { r: 22, h: 60 }, { r: 18, h: 75 }] },
  { key: "cone", name: fa.presetCone, profile: [
    { r: 25, h: 0 }, { r: 24, h: 30 }, { r: 16, h: 70 }, { r: 5, h: 95 }] },
  { key: "sconce", name: fa.presetSconce, profile: [
    { r: 9, h: 0 }, { r: 13, h: 8 }, { r: 22, h: 70 }, { r: 24, h: 85 }] },
  { key: "bucket", name: fa.presetBucket, profile: [
    { r: 14, h: 0 }, { r: 15, h: 6 }, { r: 22, h: 70 }, { r: 23.5, h: 85 }] },
  { key: "celadon", name: fa.presetCeladon, profile: [
    { r: 13, h: 0 }, { r: 16, h: 8 }, { r: 23, h: 30 }, { r: 24, h: 48 },
    { r: 17, h: 72 }, { r: 10.5, h: 88 }, { r: 11, h: 98 }] },
  // ---- نسخهٔ ۱٫۷: ۱۴ گلدان جدید (درخواست کاربر ۴) ----
  { key: "meiping", name: fa.presetMeiping, profile: [
    { r: 14, h: 0 }, { r: 17, h: 6 }, { r: 22, h: 25 }, { r: 24.5, h: 55 },
    { r: 22, h: 75 }, { r: 15, h: 92 }, { r: 14.5, h: 100 }] },
  { key: "greek", name: fa.presetGreekUrn, profile: [
    { r: 16, h: 0 }, { r: 14, h: 5 }, { r: 22, h: 30 }, { r: 23, h: 55 },
    { r: 19, h: 75 }, { r: 20, h: 90 }, { r: 26, h: 100 }] },
  { key: "teardrop", name: fa.presetTeardrop, profile: [
    { r: 6, h: 0 }, { r: 12, h: 12 }, { r: 22, h: 45 }, { r: 24, h: 65 },
    { r: 20, h: 85 }, { r: 12, h: 97 }, { r: 8, h: 100 }] },
  { key: "artdeco", name: fa.presetArtDeco, profile: [
    { r: 18, h: 0 }, { r: 18, h: 20 }, { r: 20.5, h: 20.5 }, { r: 20.5, h: 45 },
    { r: 23.5, h: 45.5 }, { r: 23.5, h: 70 }, { r: 26, h: 70.5 },
    { r: 26, h: 92 }, { r: 27.5, h: 92.5 }, { r: 27.5, h: 100 }] },
  { key: "mushroom", name: fa.presetMushroom, profile: [
    { r: 12, h: 0 }, { r: 9, h: 25 }, { r: 8, h: 50 }, { r: 10, h: 62 },
    { r: 22, h: 78 }, { r: 26, h: 92 }, { r: 24, h: 100 }] },
  { key: "tulip", name: fa.presetTulip, profile: [
    { r: 10, h: 0 }, { r: 14, h: 15 }, { r: 21, h: 50 }, { r: 24, h: 75 },
    { r: 22, h: 92 }, { r: 17, h: 100 }] },
  { key: "tower", name: fa.presetTower, profile: [
    { r: 17, h: 0 }, { r: 18.5, h: 40 }, { r: 19.5, h: 80 }, { r: 21, h: 95 },
    { r: 20, h: 100 }] },
  { key: "waverim", name: fa.presetWaveRim, profile: [
    { r: 16, h: 0 }, { r: 20, h: 30 }, { r: 23, h: 65 }, { r: 22.5, h: 80 },
    { r: 25.5, h: 95 }, { r: 24, h: 100 }] },
  { key: "honeyjar", name: fa.presetHoneyJar, profile: [
    { r: 17, h: 0 }, { r: 21, h: 10 }, { r: 24, h: 38 }, { r: 23, h: 62 },
    { r: 25, h: 82 }, { r: 24, h: 88 }] },
  { key: "doublebulge", name: fa.presetDoubleBulge, profile: [
    { r: 12, h: 0 }, { r: 20, h: 22 }, { r: 15, h: 45 }, { r: 22, h: 70 },
    { r: 20, h: 92 }, { r: 21.5, h: 100 }] },
  { key: "lowbowl", name: fa.presetLowBowl, profile: [
    { r: 10, h: 0 }, { r: 14, h: 4 }, { r: 26, h: 28 }, { r: 31, h: 48 },
    { r: 32, h: 55 }] },
  { key: "grand", name: fa.presetGrandVase, profile: [
    { r: 20, h: 0 }, { r: 26, h: 12 }, { r: 33, h: 42 }, { r: 30, h: 70 },
    { r: 20, h: 92 }, { r: 19, h: 118 }, { r: 23, h: 128 }] },
  { key: "geometric", name: fa.presetGeometric, profile: [
    { r: 15, h: 0 }, { r: 19, h: 18 }, { r: 25, h: 45 }, { r: 24, h: 70 },
    { r: 18, h: 90 }, { r: 22, h: 100 }] },
  { key: "candle", name: fa.presetTallCandle, profile: [
    { r: 11, h: 0 }, { r: 12, h: 30 }, { r: 13.5, h: 75 }, { r: 14, h: 105 },
    { r: 13, h: 110 }] },
  { key: "rippledcol", name: fa.presetRippledColumn, profile: [
    { r: 15, h: 0 }, { r: 18, h: 15 }, { r: 16.5, h: 30 }, { r: 18.5, h: 45 },
    { r: 17, h: 60 }, { r: 18.5, h: 75 }, { r: 17, h: 90 }, { r: 19, h: 100 }] },
];

/** تخمین شمار مثلث STL — آینهٔ TS از build_mesh هسته (برای دیالوگ کیفیت) */
export function estimateTriangles(
  samples: number[][] | undefined,
  height: number,
  layerH: number,
  quality: number
): number {
  const q = Math.max(0.25, Math.min(4, quality));
  const rMax = samples && samples.length > 0
    ? Math.max(...samples.map((s) => s[1]), 5)
    : 25;
  const rowsBase = Math.min(Math.max(Math.ceil(height / Math.max(layerH, 0.05)), 8), 400) * 2;
  const steps = Math.min(Math.max(Math.ceil((2 * Math.PI * rMax) / 2), 24), 720);
  const cols = Math.min(Math.max(Math.round(steps * q), 24), 1080);
  const rows = Math.min(Math.max(Math.round(rowsBase * q), 8), 1600);
  return 2 * rows * cols;
}


// ============================================================================
// ۶ استایل آماده — ترکیب پروفایل + الگو + بافت (کاملاً قابل‌ویرایش)
// ============================================================================

interface StylePreset {
  key: string;
  name: string;
  desc: string;
  profile: ProfilePoint[];
  height: number;
  bottomD: number;
  topD: number;
  patterns: PatternParams[];
  textures: TextureParams[];
  floorPattern: FloorPatternKind;
  floorHole: number;
}

const STYLE_PRESETS: StylePreset[] = [
  {
    key: "column",
    name: "ستون کلاسیک",
    desc: "شیارهای عمودی ۱۲تایی + کف حلقه‌ای",
    profile: [{ r: 18, h: 0 }, { r: 19, h: 50 }, { r: 17, h: 95 }, { r: 21, h: 100 }],
    height: 100, bottomD: 38, topD: 42,
    patterns: [{ kind: "Flute", amplitude_mm: 1.8, count: 12, frequency: 0, phase_deg: 0, z_start_mm: 5, z_end_mm: 90 }],
    textures: [],
    floorPattern: "Rings", floorHole: 0,
  },
  {
    key: "pineapple",
    name: "آناناس",
    desc: "شبکهٔ لوزی آناناسی + کف مارپیچ",
    profile: [{ r: 20, h: 0 }, { r: 24, h: 30 }, { r: 22, h: 80 }, { r: 24, h: 100 }],
    height: 100, bottomD: 42, topD: 46,
    patterns: [{ kind: "Pineapple", amplitude_mm: 2.2, count: 10, frequency: 3, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Spiral", floorHole: 8,
  },
  {
    key: "rope",
    name: "طناب بافته",
    desc: "رشته‌های درهم‌بافته + کف شبکه",
    profile: [{ r: 16, h: 0 }, { r: 16, h: 100 }],
    height: 90, bottomD: 34, topD: 34,
    patterns: [{ kind: "Braid", amplitude_mm: 2.5, count: 6, frequency: 2, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Grid", floorHole: 0,
  },
  {
    key: "lotus",
    name: "نیلوفر آبی",
    desc: "ردیف گلبرگ‌های آبشاری",
    profile: [{ r: 12, h: 0 }, { r: 20, h: 20 }, { r: 28, h: 60 }, { r: 26, h: 100 }],
    height: 100, bottomD: 26, topD: 52,
    patterns: [{ kind: "Lotus", amplitude_mm: 2.0, count: 10, frequency: 2, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [],
    floorPattern: "Radial", floorHole: 10,
  },
  {
    key: "hammered",
    name: "مس چکشی",
    desc: "فرورفتگی‌های چکشی قطعی روی فرم ساده",
    profile: [{ r: 22, h: 0 }, { r: 26, h: 40 }, { r: 20, h: 100 }],
    height: 95, bottomD: 46, topD: 40,
    patterns: [],
    textures: [{ kind: "Hammered", amplitude_mm: 0.5, period_mm: 3.5, radial: true, phase_deg: 0, fade_in_percent: 5, fade_out_percent: 5 }],
    floorPattern: "Rings", floorHole: 0,
  },
  {
    key: "wood",
    name: "چوب کهنه",
    desc: "رگه‌های چوب + پیچش ملایم",
    profile: [{ r: 18, h: 0 }, { r: 21, h: 60 }, { r: 17, h: 100 }],
    height: 110, bottomD: 38, topD: 34,
    patterns: [{ kind: "Twist", amplitude_mm: 1.2, count: 5, frequency: 1, phase_deg: 0, z_start_mm: 0, z_end_mm: null }],
    textures: [{ kind: "WoodGrain", amplitude_mm: 0.45, period_mm: 5, radial: true, phase_deg: 0, fade_in_percent: 0, fade_out_percent: 0 }],
    floorPattern: "ZigZag", floorHole: 0,
  },
];

/** پروژهٔ پیش‌فرض — گلدان آماده (قابل‌ویرایش) با تنظیمات ۱.۲ */
function defaultProject(): DesignerProject {
  return {
    schema_version: 1,
    profile: [
      { r: 22, h: 0 }, { r: 28, h: 10 }, { r: 30, h: 35 },
      { r: 26, h: 60 }, { r: 18, h: 80 }, { r: 17, h: 88 }, { r: 21, h: 100 },
    ],
    form_height_mm: 100,
    form_bottom_diameter_mm: 44,
    form_top_diameter_mm: 42,
    taper_percent: 0,
    profile_interpolation: "Spline",
    center_on_bed: true,
    phase_start_deg: 0,
    patterns: [],
    textures: [],
    wall: {
      mode: "Solid",
      lattice: {
        style: "Diamond", cell_width_mm: 12, cell_height_mm: 10,
        rise_ratio: 0.5, speed_scale: 0.6, flow_scale: 1.1,
        phase_deg: 0, z_start_mm: 8, z_end_mm: null,
      },
      holes: {
        enabled: false, shape: "Circle", count_around: 4, rows: 3,
        width_mm: 14, height_mm: 18, row_pitch_mm: 0,
        margin_bottom_mm: 10, margin_top_mm: 10, frame_passes: 2,
        phase_deg: 0, stagger: true,
      },
    },
    pendant: {
      style: "None", rim_holes_count: 3, rim_hole_diameter_mm: 3,
      rim_hole_radius_percent: 85, rim_phase_deg: 0,
      top_holes_count: 3, top_hole_diameter_mm: 5, top_hole_from_rim_mm: 8,
      rim_reinforce_rings: 3, top_hole_frames: true,
    },
    lamp: {
      enabled: false, center_hole_diameter_mm: 16, socket_rings: 3,
      spokes_count: 4, spokes_width_mm: 2.4, spokes_style: "Parallel",
      floor_gap_multiplier: 1.6, recessed_socket: true,
    },
    floor: {
      enabled: false, pattern: "Rings", center_hole_diameter_mm: 0,
      z_start_mm: 0, ring_gap_mm: 0.45, spokes: 12, layers: 2,
      // ---- نسخهٔ ۱٫۶: تنظیمات حرفه‌ای لامپ/آویز ----
      rim_holes: 0, rim_hole_diameter_mm: 3, reinforce_rings: 0,
    },
    printer: {
      name: "Custom / Unverified", status: "unverified",
      build_volume_mm: [220, 220, 250],
      nozzle_diameter_mm: 0.4, filament_diameter_mm: 1.75,
      line_width_mm: 0.45, layer_height_mm: 0.25,
      print_speed_mm_min: 2400, travel_speed_mm_min: 6000,
      flow_multiplier: 1.0, absolute_e: true,
      // ---- نسخهٔ ۱.۳: دما/فن (پیش‌فرض PLA عمومی) ----
      nozzle_temp_c: 200, bed_temp_c: 60, fan_percent: 100, wait_for_temps: true,
      // ---- نسخهٔ ۱.۴: سرعت‌ها به mm/s + رتراکشن ----
      wall_speed_mm_s: 40, first_layer_speed_mm_s: 20, floor_speed_mm_s: 30,
      travel_speed_mm_s: 100, retract_mm: 0.8, retract_speed_mm_s: 40,
      start_gcode: "", end_gcode: "",
    },
    created_at_unix: 0,
    note: "",
  };
}

/** تبدیل پیلود سریع به payload viewport — ۱.۴: عبور مستقیم!
 *  هسته Rust همین آرایه‌های تخت را می‌فرستد؛ هیچ تبدیل O(n) در JS
 *  انجام نمی‌شود (قبلاً هر segment شیء JSON بود و دوباره کپی می‌شد). */
function toSegmentsPayload(out: FastViewport): SegmentsPayload {
  return {
    revision: 1,
    positions: out.positions,
    kinds: out.kinds,
    source_lines: [],
    ids: [],
    certainties: [],
    feedrates: out.feedrates,
  };
}

/** شعاع واقعی فرم در ارتفاع z — از radius_samples هسته (۱.۴).
 *  جست‌وجوی دودویی روی ۱۲۹ نمونهٔ (z, r) — بدون اسکن کل mesh.
 *  صادقانه: خالص و قابل‌آزمون (Speeds.test.ts) */
export function radiusAt(samples: number[][] | undefined, z: number): number {
  if (!samples || samples.length === 0) return 20;
  let lo = 0, hi = samples.length - 1;
  if (z <= samples[0][0]) return Math.max(samples[0][1], 2);
  if (z >= samples[hi][0]) return Math.max(samples[hi][1], 2);
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (samples[mid][0] <= z) lo = mid; else hi = mid;
  }
  // درون‌یابی خطی بین دو نمونه
  const z0 = samples[lo][0], r0 = samples[lo][1];
  const z1 = samples[hi][0], r1 = samples[hi][1];
  const t = z1 > z0 ? (z - z0) / (z1 - z0) : 0;
  return Math.max(r0 + (r1 - r0) * t, 2);
}

/** انتخاب اسپات (برای درگ روی 3D) */
type SpotSelection =
  | { type: "pattern"; index: number }
  | { type: "texture"; index: number }
  | { type: "floor" }
  | { type: "phase" }
  | null;

/** پریست‌های ماده — «شروع مناسب رایج» نه عدد قطعی (قاعدهٔ صداقت) */
const MATERIALS: { name: string; nozzle: number; bed: number; fan: number }[] = [
  { name: "PLA — 200/60", nozzle: 200, bed: 60, fan: 100 },
  { name: "PETG — 240/80", nozzle: 240, bed: 80, fan: 30 },
  { name: "ABS — 250/100", nozzle: 250, bed: 100, fan: 0 },
  { name: "TPU — 225/45", nozzle: 225, bed: 45, fan: 50 },
  { name: "PLA ابریشمی — 215/65", nozzle: 215, bed: 65, fan: 100 },
];

export function SpiralDesignerPage(props: {
  onSendToEditor: (gcode: string) => void;
  /** رنگ رشتهٔ چاپ در پیش‌نمایش (نمایشی — از ColorBar) */
  extrusionColor?: string;
}) {
  const [project, setProject] = useState<DesignerProject>(defaultProject);
  // ۱.۴: خروجی = پیلود تخت سریع (بدون mesh) — رفع لگ/وقفهٔ نمایشگر
  const [output, setOutput] = useState<FastViewport | null>(null);
  const [step, setStep] = useState(1);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");
  // پریست‌های ۲۰ پرینتر واقعی — از هستهٔ Rust بارگذاری می‌شود
  const [presets, setPresets] = useState<PrinterProfile[]>([]);
  // دادهٔ پیش‌نمایش کارت‌ها (۱۷ الگو + ۱۲ بافت) از ریاضی واقعی هسته
  const [previewData, setPreviewData] = useState<PreviewData | null>(null);
  // پرسش تأیید خروجی‌های دارای هشدار بحرانی (الزام §8)
  const [confirmExport, setConfirmExport] = useState<null | (() => void)>(null);
  // اسپات انتخاب‌شده برای درگ روی 3D
  const [spotSel, setSpotSel] = useState<SpotSelection>(null);
  const debounceRef = useRef<number | null>(null);
  // ---- نسخهٔ ۱.۵ ----
  // دیالوگ کیفیت STL: null = بسته
  const [stlDialog, setStlDialog] = useState<null | { quality: number }>(null);
  // دیالوگ کیفیت G-code — نسخهٔ ۱٫۶ (درخواست کاربر ۶): null = بسته
  const [gcodeDialog, setGcodeDialog] = useState<null | { quality: number; save: boolean }>(null);
  // در حال رندوم/ایمپورت SVG (برای دکمه‌ها)
  const [profileBusy, setProfileBusy] = useState(false);
  // ---- پروفایل‌های شخصی پرینتر — نسخهٔ ۱٫۶ (درخواست کاربر ۴) ----
  const [myProfiles, setMyProfiles] = useState<PrinterProfile[]>([]);
  const [profileName, setProfileName] = useState("");
  const [profileBusy2, setProfileBusy2] = useState(false);

  // ---------------- واگرد/ازنو (نسخهٔ ۱.۲) ----------------
  const historyRef = useRef<DesignerProject[]>([]);
  const futureRef = useRef<DesignerProject[]>([]);
  const [histLen, setHistLen] = useState(0);
  const [futureLen, setFutureLen] = useState(0);
  // درگ اسپات: یک قدم تاریخ در هر ژست (نه هر پیکسل)
  const spotGestureRef = useRef<number | null>(null);
  // نسخهٔ همیشه‌به‌روز پروژه برای جهش‌های تاریخ خارج از updater
  // (StrictMode updaterها را دوبار اجرا می‌کند — جهش ref داخل updater ممنوع)
  const projectRef = useRef(project);
  projectRef.current = project;

  /** ثبت تغییر پروژه با قدم تاریخ */
  const commitProject = useCallback((next: DesignerProject) => {
    historyRef.current.push(projectRef.current);
    if (historyRef.current.length > 80) historyRef.current.shift();
    futureRef.current = [];
    setProject(next);
    setHistLen(historyRef.current.length);
    setFutureLen(0);
  }, []);

  const undo = useCallback(() => {
    const prev = historyRef.current.pop();
    if (!prev) return;
    futureRef.current.push(projectRef.current);
    setProject(prev);
    setHistLen(historyRef.current.length);
    setFutureLen(futureRef.current.length);
  }, []);

  const redo = useCallback(() => {
    const next = futureRef.current.pop();
    if (!next) return;
    historyRef.current.push(projectRef.current);
    setProject(next);
    setHistLen(historyRef.current.length);
    setFutureLen(futureRef.current.length);
  }, []);

  // میان‌بر کیبورد واگرد/ازنو (فقط در صفحهٔ طراح)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        undo();
      } else if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === "y" || (e.shiftKey && e.key.toLowerCase() === "z"))) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo, redo]);

  // بارگذاری پریست‌های پرینتر و دادهٔ پیش‌نمایش از هسته
  useEffect(() => {
    if (!coreAvailable()) return;
    printerPresets()
      .then((list) => setPresets(list))
      .catch(() => setPresets([]));
    designerPreviewData()
      .then((d) => setPreviewData(d))
      .catch(() => setPreviewData(null)); // بدون هسته → کارت‌های جایگزین صادقانه
  }, []);

  // بارگذاری پروفایل‌های شخصی — نسخهٔ ۱٫۶ (فقط پوستهٔ دسکتاپ)
  const refreshMyProfiles = useCallback(() => {
    listPrinterProfiles()
      .then((list) => setMyProfiles(list))
      .catch(() => setMyProfiles([])); // در مرورگر dev: بدون پروفایل شخصی
  }, []);
  useEffect(() => {
    refreshMyProfiles();
  }, [refreshMyProfiles]);

  /** ذخیرهٔ پروفایل جاری به‌عنوان پروفایل شخصی (درخواست کاربر ۴) */
  const doSavePrinterProfile = async () => {
    setProfileBusy2(true);
    try {
      const name = profileName.trim() || project.printer.name;
      await savePrinterProfile({ ...project.printer, name });
      setProfileName("");
      refreshMyProfiles();
      setNote(`پروفایل «${name}» ذخیره شد — در فهرست «پروفایل‌های من» قابل انتخاب است.`);
    } catch (e) {
      setNote(`ذخیرهٔ پروفایل ناموفق: ${e}`);
    }
    setProfileBusy2(false);
  };

  /** حذف پروفایل شخصی */
  const doDeleteMyProfile = async (name: string) => {
    try {
      await deletePrinterProfile(name);
      refreshMyProfiles();
      setNote(`پروفایل «${name}» حذف شد.`);
    } catch (e) {
      setNote(`حذف ناموفق: ${e}`);
    }
  };

  // ------------------------------------------------------------ محاسبهٔ زنده
  const recompute = useCallback(async (p: DesignerProject) => {
    if (!coreAvailable()) {
      setNote("محاسبه فقط در برنامهٔ دسکتاپ انجام می‌شود.");
      return;
    }
    try {
      // ۱.۴: پیلود تخت سریع — به‌جای JSON شیءمحور کامل با mesh
      const out = await designerViewport(p);
      setOutput(out);
      setNote(out.summary_fa);
    } catch (e) {
      setNote(`خطا: ${e}`);
    }
  }, []);

  useEffect(() => {
    // debounce پارامترها (الزام §6: «هر تغییر، نما و IR را همگام به‌روز کند»)
    if (debounceRef.current !== null) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => recompute(project), 300);
    return () => { if (debounceRef.current !== null) clearTimeout(debounceRef.current); };
  }, [project, recompute]);

  // محاسبهٔ اولیه
  useEffect(() => { recompute(project); /* eslint-disable-next-line */ }, []);

  // ------------------------------------------------------------ helpers
  const setP = (patch: Partial<DesignerProject>) => commitProject({ ...project, ...patch });
  const setPrinter = (patch: Partial<DesignerProject["printer"]>) =>
    commitProject({ ...project, printer: { ...project.printer, ...patch } });
  const setFloor = (patch: Partial<DesignerProject["floor"]>) =>
    commitProject({ ...project, floor: { ...project.floor, ...patch } });
  // ---- نسخهٔ ۱٫۷: به‌روزرسانی دیواره/آویز/لامپ ----
  const wallOf = () => project.wall ?? {};
  const setWall = (patch: Partial<NonNullable<DesignerProject["wall"]>>) =>
    commitProject({ ...project, wall: { ...wallOf(), ...patch } });
  const setLattice = (patch: Partial<NonNullable<DesignerProject["wall"]>["lattice"]>) =>
    commitProject({
      ...project,
      wall: { ...wallOf(), lattice: { ...(wallOf().lattice ?? {}), ...patch } },
    });
  const setHoles = (patch: Partial<NonNullable<DesignerProject["wall"]>["holes"]>) =>
    commitProject({
      ...project,
      wall: { ...wallOf(), holes: { ...(wallOf().holes ?? {}), ...patch } },
    });
  const pendantOf = () => project.pendant ?? {};
  const setPendant = (patch: Partial<NonNullable<DesignerProject["pendant"]>>) =>
    commitProject({ ...project, pendant: { ...pendantOf(), ...patch } });
  const lampOf = () => project.lamp ?? {};
  const setLamp = (patch: Partial<NonNullable<DesignerProject["lamp"]>>) =>
    commitProject({ ...project, lamp: { ...lampOf(), ...patch } });

  const addPattern = (kind: PatternParams["kind"]) =>
    commitProject({
      ...project,
      patterns: [...project.patterns, {
        kind, amplitude_mm: 1.5, count: 8, frequency: 1,
        phase_deg: 0, z_start_mm: 0, z_end_mm: null,
      }],
    });
  const updatePattern = (i: number, patch: Partial<PatternParams>) =>
    commitProject({
      ...project,
      patterns: project.patterns.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    });
  const removePattern = (i: number) =>
    commitProject({ ...project, patterns: project.patterns.filter((_, j) => j !== i) });

  const addTexture = (kind: TextureParams["kind"]) =>
    commitProject({
      ...project,
      textures: [...project.textures, {
        kind, amplitude_mm: 0.4, period_mm: 4, radial: true,
        phase_deg: 0, fade_in_percent: 0, fade_out_percent: 0,
      }],
    });
  const updateTexture = (i: number, patch: Partial<TextureParams>) =>
    commitProject({
      ...project,
      textures: project.textures.map((x, j) => (j === i ? { ...x, ...patch } : x)),
    });
  const removeTexture = (i: number) =>
    commitProject({ ...project, textures: project.textures.filter((_, j) => j !== i) });

  /** اعمال استایل آماده — کاملاً قابل‌ویرایش بعد از اعمال */
  const applyStyle = (sp: StylePreset) =>
    commitProject({
      ...project,
      profile: sp.profile.map((p) => ({ ...p })),
      form_height_mm: sp.height,
      form_bottom_diameter_mm: sp.bottomD,
      form_top_diameter_mm: sp.topD,
      patterns: sp.patterns.map((p) => ({ ...p })),
      textures: sp.textures.map((t) => ({ ...t })),
      floor: {
        ...project.floor,
        enabled: true,
        pattern: sp.floorPattern,
        center_hole_diameter_mm: sp.floorHole,
      },
    });

  /** پروفایل تصادفی چاپ‌پسند — نسخهٔ ۱.۵ (هستهٔ Rust: قواعد چاپ خوب) */
  const doRandomProfile = async () => {
    if (!coreAvailable()) {
      setNote("تولید پروفایل تصادفی فقط در برنامهٔ دسکتاپ/سرور توسعه کار می‌کند.");
      return;
    }
    setProfileBusy(true);
    try {
      const seed = Date.now() % 1_000_000;
      const pts = await designerRandomProfile(seed);
      setP({ profile: pts });
      setNote(`${fa.randomProfile} (seed=${seed}) — ${fa.randomProfileHint}`);
    } catch (e) {
      setNote(`خطا در پروفایل تصادفی: ${e}`);
    }
    setProfileBusy(false);
  };

  /** ایمپورت پروفایل از SVG — نسخهٔ ۱.۵ */
  const doImportSvg = async () => {
    const path = await openSvgDialog();
    if (!path) return;
    setProfileBusy(true);
    try {
      const text = await readTextFile(path);
      const res = importProfileFromSvg(text, project.form_height_mm, 45);
      setP({ profile: res.points });
      setNote(
        `${fa.importSvgDone}: ${res.points.length} نقطه از ${res.path_count} مسیر ` +
        `(ارتفاع خام ${res.raw_height.toFixed(0)} واحد SVG → ${project.form_height_mm}mm)`
      );
    } catch (e) {
      setNote(`ایمپورت SVG ناموفق: ${e}`);
    }
    setProfileBusy(false);
  };

  /** ذخیرهٔ مستقیم G-code — ۱.۶: ابتدا دیالوگ کیفیت (درخواست کاربر ۶) */
  const doSaveDirectGcode = async () => {
    setGcodeDialog({ quality: 1, save: true });
  };

  /** تخمین شمار خط خروجی G-code در کیفیت q — آینهٔ هسته (arc≈۲/q mm) */
  function estimateGcodeLines(currentCount: number, quality: number): number {
    const q = Math.max(0.25, Math.min(4, quality));
    // شمار خط ≈ خطوط فعلی (کیفیت ۱) × نسبت تراکم؛ +۵٪ سربار ثابت
    return Math.round(currentCount * q * 1.05 + 30);
  }

  /** اجرای خروجی G-code با کیفیت — نسخهٔ ۱.۶ (دیالوگ کیفیت) */
  const runGcodeExport = async (quality: number, directSave: boolean) => {
    setGcodeDialog(null);
    setBusy(true);
    try {
      const gcode = await designerExportGcode(project, quality);
      if (directSave) {
        const path = await saveFileDialog("spiral-form.gcode", "gcode");
        if (path) {
          await writeTextFile(path, gcode);
          setNote(
            `G-code (کیفیت ${quality}) ذخیره شد: ${path} ` +
            `(${gcode.length.toLocaleString("fa-IR")} بایت)`
          );
        } else {
          setNote("ذخیره لغو شد.");
        }
      } else {
        props.onSendToEditor(gcode);
        setNote("G-code تولید و به ویرایشگر ارسال شد.");
      }
    } catch (e) {
      setNote(`خطا در خروجی: ${e}`);
    }
    setBusy(false);
  };

  /** خروجی STL با کیفیت انتخابی — نسخهٔ ۱.۵ (دیالوگ کیفیت) */
  const exportStlWithQuality = async (quality: number) => {
    const path = await saveFileDialog("spiral-form.stl", "stl");
    if (!path) {
      setStlDialog(null);
      return;
    }
    const hasCritical = output?.diagnostics.some(
      (d) => d.severity === "Error" || d.code.startsWith("W-DESIGN")
    );
    const doExport = async () => {
      setConfirmExport(null);
      setStlDialog(null);
      setBusy(true);
      try {
        const r = await designerExportStl(project, path, quality);
        setNote(`STL ذخیره شد: ${r.path} (${r.triangle_count.toLocaleString("fa-IR")} مثلث)`);
      } catch (e) {
        setNote(`خطا: ${e}`);
      }
      setBusy(false);
    };
    if (hasCritical) {
      setConfirmExport(() => doExport);
    } else {
      await doExport();
    }
  };

  const patternName = useMemo(() => {
    const m: Record<string, string> = {};
    PATTERNS.forEach((p) => (m[p.kind] = p.name));
    return m;
  }, []);
  const textureName = useMemo(() => {
    const m: Record<string, string> = {};
    TEXTURES.forEach((t) => (m[t.kind] = t.name));
    return m;
  }, []);

  /** نازل‌های رایج (پیشنهادی نه دادهٔ ثابت — از پژوهش مرجع) */
  const nozzlePresets = [0.4, 0.6, 0.8];

  const maxR = Math.max(...project.profile.map((p) => p.r), 10) * 1.2;

  // ------------------------------------------------------------ خروجی‌ها
  /** ارسال به ویرایشگر — ۱.۶: ابتدا دیالوگ کیفیت (درخواست کاربر ۶) */
  const exportGcode = async () => {
    setGcodeDialog({ quality: 1, save: false });
  };

  const doSaveProject = async () => {
    const path = await saveFileDialog("project.gcsproj.json", "gcsproj.json");
    if (!path) return;
    try {
      await saveProject({ ...project, created_at_unix: Math.floor(Date.now() / 1000) }, path);
      setNote(`پروژه ذخیره شد: ${path}`);
    } catch (e) {
      setNote(`خطا: ${e}`);
    }
  };

  const doLoadProject = async () => {
    const path = await openFileDialog();
    if (!path) return;
    try {
      const p = await loadProject(path);
      // ادغام با پیش‌فرض‌ها: پروژه‌های ۱.۲/۱.۱ فاز و دما ندارند →
      // مقادیر پیش‌فرض ۱.۳ برایشان می‌نشیند (بدون از دست رفتن داده)
      commitProject({
        ...defaultProject(),
        ...p,
        printer: { ...defaultProject().printer, ...p.printer },
        floor: { ...defaultProject().floor, ...p.floor },
      });
      setNote("پروژه بارگذاری شد.");
    } catch (e) {
      // schema خراب → پروژهٔ جاری دست‌نخورده می‌ماند (الزام §7)
      setNote(`بارگذاری ناموفق: ${e} — پروژهٔ جاری تغییر نکرد.`);
    }
  };

  const segPayload = useMemo(
    () => (output ? toSegmentsPayload(output) : null),
    [output]
  );
  const warningsFa = output?.diagnostics.map((d) => `${d.severity === "Error" ? "خطا" : "هشدار"} [${d.code}]: ${d.message_fa}`) ?? [];

  // ---------------- اسپات: اتصال انتخاب به هندل‌های 3D ----------------
  // (useMemo — آرایهٔ پایدار تا حلقه‌ها در هر رندر بازسازی نشوند)
  const bedOffset = useMemo<[number, number]>(() =>
    project.center_on_bed !== false
      ? [project.printer.build_volume_mm[0] / 2, project.printer.build_volume_mm[1] / 2]
      : [0, 0],
    [project.center_on_bed, project.printer.build_volume_mm]
  );

  const spot: SpotHandle | null = useMemo(() => {
    if (!spotSel) return null;
    const height = Math.max(project.form_height_mm, 1);
    // ۱.۴: شعاع از radius_samples هسته (جست‌وجوی دودویی — بدون mesh)
    const samples = output?.radius_samples;
    if (spotSel.type === "phase") {
      // ---- دیال چرخش فاز θ (نسخهٔ ۱.۳) ----
      // دیال در ارتفاع کم (~۸٪ ارتفاع، بین ۳ تا ۱۲mm) روی شعاع واقعی فرم
      const zDial = Math.min(Math.max(height * 0.08, 3), 12);
      const r = radiusAt(samples, zDial);
      return {
        kind: "phaseRotation",
        zStart: zDial,
        zEnd: null,
        radius: Math.max(r, 8),
        height,
        bedOffset,
        phaseDeg: project.phase_start_deg ?? 0,
      };
    }
    if (spotSel.type === "pattern") {
      const p = project.patterns[spotSel.index];
      if (!p) return null;
      const r = radiusAt(samples, p.z_start_mm);
      return {
        kind: "patternRange",
        zStart: p.z_start_mm,
        zEnd: p.z_end_mm,
        radius: Math.max(r, 6),
        height,
        bedOffset,
        // ۱.۴: فاز «خود الگو» — دسته روی سطح در همین زاویه؛ درگ افقی = چرخش
        phaseDeg: p.phase_deg ?? 0,
        // ۱٫۶: پنجرهٔ اسپات زاویه‌ای (درخواست کاربر ۵) — span<۳۶۰ فعال
        spot:
          (p.spot_span_deg ?? 360) < 359
            ? { centerDeg: p.spot_center_deg ?? 0, spanDeg: p.spot_span_deg ?? 360 }
            : undefined,
      };
    }
    if (spotSel.type === "texture") {
      const t = project.textures[spotSel.index];
      if (!t) return null;
      const zStart = (t.fade_in_percent / 100) * height;
      const zEnd = height - (t.fade_out_percent / 100) * height;
      const r = radiusAt(samples, zStart + 2);
      return {
        kind: "textureFade",
        zStart: Math.max(zStart, 0.5),
        zEnd: zEnd > zStart + 1 ? zEnd : null,
        radius: Math.max(r, 6),
        height,
        bedOffset,
        phaseDeg: t.phase_deg ?? 0,
        // ۱٫۶: پنجرهٔ اسپات زاویه‌ای (درخواست کاربر ۵)
        spot:
          (t.spot_span_deg ?? 360) < 359
            ? { centerDeg: t.spot_center_deg ?? 0, spanDeg: t.spot_span_deg ?? 360 }
            : undefined,
      };
    }
    // کف — سوراخ مرکزی
    if (!project.floor.enabled || project.floor.center_hole_diameter_mm < 1) return null;
    const rOuter = radiusAt(samples, 0.5);
    return {
      kind: "floorHole",
      zStart: 0.5,
      zEnd: null,
      radius: Math.min(project.floor.center_hole_diameter_mm / 2, rOuter - 1),
      height,
      bedOffset,
    };
  }, [spotSel, project, output, bedOffset]);

  /** تغییر اسپات با درگ — نگاشت به فیلدهای پروژه + قدم تاریخ در هر ژست
   *  (جهش ref خارج از updater — سازگار با StrictMode) */
  const onSpotChange = useCallback((change: SpotChange) => {
    const cur = projectRef.current;
    // یک قدم تاریخ در آغاز ژست (۱۵۰۰ms بی‌حرکتی = پایان ژست)
    if (spotGestureRef.current === null) {
      historyRef.current.push(cur);
      if (historyRef.current.length > 80) historyRef.current.shift();
      futureRef.current = [];
      setHistLen(historyRef.current.length);
      setFutureLen(0);
    } else {
      clearTimeout(spotGestureRef.current);
    }
    spotGestureRef.current = window.setTimeout(() => {
      spotGestureRef.current = null;
    }, 1500);

    const height = Math.max(cur.form_height_mm, 1);
    let next: DesignerProject | null = null;
    if (change.phaseDeg !== undefined && spotSel?.type === "pattern") {
      // ---- ۱.۴: چرخش فاز «خودِ الگو» با درگ افقی دستهٔ سطحی ----
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index ? { ...x, phase_deg: change.phaseDeg! } : x
      );
      next = { ...cur, patterns };
    } else if (change.phaseDeg !== undefined && spotSel?.type === "texture") {
      // ---- ۱.۴: چرخش فاز بافت با درگ افقی دستهٔ سطحی ----
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index ? { ...x, phase_deg: change.phaseDeg! } : x
      );
      next = { ...cur, textures };
    } else if (change.phaseDeg !== undefined) {
      // ---- چرخش فاز کل فرم با دیال θ (نسخهٔ ۱.۳) ----
      next = { ...cur, phase_start_deg: change.phaseDeg };
    } else if (spotSel?.type === "pattern" && change.zStart !== undefined) {
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index ? {
          ...x,
          z_start_mm: change.zStart!,
          z_end_mm: x.z_end_mm !== null ? Math.max(x.z_end_mm, change.zStart! + 1) : null,
        } : x
      );
      next = { ...cur, patterns };
    } else if (spotSel?.type === "pattern" && change.zEnd !== undefined) {
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index ? { ...x, z_end_mm: change.zEnd ?? null } : x
      );
      next = { ...cur, patterns };
    } else if (spotSel?.type === "texture" && change.zStart !== undefined) {
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index ? { ...x, fade_in_percent: (change.zStart! / height) * 100 } : x
      );
      next = { ...cur, textures };
    } else if (spotSel?.type === "texture" && change.zEnd !== undefined && change.zEnd !== null) {
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index ? { ...x, fade_out_percent: ((height - change.zEnd!) / height) * 100 } : x
      );
      next = { ...cur, textures };
    } else if (spotSel?.type === "floor" && change.holeDiameter !== undefined) {
      next = { ...cur, floor: { ...cur.floor, center_hole_diameter_mm: change.holeDiameter } };
    } else if (
      spotSel?.type === "pattern" &&
      (change.spotCenterDeg !== undefined || change.spotSpanDeg !== undefined)
    ) {
      // ---- ۱٫۶: اسپات زاویه‌ای الگو (درخواست کاربر ۵) ----
      const patterns = cur.patterns.map((x, j) =>
        j === spotSel.index
          ? {
              ...x,
              spot_center_deg:
                change.spotCenterDeg !== undefined ? change.spotCenterDeg : (x.spot_center_deg ?? 0),
              spot_span_deg:
                change.spotSpanDeg !== undefined ? change.spotSpanDeg : (x.spot_span_deg ?? 360),
            }
          : x
      );
      next = { ...cur, patterns };
    } else if (
      spotSel?.type === "texture" &&
      (change.spotCenterDeg !== undefined || change.spotSpanDeg !== undefined)
    ) {
      // ---- ۱٫۶: اسپات زاویه‌ای بافت ----
      const textures = cur.textures.map((x, j) =>
        j === spotSel.index
          ? {
              ...x,
              spot_center_deg:
                change.spotCenterDeg !== undefined ? change.spotCenterDeg : (x.spot_center_deg ?? 0),
              spot_span_deg:
                change.spotSpanDeg !== undefined ? change.spotSpanDeg : (x.spot_span_deg ?? 360),
            }
          : x
      );
      next = { ...cur, textures };
    }
    if (next) setProject(next);
  }, [spotSel]);

  // شمارش فعال بودن هر الگو/بافت روی کارت‌ها
  const patternCounts = useMemo(() => {
    const m: Record<string, number> = {};
    project.patterns.forEach((p) => {
      m[p.kind] = (m[p.kind] ?? 0) + 1;
    });
    return m;
  }, [project.patterns]);
  const textureCounts = useMemo(() => {
    const m: Record<string, number> = {};
    project.textures.forEach((t) => {
      m[t.kind] = (m[t.kind] ?? 0) + 1;
    });
    return m;
  }, [project.textures]);

  // ------------------------------------------------------------ رندر
  return (
    <div className="designer-layout">
      {/* ---------- پنل کناری ۵ گام ---------- */}
      <div className="designer-side">
        <div className="designer-head">
          <h3 style={{ marginTop: 0, color: "var(--accent)", marginBottom: 0 }}>{fa.designerTitle}</h3>
          <div className="history-btns">
            <button className="btn small" disabled={histLen === 0} onClick={undo}
              title="Ctrl+Z">{fa.designerUndo} ↶</button>
            <button className="btn small" disabled={futureLen === 0} onClick={redo}
              title="Ctrl+Y">{fa.designerRedo} ↷</button>
          </div>
        </div>

        <div className="steps">
          {[1, 2, 3, 4, 5, 6].map((s) => (
            <button
              key={s}
              className={`step-btn ${step === s ? "active" : ""}`}
              onClick={() => setStep(s)}
            >
              {s === 1 ? fa.step1Profile : s === 2 ? fa.step2Pattern
                : s === 3 ? fa.step3Texture : s === 4 ? fa.step4Wall
                : s === 5 ? fa.step5Floor : fa.step6Printer}
            </button>
          ))}
        </div>

        {/* ================= گام ۱: پروفایل ================= */}
        {step === 1 && (
          <div>
            <ProfileEditor
              points={project.profile}
              height={project.form_height_mm}
              maxR={maxR}
              interpolation={project.profile_interpolation ?? "Spline"}
              onChange={(pts: ProfilePoint[]) => setP({ profile: pts })}
            />
            {/* ---- ابزارهای پروفایل — نسخهٔ ۱.۵ (رندوم/ایمپورت SVG) ---- */}
            <div className="profile-tools">
              <button
                className="btn small"
                disabled={profileBusy}
                onClick={doRandomProfile}
                title={fa.randomProfileHint}
              >
                🎲 {fa.randomProfile}
              </button>
              <button
                className="btn small"
                disabled={profileBusy}
                onClick={doImportSvg}
                title={fa.importSvgHint}
              >
                📥 {fa.importSvg}
              </button>
            </div>
            <span className="hint-text">{fa.importSvgHint}</span>
            {/* ---- کتابخانهٔ ۱۸ پروفایل — نسخهٔ ۱.۵ ---- */}
            <div className="field" style={{ marginTop: 8 }}>
              <label>{fa.moreProfiles} ({PROFILE_LIBRARY.length}):</label>
              <div className="profile-library">
                {PROFILE_LIBRARY.map((p) => (
                  <button
                    key={p.key}
                    className="profile-lib-btn"
                    onClick={() => setP({ profile: p.profile.map((q) => ({ ...q })) })}
                    title={p.name}
                  >
                    <ProfileMiniThumb points={p.profile} width={52} height={62} />
                    <span className="name">{p.name}</span>
                  </button>
                ))}
              </div>
            </div>
            {/* درون‌یابی: اسپلاین مونوتون (نسخهٔ ۱.۲) */}
            <div className="field">
              <label>{fa.interpolationMode}:</label>
              <div className="radio-row" role="radiogroup" aria-label={fa.interpolationMode}>
                <button
                  className={`btn small ${project.profile_interpolation !== "Linear" ? "primary" : ""}`}
                  role="radio" aria-checked={project.profile_interpolation !== "Linear"}
                  onClick={() => setP({ profile_interpolation: "Spline" })}
                >{fa.interpolationSpline}</button>
                <button
                  className={`btn small ${project.profile_interpolation === "Linear" ? "primary" : ""}`}
                  role="radio" aria-checked={project.profile_interpolation === "Linear"}
                  onClick={() => setP({ profile_interpolation: "Linear" })}
                >{fa.interpolationLinear}</button>
              </div>
              <span className="hint-text">{fa.interpolationHint}</span>
            </div>
            <div className="field">
              <label>{fa.height}</label>
              <input type="number" step="1" min="5" max="400"
                value={project.form_height_mm}
                onChange={(e) => setP({ form_height_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.bottomDiameter}</label>
              <input type="number" step="1" min="2"
                value={project.form_bottom_diameter_mm}
                onChange={(e) => setP({ form_bottom_diameter_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.topDiameter}</label>
              <input type="number" step="1" min="2"
                value={project.form_top_diameter_mm}
                onChange={(e) => setP({ form_top_diameter_mm: +e.target.value || 1 })} />
            </div>
            <div className="field">
              <label>{fa.taper}</label>
              <input type="number" step="1" min="0" max="95"
                value={project.taper_percent}
                onChange={(e) => setP({ taper_percent: +e.target.value || 0 })} />
            </div>
          </div>
        )}

        {/* ================= استایل‌های آماده ================= */}
        {step === 1 && (
          <div className="field" style={{ marginTop: 10 }}>
            <label>{fa.stylePresets}:</label>
            <span className="hint-text">{fa.stylePresetHint}</span>
            <div className="style-grid">
              {STYLE_PRESETS.map((sp) => (
                <button key={sp.key} className="style-card" onClick={() => applyStyle(sp)}>
                  <ProfileMiniThumb points={sp.profile} width={40} height={52} />
                  <div className="style-meta">
                    <strong>{sp.name}</strong>
                    <span>{sp.desc}</span>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* ================= گام ۲: الگو (کارت‌های تصویری بزرگ) ================= */}
        {step === 2 && (
          <div>
            {/* ---- چرخش فاز θ — نسخهٔ ۱.۳ (درخواست کاربر) ---- */}
            <div className="field">
              <label>{fa.phaseRotate}</label>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                <input type="number" step="15" min="-360" max="360" style={{ width: 90 }}
                  value={Math.round((project.phase_start_deg ?? 0) * 10) / 10}
                  onChange={(e) => {
                    const v = +e.target.value || 0;
                    setP({ phase_start_deg: ((v % 360) + 360) % 360 });
                  }} />
                <span className="ltr" style={{ fontSize: 12, color: "var(--fg-2)" }}>°</span>
                <button
                  className={`btn small ${spotSel?.type === "phase" ? "primary" : ""}`}
                  title={fa.spotPhaseRotate}
                  onClick={() => setSpotSel(spotSel?.type === "phase" ? null : { type: "phase" })}
                >🔄</button>
              </div>
              <span className="hint-text">{fa.phaseRotateHint}</span>
              <span className="hint-text">{fa.phaseRotateSeam}</span>
              {spotSel?.type === "phase" && (
                <div className="spot-note">🔄 {fa.spotPhaseRotate} — Shift = چفت ۱۵°</div>
              )}
            </div>
            <p className="step-note">
              الگو <strong>هندسهٔ مدل</strong> را تغییر می‌دهد؛ اثر آن در <strong>STL</strong> هم باقی می‌ماند.
              روی کارت کلیک کنید تا افزوده شود؛ سپس با دکمهٔ 🎯 آن را روی نمای 3D جابه‌جا کنید.
            </p>
            <div className="card-grid">
              {PATTERNS.map((p) => {
                const count = patternCounts[p.kind] ?? 0;
                return (
                  <button
                    key={p.kind}
                    className={`asset-card ${count > 0 ? "active" : ""}`}
                    onClick={() => addPattern(p.kind)}
                    title={p.hint}
                  >
                    <SurfaceThumb grid={previewData?.patterns[p.kind]} variant="pattern" width={104} height={76} />
                    <span className="asset-name">{p.name}</span>
                    <span className="asset-badge">
                      {count > 0 ? `${count}× ${fa.cardActive}` : `+ ${fa.cardAdd}`}
                    </span>
                  </button>
                );
              })}
            </div>
            {project.patterns.map((pat, i) => (
              <div className={`pattern-item ${spotSel?.type === "pattern" && spotSel.index === i ? "spot-selected" : ""}`} key={i}>
                <div className="head">
                  <span className="name">{patternName[pat.kind]}</span>
                  <div style={{ display: "flex", gap: 4 }}>
                    <button
                      className={`btn small ${spotSel?.type === "pattern" && spotSel.index === i ? "primary" : ""}`}
                      title={fa.spotPatternRange}
                      onClick={() => setSpotSel(spotSel?.type === "pattern" && spotSel.index === i ? null : { type: "pattern", index: i })}
                    >🎯</button>
                    <button className="btn small danger" onClick={() => removePattern(i)}>×</button>
                  </div>
                </div>
                <div className="field"><label>{fa.patternAmplitude}</label>
                  <input type="number" step="0.1" min="0" value={pat.amplitude_mm}
                    onChange={(e) => updatePattern(i, { amplitude_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.patternCount}</label>
                  <input type="number" step="1" min="1" max="64" value={pat.count}
                    onChange={(e) => updatePattern(i, { count: Math.round(+e.target.value) || 1 })} /></div>
                <div className="field"><label>{fa.patternFrequency}</label>
                  <input type="number" step="0.5" min="0" value={pat.frequency}
                    onChange={(e) => updatePattern(i, { frequency: +e.target.value })} /></div>
                <div className="field"><label>{fa.patternPhase}</label>
                  <input type="number" step="15" value={pat.phase_deg}
                    onChange={(e) => updatePattern(i, { phase_deg: +e.target.value })} /></div>
                <div className="field"><label>شروع بازه (mm)</label>
                  <input type="number" step="5" min="0" value={pat.z_start_mm}
                    onChange={(e) => updatePattern(i, { z_start_mm: +e.target.value })} /></div>
                <div className="field"><label>پایان بازه (mm؛ خالی = تا بالا)</label>
                  <input type="number" step="5" value={pat.z_end_mm ?? ""}
                    onChange={(e) => updatePattern(i, {
                      z_end_mm: e.target.value === "" ? null : +e.target.value,
                    })} /></div>
                {/* ---- اسپات زاویه‌ای — نسخهٔ ۱٫۶ (درخواست کاربر ۵) ---- */}
                <div className="spot-fields">
                  <div className="field"><label>مرکز اسپات (درجه)</label>
                    <input type="number" step="15" min="0" max="360" style={{ width: 90 }}
                      value={Math.round((pat.spot_center_deg ?? 0) * 10) / 10}
                      onChange={(e) => updatePattern(i, {
                        spot_center_deg: ((+e.target.value || 0) % 360 + 360) % 360,
                      })} /></div>
                  <div className="field"><label>گستره (درجه؛ ۳۶۰ = کل)</label>
                    <input type="number" step="15" min="10" max="360" style={{ width: 90 }}
                      value={pat.spot_span_deg ?? 360}
                      onChange={(e) => updatePattern(i, {
                        spot_span_deg: Math.min(360, Math.max(10, Math.round(+e.target.value) || 360)),
                      })} /></div>
                </div>
                {(pat.spot_span_deg ?? 360) < 359 && (
                  <span className="hint-text">
                    اسپات فعال: الگو فقط روی قوس {Math.round(pat.spot_span_deg ?? 360)}°
                    حول {Math.round(pat.spot_center_deg ?? 0)}° اعمال می‌شود —
                    با 🎯 روی 3D لبه‌های طلایی را بکشید تا قوس پهن/باریک شود.
                  </span>
                )}
                {spotSel?.type === "pattern" && spotSel.index === i && (
                  <div className="spot-note">🎯 {fa.spotHint}</div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۳: بافت (کارت‌های تصویری بزرگ) ================= */}
        {step === 3 && (
          <div>
            <p className="step-note">
              بافت <strong>مسیر حرکت نازل</strong> را تعدیل می‌کند و در STL ذخیره <strong>نمی‌شود</strong>؛
              فقط در G-code و پیش‌نمایش مسیر دیده می‌شود. با 🎯 ناحیهٔ اعمال را روی 3D جابه‌جا کنید.
            </p>
            <div className="card-grid">
              {TEXTURES.map((t) => {
                const count = textureCounts[t.kind] ?? 0;
                return (
                  <button
                    key={t.kind}
                    className={`asset-card texture ${count > 0 ? "active" : ""}`}
                    onClick={() => addTexture(t.kind)}
                    title={t.hint}
                  >
                    <SurfaceThumb grid={previewData?.textures[t.kind]} variant="texture" width={104} height={76} />
                    <span className="asset-name">{t.name}</span>
                    <span className="asset-badge">
                      {count > 0 ? `${count}× ${fa.cardActive}` : `+ ${fa.cardAdd}`}
                    </span>
                  </button>
                );
              })}
            </div>
            {project.textures.map((tex, i) => (
              <div className={`texture-item ${spotSel?.type === "texture" && spotSel.index === i ? "spot-selected" : ""}`} key={i}>
                <div className="head">
                  <span className="name">{textureName[tex.kind]}</span>
                  <div style={{ display: "flex", gap: 4 }}>
                    <button
                      className={`btn small ${spotSel?.type === "texture" && spotSel.index === i ? "primary" : ""}`}
                      title={fa.spotTextureFade}
                      onClick={() => setSpotSel(spotSel?.type === "texture" && spotSel.index === i ? null : { type: "texture", index: i })}
                    >🎯</button>
                    <button className="btn small danger" onClick={() => removeTexture(i)}>×</button>
                  </div>
                </div>
                <div className="field"><label>{fa.textureAmplitude}</label>
                  <input type="number" step="0.05" min="0" value={tex.amplitude_mm}
                    onChange={(e) => updateTexture(i, { amplitude_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.texturePeriod}</label>
                  <input type="number" step="0.5" min="0.2" value={tex.period_mm}
                    onChange={(e) => updateTexture(i, { period_mm: +e.target.value })} /></div>
                <div className="field"><label>{fa.textureDirection}</label>
                  <select value={tex.radial ? "r" : "z"}
                    onChange={(e) => updateTexture(i, { radial: e.target.value === "r" })}>
                    <option value="r">{fa.textureRadial}</option>
                    <option value="z">{fa.textureVertical}</option>
                  </select></div>
                <div className="field"><label>{fa.fade} ↓%</label>
                  <input type="number" step="5" min="0" max="100" value={tex.fade_in_percent}
                    onChange={(e) => updateTexture(i, { fade_in_percent: +e.target.value })} /></div>
                <div className="field"><label>{fa.fade} ↑%</label>
                  <input type="number" step="5" min="0" max="100" value={tex.fade_out_percent}
                    onChange={(e) => updateTexture(i, { fade_out_percent: +e.target.value })} /></div>
                {/* ---- اسپات زاویه‌ای — نسخهٔ ۱٫۶ (درخواست کاربر ۵) ---- */}
                <div className="spot-fields">
                  <div className="field"><label>مرکز اسپات (درجه)</label>
                    <input type="number" step="15" min="0" max="360" style={{ width: 90 }}
                      value={Math.round((tex.spot_center_deg ?? 0) * 10) / 10}
                      onChange={(e) => updateTexture(i, {
                        spot_center_deg: ((+e.target.value || 0) % 360 + 360) % 360,
                      })} /></div>
                  <div className="field"><label>گستره (درجه؛ ۳۶۰ = کل)</label>
                    <input type="number" step="15" min="10" max="360" style={{ width: 90 }}
                      value={tex.spot_span_deg ?? 360}
                      onChange={(e) => updateTexture(i, {
                        spot_span_deg: Math.min(360, Math.max(10, Math.round(+e.target.value) || 360)),
                      })} /></div>
                </div>
                {(tex.spot_span_deg ?? 360) < 359 && (
                  <span className="hint-text">
                    اسپات فعال: بافت فقط روی قوس {Math.round(tex.spot_span_deg ?? 360)}°
                    حول {Math.round(tex.spot_center_deg ?? 0)}° اعمال می‌شود —
                    با 🎯 روی 3D لبه‌های طلایی را بکشید.
                  </span>
                )}
                {spotSel?.type === "texture" && spotSel.index === i && (
                  <div className="spot-note">🎯 {fa.spotHint}</div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* ================= گام ۴: دیواره — توری/سوراخ (نسخهٔ ۱٫۷) ================= */}
        {step === 4 && (
          <div>
            <p className="step-note">
              حالت دیواره را انتخاب کنید: <strong>جامد</strong> = مارپیچ پیوستهٔ کلاسیک؛
              <strong> توری</strong> = تارهای متقاطع با خانه‌های باز (مثل مرجع)؛
              سوراخ‌های دیواره در هر دو حالت کار می‌کنند.
            </p>

            {/* ---------- کارت‌های حالت دیواره ---------- */}
            <div className="asset-cards" style={{ gridTemplateColumns: "1fr 1fr" }}>
              {([
                { mode: "Solid", name: fa.wallSolid, hint: "مارپیچ پیوسته — سطح بسته", icon: "🧱" },
                { mode: "Lattice", name: fa.wallLattice, hint: "تارهای متقاطع — خانه‌های باز", icon: "🕸️" },
              ] as const).map((w) => (
                <button
                  key={w.mode}
                  className={`asset-card ${((project.wall?.mode ?? "Solid") === w.mode) ? "selected" : ""}`}
                  onClick={() => setWall({ mode: w.mode })}
                >
                  <div style={{ fontSize: 30, lineHeight: 1.2 }}>{w.icon}</div>
                  <span className="asset-name">{w.name}</span>
                  <span className="hint-text">{w.hint}</span>
                  {(project.wall?.mode ?? "Solid") === w.mode && (
                    <span className="asset-badge">{fa.cardActive}</span>
                  )}
                </button>
              ))}
            </div>

            {/* ---------- تنظیمات توری ---------- */}
            {(project.wall?.mode ?? "Solid") === "Lattice" && (
              <>
                <div className="field" style={{ marginTop: 12 }}>
                  <label>🕸️ {fa.latticeStyle}</label>
                  <select
                    value={project.wall?.lattice?.style ?? "Diamond"}
                    onChange={(e) => setLattice({ style: e.target.value as never })}
                  >
                    <option value="Diamond">الماسی (کلاسیک)</option>
                    <option value="Square">مربعی (سکودار)</option>
                    <option value="Chevron">مهری (شخورداده)</option>
                    <option value="Twisted">پیچان (چرخان)</option>
                  </select>
                  <span className="hint-text">
                    دورهٔ موج طوری انتخاب می‌شود که تارهای انقلاب‌های مجاور در
                    تقاطع‌ها روی هم بنشینند — خانه‌ها واقعاً باز می‌مانند
                  </span>
                </div>
                <div className="field"><label>{fa.latticeCellW}</label>
                  <input type="number" step="1" min="6" max="40"
                    value={project.wall?.lattice?.cell_width_mm ?? 12}
                    onChange={(e) => setLattice({ cell_width_mm: Math.max(6, +e.target.value || 12) })} />
                  <span className="hint-text">کوچک‌تر = توری ریزتر و ظریف‌تر (حداقل ~۶mm)</span>
                </div>
                <div className="field"><label>{fa.latticeCellH}</label>
                  <input type="number" step="1" min="4" max="30"
                    value={project.wall?.lattice?.cell_height_mm ?? 10}
                    onChange={(e) => setLattice({ cell_height_mm: Math.max(4, +e.target.value || 10) })} />
                  <span className="hint-text">ارتفاع هر خانهٔ لوزی روی دیواره</span>
                </div>
                <div className="field"><label>{fa.latticeRise}</label>
                  <input type="number" step="0.05" min="0.2" max="0.8"
                    value={project.wall?.lattice?.rise_ratio ?? 0.5}
                    onChange={(e) => setLattice({ rise_ratio: Math.min(0.8, Math.max(0.2, +e.target.value || 0.5)) })} />
                  <span className="hint-text">۰٫۵ = الماس متقارن؛ کمتر = خانه‌های کشیده‌تر عمودی</span>
                </div>
                <div className="field"><label>{fa.latticeSpeed}</label>
                  <input type="number" step="0.05" min="0.3" max="1"
                    value={project.wall?.lattice?.speed_scale ?? 0.6}
                    onChange={(e) => setLattice({ speed_scale: Math.min(1, Math.max(0.3, +e.target.value || 0.6)) })} />
                  <span className="hint-text">تارهای توری روی هوا چاپ می‌شوند — آرام‌تر = پیوند بهتر</span>
                </div>
                <div className="field"><label>{fa.latticeFlow}</label>
                  <input type="number" step="0.05" min="0.5" max="2"
                    value={project.wall?.lattice?.flow_scale ?? 1.1}
                    onChange={(e) => setLattice({ flow_scale: Math.min(2, Math.max(0.5, +e.target.value || 1.1)) })} />
                  <span className="hint-text">۱٫۱ = تار کمی پرتر برای استحکام تقاطع‌ها</span>
                </div>
                <div className="field"><label>{fa.latticePhase}</label>
                  <input type="number" step="5" min="0" max="360"
                    value={project.wall?.lattice?.phase_deg ?? 0}
                    onChange={(e) => setLattice({ phase_deg: +e.target.value || 0 })} /></div>
                <div className="field"><label>{fa.latticeZStart}</label>
                  <input type="number" step="1" min="0" max="200"
                    value={project.wall?.lattice?.z_start_mm ?? 8}
                    onChange={(e) => setLattice({ z_start_mm: Math.max(0, +e.target.value || 0) })} />
                  <span className="hint-text">پایین دیواره جامد بماند — لبهٔ مقاوم و چسبندگی کف</span>
                </div>
                <div className="field"><label>{fa.latticeZEnd}</label>
                  <input type="number" step="1" min="0" max="400"
                    value={project.wall?.lattice?.z_end_mm ?? ""}
                    placeholder="خالی = تا بالای فرم"
                    onChange={(e) => setLattice({ z_end_mm: e.target.value === "" ? null : Math.max(0, +e.target.value || 0) })} />
                  <span className="hint-text">مثلاً ۸۰ = فقط تا ۸۰mm توری، بالای فرم دوباره جامد</span>
                </div>
              </>
            )}

            {/* ---------- سوراخ‌های دیواره (درخواست ۹) ---------- */}
            <div className="field" style={{ marginTop: 14 }}>
              <label className="check-row" style={{ marginBottom: 0 }}>
                <input type="checkbox"
                  checked={project.wall?.holes?.enabled ?? false}
                  onChange={(e) => setHoles({ enabled: e.target.checked })} />
                <strong>🕳️ {fa.holesTitle}</strong>
              </label>
              <span className="hint-text">
                سوراخ‌ها در «خود دیواره» ساخته می‌شوند: مارپیچ هنگام عبور از
                دهانه travel می‌کند (دهانهٔ واقعی باز) و دور هر سوراخ قاب
                تقویتی بسته چاپ می‌شود — نه فقط پوشش لایهٔ آخر.
              </span>
            </div>
            {(project.wall?.holes?.enabled ?? false) && (
              <>
                <div className="field"><label>{fa.holesShape}</label>
                  <select
                    value={project.wall?.holes?.shape ?? "Circle"}
                    onChange={(e) => setHoles({ shape: e.target.value as never })}
                  >
                    <option value="Circle">دایره</option>
                    <option value="RoundedRect">مستطیل گردگوشه</option>
                    <option value="Diamond">لوزی</option>
                    <option value="Hexagon">شش‌ضلعی</option>
                    <option value="Star">ستارهٔ چهارپر</option>
                  </select>
                  <span className="hint-text">شکل قاب تقویتی هم به‌تبع شکل سوراخ گرد می‌شود</span>
                </div>
                <div className="field"><label>{fa.holesAround}</label>
                  <input type="number" step="1" min="1" max="12"
                    value={project.wall?.holes?.count_around ?? 4}
                    onChange={(e) => setHoles({ count_around: Math.min(12, Math.max(1, Math.round(+e.target.value) || 4)) })} /></div>
                <div className="field"><label>{fa.holesRows}</label>
                  <input type="number" step="1" min="1" max="8"
                    value={project.wall?.holes?.rows ?? 3}
                    onChange={(e) => setHoles({ rows: Math.min(8, Math.max(1, Math.round(+e.target.value) || 3)) })} /></div>
                <div className="field"><label>{fa.holesWidth}</label>
                  <input type="number" step="1" min="4" max="60"
                    value={project.wall?.holes?.width_mm ?? 14}
                    onChange={(e) => setHoles({ width_mm: Math.max(4, +e.target.value || 14) })} />
                  <span className="hint-text">عرض در «طول محیط» فرم (قوس mm)</span>
                </div>
                <div className="field"><label>{fa.holesHeight}</label>
                  <input type="number" step="1" min="4" max="80"
                    value={project.wall?.holes?.height_mm ?? 18}
                    onChange={(e) => setHoles({ height_mm: Math.max(4, +e.target.value || 18) })} /></div>
                <div className="field"><label>{fa.holesPitch}</label>
                  <input type="number" step="1" min="0" max="100"
                    value={project.wall?.holes?.row_pitch_mm ?? 0}
                    placeholder="۰ = خودکار"
                    onChange={(e) => setHoles({ row_pitch_mm: Math.max(0, +e.target.value || 0) })} /></div>
                <div className="field"><label>{fa.holesMarginBottom}</label>
                  <input type="number" step="1" min="2" max="100"
                    value={project.wall?.holes?.margin_bottom_mm ?? 10}
                    onChange={(e) => setHoles({ margin_bottom_mm: Math.max(2, +e.target.value || 10) })} /></div>
                <div className="field"><label>{fa.holesMarginTop}</label>
                  <input type="number" step="1" min="0" max="100"
                    value={project.wall?.holes?.margin_top_mm ?? 10}
                    onChange={(e) => setHoles({ margin_top_mm: Math.max(0, +e.target.value || 10) })} /></div>
                <div className="field"><label>{fa.holesFrames}</label>
                  <input type="number" step="1" min="0" max="3"
                    value={project.wall?.holes?.frame_passes ?? 2}
                    onChange={(e) => setHoles({ frame_passes: Math.min(3, Math.max(0, Math.round(+e.target.value) || 0)) })} />
                  <span className="hint-text">۲ = لبهٔ مسلح دور سوراخ (هر پاس یک دور اضافه)</span>
                </div>
                <div className="field"><label>{fa.holesPhase}</label>
                  <input type="number" step="5" min="0" max="360"
                    value={project.wall?.holes?.phase_deg ?? 0}
                    onChange={(e) => setHoles({ phase_deg: +e.target.value || 0 })} /></div>
                <label className="check-row">
                  <input type="checkbox"
                    checked={project.wall?.holes?.stagger ?? true}
                    onChange={(e) => setHoles({ stagger: e.target.checked })} />
                  {fa.holesStagger}
                </label>
                <span className="hint-text">ردیف‌های زوج نیم‌فاصله می‌لغزند — چینش آجری زیباتر</span>
              </>
            )}
          </div>
        )}

        {/* ================= گام ۵: کف (لامپ/آویز) (۵ الگوی تصویری) ================= */}
        {step === 5 && (
          <div>
            <p className="step-note">
              پنج الگوی چاپ کف — همه مسیر واقعی تولید می‌کنند. نمای بالا هر الگو را نشان می‌دهد.
            </p>
            <label className="check-row">
              <input type="checkbox" checked={project.floor.enabled}
                onChange={(e) => setFloor({ enabled: e.target.checked })} />
              {fa.floorEnabled}
            </label>
            <div className="field">
              <label>{fa.floorPattern}:</label>
              <div className="card-grid floor">
                {FLOORS.map((f) => (
                  <button
                    key={f.kind}
                    className={`asset-card floor ${(project.floor.pattern ?? "Rings") === f.kind && project.floor.enabled ? "active" : ""}`}
                    onClick={() => setFloor({ pattern: f.kind, enabled: true })}
                    title={f.hint}
                    disabled={!project.floor.enabled}
                  >
                    <FloorThumb kind={f.kind} hole={f.kind === "Radial" ? 0.22 : 0.16} size={84} />
                    <span className="asset-name">{f.name}</span>
                    {(project.floor.pattern ?? "Rings") === f.kind && project.floor.enabled && (
                      <span className="asset-badge">{fa.cardActive}</span>
                    )}
                  </button>
                ))}
              </div>
            </div>
            <div className="field"><label>{fa.floorHole}</label>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                <input type="number" step="1" min="0" disabled={!project.floor.enabled}
                  value={project.floor.center_hole_diameter_mm}
                  onChange={(e) => setFloor({ center_hole_diameter_mm: +e.target.value })} />
                <button
                  className={`btn small ${spotSel?.type === "floor" ? "primary" : ""}`}
                  disabled={!project.floor.enabled || project.floor.center_hole_diameter_mm < 1}
                  title={fa.spotFloorHole}
                  onClick={() => setSpotSel(spotSel?.type === "floor" ? null : { type: "floor" })}
                >🎯</button>
              </div>
              {spotSel?.type === "floor" && (
                <span className="hint-text">🎯 قطر سوراخ را با کشیدن حلقه در نمای 3D تغییر دهید</span>
              )}
            </div>
            <div className="field"><label>{fa.floorGap}</label>
              <input type="number" step="0.05" min="0.2" disabled={!project.floor.enabled}
                value={project.floor.ring_gap_mm}
                onChange={(e) => setFloor({ ring_gap_mm: +e.target.value })} /></div>
            {/* ---- تعداد لایه‌های کف — نسخهٔ ۱.۴ (درخواست کاربر) ---- */}
            <div className="field">
              <label>{fa.floorLayers}</label>
              <input type="number" step="1" min="1" max="12" disabled={!project.floor.enabled}
                value={project.floor.layers ?? 2}
                onChange={(e) => setFloor({ layers: Math.max(1, Math.min(12, Math.round(+e.target.value) || 2)) })} />
              <span className="hint-text">{fa.floorLayersHint}</span>
            </div>
            {(project.floor.pattern ?? "Rings") === "Radial" && (
              <div className="field"><label>{fa.floorSpokes}</label>
                <input type="number" step="1" min="3" max="96" disabled={!project.floor.enabled}
                  value={project.floor.spokes ?? 12}
                  onChange={(e) => setFloor({ spokes: Math.round(+e.target.value) || 12 })} /></div>
            )}
            {(project.floor.pattern ?? "Rings") === "Wave" && (
              <div className="field"><label>تعداد موج در هر دور</label>
                <input type="number" step="1" min="3" max="24" disabled={!project.floor.enabled}
                  value={project.floor.spokes ?? 12}
                  onChange={(e) => setFloor({ spokes: Math.round(+e.target.value) || 12 })} />
                <span className="hint-text">موج‌های شعاعی کف تزئینی — از فیلد پرتوها استفاده می‌کند</span>
              </div>
            )}
            {(project.floor.pattern ?? "Rings") === "Star" && (
              <div className="field"><label>تعداد پرتوهای ستاره (۵..۲۴)</label>
                <input type="number" step="2" min="5" max="24" disabled={!project.floor.enabled}
                  value={project.floor.spokes ?? 12}
                  onChange={(e) => setFloor({ spokes: Math.round(+e.target.value) || 12 })} /></div>
            )}
            {/* ============ آویز حرفه‌ای — نسخهٔ ۱٫۷ (درخواست کاربر ۷) ============ */}
            <div className="field" style={{ marginTop: 14 }}>
              <label>🪝 {fa.pendantTitle}</label>
              <select
                value={project.pendant?.style ?? "None"}
                onChange={(e) => setPendant({ style: e.target.value as never })}
              >
                <option value="None">بدون آویز</option>
                <option value="RimHoles">سوراخ لبهٔ کف (نخ از زیر)</option>
                <option value="TopHoles">سوراخ بالای دیواره (زنجیر از بالا)</option>
                <option value="Both">ترکیبی (کف + بالا)</option>
              </select>
              <span className="hint-text">
                سوراخ‌های کف در الگوهای کف خالی می‌مانند؛ سوراخ‌های بالا با قاب
                تقویتی در خود دیواره ساخته می‌شوند و لبهٔ فرم با حلقه‌های
                تقویت ضخیم می‌شود — آویز سنگین ایمن است
              </span>
            </div>
            {["RimHoles", "Both"].includes(project.pendant?.style ?? "None") && (
              <>
                <div className="field"><label>{fa.pendantRimCount}</label>
                  <input type="number" step="1" min="3" max="12"
                    value={project.pendant?.rim_holes_count ?? 3}
                    onChange={(e) => setPendant({ rim_holes_count: Math.min(12, Math.max(3, Math.round(+e.target.value) || 3)) })} />
                  <span className="hint-text">۳ = تعادل مثلثی؛ ۴ = مربعی؛ بیشتر برای سبد</span>
                </div>
                <div className="field"><label>{fa.pendantRimD}</label>
                  <input type="number" step="0.5" min="1" max="20"
                    value={project.pendant?.rim_hole_diameter_mm ?? 3}
                    onChange={(e) => setPendant({ rim_hole_diameter_mm: Math.max(1, +e.target.value || 3) })} />
                  <span className="hint-text">۳mm نخ؛ ۵mm سیم پارچه؛ ۸mm زنجیر ظریف</span>
                </div>
                <div className="field"><label>{fa.pendantRimPos}</label>
                  <input type="number" step="5" min="40" max="95"
                    value={project.pendant?.rim_hole_radius_percent ?? 85}
                    onChange={(e) => setPendant({ rim_hole_radius_percent: Math.min(95, Math.max(40, +e.target.value || 85)) })} />
                  <span className="hint-text">نزدیک لبه (۸۵-۹۰٪) = گشتاور کمتر و تعادل بهتر</span>
                </div>
              </>
            )}
            {["TopHoles", "Both"].includes(project.pendant?.style ?? "None") && (
              <>
                <div className="field"><label>{fa.pendantTopCount}</label>
                  <input type="number" step="1" min="2" max="6"
                    value={project.pendant?.top_holes_count ?? 3}
                    onChange={(e) => setPendant({ top_holes_count: Math.min(6, Math.max(2, Math.round(+e.target.value) || 3)) })} /></div>
                <div className="field"><label>{fa.pendantTopD}</label>
                  <input type="number" step="0.5" min="2" max="15"
                    value={project.pendant?.top_hole_diameter_mm ?? 5}
                    onChange={(e) => setPendant({ top_hole_diameter_mm: Math.max(2, +e.target.value || 5) })} /></div>
                <div className="field"><label>{fa.pendantTopFrom}</label>
                  <input type="number" step="1" min="3" max="30"
                    value={project.pendant?.top_hole_from_rim_mm ?? 8}
                    onChange={(e) => setPendant({ top_hole_from_rim_mm: Math.max(3, +e.target.value || 8) })} />
                  <span className="hint-text">فاصلهٔ مرکز سوراخ از لبهٔ بالای فرم</span>
                </div>
              </>
            )}
            {(project.pendant?.style ?? "None") !== "None" && (
              <>
                <div className="field"><label>{fa.pendantRings}</label>
                  <input type="number" step="1" min="0" max="6"
                    value={project.pendant?.rim_reinforce_rings ?? 3}
                    onChange={(e) => setPendant({ rim_reinforce_rings: Math.min(6, Math.max(0, Math.round(+e.target.value) || 0)) })} />
                  <span className="hint-text">حلقه‌های بسته در بالای دیواره — لبهٔ تحت کشش ضخیم می‌شود</span>
                </div>
                <label className="check-row">
                  <input type="checkbox"
                    checked={project.pendant?.top_hole_frames ?? true}
                    onChange={(e) => setPendant({ top_hole_frames: e.target.checked })} />
                  قاب تقویتی دور سوراخ‌های بالا
                </label>
              </>
            )}

            {/* ============ لامپ حرفه‌ای — نسخهٔ ۱٫۷ (درخواست کاربر ۸) ============ */}
            <div className="field" style={{ marginTop: 14 }}>
              <label className="check-row" style={{ marginBottom: 0 }}>
                <input type="checkbox"
                  checked={project.lamp?.enabled ?? false}
                  onChange={(e) => setLamp({ enabled: e.target.checked })} />
                <strong>💡 {fa.lampTitle}</strong>
              </label>
              <span className="hint-text">
                سوکت مرکزی + پره‌های متصل‌کنندهٔ سوراخ به بدنه + کف نورپذیر؛
                قطر سوکت بر سوراخ مرکزی معمولی کف تقدم دارد
              </span>
            </div>
            {(project.lamp?.enabled ?? false) && (
              <>
                <div className="field"><label>{fa.lampHole}</label>
                  <div style={{ display: "flex", gap: 4 }}>
                    <input type="number" step="1" min="4" max="80"
                      value={project.lamp?.center_hole_diameter_mm ?? 16}
                      onChange={(e) => setLamp({ center_hole_diameter_mm: Math.max(4, +e.target.value || 16) })} />
                    <button className="btn small" type="button" title="سوکت E14 (۲۸mm)"
                      onClick={() => setLamp({ center_hole_diameter_mm: 28 })}>E14</button>
                    <button className="btn small" type="button" title="سوکت E27 (۴۰mm)"
                      onClick={() => setLamp({ center_hole_diameter_mm: 40 })}>E27</button>
                  </div>
                  <span className="hint-text">۸ = عبور سیم؛ ۱۶ = سوکت کوچک؛ ۲۸/۴۰ = سوکت استاندارد E14/E27</span>
                </div>
                <div className="field"><label>{fa.lampRings}</label>
                  <input type="number" step="1" min="0" max="6"
                    value={project.lamp?.socket_rings ?? 3}
                    onChange={(e) => setLamp({ socket_rings: Math.min(6, Math.max(0, Math.round(+e.target.value) || 0)) })} />
                  <span className="hint-text">حلقه‌های اضافه دور سوکت — گریپ مطمئن لامپ</span>
                </div>
                <div className="field"><label>{fa.lampSpokes}</label>
                  <input type="number" step="1" min="3" max="12"
                    value={project.lamp?.spokes_count ?? 4}
                    onChange={(e) => setLamp({ spokes_count: Math.min(12, Math.max(3, Math.round(+e.target.value) || 4)) })} />
                  <span className="hint-text">پایه‌های متصل‌کنندهٔ سوراخ سوکت به بدنه</span>
                </div>
                <div className="field"><label>{fa.lampSpokeW}</label>
                  <input type="number" step="0.2" min="0.5" max="8"
                    value={project.lamp?.spokes_width_mm ?? 2.4}
                    onChange={(e) => setLamp({ spokes_width_mm: Math.max(0.5, +e.target.value || 2.4) })} /></div>
                <div className="field"><label>{fa.lampSpokeStyle}</label>
                  <select
                    value={project.lamp?.spokes_style ?? "Parallel"}
                    onChange={(e) => setLamp({ spokes_style: e.target.value as never })}
                  >
                    <option value="Parallel">پره‌های پهن موازی (مقاوم)</option>
                    <option value="Single">تک‌خط باریک (نور بیشتر)</option>
                    <option value="Web">شبکهٔ وب (پره + کمان)</option>
                  </select></div>
                <div className="field"><label>{fa.lampGap}</label>
                  <input type="number" step="0.1" min="0.5" max="3"
                    value={project.lamp?.floor_gap_multiplier ?? 1.6}
                    onChange={(e) => setLamp({ floor_gap_multiplier: Math.min(3, Math.max(0.5, +e.target.value || 1.6)) })} />
                  <span className="hint-text">بیشتر از ۱ = فاصلهٔ خطوط کف بیشتر ⇒ نور ملایم‌تر</span>
                </div>
                <label className="check-row">
                  <input type="checkbox"
                    checked={project.lamp?.recessed_socket ?? true}
                    onChange={(e) => setLamp({ recessed_socket: e.target.checked })} />
                  {fa.lampRecessed}
                </label>
                <span className="hint-text">حلقهٔ فرورفته دور سوکت در لایهٔ دوم کف — ظاهر حرفه‌ای</span>
                {["LampWeb", "Sunburst"].includes(project.floor.pattern ?? "Rings") && (
                  <div className="field" style={{ marginTop: 8 }}>
                    <span className="hint-text">
                      💡 الگوهای «وب لامپ» و «خورشیدشکاف» برای کف نور طراحی
                      شده‌اند — تعداد پره‌ها از همین تنظیمات لامپ خوانده می‌شود
                    </span>
                  </div>
                )}
              </>
            )}

            {/* ---- تقویت عمومی — حفظ از ۱٫۶ ---- */}
            <div className="field"><label>حلقه‌های تقویت عمومی کف (۰..۴)</label>
              <input type="number" step="1" min="0" max="4" disabled={!project.floor.enabled}
                value={project.floor.reinforce_rings ?? 0}
                onChange={(e) => setFloor({ reinforce_rings: Math.min(4, Math.max(0, Math.round(+e.target.value) || 0)) })} />
              <span className="hint-text">
                لایهٔ آخر کف ضخیم‌تر می‌شود: دور لبه + سوراخ مرکزی — استحکام باربری
              </span>
            </div>
          </div>
        )}

        {/* ================= گام ۶: پرینتر ================= */}
        {step === 6 && (
          <div>
            <p className="step-note">
              {fa.printerProfileStatus}: <strong className="ltr">{project.printer.status}</strong> —
              presetها دادهٔ ثابت نیستند؛ قبل از چاپ صحت‌سنجی کنید.
            </p>
            {/* مرکز‌گذاری روی صفحه (نسخهٔ ۱.۲ — رفع باگ 0,0) */}
            <label className="check-row">
              <input type="checkbox" checked={project.center_on_bed !== false}
                onChange={(e) => setP({ center_on_bed: e.target.checked })} />
              {fa.centerOnBed}
            </label>
            <span className="hint-text">{fa.centerOnBedHint}</span>
            {/* انتخاب از ۲۰ پریست پرینتر واقعی (وضعیت community) */}
            <div className="field">
              <label>{fa.printerPresets}</label>
              <select
                className="ltr"
                value={presets.some((x) => x.name === project.printer.name) ? project.printer.name : ""}
                onChange={(e) => {
                  const p = presets.find((x) => x.name === e.target.value);
                  if (p) commitProject({ ...project, printer: { ...p } });
                }}
              >
                <option value="">— انتخاب از فهرست —</option>
                {presets.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.name} — {p.build_volume_mm[0]}×{p.build_volume_mm[1]}×{p.build_volume_mm[2]} mm
                  </option>
                ))}
              </select>
              {presets.length > 0 && (
                <span className="hint-text">
                  {presets.length} مدل — مشخصات از صفحات رسمی سازنده‌ها؛
                  وضعیت همه community است (تست دستی نداده‌ایم).
                </span>
              )}
            </div>
            {/* نمایش نام/برند پرینتر جاری وقتی از فهرست انتخاب شده */}
            {presets.some((x) => x.name === project.printer.name) && (
              <div className="printer-badge">
                <span className="dot" aria-hidden="true" /> {project.printer.name}
              </div>
            )}
            {/* ---- پروفایل‌های شخصی — نسخهٔ ۱٫۶ (درخواست کاربر ۴) ---- */}
            <div className="field" style={{ marginTop: 10 }}>
              <label>💾 پروفایل‌های من</label>
              <span className="hint-text">
                تنظیمات فعلی پرینتر را با نام دلخواه ذخیره کنید — دفعات بعد از
                همین فهرست انتخاب می‌شود (فقط در برنامهٔ دسکتاپ).
              </span>
              <div style={{ display: "flex", gap: 4, marginTop: 4, flexWrap: "wrap" }}>
                <input
                  type="text"
                  placeholder="نام پروفایل (مثلاً: Ender سفارشی)"
                  style={{ flex: 1, minWidth: 140 }}
                  value={profileName}
                  onChange={(e) => setProfileName(e.target.value)}
                />
                <button
                  className="btn small primary"
                  disabled={profileBusy2}
                  onClick={doSavePrinterProfile}
                  title="ذخیرهٔ تنظیمات فعلی پرینتر"
                >
                  {profileBusy2 ? "…" : "ذخیرهٔ پروفایل"}
                </button>
              </div>
              {myProfiles.length > 0 ? (
                <div className="my-profiles" style={{ marginTop: 6 }}>
                  {myProfiles.map((p) => (
                    <div key={p.name} className="my-profile-row">
                      <button
                        className={`btn small ${project.printer.name === p.name ? "primary" : ""}`}
                        title={`${p.build_volume_mm[0]}×${p.build_volume_mm[1]}×${p.build_volume_mm[2]}mm`}
                        onClick={() => commitProject({ ...project, printer: { ...p } })}
                      >
                        {p.name}
                      </button>
                      <button
                        className="btn small danger"
                        title="حذف پروفایل"
                        onClick={() => doDeleteMyProfile(p.name)}
                      >×</button>
                    </div>
                  ))}
                </div>
              ) : (
                <span className="hint-text">هنوز پروفایل شخصی ذخیره نشده است.</span>
              )}
            </div>
            {/* ---- تنظیمات دما و خنک‌کاری — نسخهٔ ۱.۳ (درخواست کاربر) ---- */}
            <div className="field" style={{ marginTop: 10 }}>
              <label>{fa.tempsTitle}</label>
              <span className="hint-text">{fa.materialHint}</span>
              <div className="preset-row">
                {MATERIALS.map((m) => {
                  const active = project.printer.nozzle_temp_c === m.nozzle
                    && project.printer.bed_temp_c === m.bed
                    && (project.printer.fan_percent ?? 100) === m.fan;
                  return (
                    <button key={m.name}
                      className={`btn preset-btn small ${active ? "primary" : ""}`}
                      onClick={() => setPrinter({
                        nozzle_temp_c: m.nozzle,
                        bed_temp_c: m.bed,
                        fan_percent: m.fan,
                      })}
                    >
                      <span>{m.name}</span>
                    </button>
                  );
                })}
              </div>
            </div>
            <div className="field"><label>{fa.nozzleTemp}</label>
              <input type="number" step="5" min="0" max="500" style={{ width: 100 }}
                value={project.printer.nozzle_temp_c ?? 200}
                onChange={(e) => setPrinter({ nozzle_temp_c: +e.target.value })} /></div>
            <div className="field"><label>{fa.bedTemp}</label>
              <input type="number" step="5" min="0" max="150" style={{ width: 100 }}
                value={project.printer.bed_temp_c ?? 60}
                onChange={(e) => setPrinter({ bed_temp_c: +e.target.value })} /></div>
            <div className="field"><label>{fa.fanSpeed}</label>
              <input type="number" step="5" min="0" max="100" style={{ width: 100 }}
                value={project.printer.fan_percent ?? 100}
                onChange={(e) => setPrinter({ fan_percent: +e.target.value })} /></div>
            <label className="check-row">
              <input type="checkbox" checked={project.printer.wait_for_temps !== false}
                onChange={(e) => setPrinter({ wait_for_temps: e.target.checked })} />
              {fa.waitTemps}
            </label>
            <span className="hint-text">{fa.waitTempsHint}</span>
            <span className="hint-text">{fa.tempsZeroNote}</span>
            <div className="field"><label>{fa.nozzleDiameter}</label>
              <div style={{ display: "flex", gap: 4 }}>
                {nozzlePresets.map((n) => (
                  <button key={n} className="btn small ltr"
                    style={{ borderColor: project.printer.nozzle_diameter_mm === n ? "var(--accent)" : undefined }}
                    onClick={() => setPrinter({ nozzle_diameter_mm: n })}>{n}</button>
                ))}
                <input type="number" step="0.05" min="0.1" style={{ width: 70 }}
                  value={project.printer.nozzle_diameter_mm}
                  onChange={(e) => setPrinter({ nozzle_diameter_mm: +e.target.value })} />
              </div></div>
            <div className="field"><label>{fa.lineWidth}</label>
              <input type="number" step="0.05" value={project.printer.line_width_mm}
                onChange={(e) => setPrinter({ line_width_mm: +e.target.value })} /></div>
            <div className="field"><label>{fa.layerHeight}</label>
              <input type="number" step="0.05" min="0.05" value={project.printer.layer_height_mm}
                onChange={(e) => setPrinter({ layer_height_mm: +e.target.value })} /></div>
            {/* ---- سرعت‌ها (mm/s) — نسخهٔ ۱.۴ (درخواست کاربر) ---- */}
            <div className="field" style={{ marginTop: 10 }}>
              <label>{fa.speedsTitle}</label>
              <span className="hint-text">{fa.speedsHint}</span>
            </div>
            <div className="field"><label>{fa.wallSpeed}</label>
              <input type="number" step="5" min="1" max="300" style={{ width: 100 }}
                value={project.printer.wall_speed_mm_s
                  || Math.round(project.printer.print_speed_mm_min / 60)}
                onChange={(e) => {
                  const v = Math.max(1, +e.target.value || 1);
                  // همگام‌سازی با فیلد mm/min قدیمی (سازگاری پروژه‌های قدیمی)
                  setPrinter({ wall_speed_mm_s: v, print_speed_mm_min: v * 60 });
                }} /></div>
            <div className="field"><label>{fa.firstLayerSpeed}</label>
              <input type="number" step="5" min="1" max="150" style={{ width: 100 }}
                value={project.printer.first_layer_speed_mm_s
                  || Math.round((project.printer.wall_speed_mm_s
                    || project.printer.print_speed_mm_min / 60) * 0.5)}
                onChange={(e) => setPrinter({ first_layer_speed_mm_s: Math.max(1, +e.target.value || 1) })} /></div>
            <div className="field"><label>{fa.floorSpeed}</label>
              <input type="number" step="5" min="1" max="200" style={{ width: 100 }}
                value={project.printer.floor_speed_mm_s
                  || Math.round((project.printer.wall_speed_mm_s
                    || project.printer.print_speed_mm_min / 60) * 0.75)}
                onChange={(e) => setPrinter({ floor_speed_mm_s: Math.max(1, +e.target.value || 1) })} /></div>
            <div className="field"><label>{fa.travelSpeed}</label>
              <input type="number" step="10" min="5" max="400" style={{ width: 100 }}
                value={project.printer.travel_speed_mm_s
                  || Math.round(project.printer.travel_speed_mm_min / 60)}
                onChange={(e) => {
                  const v = Math.max(5, +e.target.value || 5);
                  setPrinter({ travel_speed_mm_s: v, travel_speed_mm_min: v * 60 });
                }} /></div>
            {/* ---- رتراکشن — نسخهٔ ۱.۴ (استاندارد G-code) ---- */}
            <div className="field"><label>{fa.retractLength}</label>
              <input type="number" step="0.1" min="0" max="10" style={{ width: 100 }}
                value={project.printer.retract_mm ?? 0.8}
                onChange={(e) => setPrinter({ retract_mm: Math.max(0, +e.target.value || 0) })} /></div>
            <div className="field"><label>{fa.retractSpeed}</label>
              <input type="number" step="5" min="5" max="120" style={{ width: 100 }}
                value={project.printer.retract_speed_mm_s ?? 40}
                onChange={(e) => setPrinter({ retract_speed_mm_s: Math.max(5, +e.target.value || 5) })} />
              <span className="hint-text">{fa.retractHint}</span>
            </div>
            <div className="field"><label>{fa.filamentDiameter}</label>
              <input type="number" step="0.05" value={project.printer.filament_diameter_mm}
                onChange={(e) => setPrinter({ filament_diameter_mm: +e.target.value })} /></div>
            <div className="field"><label>{fa.flowMultiplier}</label>
              <input type="number" step="0.05" min="0.5" max="2" value={project.printer.flow_multiplier}
                onChange={(e) => setPrinter({ flow_multiplier: +e.target.value })} /></div>
            <div className="field"><label>{fa.buildVolume}</label>
              <div className="ltr" style={{ display: "flex", gap: 4 }}>
                <input type="number" value={project.printer.build_volume_mm[0]}
                  onChange={(e) => setPrinter({ build_volume_mm: [+e.target.value, project.printer.build_volume_mm[1], project.printer.build_volume_mm[2]] })} />
                <input type="number" value={project.printer.build_volume_mm[1]}
                  onChange={(e) => setPrinter({ build_volume_mm: [project.printer.build_volume_mm[0], +e.target.value, project.printer.build_volume_mm[2]] })} />
                <input type="number" value={project.printer.build_volume_mm[2]}
                  onChange={(e) => setPrinter({ build_volume_mm: [project.printer.build_volume_mm[0], project.printer.build_volume_mm[1], +e.target.value] })} />
              </div></div>
            <div className="field"><label>{fa.absoluteE} / {fa.relativeE}</label>
              <select value={project.printer.absolute_e ? "abs" : "rel"}
                onChange={(e) => setPrinter({ absolute_e: e.target.value === "abs" })}>
                <option value="abs">M82 — {fa.absoluteE}</option>
                <option value="rel">M83 — {fa.relativeE}</option>
              </select></div>
            <div className="field"><label>{fa.startGcode} (پیش‌نمایش در خروجی)</label>
              <textarea rows={3} value={project.printer.start_gcode}
                onChange={(e) => setPrinter({ start_gcode: e.target.value })} /></div>
            {/* ---- پالت دستورات شروع — نسخهٔ ۱٫۶ (درخواست کاربر ۳) ---- */}
            <GcodeCommandPalette
              slot="start"
              onInsert={(snippet) => {
                const cur = project.printer.start_gcode.trimEnd();
                const nextGcode = cur ? cur + "\n" + snippet : snippet;
                setPrinter({ start_gcode: nextGcode });
              }}
            />
            <div className="field" style={{ marginTop: 10 }}><label>{fa.endGcode}</label>
              <textarea rows={3} value={project.printer.end_gcode}
                onChange={(e) => setPrinter({ end_gcode: e.target.value })} /></div>
            {/* ---- پالت دستورات پایان — نسخهٔ ۱٫۶ (درخواست کاربر ۳) ---- */}
            <GcodeCommandPalette
              slot="end"
              onInsert={(snippet) => {
                const cur = project.printer.end_gcode.trimEnd();
                const nextGcode = cur ? cur + "\n" + snippet : snippet;
                setPrinter({ end_gcode: nextGcode });
              }}
            />
          </div>
        )}

        {/* ---------- دکمه‌های پروژه ---------- */}
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 12 }}>
          <button className="btn small" onClick={doSaveProject}>{fa.saveProject}</button>
          <button className="btn small" onClick={doLoadProject}>{fa.loadProject}</button>
        </div>
      </div>

      {/* ---------- ناحیهٔ اصلی: پیش‌نمایش + خروجی ---------- */}
      <div className="designer-main">
        <Viewport
          segments={segPayload}
          highlightIds={new Set()}
          filter={{ mode: "all", upToId: 0 }}
          buildVolume={project.center_on_bed !== false || project.floor.enabled || step === 5
            ? [project.printer.build_volume_mm[0], project.printer.build_volume_mm[1], project.printer.build_volume_mm[2]]
            : null}
          spot={spot}
          onSpotChange={onSpotChange}
          emptyMessage={busy ? "در حال محاسبه…" : fa.openHint}
          webglMessage={fa.webglMissing}
          extrusionColor={props.extrusionColor}
          surfaceSamples={output?.radius_samples}
          wallMesh={output?.wall_mesh ?? null}
          floorDisk={project.floor.enabled
            ? { holeDiameter: project.floor.center_hole_diameter_mm }
            : null}
        />

        {/* خلاصه + هشدارها + خروجی */}
        <div style={{ padding: "10px 14px", borderTop: "1px solid var(--line)", overflowY: "auto", maxHeight: "34vh" }}>
          <div className="export-summary">
            <strong>{fa.exportSummary}:</strong>{" "}
            {note || "در حال محاسبه…"}
            <div className="note">⚠ {fa.exportExperimental} — {fa.safetyNotice}</div>
            <div className="note">{fa.stlNote}</div>
          </div>
          {warningsFa.length > 0 && (
            <ul style={{ paddingInlineStart: 18, color: "var(--warn)", fontSize: 12, margin: "4px 0" }}>
              {warningsFa.map((w, i) => <li key={i}>{w}</li>)}
            </ul>
          )}
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
            <button className="btn primary" disabled={busy || !output} onClick={exportGcode}>
              {fa.sendToEditor} / {fa.exportGcode}
            </button>
            {/* ذخیرهٔ مستقیم بدون ویرایشگر — نسخهٔ ۱.۵ (درخواست کاربر) */}
            <button className="btn" disabled={busy || !output} onClick={doSaveDirectGcode}
              title="ذخیرهٔ فایل G-code مستقیماً روی دیسک — بدون رفتن به ویرایشگر">
              💾 {fa.saveDirectGcode}
            </button>
            {/* STL → ابتدا دیالوگ کیفیت — نسخهٔ ۱.۵ (درخواست کاربر) */}
            <button className="btn" disabled={busy || !output} onClick={() => setStlDialog({ quality: 1 })}>
              {fa.exportStl}…
            </button>
          </div>
          <div className="note" style={{ marginTop: 6 }}>{fa.solidViewHint}</div>
        </div>
      </div>

      {/* ---------- دیالوگ کیفیت خروجی STL — نسخهٔ ۱.۵ ---------- */}
      {stlDialog && (
        <div className="modal-back" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>{fa.stlQualityTitle}</h3>
            <p style={{ color: "var(--fg-2)", fontSize: 12.5, marginTop: 4 }}>
              {fa.stlQualityHint}
            </p>
            <div className="stl-quality-grid">
              {([
                [0.5, fa.stlQualityDraft, "سریع‌ترین رندر؛ برای پیش‌نمایش سریع شکل"],
                [1, fa.stlQualityStandard, "تعادل ریزباشد/حجم — مناسب چاپ معمول"],
                [2, fa.stlQualityFine, "سطح نرم‌تر؛ فایل بزرگ‌تر — چاپ نمایشی"],
                [3, fa.stlQualityUltra, "بالاترین ریزباشد؛ فایل بسیار بزرگ"],
              ] as [number, string, string][]).map(([q, name, desc]) => (
                <button
                  key={q}
                  className={`stl-q-card ${stlDialog.quality === q ? "selected" : ""}`}
                  onClick={() => setStlDialog({ quality: q })}
                >
                  <span className="t">{name}</span>
                  <span className="d">{desc}</span>
                  <span className="n ltr">
                    ≈ {estimateTriangles(
                      output?.radius_samples,
                      project.form_height_mm,
                      project.printer.layer_height_mm,
                      q
                    ).toLocaleString("en-US")} مثلث
                  </span>
                </button>
              ))}
            </div>
            <div className="actions">
              <button
                className="btn primary"
                disabled={busy}
                onClick={() => exportStlWithQuality(stlDialog.quality)}
              >
                {fa.stlQualityExport} — {fa.stlQualityTriangles}:{" "}
                <span className="ltr">
                  {estimateTriangles(
                    output?.radius_samples,
                    project.form_height_mm,
                    project.printer.layer_height_mm,
                    stlDialog.quality
                  ).toLocaleString("en-US")}
                </span>
              </button>
              <button className="btn" onClick={() => setStlDialog(null)}>انصراف</button>
            </div>
          </div>
        </div>
      )}

      {/* ---------- دیالوگ کیفیت خروجی G-code — نسخهٔ ۱٫۶ (درخواست کاربر ۶) ---------- */}
      {gcodeDialog && (
        <div className="modal-back" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>کیفیت خروجی G-code</h3>
            <p style={{ color: "var(--fg-2)", fontSize: 12.5, marginTop: 4 }}>
              کیفیت، «تراکم نقاط مسیر» و «دقت اعشار مختصات» را تعیین می‌کند —
              مسیر ریزتر = سطح نرم‌تر و فایل بزرگ‌تر. پیش‌نمایش همیشه استاندارد است.
            </p>
            <div className="stl-quality-grid">
              {([
                [0.5, "زبر (پیش‌نمایشی)", "قوس ۴mm · اعشار ۲ — فایل کوچک؛ تست شکل"],
                [1, "استاندارد", "قوس ۲mm · اعشار ۳ — تعادل حجم/کیفیت (پیش‌فرض)"],
                [2, "ریز", "قوس ۱mm · اعشار ۳ — سطوح نرم‌تر؛ چاپ نمایشی"],
                [3, "بسیار ریز", "قوس ۰٫۶۷mm · اعشار ۴ — بالاترین کیفیت؛ فایل بزرگ"],
              ] as [number, string, string][]).map(([q, name, desc]) => (
                <button
                  key={q}
                  className={`stl-q-card ${gcodeDialog.quality === q ? "selected" : ""}`}
                  onClick={() => setGcodeDialog({ ...gcodeDialog, quality: q })}
                >
                  <span className="t">{name}</span>
                  <span className="d">{desc}</span>
                  <span className="n ltr">
                    ≈ {estimateGcodeLines(output?.count ?? 0, q).toLocaleString("en-US")} خط
                  </span>
                </button>
              ))}
            </div>
            <div className="actions">
              <button
                className="btn primary"
                disabled={busy}
                onClick={() => runGcodeExport(gcodeDialog.quality, gcodeDialog.save)}
              >
                {gcodeDialog.save ? "💾 ذخیرهٔ مستقیم" : "ارسال به ویرایشگر"}
                {" "}
                (کیفیت {gcodeDialog.quality})
              </button>
              <button className="btn" onClick={() => setGcodeDialog(null)}>انصراف</button>
            </div>
          </div>
        </div>
      )}

      {/* ---------- تأیید خروجی با هشدار بحرانی ---------- */}
      {confirmExport && (
        <div className="modal-back" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>هشدار پیش از خروجی</h3>
            <p style={{ color: "var(--fg-1)" }}>
              این طرح هشدار بحرانی دارد (خارج از bounds یا خطای هندسه).
              خروجی تولید شود؟
            </p>
            <p style={{ color: "var(--warn)", fontSize: 12 }}>{fa.safetyNotice}</p>
            <div className="actions">
              <button className="btn primary" onClick={() => confirmExport && confirmExport()}>
                بله، تولید کن
              </button>
              <button className="btn" onClick={() => setConfirmExport(null)}>انصراف</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
