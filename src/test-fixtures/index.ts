/**
 * ============================================================================
 * test-fixtures/index.ts — fixtureهای مشترک آزمون (مطابق Docs/TEST_GCODE_FIXTURES.md)
 * ============================================================================
 * «این متن‌ها دادهٔ ساختگیِ غیرحساس برای تست‌اند؛ به‌عنوان فایل آمادهٔ چاپ
 * واقعی معرفی نشوند» (نقل از سند).
 * همین متن‌ها در آزمون‌های Rust (core tests) هم استفاده شده‌اند تا دو
 * سمت حول یک دادهٔ واحد آزموده شوند.
 * ============================================================================
 */

/** fixture ۱: مسیر پایه، E مطلق، retract و قوس */
export const FIXTURE1 = `; TEST FIXTURE ONLY - NOT A VERIFIED PRINT FILE
;LAYER_COUNT:2
G21 ; millimeters
G90 ; absolute XYZ
M82 ; absolute E
G28 ; home - position depends on selected printer profile
G92 E0
M140 S60
M104 S200
M190 S60
M109 S200
G1 Z0.2 F300
G1 X10 Y10 F6000
G1 X20 Y10 E0.5 F1200
G1 X20 Y20 E1.0
G1 E0.2 F1800 ; retract in absolute-E mode
G1 X30 Y30 F6000 ; travel
G1 E1.0 F1800 ; unretract
G1 Z0.4 F300
G1 X30 Y40 E1.8 F1200
G2 X40 Y40 I5 J0 E2.8 F1200 ; semicircle, clockwise in XY
G3 X30 Y40 I-5 J0 E3.8 F1200 ; semicircle, counter-clockwise in XY
M106 S128
M117 Hello printer
M107
M104 S0
M140 S0
M84
`;

/** fixture ۲: مختصات و اکستروژن نسبی */
export const FIXTURE2 = `; TEST FIXTURE ONLY
G21
G91
M83
G1 X1 Y0 Z0.2 E0.05 F600
G1 X-1 Y1 E0.10 F600
G1 X0 Y0 Z0.2 E-0.20 F1200 ; relative retract
G1 X1 E0.20 F600 ; relative unretract/positive extrusion
`;

/** fixture ۳: تغییر واحد به اینچ */
export const FIXTURE3 = `; TEST FIXTURE ONLY
G20
G90
M82
G92 X0 Y0 Z0 E0
G1 X1 Y0 Z0 E0.05 F60
G21
G1 X25.4 Y0 Z0 E0.10 F1524
`;

/** fixture ۴: فرمت فشرده، comment پرخطرنما، checksum */
export const FIXTURE4 = `; G1 X999 Y999 E999 -- this is only a comment
G1X10Y-2E0.42F1200
N1 G21*27
N2 G90*18
N3 M82*26
N4 G1 X10 Y10 E0.5 F1200*6
`;

/** fixture ۵: خطای نحوی و فرمان ناشناخته */
export const FIXTURE5 = `; TEST FIXTURE ONLY
G21
G90
G1 X10 Y5 E0.2 F900
G1 XNaN Y6 E0.3 F900
M900 K0.05
M117 Unknown values: X999 E999
`;

/** همه fixtureها به‌صورت یک شیء برای پیمایش آزمون‌ها */
export const ALL_FIXTURES: Record<string, string> = {
  fixture1: FIXTURE1,
  fixture2: FIXTURE2,
  fixture3: FIXTURE3,
  fixture4: FIXTURE4,
  fixture5: FIXTURE5,
};
