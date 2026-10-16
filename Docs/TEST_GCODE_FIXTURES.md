# 🧪 Fixtureهای آزمون G-code

این متن‌ها دادهٔ ساختگیِ غیرحساس برای تست‌اند؛ آن‌ها را به‌عنوان فایل آمادهٔ چاپ واقعی معرفی نکن. مقادیر دما و مسیر فقط برای آزمون parser هستند.

## 1. مسیر پایه، E مطلق، retract و قوس

```gcode
; TEST FIXTURE ONLY - NOT A VERIFIED PRINT FILE
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
```

### انتظارهای تست

- commentها و metadata به‌عنوان مسیر/command تفسیر نشوند.
- `G28` بدون پروفایل فقط وضعیتی را که واقعاً معلوم است نشان دهد؛ موقعیت Home را بی‌دلیل تضمین نکند.
- کاهش E از `1.0` به `0.2` در حالت M82، retract باشد نه extrusion چاپی.
- حرکت `G1 X30 Y30` بدون تغییر E، travel باشد.
- `G2/G3` در plane XY arc بسازند و به خط مبدأ نگاشت شوند.
- `M104/M109/M140/M190` وضعیت دما را تغییر دهند و segment نسازند.
- `M117 Hello printer` به‌عنوان payload متنی بماند؛ حروف `H` و `T` داخل پیام به parameter حرکت تبدیل نشوند.

## 2. مختصات و اکستروژن نسبی

```gcode
; TEST FIXTURE ONLY
G21
G91
M83
G1 X1 Y0 Z0.2 E0.05 F600
G1 X-1 Y1 E0.10 F600
G1 X0 Y0 Z0.2 E-0.20 F1200 ; relative retract
G1 X1 E0.20 F600 ; relative unretract/positive extrusion
```

### انتظارهای تست

- در G91 مقصدها نسبت به موقعیت قبلی محاسبه شوند.
- در M83 مقدار E هر خط، delta باشد.
- `E-0.20` به‌عنوان retract نشان داده شود.
- تبدیل واحدها در mm ثابت بماند، مگر اینکه G20 وارد شود.

## 3. تغییر واحد به اینچ

```gcode
; TEST FIXTURE ONLY
G20
G90
M82
G92 X0 Y0 Z0 E0
G1 X1 Y0 Z0 E0.05 F60
G21
G1 X25.4 Y0 Z0 E0.10 F1524
```

### انتظارهای تست

- خط `X1` در حالت G20 معادل 25.4 mm باشد.
- پس از G21 مقادیر دوباره به‌صورت mm تفسیر شوند.
- نمایش مقدار خام در inspector و مقدار نرمال‌شدهٔ داخلی هر دو قابل‌مشاهده باشند.

## 4. فرمت فشرده، comment پرخطرنما، checksum

```gcode
; G1 X999 Y999 E999 -- this is only a comment
G1X10Y-2E0.42F1200
N1 G21*27
N2 G90*18
N3 M82*26
N4 G1 X10 Y10 E0.5 F1200*6
```

### انتظارهای تست

- فرمت فشردهٔ بدون فاصله parse شود.
- comment هیچ حرکت اضافه‌ای تولید نکند.
- نمونهٔ checksum از XOR استانداردِ متن پیش از `*` پیروی می‌کند؛ آزمون parser/serializer می‌تواند اعتبار آن را بسنجد.
- checksumهای fixture فقط برای آزمون‌اند، نه تضمین سازگاری با همهٔ firmwareها.

## 5. خطای نحوی و فرمان ناشناخته

```gcode
; TEST FIXTURE ONLY
G21
G90
G1 X10 Y5 E0.2 F900
G1 XNaN Y6 E0.3 F900
M900 K0.05
M117 Unknown values: X999 E999
```

### انتظارهای تست

- `XNaN` diagnostic خطای قابل‌فهم بسازد؛ متن خام حفظ شود.
- آخرین preview معتبر با برچسب «قدیمی/در انتظار اصلاح» بماند، نه اینکه مسیر غلط تازه نمایش داده شود.
- `M900` اگر در knowledge base دقیق پیاده‌سازی نشده، ناشناخته/وابسته به firmware باشد؛ معنی حدسی نساز.
- عددهای داخل متن M117 به‌عنوان مختصات/اکستروژن تفسیر نشوند.

## 6. تست‌های بدون fixture متنی ثابت

- فایل CRLF و فایل دارای UTF-8 BOM را باز و Save As کن؛ پایان‌خط و متن اصلی تا حد ممکن حفظ شوند.
- فایل دارای چند comment طولانی و خط‌های خالی را باز کن.
- فایل دارای مختصات خارج از printer profile را فقط در صورت انتخاب profile، با هشدار bounds نشان بده.
- فایل با `G2/G3` و radius نامعتبر، عدد `NaN/Infinity` یا تعداد حرکت غیرعادی را امن رد/علامت‌گذاری کن.
- فایل دارای `N...*checksum` را تغییر بده و سیاست save را بررسی کن.
