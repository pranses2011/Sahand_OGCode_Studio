//! ============================================================================
//! ipc.rs — منطق دامنه‌ای دستورهای IPC (خالص و قابل‌آزمون)
//! ============================================================================
//! این ماژول «منطق» هر دستور IPC را بدون وابستگی به Tauri پیاده می‌کند تا:
//!   ۱) روی هر پلتفرمی با cargo test آزمودنی باشد؛
//!   ۲) پوستهٔ Tauri فقط wrapper نازک #[tauri::command] باشد.
//!
//! الزام امنیتی Docs/PROMPT_04 §2 همچنان برقرار است: هیچ shell/eval/شبکه‌ای
//! در این ماژول وجود ندارد؛ تنها I/O فایل، نوشتن/خواندن مسیرِ انتخابی کاربر
//! (از دیالوگ native پوسته) در save/read است.
//! ============================================================================

use crate::designer::spiral::{compute, DesignerInput};
// re-export عمومی — پوستهٔ Tauri نوع را از ipc می‌خواند (قبلاً use خصوصی
// بود و بیلد ویندوزی پوسته با E0603 می‌شکست)
pub use crate::designer::spiral::DesignerOutput;
use crate::designer::{validate_project, DesignerProject};
use crate::export::{
    recompose_with_checksum, serialize_gcode, serialize_project, serialize_stl,
    validate_save, ChecksumPolicy, SaveValidation,
};
use crate::gcode::doc::{GcodeDocument, LineEnding};
use crate::gcode::interpreter::{interpret, InterpretOptions};
use crate::gcode::lexer::lex_line;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// شناسهٔ نسخهٔ parse — برای دورریختن نتایج async قدیمی پس از edit
/// (الزام PROMPT_04 §2: «generation ID/revision به parse result نسبت بده»)
static PARSE_REVISION: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// پاسخ‌های IPC — فشرده برای انتقال کارآمد (الزام: نه JSON میلیونی)
// ============================================================================

/// bundle فشردهٔ مسیر برای viewport:
/// positions: f64×3 به‌ترتیب [x0,y0,z0, x1,y1,z1, ...]
/// فقط نقاط «معلوم» می‌روند؛ نقطهٔ نامعلوم با تکرار قبلی ممتد می‌شود.
#[derive(Debug, Clone, Serialize)]
pub struct SegmentsPayload {
    /// شناسهٔ نسخه
    pub revision: u64,
    /// [x,y,z] همهٔ نقاط شروع+پایان به‌ترتیب
    pub positions: Vec<f64>,
    /// نوع هر segment: 0=Extrusion 1=Travel 2=Retract 3=Unretract 4=Arc 5=Unknown
    pub kinds: Vec<u8>,
    /// ایندکس خط مبدأ هر segment
    pub source_lines: Vec<u32>,
    /// id هر segment
    pub ids: Vec<u64>,
    /// certainty هر segment: 0=Known 1=Approximate 2=Unknown
    pub certainties: Vec<u8>,
    /// feedrate هر segment (NaN برای None — برای رنگ‌بندی سرعت)
    pub feedrates: Vec<f64>,
}

/// خروجی کامل parse برای UI
#[derive(Debug, Clone, Serialize)]
pub struct ParseResult {
    pub revision: u64,
    /// خطوط خام (برای CodeMirror)
    pub raw_lines: Vec<String>,
    /// شمارهٔ N خطوط (None → 0)
    pub n_words: Vec<u32>,
    /// نوع هر خط طبق LineType (رشتهٔ فارسی مستقیم برای badge)
    pub line_types_fa: Vec<String>,
    /// ایندکس‌های segment هر خط (flat: line_segments_offsets[i]..[i+1])
    pub line_segment_offsets: Vec<u32>,
    /// segmentهای مسیر فشرده
    pub segments: SegmentsPayload,
    /// لایه‌ها
    pub layers: Vec<LayerPayload>,
    /// توضیحات خط (سبک — فقط برای خط درخواستی کامل بازگردانده می‌شود)
    pub diagnostics: Vec<DiagPayload>,
    /// آمار
    pub stats: StatsPayload,
    /// حالت مارپیچی
    pub spiral_mode: bool,
    /// فرادادهٔ سند
    pub has_bom: bool,
    pub line_ending_fa: String,
    /// نام مبدأ برای Save As
    pub default_save_as: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayerPayload {
    pub index: u32,
    pub z_mm: Option<f64>,
    pub first_segment: u64,
    pub last_segment: u64,
    pub source_line_index: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagPayload {
    pub severity_fa: String,
    pub code: String,
    pub message_fa: String,
    pub line_index: Option<usize>,
    pub uncertain: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsPayload {
    pub segment_count: usize,
    pub extrusion_segments: usize,
    pub travel_segments: usize,
    pub retract_count: usize,
    pub extrusion_length_mm: f64,
    pub travel_length_mm: f64,
    pub total_e_mm: f64,
}

// ============================================================================
// دستور ۱: بازکردن و تفسیر کامل فایل G-code
// ============================================================================
pub fn parse_gcode(content: String, file_name: Option<String>) -> ParseResult {
    let revision = PARSE_REVISION.fetch_add(1, Ordering::SeqCst);

    // ---------- سند خام ----------
    let doc = GcodeDocument::from_text(&content, None, file_name.clone());

    // ---------- لغزنده + مفسر ----------
    let lexed: Vec<_> = doc.lines.iter().enumerate().map(|(i, l)| lex_line(l, i)).collect();
    let interp = interpret(&lexed, &InterpretOptions::default());

    // ---------- بسته‌بندی فشردهٔ segmentها ----------
    let mut positions = Vec::with_capacity(interp.segments.len() * 6);
    let mut kinds = Vec::with_capacity(interp.segments.len());
    let mut source_lines = Vec::with_capacity(interp.segments.len());
    let mut ids = Vec::with_capacity(interp.segments.len());
    let mut certainties = Vec::with_capacity(interp.segments.len());
    let mut feedrates = Vec::with_capacity(interp.segments.len());
    for s in &interp.segments {
        // نقاط نامعلوم با تکرار نقطهٔ قبلی ممتد می‌شوند (viewport پیوسته می‌ماند)
        let start = s.start_mm.unwrap_or([0.0, 0.0, 0.0]);
        let end = s.end_mm.unwrap_or(start);
        positions.extend_from_slice(&start);
        positions.extend_from_slice(&end);
        kinds.push(match s.kind {
            crate::toolpath::SegmentKind::Extrusion => 0,
            crate::toolpath::SegmentKind::Travel => 1,
            crate::toolpath::SegmentKind::Retract => 2,
            crate::toolpath::SegmentKind::Unretract => 3,
            crate::toolpath::SegmentKind::Arc => 4,
            crate::toolpath::SegmentKind::Unknown => 5,
        });
        source_lines.push(s.source_line_index as u32);
        ids.push(s.id);
        certainties.push(match s.certainty {
            crate::toolpath::Certainty::Known => 0,
            crate::toolpath::Certainty::Approximate => 1,
            crate::toolpath::Certainty::Unknown => 2,
        });
        feedrates.push(s.feedrate_mm_min.unwrap_or(f64::NAN));
    }

    // ---------- نگاشت خط→segment ----------
    let mut line_segment_offsets: Vec<u32> = Vec::with_capacity(interp.lines.len() + 1);
    let mut acc: u32 = 0;
    line_segment_offsets.push(0);
    for l in &interp.lines {
        acc += l.segment_indices.len() as u32;
        line_segment_offsets.push(acc);
    }

    ParseResult {
        revision,
        raw_lines: doc.lines.clone(),
        n_words: lexed.iter().map(|l| l.n_word.unwrap_or(0)).collect(),
        line_types_fa: interp.lines.iter().map(|l| l.line_type.fa().to_string()).collect(),
        line_segment_offsets,
        segments: SegmentsPayload {
            revision,
            positions,
            kinds,
            source_lines,
            ids,
            certainties,
            feedrates,
        },
        layers: interp
            .layers
            .iter()
            .map(|lay| LayerPayload {
                index: lay.index,
                z_mm: lay.z_mm,
                first_segment: lay.first_segment,
                last_segment: lay.last_segment,
                source_line_index: lay.source_line_index,
            })
            .collect(),
        diagnostics: interp
            .diagnostics
            .iter()
            .map(|d| DiagPayload {
                severity_fa: match d.severity {
                    crate::diagnostics::Severity::Error => "خطا".into(),
                    crate::diagnostics::Severity::Warning => "هشدار".into(),
                    crate::diagnostics::Severity::Info => "اطلاعات".into(),
                },
                code: d.code.clone(),
                message_fa: d.message_fa.clone(),
                line_index: d.line_index,
                uncertain: d.uncertain,
            })
            .collect(),
        stats: StatsPayload {
            segment_count: interp.stats.segment_count,
            extrusion_segments: interp.stats.extrusion_segments,
            travel_segments: interp.stats.travel_segments,
            retract_count: interp.stats.retract_count,
            extrusion_length_mm: interp.stats.extrusion_length_mm,
            travel_length_mm: interp.stats.travel_length_mm,
            total_e_mm: interp.stats.total_e_mm,
        },
        spiral_mode: interp.spiral_mode,
        has_bom: doc.has_bom,
        line_ending_fa: match doc.line_ending {
            LineEnding::Lf => "LF (یونیکس)".into(),
            LineEnding::Crlf => "CRLF (ویندوز)".into(),
            LineEnding::Mixed => "مخلوط".into(),
        },
        default_save_as: doc.default_save_as_name(),
    }
}

// ============================================================================
// دستور ۲: توضیح کامل یک خط (inspector)
// ============================================================================
#[derive(Debug, Clone, Serialize)]
pub struct LineDetail {
    /// متن خام
    pub raw: String,
    pub line_index: usize,
    pub line_no: usize,
    /// نوع فارسی
    pub line_type_fa: String,
    /// فرمان‌ها با پارامترها و شرح
    pub commands: Vec<CommandDetail>,
    /// segmentهای این خط (ids برای برجسته‌سازی)
    pub segment_ids: Vec<u64>,
    /// وضعیت قبل/بعد
    pub state_before: serde_json::Value,
    pub state_after: serde_json::Value,
    /// ΔE
    pub e_delta: Option<f64>,
    /// کامنت
    pub comment: Option<String>,
    /// N و checksum
    pub n_word: Option<u32>,
    pub checksum_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandDetail {
    pub code: String,
    pub summary_fa: String,
    pub action_fa: String,
    pub params: Vec<ParamDetail>,
    pub firmware_dependent: bool,
    pub unknown: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamDetail {
    pub letter: char,
    pub raw_value: String,
    pub normalized: Option<f64>,
    pub meaning_fa: String,
}

/// توضیح کامل خط i را از متن کامل بازمی‌گرداند
/// (متن کامل برای state modal از ابتدا لازم است — بازتفسیر سریع است)
pub fn explain_line(content: String, line_index: usize) -> Option<LineDetail> {
    let doc = GcodeDocument::from_text(&content, None, None);
    if line_index >= doc.lines.len() {
        return None;
    }
    let lexed: Vec<_> = doc.lines.iter().enumerate().map(|(i, l)| lex_line(l, i)).collect();
    let interp = interpret(&lexed, &InterpretOptions::default());
    interp.lines.get(line_index).map(|l| LineDetail {
        raw: lexed.get(line_index).map(|x| x.raw.clone()).unwrap_or_default(),
        line_index: l.line_index,
        line_no: l.file_line_no,
        line_type_fa: l.line_type.fa().to_string(),
        commands: l
            .commands
            .iter()
            .map(|c| CommandDetail {
                code: c.code.clone(),
                summary_fa: c.summary_fa.clone(),
                action_fa: c.action_fa.clone(),
                params: c
                    .params
                    .iter()
                    .map(|p| ParamDetail {
                        letter: p.letter,
                        raw_value: p.raw_value.clone(),
                        normalized: p.normalized,
                        meaning_fa: p.meaning_fa.clone(),
                    })
                    .collect(),
                firmware_dependent: c.firmware_dependent,
                unknown: c.unknown,
            })
            .collect(),
        segment_ids: l
            .segment_indices
            .iter()
            .map(|&i| interp.segments[i].id)
            .collect(),
        state_before: serde_json::to_value(&l.state_before).unwrap_or_default(),
        state_after: serde_json::to_value(&l.state_after).unwrap_or_default(),
        e_delta: l.e_delta,
        comment: lexed.get(line_index).and_then(|x| x.comment.clone()),
        n_word: lexed.get(line_index).and_then(|x| x.n_word),
        checksum_valid: lexed.get(line_index).and_then(|x| x.checksum_valid),
    })
}

// ============================================================================
// دستور ۳: ذخیره با نام جدید (Save As) — هرگز فایل مبدأ را بازنویسی نمی‌کند
// ============================================================================
#[derive(Debug, Clone, Deserialize)]
pub struct SaveAsRequest {
    /// متن کامل (شامل ویرایش‌ها)
    pub content: String,
    /// متن مبدأ برای تشخیص BOM/پایان‌خط
    pub original_content: String,
    /// مسیر مقصد انتخابی کاربر از دیالوگ
    pub path: String,
    /// سیاست checksum
    pub checksum_policy: String, // "recompute" | "strip" | "keep"
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveAsResult {
    pub bytes_written: usize,
    pub checksum_policy_fa: String,
}

pub fn save_as(req: SaveAsRequest) -> Result<SaveAsResult, String> {
    // ---------- بازتولید BOM/پایان‌خط طبق مبدأ ----------
    let orig_doc = GcodeDocument::from_text(&req.original_content, None, None);
    let mut doc = GcodeDocument::from_text(&req.content, None, None);
    doc.has_bom = orig_doc.has_bom;
    doc.line_ending = orig_doc.line_ending;

    // ---------- سیاست checksum ----------
    // "keep": متن همان‌طور که هست (خطاهای checksum در validation هشدار شد)
    let final_text = match req.checksum_policy.as_str() {
        "strip" => recompose_with_checksum(&doc.to_text(), ChecksumPolicy::Strip),
        "recompute" => recompose_with_checksum(&doc.to_text(), ChecksumPolicy::Recompute),
        _ => doc.to_text(),
    };
    let policy_fa = match req.checksum_policy.as_str() {
        "strip" => "N و checksum حذف شدند (با تأیید شما)".to_string(),
        "recompute" => "شماره‌های N و checksum‌ها بازمحاسبه شدند".to_string(),
        _ => "ساختار N/checksum دست‌نخورده حفظ شد".to_string(),
    };

    // ---------- نوشتن فایل (فقط مسیر انتخابی کاربر) ----------
    std::fs::write(&req.path, final_text.as_bytes())
        .map_err(|e| format!("نوشتن فایل ناموفق بود: {e}"))?;
    Ok(SaveAsResult {
        bytes_written: final_text.len(),
        checksum_policy_fa: policy_fa,
    })
}

// ============================================================================
// دستور ۴: اعتبارسنجی پیش از ذخیره
// ============================================================================
pub fn validate_gcode(content: String, build_volume: Option<[f64; 3]>) -> SaveValidation {
    validate_save(&content, build_volume)
}

// ============================================================================
// دستور ۵: محاسبهٔ طراح مارپیچی
// ============================================================================
pub fn designer_compute(project: DesignerProject) -> DesignerOutput {
    let input = DesignerInput {
        profile: project.profile.clone(),
        form: crate::designer::profile::FormAdjust {
            height_mm: project.form_height_mm,
            bottom_diameter_mm: project.form_bottom_diameter_mm,
            top_diameter_mm: project.form_top_diameter_mm,
            taper_percent: project.taper_percent,
            smoothing_passes: 0,
            interpolation: project.profile_interpolation,
        },
        patterns: project.patterns.clone(),
        textures: project.textures.clone(),
        floor: project.floor.clone(),
        center_on_bed: project.center_on_bed,
        phase_start_deg: project.phase_start_deg,
        printer: project.printer.clone(),
    };
    compute(&input)
}

// ============================================================================
// دستور ۵-الف: پیلود سریع پیش‌نمایش طراح — نسخهٔ ۱.۴ (کارایی نمایشگر 3D)
// ============================================================================
// مشکل: designer_compute خروجی کامل (segments شیءمحور + mesh) سریال می‌کند؛
// برای یک گلدان ۱۰۰mm با بافت = ده‌ها هزار segment ⇒ چند MB JSON در هر
// تغییر اسلایدر ⇒ لگ/وقفهٔ محسوس نمایشگر.
// راه‌حل: پیلود تختِ حداقلی برای viewport — آرایه‌های عددی، مختصات گردشده
// به ۳ رقم اعصاری (دقت نمایش)، بدون mesh و بدون id/سطر مبدأ (ویرایشگر
// نیست). خلاصه/آمار/هشدار/نمونه‌های شعاع برای اسپات هم می‌آید.
/// پیلود فشردهٔ viewport طراح
#[derive(Debug, Clone, Serialize)]
pub struct FastViewport {
    /// شناسهٔ نسخه
    pub revision: u32,
    /// تعداد segmentها
    pub count: usize,
    /// مختصات تخت: [x0,y0,z0, x1,y1,z1] × n (گردشده به ۰٫۰۰۱mm)
    pub positions: Vec<f64>,
    /// نوع هر segment: 0=Extrusion 1=Travel 2=Retract 3=Unretract 4=Arc 5=Unknown
    pub kinds: Vec<u8>,
    /// سرعت هر segment (mm/min — برای رنگ‌بندی سرعت آینده)
    pub feedrates: Vec<f64>,
    /// نمونه‌های شعاع (z, r) برای اسپات UI بدون mesh
    pub radius_samples: Vec<[f64; 2]>,
    /// خلاصهٔ فارسی
    pub summary_fa: String,
    /// آمار مسیر
    pub stats: crate::toolpath::ToolpathStats,
    /// هشدارها
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

/// گردکردن به ۳ رقم اعصاری (دقت نمایش/G-code) — JSON کوچک‌تر و سریع‌تر
#[inline]
fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// محاسبهٔ طراح + فشرده‌سازی فوری برای viewport
pub fn designer_viewport(project: DesignerProject) -> FastViewport {
    use crate::toolpath::SegmentKind;
    let out = designer_compute(project);
    let n = out.segments.len();
    let mut positions = Vec::with_capacity(n * 6);
    let mut kinds = Vec::with_capacity(n);
    let mut feedrates = Vec::with_capacity(n);
    for s in &out.segments {
        let st = s.start_mm.unwrap_or([0.0; 3]);
        let en = s.end_mm.unwrap_or(st);
        positions.extend_from_slice(&[r3(st[0]), r3(st[1]), r3(st[2])]);
        positions.extend_from_slice(&[r3(en[0]), r3(en[1]), r3(en[2])]);
        kinds.push(match s.kind {
            SegmentKind::Extrusion => 0,
            SegmentKind::Travel => 1,
            SegmentKind::Retract => 2,
            SegmentKind::Unretract => 3,
            SegmentKind::Arc => 4,
            SegmentKind::Unknown => 5,
        });
        feedrates.push(s.feedrate_mm_min.unwrap_or(0.0));
    }
    FastViewport {
        revision: 1,
        count: n,
        positions,
        kinds,
        feedrates,
        radius_samples: out.radius_samples,
        summary_fa: out.summary_fa,
        stats: out.stats,
        diagnostics: out.diagnostics,
    }
}

// ============================================================================
// دستور ۵-ب: دادهٔ پیش‌نمایش الگو/بافت برای کارت‌های تصویری UI
// ============================================================================
// هر الگو/بافت روی یک استوانهٔ نرمال‌شده (ارتفاع ۱، شعاع ۱) با پارامترهای
// استاندارد ارزیابی می‌شود؛ UI از همین مقادیر «واقعی هسته» بندانگشتی می‌سازد
// (قاعدهٔ صداقت: تصویر کارت = همان ریاضی خروجی، نه آیکون تزئینی).

/// شبکهٔ مقادیر یک الگو/بافت — ردیف‌محور [rows × cols]
#[derive(Debug, Clone, Serialize)]
pub struct PreviewGrid {
    pub rows: usize,
    pub cols: usize,
    /// مقادیر نرمال [−۱..۱] — انحراف شعاعی در (z, θ)
    pub values: Vec<f64>,
}

/// پاسخ کامل دستور پیش‌نمایش
#[derive(Debug, Clone, Serialize)]
pub struct PreviewData {
    /// کلید = نام variant الگو (Wave و…)
    pub patterns: std::collections::BTreeMap<String, PreviewGrid>,
    /// کلید = نام variant بافت (Ripple و…)
    pub textures: std::collections::BTreeMap<String, PreviewGrid>,
}

/// ارزیابی همهٔ الگوها روی شبکهٔ استاندارد
fn eval_patterns_grid() -> std::collections::BTreeMap<String, PreviewGrid> {
    use crate::designer::pattern::{apply_patterns, PatternKind};
    let rows = 56usize;
    let cols = 112usize;
    let mut map = std::collections::BTreeMap::new();
    for kind in [
        PatternKind::Wave, PatternKind::Petal, PatternKind::Twist, PatternKind::Flute,
        PatternKind::Diamond, PatternKind::Chevron, PatternKind::Rings,
        PatternKind::Scale, PatternKind::Facet, PatternKind::Braid,
        PatternKind::Pineapple, PatternKind::Lotus, PatternKind::TwistedFlute,
        PatternKind::Pearl, PatternKind::Meander, PatternKind::Brick,
        PatternKind::Spikes,
    ] {
        // پارامترهای نمایشی استاندارد — همان پیش‌فرض UI
        let params = crate::designer::pattern::PatternParams {
            kind,
            amplitude_mm: 1.0,
            count: 8,
            frequency: 1.0,
            phase_deg: 0.0,
            z_start_mm: 0.0,
            z_end_mm: None,
        };
        let mut values = Vec::with_capacity(rows * cols);
        for i in 0..rows {
            let z = i as f64 / (rows as f64 - 1.0);
            for j in 0..cols {
                let theta = j as f64 * 2.0 * std::f64::consts::PI / cols as f64;
                let (r, _) = apply_patterns(1.0, z, theta, 1.0, &[params.clone()]);
                // انحراف از شعاع پایهٔ ۱ → [−۱..۱] تقریبی
                values.push((r - 1.0).clamp(-1.0, 1.0));
            }
        }
        map.insert(format!("{kind:?}"), PreviewGrid { rows, cols, values });
    }
    map
}

/// ارزیابی همهٔ بافت‌ها روی مسیر مارپیچی نرمال‌شده
fn eval_textures_grid() -> std::collections::BTreeMap<String, PreviewGrid> {
    use crate::designer::texture::{apply_texture, TextureKind};
    let rows = 56usize;
    let cols = 112usize;
    let mut map = std::collections::BTreeMap::new();
    for kind in [
        TextureKind::Ripple, TextureKind::Weave, TextureKind::Stitch,
        TextureKind::SpiralGroove, TextureKind::Hammered, TextureKind::Basket,
        TextureKind::WoodGrain, TextureKind::Leather, TextureKind::Knurl,
        TextureKind::Ocean, TextureKind::Bubbles, TextureKind::Rows,
    ] {
        let t = crate::designer::texture::TextureParams {
            kind,
            amplitude_mm: 1.0,
            period_mm: 4.0,
            radial: true,
            phase_deg: 0.0,
            fade_in_percent: 0.0,
            fade_out_percent: 0.0,
        };
        let mut values = Vec::with_capacity(rows * cols);
        // مسیر مارپیچی نمونه: θ یکنواخت، z یکنواخت، s متناسب
        for i in 0..rows {
            let z_norm = i as f64 / (rows as f64 - 1.0);
            for j in 0..cols {
                let theta = j as f64 * 2.0 * std::f64::consts::PI / cols as f64;
                // طول مسیر تا این نقطه (شعاع ۱): تقریب خطی
                let s = i as f64 * cols as f64 * (2.0 * std::f64::consts::PI / cols as f64);
                let point = [theta.cos(), theta.sin(), z_norm];
                let (out, _) = apply_texture(point, s, 1.0, z_norm, &[t.clone()]);
                let r = (out[0] * out[0] + out[1] * out[1]).sqrt();
                values.push((r - 1.0).clamp(-1.0, 1.0));
            }
        }
        map.insert(format!("{kind:?}"), PreviewGrid { rows, cols, values });
    }
    map
}

/// دستور IPC: دادهٔ پیش‌نمایش برای کارت‌های الگو/بافت UI
pub fn designer_preview_data() -> PreviewData {
    PreviewData {
        patterns: eval_patterns_grid(),
        textures: eval_textures_grid(),
    }
}

// ============================================================================
// دستور ۶: خروجی G-code از طراح
// ============================================================================
pub fn designer_export_gcode(project: DesignerProject) -> Result<String, String> {
    // اعتبارسنجی پروژه پیش از تولید
    let proj_json = serde_json::to_string(&project).map_err(|e| e.to_string())?;
    let validated = validate_project(&proj_json)?;

    let output = designer_compute(validated.clone());
    Ok(serialize_gcode(
        &DesignerInput {
            profile: validated.profile,
            form: crate::designer::profile::FormAdjust {
                height_mm: validated.form_height_mm,
                bottom_diameter_mm: validated.form_bottom_diameter_mm,
                top_diameter_mm: validated.form_top_diameter_mm,
                taper_percent: validated.taper_percent,
                smoothing_passes: 0,
                interpolation: validated.profile_interpolation,
            },
            patterns: validated.patterns,
            textures: validated.textures,
            floor: validated.floor,
            center_on_bed: validated.center_on_bed,
            phase_start_deg: validated.phase_start_deg,
            printer: validated.printer,
        },
        &output,
    ))
}

// ============================================================================
// دستور ۷: خروجی STL از طراح
// ============================================================================
#[derive(Debug, Clone, Serialize)]
pub struct StlExportResult {
    /// بایت‌های STL باینری (base64 نمی‌کنیم؛ Tauri آرایهٔ عددی را می‌برد
    /// ولی برای حجم کم، مسیر مستقیم نوشتن بهتر است)
    pub byte_count: usize,
    /// مسیر نوشته‌شده
    pub path: String,
    /// مثلث‌ها
    pub triangle_count: usize,
}

pub fn designer_export_stl(project: DesignerProject, path: String) -> Result<StlExportResult, String> {
    let proj_json = serde_json::to_string(&project).map_err(|e| e.to_string())?;
    let validated = validate_project(&proj_json)?;

    let output = designer_compute(validated);
    let stl = serialize_stl(&output.mesh);
    std::fs::write(&path, &stl).map_err(|e| format!("نوشتن STL ناموفق بود: {e}"))?;
    Ok(StlExportResult {
        byte_count: stl.len(),
        path,
        triangle_count: output.mesh.triangles.len(),
    })
}

// ============================================================================
// دستور ۸: ذخیرهٔ پروژهٔ طراح
// ============================================================================
pub fn save_project(project: DesignerProject, path: String) -> Result<usize, String> {
    let json = serialize_project(&project);
    std::fs::write(&path, json.as_bytes()).map_err(|e| format!("ذخیرهٔ پروژه ناموفق بود: {e}"))?;
    Ok(json.len())
}

// ============================================================================
// دستور ۹: بارگذاری پروژهٔ طراح با اعتبارسنجی ایمن
// ============================================================================
pub fn load_project(path: String) -> Result<DesignerProject, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("خواندن پروژه ناموفق بود: {e}"))?;
    validate_project(&content) // schema ناشناخته/خراب → Err بدون خراب‌کردن جاری
}

// ============================================================================
// دستور ۱۰: خواندن فایل انتخابی از دیالوگ (فقط مسیر دیالوگ)
// ============================================================================
pub fn read_text_file(path: String) -> Result<String, String> {
    // خواندن با UTF-8 و مدیریت خطای واضح؛ فایل بزرگ با progress در نسخهٔ
    // بعدی — فعلاً سقف ۲۵۶MB برای جلوگیری از OOM
    let meta = std::fs::metadata(&path).map_err(|e| format!("دسترسی به فایل ناموفق بود: {e}"))?;
    if meta.len() > 256 * 1024 * 1024 {
        return Err("فایل بزرگ‌تر از ۲۵۶ مگابایت است؛ در این نسخه پشتیبانی نمی‌شود".into());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("خواندن فایل ناموفق بود: {e}"))
}

// ============================================================================
// دستور ۱۱: پریست‌های پرینتر واقعی (گام ۵) — فقط خواندنی و بدون state
// ============================================================================
pub fn printer_presets() -> Vec<crate::designer::PrinterProfile> {
    crate::designer::printer_presets()
}

// ============================================================================
// آزمون‌های داخلی IPC — داده‌های فشرده و رفتار Save As
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gcode_payload() {
        // payload فشرده باید با تعداد segment هم‌خوان باشد
        let content = "G21\nG90\nG28\nG1 X10 Y10 Z0.2 F3000\nG1 X20 Y20 E1.0 F1200\n";
        let r = parse_gcode(content.to_string(), Some("test.gcode".into()));
        // دو segment: travel + extrusion
        assert_eq!(r.segments.kinds.len(), 2);
        assert_eq!(r.segments.positions.len(), 12); // 2×6
        assert_eq!(r.segments.source_lines, vec![3, 4]);
        // revision صعودی
        let r2 = parse_gcode(content.to_string(), None);
        assert!(r2.revision > r.revision);
        // نام Save As
        assert_eq!(r.default_save_as, "test-edited.gcode");
        // نوع خطوط فارسی
        assert!(r.line_types_fa.contains(&"حرکت چاپی".to_string()));
    }

    #[test]
    fn test_explain_line_full() {
        let content = "G21\nM104 S200\nG1 X10 Y10 E0.5 F1200\n";
        // خط دما: state تغییر می‌کند ولی segment ندارد
        let d = explain_line(content.into(), 1).unwrap();
        assert_eq!(d.line_type_fa, "دما");
        assert!(d.segment_ids.is_empty());
        assert!(d.commands[0].action_fa.contains("200"));
        // خط حرکتی: segment دارد
        let m = explain_line(content.into(), 2).unwrap();
        assert_eq!(m.segment_ids.len(), 1);
        assert_eq!(m.commands[0].code, "G1");
        // خط ناموجود
        assert!(explain_line(content.into(), 99).is_none());
    }

    #[test]
    fn test_save_as_writes_file() {
        // Save As واقعاً می‌نویسد و بازتولید پایان‌خط را رعایت می‌کند
        let dir = std::env::temp_dir().join("gcs_test_save");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.gcode").to_str().unwrap().to_string();

        let req = SaveAsRequest {
            content: "G21\nG1 X10\n".into(),
            original_content: "G21\r\nG1 X10\r\n".into(), // CRLF مبدأ
            path: path.clone(),
            checksum_policy: "keep".into(),
        };
        let res = save_as(req).unwrap();
        assert!(res.bytes_written > 0);
        let written = std::fs::read_to_string(&path).unwrap();
        // پایان‌خط CRLF مبدأ بازتولید شد
        assert!(written.contains("\r\n"));
        // پاکسازی
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_designer_compute_command() {
        let proj = DesignerProject::default();
        let out = designer_compute(proj);
        assert!(out.segments.len() > 50);
        assert!(!out.mesh.triangles.is_empty());
    }

    #[test]
    fn test_designer_viewport_fast_payload() {
        // پیلود سریع — نسخهٔ ۱.۴: سازگاری شمارش، انواع، دقت گردکردن،
        // نمونه‌های شعاع و خلاصه — و «بدون mesh» (کارایی)
        let mut proj = DesignerProject::default();
        proj.floor.enabled = true;
        proj.floor.layers = 2;
        let full = designer_compute(proj.clone());
        let fast = designer_viewport(proj);
        assert_eq!(fast.count, full.segments.len());
        assert_eq!(fast.positions.len(), fast.count * 6);
        assert_eq!(fast.kinds.len(), fast.count);
        assert_eq!(fast.feedrates.len(), fast.count);
        // kinds صرفاً 0..5
        assert!(fast.kinds.iter().all(|k| *k <= 5));
        // دقت ۳ رقم: فاصلهٔ تا نزدیک‌ترین عدد صحیح پس از ×۱۰۰۰ ناچیز است
        assert!(fast.positions.iter().all(|p| {
            let s = p * 1000.0;
            (s - s.round()).abs() < 1e-6
        }));
        // travel و extrusion هر دو حضور دارند (کف فعال است)
        assert!(fast.kinds.contains(&1), "باید travel باشد");
        assert!(fast.kinds.contains(&0), "باید extrusion باشد");
        // نمونه‌های شعاع و خلاصه (برای اسپات UI)
        assert_eq!(fast.radius_samples.len(), 129);
        assert!(fast.summary_fa.contains("مارپیچی"));
        assert_eq!(fast.stats.extrusion_segments, full.stats.extrusion_segments);
        // سرعت‌ها (mm/min): travel=6000، لایهٔ اول کف=1200،
        // لایهٔ ۲ کف=1800، دیواره=2400 — همه در feedrates حضور دارند
        let travel_f = fast
            .kinds
            .iter()
            .zip(fast.feedrates.iter())
            .find(|(k, _)| **k == 1)
            .map(|(_, f)| *f)
            .unwrap();
        assert!((travel_f - 6000.0).abs() < 1e-6, "travel = {travel_f}");
        assert!(fast.feedrates.iter().any(|f| (f - 1200.0).abs() < 1e-6), "لایهٔ اول");
        assert!(fast.feedrates.iter().any(|f| (f - 1800.0).abs() < 1e-6), "لایهٔ ۲ کف");
        assert!(fast.feedrates.iter().any(|f| (f - 2400.0).abs() < 1e-6), "دیواره");
    }

    #[test]
    fn test_designer_export_gcode() {
        let proj = DesignerProject::default();
        let g = designer_export_gcode(proj).unwrap();
        assert!(g.contains("G21"));
        assert!(g.contains("EXPERIMENTAL"));
        assert!(g.contains("G1 X"));
    }
}
