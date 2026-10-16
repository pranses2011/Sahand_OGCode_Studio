//! ============================================================================
//! gcode/interpreter.rs — مفسر وضعیت modal ماشین و تولید Toolpath IR
//! ============================================================================
//! ورودی: خروجی لغزنده (LexedLine ها)
//! خروجی: Interpretation شامل:
//!   - segmentهای مسیر (با نگاشت به خط مبدأ)؛
//!   - وضعیت ماشین قبل/بعد هر خط (برای پنل توضیح)؛
//!   - لایه‌ها (با تشخیص حالت مارپیچی)؛
//!   - Diagnostic های فارسی؛
//!   - آمار مسیر.
//!
//! قواعد کلان (از Docs/PROMPT_02 §4):
//!   - وضعیت modal از خطی به خط بعد منتقل می‌شود: واحد، مطلق/نسبی XYZ و E،
//!     صفحهٔ قوس، سرعت، دما/فن/ابزار.
//!   - در E مطلق، ΔE = E فرمان − شمارندهٔ قبلی؛ در E نسبی ΔE = مقدار فرمان.
//!   - G92 شمارنده را «ادعا» می‌کند و segment نمی‌سازد.
//!   - G28 موقعیت را نامعلوم می‌کند (بدون پروفایل، home تضمین نمی‌شود).
//!   - موقعیت نامعلوم هرگز بی‌صدا صفر نمی‌شود؛ با Certainty برچسب می‌خورد.
//!   - حرکت خالص E مثبت = unretract، منفی = retract؛ حرکت با ΔE>0 = چاپ.
//!   - لایه‌بندی از تغییر گسستهٔ Z حرکت‌های چاپی؛ در spiral پیوسته، لایهٔ
//!     گسسته ساخته نمی‌شود (الزام: «لایه را به‌زور نساز»).
//! ============================================================================

use crate::diagnostics::Diagnostic;
use crate::gcode::lexer::{LexedLine, LexedWord, LineKind};
use crate::knowledge::{self, CommandFamily, CommandInfo};
use crate::toolpath::arcs::{tessellate_arc, ArcError, ArcParams, Plane};
use crate::toolpath::{
    Certainty, LayerInfo, SegmentKind, ToolpathSegment, ToolpathStats,
};
use serde::{Deserialize, Serialize};

/// گزینه‌های تفسیر — از UI قابل‌تنظیم
#[derive(Debug, Clone)]
pub struct InterpretOptions {
    /// تلورانس تبدیل قوس به خط (mm)
    pub arc_tolerance_mm: f64,
    /// سقف قطعات هر قوس
    pub arc_max_segments: usize,
    /// حجم ساخت پرینتر اگر پروفایل انتخاب شده باشد: [x,y,z] به mm
    /// None یعنی بدون پروفایل → هیچ بازرسی bounds انجام نمی‌شود (الزام:
    /// «حدود را جعل نکن»)
    pub build_volume_mm: Option<[f64; 3]>,
}

impl Default for InterpretOptions {
    fn default() -> Self {
        Self {
            arc_tolerance_mm: 0.05,
            arc_max_segments: 1000,
            build_volume_mm: None,
        }
    }
}

/// علامت واحد فعال
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Units {
    Mm,
    Inch,
}

/// وضعیت کامل ماشین — داخلی؛ snapshot سریال‌شونده برای UI ساخته می‌شود
#[derive(Debug, Clone)]
struct MachineState {
    units: Units,
    absolute_xyz: bool,
    absolute_e: bool,
    plane: Plane,
    /// سه محور با مقدار اختیاری و درجهٔ اطمینان
    x: Axis,
    y: Axis,
    z: Axis,
    /// شمارندهٔ E (mm فیلامنت)
    e_counter: Option<f64>,
    /// آیا مبنای E معتبر است (پس از G92 E یا حالت نسبی)
    e_known: bool,
    /// آخرین سرعت (mm/min — همیشه نرمال‌شده)
    feedrate: Option<f64>,
    nozzle_temp_c: Option<f64>,
    bed_temp_c: Option<f64>,
    fan_255: Option<u16>,
    tool: u8,
    /// ضریب سرعت M220 (درصد) — فقط نمایش؛ روی هندسه اثر ندارد
    feedrate_override_percent: Option<u32>,
}

/// یک محور با نشانگر اطمینان — «معلوم / تقریبی / نامعلوم»
#[derive(Debug, Clone, Copy)]
struct Axis {
    value: Option<f64>,
    certainty: Certainty,
}

impl Axis {
    fn initial() -> Self {
        // وضعیت آغازین: فرض نمایش (0) با برچسب «تقریبی» — بی‌صدا دقیق نیست
        Self { value: Some(0.0), certainty: Certainty::Approximate }
    }
    fn unknown() -> Self {
        Self { value: None, certainty: Certainty::Unknown }
    }
}

impl MachineState {
    fn new() -> Self {
        Self {
            units: Units::Mm,
            absolute_xyz: true,   // G90 رایج‌ترین حالت آغازین
            absolute_e: true,     // M82
            plane: Plane::Xy,
            x: Axis::initial(),
            y: Axis::initial(),
            z: Axis::initial(),
            e_counter: None,
            e_known: false,
            feedrate: None,
            nozzle_temp_c: None,
            bed_temp_c: None,
            fan_255: None,
            tool: 0,
            feedrate_override_percent: None,
        }
    }

    /// تبدیل مقدار ورودی به mm بر اساس واحد فعال
    fn to_mm(&self, v: f64) -> f64 {
        match self.units {
            Units::Mm => v,
            Units::Inch => v * 25.4,
        }
    }

    /// نقطهٔ فعلی اگر هر سه محور مقدار دارند
    fn position(&self) -> Option<[f64; 3]> {
        Some([self.x.value?, self.y.value?, self.z.value?])
    }

    /// بدترین اطمینان سه محور (برای Certainty سگمنت)
    fn position_certainty(&self) -> Certainty {
        let c = [
            self.x.certainty,
            self.y.certainty,
            self.z.certainty,
        ];
        if c.iter().any(|&c| c == Certainty::Unknown) {
            Certainty::Unknown
        } else if c.iter().any(|&c| c == Certainty::Approximate) {
            Certainty::Approximate
        } else {
            Certainty::Known
        }
    }
}

/// اسnapshot سریال‌شوندهٔ وضعیت برای پنل توضیح خط
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub units_fa: String,
    pub absolute_xyz: bool,
    pub absolute_e: bool,
    pub plane_fa: String,
    pub x_mm: Option<f64>,
    pub y_mm: Option<f64>,
    pub z_mm: Option<f64>,
    pub e_mm: Option<f64>,
    pub feedrate_mm_min: Option<f64>,
    pub nozzle_temp_c: Option<f64>,
    pub bed_temp_c: Option<f64>,
    pub fan_255: Option<u16>,
    pub fan_percent: Option<u32>,
    pub tool: u8,
    pub position_certain_fa: String,
    pub feedrate_override_percent: Option<u32>,
}

impl MachineState {
    /// ساخت snapshot فارسی برای UI
    fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            units_fa: match self.units {
                Units::Mm => "میلی‌متر (G21)".into(),
                Units::Inch => "اینچ (G20)".into(),
            },
            absolute_xyz: self.absolute_xyz,
            absolute_e: self.absolute_e,
            plane_fa: match self.plane {
                Plane::Xy => "XY (G17)".into(),
                Plane::Zx => "ZX (G18)".into(),
                Plane::Yz => "YZ (G19)".into(),
            },
            x_mm: self.x.value,
            y_mm: self.y.value,
            z_mm: self.z.value,
            e_mm: self.e_counter,
            feedrate_mm_min: self.feedrate,
            nozzle_temp_c: self.nozzle_temp_c,
            bed_temp_c: self.bed_temp_c,
            fan_255: self.fan_255,
            fan_percent: self.fan_255.map(|f| ((f as f64 / 255.0) * 100.0).round() as u32),
            tool: self.tool,
            position_certain_fa: match self.position_certainty() {
                Certainty::Known => "معلوم".into(),
                Certainty::Approximate => "تقریبی (فرض اولیه یا homing نامشخص)".into(),
                Certainty::Unknown => "نامعلوم".into(),
            },
            feedrate_override_percent: self.feedrate_override_percent,
        }
    }
}

/// نوع نمایشی خط در پنل توضیح (badge فارسی)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    Extrusion,
    Travel,
    Retract,
    Unretract,
    Arc,
    Homing,
    Temperature,
    Fan,
    Setting,
    Wait,
    ToolChange,
    Display,
    Comment,
    Metadata,
    Empty,
    UnknownCommand,
    Malformed,
    Motors,
}

impl LineType {
    /// نام فارسی برای UI
    pub fn fa(&self) -> &'static str {
        match self {
            LineType::Extrusion => "حرکت چاپی",
            LineType::Travel => "حرکت خالی (travel)",
            LineType::Retract => "عقب‌کشیدن فیلامنت",
            LineType::Unretract => "بازگرداندن فیلامنت",
            LineType::Arc => "حرکت قوسی",
            LineType::Homing => "هم‌کردن محورها",
            LineType::Temperature => "دما",
            LineType::Fan => "فن",
            LineType::Setting => "تنظیم حالت",
            LineType::Wait => "انتظار/مکث",
            LineType::ToolChange => "تعویض ابزار",
            LineType::Display => "نمایش پیام",
            LineType::Comment => "کامنت",
            LineType::Metadata => "فراداده",
            LineType::Empty => "خط خالی",
            LineType::UnknownCommand => "فرمان ناشناخته",
            LineType::Malformed => "نحوی نامعتبر",
            LineType::Motors => "موتورها",
        }
    }
}

/// یک ردیف پارامتر برای جدول inspector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamRow {
    pub letter: char,
    /// مقدار خام همان‌طور که در فایل نوشته شده
    pub raw_value: String,
    /// مقدار نرمال‌شده به mm (یا mm/min برای F) اگر عدد معتبر بود
    pub normalized: Option<f64>,
    pub meaning_fa: String,
}

/// اجرای یک فرمان در یک خط — دادهٔ توضیح کامل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub code: String,
    pub summary_fa: String,
    pub family_fa: String,
    pub params: Vec<ParamRow>,
    /// شرح «چه اتفاقی می‌افتد» با مقادیر واقعی این خط
    pub action_fa: String,
    pub firmware_dependent: bool,
    pub unknown: bool,
}

/// نتیجهٔ تفسیر یک خط
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretedLine {
    pub line_index: usize,
    pub file_line_no: usize,
    /// نوع کلی خط
    pub line_type: LineType,
    /// فرمان‌های اجراشده در این خط (به ترتیب)
    pub commands: Vec<CommandExecution>,
    /// ایندکس segmentهای تولیدی در Interpretation.segments
    pub segment_indices: Vec<usize>,
    /// ΔE کل این خط (mm فیلامنت؛ منفی=retract)
    pub e_delta: Option<f64>,
    pub state_before: StateSnapshot,
    pub state_after: StateSnapshot,
}

/// خروجی نهایی مفسر
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Interpretation {
    pub segments: Vec<ToolpathSegment>,
    pub lines: Vec<InterpretedLine>,
    pub layers: Vec<LayerInfo>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: ToolpathStats,
    /// آیا مسیر الگوی مارپیچی پیوسته دارد (لایهٔ گسسته ندارد)
    pub spiral_mode: bool,
}

/// نام فارسی دستهٔ فرمان برای UI
fn family_fa(f: CommandFamily) -> &'static str {
    match f {
        CommandFamily::Motion => "حرکت",
        CommandFamily::Units => "واحد",
        CommandFamily::CoordinateMode => "حالت مختصات",
        CommandFamily::ExtruderMode => "حالت اکسترودر",
        CommandFamily::Homing => "مبنا و homing",
        CommandFamily::Temperature => "دما",
        CommandFamily::Fan => "فن",
        CommandFamily::Wait => "انتظار",
        CommandFamily::Tool => "ابزار",
        CommandFamily::Display => "نمایش",
        CommandFamily::FirmwareSetting => "تنظیم فریم‌ور",
        CommandFamily::Mesh => "سطح‌سازی",
        CommandFamily::RetractMacro => "ماکروی retract",
        CommandFamily::Workspace => "مختصات کاری",
        CommandFamily::Motors => "موتورها",
        CommandFamily::Unknown => "نامشخص",
    }
}

/// قالب‌کردن عدد برای متن فارسی (نمایش LTR عدد فنی)
fn fmt_num(v: f64) -> String {
    // حداکثر ۳ رقم اعشار؛ صفرهای انتهایی حذف می‌شوند
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() { "0".into() } else { s }
}

// ============================================================================
// مفسر
// ============================================================================
struct Interpreter {
    state: MachineState,
    segments: Vec<ToolpathSegment>,
    lines: Vec<InterpretedLine>,
    diagnostics: Vec<Diagnostic>,
    stats: ToolpathStats,
    next_seg_id: u64,
    opts: InterpretOptions,
}

/// تفسیر کامل فایل — نقطهٔ ورود عمومی
pub fn interpret(lexed: &[LexedLine], opts: &InterpretOptions) -> Interpretation {
    let mut itp = Interpreter {
        state: MachineState::new(),
        segments: Vec::new(),
        lines: Vec::new(),
        diagnostics: Vec::new(),
        stats: ToolpathStats::default(),
        next_seg_id: 0,
        opts: opts.clone(),
    };
    for line in lexed {
        itp.process_line(line);
    }
    // لایه‌بندی و تشخیص حالت مارپیچی پس از تفسیر کامل
    let (layers, spiral_mode) = itp.finalize_layers();
    Interpretation {
        segments: itp.segments,
        lines: itp.lines,
        layers,
        diagnostics: itp.diagnostics,
        stats: itp.stats,
        spiral_mode,
    }
}

impl Interpreter {
    /// ثبت segment جدید در همهٔ ساختارها
    fn push_segment(&mut self, mut seg: ToolpathSegment) -> usize {
        seg.id = self.next_seg_id;
        self.next_seg_id += 1;
        self.stats.accumulate(&seg);
        // بازرسی bounds فقط با پروفایل معتبر (الزام: بدون پروفایل جعل نکن)
        if let Some(vol) = self.opts.build_volume_mm {
            if let Some(end) = seg.end_mm {
                if seg.kind == SegmentKind::Extrusion || seg.kind == SegmentKind::Arc {
                    if end[0].abs() > vol[0] / 2.0
                        || end[1].abs() > vol[1] / 2.0
                        || end[2] < -0.01
                        || end[2] > vol[2] + 0.01
                    {
                        self.diagnostics.push(Diagnostic::warning(
                            "W-BOUNDS",
                            format!(
                                "مختصات ({}, {}, {}) خارج از حجم ساخت پروفایل است",
                                fmt_num(end[0]),
                                fmt_num(end[1]),
                                fmt_num(end[2])
                            ),
                            seg.source_line_index,
                        ));
                    }
                }
            }
        }
        self.segments.push(seg);
        self.segments.len() - 1
    }

    /// پردازش یک خط کامل
    fn process_line(&mut self, line: &LexedLine) {
        let state_before = self.state.snapshot();

        // ---------- خطوط خالی/کامنت ----------
        if line.kind == LineKind::Empty {
            self.lines.push(InterpretedLine {
                line_index: line.line_index,
                file_line_no: line.file_line_no,
                line_type: LineType::Empty,
                commands: vec![],
                segment_indices: vec![],
                e_delta: None,
                state_before: state_before.clone(),
                state_after: state_before,
            });
            return;
        }
        if line.kind == LineKind::CommentOrMeta {
            self.lines.push(InterpretedLine {
                line_index: line.line_index,
                file_line_no: line.file_line_no,
                line_type: if line.raw.trim() == "%" {
                    LineType::Metadata
                } else {
                    LineType::Comment
                },
                commands: vec![],
                segment_indices: vec![],
                e_delta: None,
                state_before: state_before.clone(),
                state_after: state_before,
            });
            return;
        }

        // ---------- واژه‌های بدشکل → Diagnostic خطا (متن خام حفظ می‌شود) ----------
        // طبق الزام PROMPT_02 §6: خط نامعتبر کامل نادیده گرفته می‌شود تا
        // «مسیر غلط تازه» ساخته نشود؛ آخرین preview معتبر باقی می‌ماند.
        for w in &line.malformed_words {
            self.diagnostics.push(Diagnostic::error(
                "E-BAD-NUMBER",
                format!("مقدار پارامتر «{w}» عدد معتبر نیست؛ این خط نادیده گرفته شد اما متن آن حفظ شده است"),
                line.line_index,
            ));
        }
        if line.kind == LineKind::Malformed {
            // کل خط رد می‌شود؛ وضعیت ماشین بدون تغییر می‌ماند
            self.lines.push(InterpretedLine {
                line_index: line.line_index,
                file_line_no: line.file_line_no,
                line_type: LineType::Malformed,
                commands: vec![],
                segment_indices: vec![],
                e_delta: None,
                state_before: state_before.clone(),
                state_after: state_before,
            });
            return;
        }

        // ---------- سینتکس ماکرو/متغیر پشتیبانی‌نشده ----------
        let upper_raw = line.raw.trim().to_ascii_uppercase();
        if line.raw.contains('#')
            || upper_raw.starts_with("IF ")
            || upper_raw.starts_with("WHILE ")
            || upper_raw.starts_with("GOTO ")
        {
            self.diagnostics.push(Diagnostic::firmware(
                "I-MACRO-SYNTAX",
                "این خط سینتکس متغیر/شرط/ماکرو دارد که در مجموعهٔ پایه پشتیبانی نمی‌شود؛ متن خام حفظ شد",
                line.line_index,
            ));
        }

        // ---------- گروه‌بندی فرمان‌ها و پارامترهایشان ----------
        // هر فرمان G/M/T پارامترهای بعد از خود تا فرمان بعدی را می‌گیرد
        let groups = split_command_groups(&line.words);
        let mut commands: Vec<CommandExecution> = Vec::new();
        let mut segment_indices: Vec<usize> = Vec::new();
        let mut e_delta_total: Option<f64> = None;

        for (cmd_word, params) in groups {
            let exec = match cmd_word.letter {
                'G' => self.exec_g(cmd_word, &params, line, &mut segment_indices, &mut e_delta_total),
                'M' => self.exec_m(cmd_word, &params, line, &mut segment_indices, &mut e_delta_total),
                'T' => self.exec_t(cmd_word, &params, line),
                // N و حروف دیگر اینجا فرمان نیستند
                _ => continue,
            };
            commands.push(exec);
        }

        // ---------- تعیین نوع کلی خط ----------
        let line_type = if line.kind == LineKind::Malformed {
            LineType::Malformed
        } else if !segment_indices.is_empty() {
            // از نوع segment مسلط استفاده می‌کنیم
            let kinds: Vec<SegmentKind> = segment_indices
                .iter()
                .map(|&i| self.segments[i].kind)
                .collect();
            if kinds.iter().all(|&k| k == SegmentKind::Arc) {
                LineType::Arc
            } else if kinds.iter().any(|&k| k == SegmentKind::Extrusion) {
                LineType::Extrusion
            } else if kinds.iter().any(|&k| k == SegmentKind::Retract) {
                LineType::Retract
            } else if kinds.iter().any(|&k| k == SegmentKind::Unretract) {
                LineType::Unretract
            } else {
                LineType::Travel
            }
        } else if commands.is_empty() {
            LineType::Empty
        } else {
            // خط تنظیمی: از خانوادهٔ اولین فرمان
            family_to_line_type(commands[0].code.as_str())
        };

        let state_after = self.state.snapshot();
        self.lines.push(InterpretedLine {
            line_index: line.line_index,
            file_line_no: line.file_line_no,
            line_type,
            commands,
            segment_indices,
            e_delta: e_delta_total,
            state_before,
            state_after,
        });
    }

    // ------------------------------------------------------------------
    // فرمان‌های G
    // ------------------------------------------------------------------
    fn exec_g(
        &mut self,
        cmd: &LexedWord,
        params: &[LexedWord],
        line: &LexedLine,
        seg_out: &mut Vec<usize>,
        e_out: &mut Option<f64>,
    ) -> CommandExecution {
        let code_num = cmd.number.map(|n| n as i64).unwrap_or(-1) as u32;
        let info: Option<&'static CommandInfo> = knowledge::lookup('G', code_num);
        let code = format!("G{code_num}");

        // ---------- فرمان ناشناخته ----------
        let Some(info) = info else {
            self.diagnostics.push(Diagnostic::firmware(
                "I-UNKNOWN-G",
                format!("فرمان ناشناخته {code}: {}", knowledge::UNKNOWN_COMMAND_FA),
                line.line_index,
            ));
            return CommandExecution {
                code,
                summary_fa: knowledge::UNKNOWN_COMMAND_FA.into(),
                family_fa: "نامشخص".into(),
                params: param_rows(params, &self.state),
                action_fa: "این فرمان تفسیر نشد؛ متن خام حفظ شده است.".into(),
                firmware_dependent: true,
                unknown: true,
            };
        };

        let mut params = params.to_vec();
        match code_num {
            // ================= حرکت خطی G0/G1 =================
            0 | 1 => self.exec_linear_move(info, &mut params, line, seg_out, e_out, code_num == 0),
            // ================= قوس G2/G3 =================
            2 | 3 => self.exec_arc(info, &mut params, line, seg_out, e_out, code_num == 2),
            // ================= مکث G4 =================
            4 => {
                let p_ms = find_number(&params, 'P');
                let s_sec = find_number(&params, 'S');
                let action = if let Some(ms) = p_ms {
                    format!("مکث {} میلی‌ثانیه", fmt_num(ms))
                } else if let Some(s) = s_sec {
                    format!("مکث {} ثانیه", fmt_num(s))
                } else {
                    "مکث بدون مدت مشخص (وابسته به فریم‌ور)".into()
                };
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: action,
                    firmware_dependent: info.firmware_dependent,
                    unknown: false,
                }
            }
            // ================= صفحهٔ قوس =================
            17 | 18 | 19 => {
                self.state.plane = match code_num {
                    17 => Plane::Xy,
                    18 => Plane::Zx,
                    _ => Plane::Yz,
                };
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: format!("صفحهٔ قوس فعال: {}", self.state.snapshot().plane_fa),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= واحد =================
            20 | 21 => {
                self.state.units = if code_num == 21 { Units::Mm } else { Units::Inch };
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: format!("واحد فعال: {}", self.state.snapshot().units_fa),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= مطلق/نسبی =================
            90 | 91 => {
                self.state.absolute_xyz = code_num == 90;
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: if self.state.absolute_xyz {
                        "مختصات X/Y/Z از این پس مطلق تفسیر می‌شوند".into()
                    } else {
                        "مختصات X/Y/Z از این پس نسبی تفسیر می‌شوند".into()
                    },
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= G92: ادعای شمارنده‌ها =================
            92 => {
                let mut action = String::new();
                for p in &params {
                    if let Some(v) = p.number {
                        let mm = self.state.to_mm(v);
                        match p.letter {
                            'X' => { self.state.x = Axis { value: Some(mm), certainty: Certainty::Known }; }
                            'Y' => { self.state.y = Axis { value: Some(mm), certainty: Certainty::Known }; }
                            'Z' => { self.state.z = Axis { value: Some(mm), certainty: Certainty::Known }; }
                            'E' => {
                                self.state.e_counter = Some(mm);
                                self.state.e_known = true;
                            }
                            _ => {}
                        }
                    }
                }
                if find_number(&params, 'E').is_some() {
                    action = format!(
                        "شمارندهٔ اکسترودر به {} mm تنظیم شد (بدون حرکت فیزیکی)",
                        fmt_num(self.state.e_counter.unwrap_or(0.0))
                    );
                } else if find_number(&params, 'X').is_some()
                    || find_number(&params, 'Y').is_some()
                    || find_number(&params, 'Z').is_some()
                {
                    action = "مختصات منطقی محورهای ذکرشده بدون حرکت فیزیکی تنظیم شد".into();
                } else {
                    action = "G92 بدون پارامتر محور؛ در برخی فریم‌ورها یعنی صفرکردن همهٔ شمارنده‌ها (وابسته به فریم‌ور)".into();
                }
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: action,
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= G28: homing =================
            28 => {
                // محورهای ذکرشده؛ اگر هیچ‌کس ذکر نشده همه hom می‌شوند
                let mentioned = ['X', 'Y', 'Z']
                    .iter()
                    .any(|&a| params.iter().any(|p| p.letter == a));
                let axes_fa = if mentioned {
                    params
                        .iter()
                        .filter(|p| matches!(p.letter, 'X' | 'Y' | 'Z'))
                        .map(|p| p.letter.to_string())
                        .collect::<Vec<_>>()
                        .join(" و ")
                } else {
                    "همهٔ محورها".to_string()
                };
                // بدون پروفایل، موقعیت پس از homing تضمین نمی‌شود (الزام PROMPT_02)
                if mentioned {
                    for p in &params {
                        match p.letter {
                            'X' => self.state.x = Axis::unknown(),
                            'Y' => self.state.y = Axis::unknown(),
                            'Z' => self.state.z = Axis::unknown(),
                            _ => {}
                        }
                    }
                } else {
                    self.state.x = Axis::unknown();
                    self.state.y = Axis::unknown();
                    self.state.z = Axis::unknown();
                }
                self.diagnostics.push(Diagnostic::firmware(
                    "I-HOME-UNKNOWN",
                    "موقعیت پس از homing به پروفایل پرینتر وابسته است؛ تا فرمان مطلق بعدی، مختصات «نامعلوم» در نظر گرفته می‌شود",
                    line.line_index,
                ));
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: format!("{axes_fa} هم (homing) شدند؛ موقعیت دقیق تا فرمان بعدی نامعلوم است"),
                    firmware_dependent: true,
                    unknown: false,
                }
            }
            // ================= G29: سطح‌سازی =================
            29 => {
                self.diagnostics.push(Diagnostic::firmware(
                    "I-MESH-UNMODELED",
                    "مسیر سطح‌سازی G29 به فریم‌ور وابسته است و مدل‌سازی دقیق نمی‌شود",
                    line.line_index,
                ));
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: "فرمان سطح‌سازی بستر اجرا شد؛ مسیر آن در پیش‌نمایش قابل مدل‌سازی نیست".into(),
                    firmware_dependent: true,
                    unknown: false,
                }
            }
            // ================= ماکروهای retract =================
            10 | 11 => {
                self.diagnostics.push(Diagnostic::firmware(
                    "I-FW-RETRACT",
                    "مقدار عقب‌کشیدن G10/G11 در تنظیمات فریم‌ور ذخیره شده و از متن فایل معلوم نیست",
                    line.line_index,
                ));
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: if code_num == 10 {
                        "عقب‌کشیدن نرم‌افزاری با تنظیمات ذخیره‌شدهٔ فریم‌ور (مقدار دقیق نامعلوم)".into()
                    } else {
                        "بازگرداندن عقب‌کشیدن نرم‌افزاری (مقدار دقیق نامعلوم)".into()
                    },
                    firmware_dependent: true,
                    unknown: false,
                }
            }
            // ================= فضای کاری G53..G59 =================
            53..=59 => {
                self.diagnostics.push(Diagnostic::firmware(
                    "I-WORKSPACE",
                    "آفست سیستم مختصات کاری در این نسخه مدل نمی‌شود؛ مختصات بعدی «تقریبی» علامت می‌خورند",
                    line.line_index,
                ));
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(&params, &self.state),
                    action_fa: "سیستم مختصات کاری انتخاب شد؛ اثر هندسی آن در این نسخه قابل مدل‌سازی نیست".into(),
                    firmware_dependent: true,
                    unknown: false,
                }
            }
            // ================= بقیهٔ G های شناخته‌شده بدون اثر هندسی =================
            _ => CommandExecution {
                code,
                summary_fa: info.summary_fa.into(),
                family_fa: family_fa(info.family).into(),
                params: param_rows(&params, &self.state),
                action_fa: "فرمان تنظیمی؛ اثر هندسی مدلی ندارد".into(),
                firmware_dependent: info.firmware_dependent,
                unknown: false,
            },
        }
    }

    // ------------------------------------------------------------------
    // حرکت خطی G0/G1 — قلب تولید segment
    // ------------------------------------------------------------------
    fn exec_linear_move(
        &mut self,
        info: &'static CommandInfo,
        params: &mut [LexedWord],
        line: &LexedLine,
        seg_out: &mut Vec<usize>,
        e_out: &mut Option<f64>,
        rapid: bool,
    ) -> CommandExecution {
        let code = if rapid { "G0" } else { "G1" };
        let before = self.state.clone();

        // ---------- سرعت F (modal) ----------
        if let Some(f) = find_number(params, 'F') {
            self.state.feedrate = Some(self.state.to_mm(f));
        }

        // ---------- مقاصد محورها ----------
        let new_x = self.target_axis(params, 'X', before.x.value);
        let new_y = self.target_axis(params, 'Y', before.y.value);
        let new_z = self.target_axis(params, 'Z', before.z.value);

        // ---------- اکستروژن ----------
        let e_given = find_number(params, 'E');
        let (e_new, e_delta, e_approx) = match e_given {
            Some(ev) => {
                let ev_mm = self.state.to_mm(ev);
                if self.state.absolute_e {
                    match before.e_counter {
                        Some(prev) => (Some(ev_mm), ev_mm - prev, false),
                        None => (Some(ev_mm), ev_mm, true), // فرض مبنای صفر — تقریبی
                    }
                } else {
                    // نسبی: ΔE خود مقدار است
                    (
                        before.e_counter.map(|c| c + ev_mm).or(Some(ev_mm)),
                        ev_mm,
                        false,
                    )
                }
            }
            None => (before.e_counter, 0.0, false),
        };

        // ---------- به‌روزرسانی وضعیت ----------
        if let Some((v, c)) = &new_x { self.state.x = Axis { value: Some(*v), certainty: *c }; }
        if let Some((v, c)) = &new_y { self.state.y = Axis { value: Some(*v), certainty: *c }; }
        if let Some((v, c)) = &new_z { self.state.z = Axis { value: Some(*v), certainty: *c }; }
        self.state.e_counter = e_new;

        // ---------- نوع فیزیکی حرکت ----------
        let has_motion = new_x.is_some() || new_y.is_some() || new_z.is_some();
        // حرکت «واقعی» فقط وقتی است که جابه‌جایی غیرصفر یا محور جدیدی صریح آمده باشد
        let start_pos = before.position();
        let end_pos = self.state.position();
        let kind = if !has_motion {
            if e_delta > 1e-9 {
                SegmentKind::Unretract
            } else if e_delta < -1e-9 {
                SegmentKind::Retract
            } else {
                // G1 خالی بدون هیچ پارامتر — بی‌اثر
                return CommandExecution {
                    code: code.into(),
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: "فرمان حرکت بدون پارامتر مؤثر؛ هیچ حرکتی انجام نمی‌شود".into(),
                    firmware_dependent: false,
                    unknown: false,
                };
            }
        } else if e_delta < -1e-9 {
            // حرکت همراه کاهش E (مثل z-hop هنگام retract نسبی)
            SegmentKind::Retract
        } else if e_delta > 1e-9 {
            SegmentKind::Extrusion
        } else {
            SegmentKind::Travel
        };

        // ---------- Certainty سگمنت ----------
        let mut certainty = worst(
            before.position_certainty(),
            self.state.position_certainty(),
        );
        if e_approx { certainty = worst(certainty, Certainty::Approximate); }

        // ---------- ساخت segment ----------
        let seg = ToolpathSegment {
            id: 0, // در push_segment شماره می‌خورد
            source_line_index: line.line_index,
            kind,
            start_mm: start_pos,
            end_mm: end_pos,
            e_delta_mm: if e_given.is_some() { Some(e_delta) } else { None },
            feedrate_mm_min: self.state.feedrate,
            layer_index: None, // در finalize_layers
            certainty,
        };
        let idx = self.push_segment(seg);
        seg_out.push(idx);
        *e_out = Some(e_delta);

        // ---------- متن عمل (شرح کاربردی با اعداد واقعی) ----------
        let action = self.describe_linear(&before, kind, e_delta, e_given.is_some(), rapid);
        CommandExecution {
            code: code.into(),
            summary_fa: info.summary_fa.into(),
            family_fa: family_fa(info.family).into(),
            params: param_rows(params, &self.state),
            action_fa: action,
            firmware_dependent: false,
            unknown: false,
        }
    }

    /// محاسبهٔ مقدار جدید یک محور با در نظر گرفتن مطلق/نسبی
    /// خروجی: Some((مقدار mm جدید, اطمینان)) یا None (پارامتر نداد)
    fn target_axis(
        &self,
        params: &[LexedWord],
        letter: char,
        current: Option<f64>,
    ) -> Option<(f64, Certainty)> {
        let v = find_number(params, letter)?;
        let v_mm = self.state.to_mm(v);
        if self.state.absolute_xyz {
            Some((v_mm, Certainty::Known))
        } else {
            // نسبی: اگر موقعیت فعلی معلوم است جمع می‌کنیم؛ وگرنه نامعلوم می‌ماند
            current.map(|c| (c + v_mm, Certainty::Known))
            // اگر current None بود → None (محور نامعلوم می‌ماند)
        }
    }

    /// شرح فارسی حرکت خطی با اعداد واقعی — «فقط تعریف دیکشنری» ممنوع
    fn describe_linear(
        &self,
        before: &MachineState,
        kind: SegmentKind,
        e_delta: f64,
        e_given: bool,
        rapid: bool,
    ) -> String {
        let after = &self.state;
        let mut s = String::new();
        match (before.position(), after.position()) {
            (Some(a), Some(b)) => {
                let moved = (a[0] - b[0]).abs() > 1e-9
                    || (a[1] - b[1]).abs() > 1e-9
                    || (a[2] - b[2]).abs() > 1e-9;
                if moved {
                    s.push_str(&format!(
                        "نازل از ({}, {}, {}) به ({}, {}, {}) حرکت می‌کند",
                        fmt_num(a[0]), fmt_num(a[1]), fmt_num(a[2]),
                        fmt_num(b[0]), fmt_num(b[1]), fmt_num(b[2])
                    ));
                } else {
                    s.push_str("نازل در جای خود می‌ماند");
                }
            }
            (None, Some(b)) => {
                s.push_str(&format!(
                    "نقطهٔ شروع معلوم نیست؛ نازل به ({}, {}, {}) می‌رود",
                    fmt_num(b[0]), fmt_num(b[1]), fmt_num(b[2])
                ));
            }
            _ => s.push_str("مختصات کامل این حرکت معلوم نیست"),
        }
        if let Some(f) = after.feedrate {
            s.push_str(&format!(" با سرعت {} mm/min", fmt_num(f)));
        }
        if e_given {
            match kind {
                SegmentKind::Extrusion => {
                    s.push_str(&format!(
                        " و {} mm فیلامنت اکسترود می‌شود (چاپ)",
                        fmt_num(e_delta)
                    ));
                }
                SegmentKind::Retract => {
                    s.push_str(&format!(
                        "؛ {} mm فیلامنت عقب کشیده می‌شود (retract)",
                        fmt_num(e_delta.abs())
                    ));
                }
                SegmentKind::Unretract => {
                    s.push_str(&format!(
                        "؛ {} mm فیلامنت بازگردانده می‌شود (unretract)",
                        fmt_num(e_delta)
                    ));
                }
                SegmentKind::Travel => {
                    s.push_str(" بدون تغییر فیلامنت (travel)");
                }
                _ => {}
            }
        } else if rapid {
            s.push_str(" (حرکت سریع بدون اکستروژن)");
        } else {
            s.push_str(" بدون اکستروژن (travel)");
        }
        s
    }

    // ------------------------------------------------------------------
    // قوس G2/G3
    // ------------------------------------------------------------------
    fn exec_arc(
        &mut self,
        info: &'static CommandInfo,
        params: &mut [LexedWord],
        line: &LexedLine,
        seg_out: &mut Vec<usize>,
        e_out: &mut Option<f64>,
        clockwise: bool,
    ) -> CommandExecution {
        let code = if clockwise { "G2" } else { "G3" };
        let before = self.state.clone();

        // ---------- شروع نامعلوم؟ ----------
        let Some(start) = before.position() else {
            self.diagnostics.push(Diagnostic::error(
                "E-ARC-NO-START",
                "قوس بدون موقعیت شروع معلوم قابل تفسیر نیست",
                line.line_index,
            ));
            return CommandExecution {
                code: code.into(),
                summary_fa: info.summary_fa.into(),
                family_fa: family_fa(info.family).into(),
                params: param_rows(params, &self.state),
                action_fa: "قوس نادیده گرفته شد چون موقعیت فعلی نازل معلوم نیست".into(),
                firmware_dependent: false,
                unknown: false,
            };
        };

        // ---------- پارامترها ----------
        if let Some(f) = find_number(params, 'F') {
            self.state.feedrate = Some(self.state.to_mm(f));
        }
        let new_x = self.target_axis(params, 'X', before.x.value);
        let new_y = self.target_axis(params, 'Y', before.y.value);
        let new_z = self.target_axis(params, 'Z', before.z.value);
        let end = [
            new_x.map(|(v, _)| v).unwrap_or(start[0]),
            new_y.map(|(v, _)| v).unwrap_or(start[1]),
            new_z.map(|(v, _)| v).unwrap_or(start[2]),
        ];

        // آفست مرکز بر اساس صفحه (I/J در G17؛ I/K در G18؛ J/K در G19)
        let i_off = find_number(params, 'I');
        let j_off = find_number(params, 'J');
        let k_off = find_number(params, 'K');
        let r_form = find_number(params, 'R');
        let (center_uv, radius) = match self.state.plane {
            Plane::Xy => (
                match (i_off, j_off) {
                    (Some(i), Some(j)) => Some((self.state.to_mm(i), self.state.to_mm(j))),
                    (Some(i), None) => Some((self.state.to_mm(i), 0.0)),
                    (None, Some(j)) => Some((0.0, self.state.to_mm(j))),
                    _ => None,
                },
                r_form.map(|r| self.state.to_mm(r)),
            ),
            Plane::Zx => (
                match (k_off, i_off) {
                    (Some(k), Some(i)) => Some((self.state.to_mm(k), self.state.to_mm(i))),
                    (Some(k), None) => Some((self.state.to_mm(k), 0.0)),
                    (None, Some(i)) => Some((0.0, self.state.to_mm(i))),
                    _ => None,
                },
                r_form.map(|r| self.state.to_mm(r)),
            ),
            Plane::Yz => (
                match (j_off, k_off) {
                    (Some(j), Some(k)) => Some((self.state.to_mm(j), self.state.to_mm(k))),
                    (Some(j), None) => Some((self.state.to_mm(j), 0.0)),
                    (None, Some(k)) => Some((0.0, self.state.to_mm(k))),
                    _ => None,
                },
                r_form.map(|r| self.state.to_mm(r)),
            ),
        };

        // ---------- اکستروژن قوس ----------
        let e_given = find_number(params, 'E');
        let (e_new, e_delta, e_approx) = match e_given {
            Some(ev) => {
                let ev_mm = self.state.to_mm(ev);
                if self.state.absolute_e {
                    match before.e_counter {
                        Some(prev) => (Some(ev_mm), ev_mm - prev, false),
                        None => (Some(ev_mm), ev_mm, true),
                    }
                } else {
                    (before.e_counter.map(|c| c + ev_mm).or(Some(ev_mm)), ev_mm, false)
                }
            }
            None => (before.e_counter, 0.0, false),
        };

        // ---------- tessellation ----------
        let arc_params = ArcParams {
            start,
            end,
            plane: self.state.plane,
            clockwise,
            center_uv,
            radius,
            tolerance_mm: self.opts.arc_tolerance_mm,
            max_segments: self.opts.arc_max_segments,
        };
        match tessellate_arc(&arc_params) {
            Err(err) => {
                // قوس نامعتبر → Diagnostic فارسی؛ مسیر ادامه می‌یابد (بدون crash)
                let msg_fa = match &err {
                    ArcError::MissingCenterOrRadius => {
                        "قوس نه I/J/K دارد و نه R؛ پارامترهای کافی نیست"
                    }
                    ArcError::RadiusTooSmall { distance, radius } => {
                        let msg = format!(
                            "فاصلهٔ دو سر قوس ({}) از قطر شعاع ({}) بیشتر است؛ قوس نامعتبر است",
                            fmt_num(*distance),
                            fmt_num(2.0 * radius.abs())
                        );
                        return self.arc_error_line(
                            info, params, line, code, msg, seg_out, e_out, e_new, e_delta,
                            &new_x, &new_y, &new_z,
                        );
                    }
                    ArcError::EndpointRadiusMismatch { start_r, end_r } => {
                        let msg = format!(
                            "شعاع قوس ناهم‌خوان است: فاصلهٔ مرکز تا مبدأ {} و تا مقصد {} است",
                            fmt_num(*start_r),
                            fmt_num(*end_r)
                        );
                        return self.arc_error_line(
                            info, params, line, code, msg, seg_out, e_out, e_new, e_delta,
                            &new_x, &new_y, &new_z,
                        );
                    }
                    ArcError::DegenerateR => {
                        "قوس R با مبدأ و مقصد یکسان تعریف‌نشده است"
                    }
                };
                self.diagnostics.push(Diagnostic::error(
                    "E-ARC-INVALID",
                    msg_fa.to_string(),
                    line.line_index,
                ));
                CommandExecution {
                    code: code.into(),
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: format!("قوس نامعتبر بود و نادیده گرفته شد: {msg_fa}"),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            Ok(arc) => {
                // به‌روزرسانی وضعیت به مقصد
                if let Some((v, c)) = &new_x { self.state.x = Axis { value: Some(*v), certainty: *c }; }
                if let Some((v, c)) = &new_y { self.state.y = Axis { value: Some(*v), certainty: *c }; }
                if let Some((v, c)) = &new_z { self.state.z = Axis { value: Some(*v), certainty: *c }; }
                self.state.e_counter = e_new;

                // segmentهای قوسی؛ ΔE به‌طور مساوی تقسیم می‌شود
                let n = arc.points.len().max(1) as f64;
                let mut prev = start;
                let mut certainty = worst(
                    before.position_certainty(),
                    self.state.position_certainty(),
                );
                if e_approx { certainty = worst(certainty, Certainty::Approximate); }
                for (i, pt) in arc.points.iter().enumerate() {
                    let seg = ToolpathSegment {
                        id: 0,
                        source_line_index: line.line_index,
                        kind: SegmentKind::Arc,
                        start_mm: Some(prev),
                        end_mm: Some(*pt),
                        e_delta_mm: if e_given.is_some() {
                            Some(e_delta / n)
                        } else {
                            None
                        },
                        feedrate_mm_min: self.state.feedrate,
                        layer_index: None,
                        certainty,
                    };
                    let idx = self.push_segment(seg);
                    seg_out.push(idx);
                    prev = *pt;
                    let _ = i;
                }
                if arc.is_full_circle {
                    self.diagnostics.push(Diagnostic::warning(
                        "W-ARC-FULL",
                        "قوس کامل (۳۶۰ درجه) تشخیص داده شد",
                        line.line_index,
                    ));
                }
                *e_out = Some(e_delta);

                let action = format!(
                    "قوس {} از ({}, {}, {}) تا ({}, {}, {}) با شعاع {} mm به {} قطعه خطی تقسیم و نمایش داده می‌شود",
                    if clockwise { "ساعت‌گرد" } else { "پادساعت‌گرد" },
                    fmt_num(start[0]), fmt_num(start[1]), fmt_num(start[2]),
                    fmt_num(end[0]), fmt_num(end[1]), fmt_num(end[2]),
                    fmt_num(arc.radius),
                    arc.points.len()
                );
                CommandExecution {
                    code: code.into(),
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: action,
                    firmware_dependent: false,
                    unknown: false,
                }
            }
        }
    }

    /// خروجی یکنواخت برای قوس‌های خطادار
    #[allow(clippy::too_many_arguments)]
    fn arc_error_line(
        &mut self,
        info: &'static CommandInfo,
        params: &[LexedWord],
        line: &LexedLine,
        code: &str,
        msg: String,
        seg_out: &mut Vec<usize>,
        e_out: &mut Option<f64>,
        e_new: Option<f64>,
        e_delta: f64,
        new_x: &Option<(f64, Certainty)>,
        new_y: &Option<(f64, Certainty)>,
        new_z: &Option<(f64, Certainty)>,
    ) -> CommandExecution {
        self.diagnostics.push(Diagnostic::error(
            "E-ARC-INVALID",
            msg.clone(),
            line.line_index,
        ));
        // حتی در خطا، وضعیت به مقصد حرکت می‌کند تا ادامهٔ فایل معتبر بماند
        if let Some((v, c)) = new_x { self.state.x = Axis { value: Some(*v), certainty: *c }; }
        if let Some((v, c)) = new_y { self.state.y = Axis { value: Some(*v), certainty: *c }; }
        if let Some((v, c)) = new_z { self.state.z = Axis { value: Some(*v), certainty: *c }; }
        self.state.e_counter = e_new;
        if e_delta.abs() > 1e-9 { *e_out = Some(e_delta); }
        seg_out.clear();
        CommandExecution {
            code: code.into(),
            summary_fa: info.summary_fa.into(),
            family_fa: family_fa(info.family).into(),
            params: param_rows(params, &self.state),
            action_fa: format!("قوس نامعتبر بود و نادیده گرفته شد: {msg}"),
            firmware_dependent: false,
            unknown: false,
        }
    }

    // ------------------------------------------------------------------
    // فرمان‌های M
    // ------------------------------------------------------------------
    fn exec_m(
        &mut self,
        cmd: &LexedWord,
        params: &[LexedWord],
        line: &LexedLine,
        _seg_out: &mut Vec<usize>,
        _e_out: &mut Option<f64>,
    ) -> CommandExecution {
        let code_num = cmd.number.map(|n| n as i64).unwrap_or(-1) as u32;
        let info = knowledge::lookup('M', code_num);
        let code = format!("M{code_num}");

        let Some(info) = info else {
            self.diagnostics.push(Diagnostic::firmware(
                "I-UNKNOWN-M",
                format!("فرمان ناشناخته M{code_num}: {}", knowledge::UNKNOWN_COMMAND_FA),
                line.line_index,
            ));
            return CommandExecution {
                code,
                summary_fa: knowledge::UNKNOWN_COMMAND_FA.into(),
                family_fa: "نامشخص".into(),
                params: param_rows(params, &self.state),
                action_fa: "این فرمان تفسیر نشد؛ متن خام حفظ شده است.".into(),
                firmware_dependent: true,
                unknown: true,
            };
        };

        match code_num {
            // ================= حالت E =================
            82 | 83 => {
                self.state.absolute_e = code_num == 82;
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: if self.state.absolute_e {
                        "شمارندهٔ E از این پس مطلق است (M82)".into()
                    } else {
                        "مقدار E از این پس نسبت به همان حرکت است (M83)".into()
                    },
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= دما =================
            104 | 109 => {
                let s = find_number(params, 'S');
                if let Some(s) = s {
                    self.state.nozzle_temp_c = Some(s);
                }
                let action = match s {
                    Some(t) => format!(
                        "دمای هدف نازل به {}°C تنظیم شد{}",
                        fmt_num(t),
                        if code_num == 109 { " و چاپ تا رسیدن به آن متوقف می‌ماند" } else { " (بدون انتظار)" }
                    ),
                    None => "فرمان دما بدون مقدار S".into(),
                };
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: action,
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            140 | 190 => {
                let s = find_number(params, 'S');
                if let Some(s) = s {
                    self.state.bed_temp_c = Some(s);
                }
                let action = match s {
                    Some(t) => format!(
                        "دمای هدف بستر به {}°C تنظیم شد{}",
                        fmt_num(t),
                        if code_num == 190 { " و چاپ تا رسیدن به آن متوقف می‌ماند" } else { " (بدون انتظار)" }
                    ),
                    None => "فرمان دمای بستر بدون مقدار S".into(),
                };
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: action,
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= فن =================
            106 => {
                // بدون S یعنی تمام‌قد (۲۵۵) طبق قرارداد رایج
                let s = find_number(params, 'S').unwrap_or(255.0).clamp(0.0, 255.0);
                self.state.fan_255 = Some(s as u16);
                let pct = ((s / 255.0) * 100.0).round() as u32;
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: format!("فن قطعه به سرعت {} از ۲۵۵ (حدود {} درصد) تنظیم شد", fmt_num(s), pct),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            107 => {
                self.state.fan_255 = Some(0);
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: "فن قطعه خاموش شد".into(),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= پیام نمایش =================
            117 => {
                let msg = cmd
                    .text
                    .clone()
                    .unwrap_or_default();
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: vec![],
                    action_fa: format!("پیام «{msg}» روی نمایشگر پرینتر نشان داده می‌شود؛ این متن هرگز به‌عنوان فرمان تفسیر نمی‌شود"),
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= ضریب سرعت =================
            220 => {
                let s = find_number(params, 'S');
                if let Some(s) = s {
                    self.state.feedrate_override_percent = Some(s.max(0.0) as u32);
                }
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: format!(
                        "ضریب سرعت کل به {} درصد تنظیم شد (فقط زمان واقعی را تغییر می‌دهد، نه مسیر)",
                        fmt_num(s.unwrap_or(100.0))
                    ),
                    firmware_dependent: true,
                    unknown: false,
                }
            }
            // ================= موتورها =================
            17 | 18 | 84 => {
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: if code_num == 17 {
                        "جریان موتورهای گام وصل شد".into()
                    } else {
                        "جریان موتورها قطع شد؛ تا حرکت بعدی موقعیت «مطمئن» نیست".into()
                    },
                    firmware_dependent: false,
                    unknown: false,
                }
            }
            // ================= بقیهٔ M های شناخته‌شده =================
            _ => {
                if info.firmware_dependent {
                    self.diagnostics.push(Diagnostic::firmware(
                        "I-FW-CMD",
                        format!("M{code_num} به firmware وابسته است؛ اثر دقیق آن مدل نمی‌شود"),
                        line.line_index,
                    ));
                }
                CommandExecution {
                    code,
                    summary_fa: info.summary_fa.into(),
                    family_fa: family_fa(info.family).into(),
                    params: param_rows(params, &self.state),
                    action_fa: "فرمان تنظیمی فریم‌ور؛ اثر هندسی مدلی ندارد".into(),
                    firmware_dependent: info.firmware_dependent,
                    unknown: false,
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // تعویض ابزار T
    // ------------------------------------------------------------------
    fn exec_t(&mut self, cmd: &LexedWord, params: &[LexedWord], _line: &LexedLine) -> CommandExecution {
        let tool = cmd.number.map(|n| n.max(0.0) as u8).unwrap_or(0);
        self.state.tool = tool;
        CommandExecution {
            code: format!("T{tool}"),
            summary_fa: format!("انتخاب ابزار/اکسترودر شمارهٔ {tool}"),
            family_fa: "ابزار".into(),
            params: param_rows(params, &self.state),
            action_fa: format!(
                "ابزار فعال به شمارهٔ {tool} تغییر کرد؛ پارامترهای هر ابزار جداگانه دیده می‌شوند"
            ),
            firmware_dependent: false,
            unknown: false,
        }
    }

    // ------------------------------------------------------------------
    // لایه‌بندی پس از تفسیر — با تشخیص حالت مارپیچی
    // ------------------------------------------------------------------
    fn finalize_layers(&mut self) -> (Vec<LayerInfo>, bool) {
        // ---------- تشخیص spiral: آیا اکثر حرکت‌های چاپی Z صعودی دارند؟ ----------
        let extrusion_z: Vec<Option<f64>> = self
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::Extrusion | SegmentKind::Arc))
            .map(|s| s.end_mm.map(|p| p[2]))
            .collect();
        let known_z: Vec<f64> = extrusion_z.iter().flatten().copied().collect();
        let mut increasing = 0usize;
        for w in known_z.windows(2) {
            if w[1] > w[0] + 1e-6 {
                increasing += 1;
            }
        }
        // بیش از نیمی از گذارها صعودی و پیوسته → الگوی مارپیچی
        let spiral_mode = known_z.len() > 20 && increasing > known_z.len() / 2;

        let mut layers: Vec<LayerInfo> = Vec::new();
        if spiral_mode {
            // حالت مارپیچی: لایهٔ گسسته نمی‌سازیم (الزام: «به‌زور نساز»)
            // layer_index همه None می‌ماند
            return (layers, true);
        }

        // ---------- لایه‌بندی گسسته از تغییر Z چاپ‌ها ----------
        let mut current_layer: u32 = 0;
        let mut last_print_z: Option<f64> = None;
        let mut current_first: u64 = 0;
        let mut current_source: usize = 0;
        let mut current_z: Option<f64> = None;
        for seg in self.segments.iter_mut() {
            let is_print = matches!(seg.kind, SegmentKind::Extrusion | SegmentKind::Arc);
            if is_print {
                if let Some(z) = seg.end_mm.map(|p| p[2]) {
                    let changed = last_print_z.map(|lz| (z - lz).abs() > 1e-6).unwrap_or(true);
                    if changed {
                        if last_print_z.is_some() {
                            // بستن لایهٔ قبلی
                            layers.push(LayerInfo {
                                index: current_layer,
                                z_mm: current_z,
                                first_segment: current_first,
                                last_segment: seg.id,
                                source_line_index: current_source,
                            });
                            current_layer += 1;
                        }
                        current_first = seg.id;
                        current_source = seg.source_line_index;
                        current_z = Some(z);
                        last_print_z = Some(z);
                    }
                }
            }
            seg.layer_index = Some(current_layer);
        }
        // لایهٔ آخر
        if last_print_z.is_some() {
            let last_id = self.segments.last().map(|s| s.id).unwrap_or(0);
            layers.push(LayerInfo {
                index: current_layer,
                z_mm: current_z,
                first_segment: current_first,
                last_segment: last_id,
                source_line_index: current_source,
            });
        }
        (layers, false)
    }
}

/// بدترین (کم‌اطمینان‌ترین) دو درجهٔ اطمینان
fn worst(a: Certainty, b: Certainty) -> Certainty {
    use Certainty::*;
    match (a, b) {
        (Unknown, _) | (_, Unknown) => Unknown,
        (Approximate, _) | (_, Approximate) => Approximate,
        _ => Known,
    }
}

/// تقسیم واژه‌های یک خط به گروه‌های (فرمان، پارامترها)
/// هر فرمان G/M/T با شماره، پارامترهای بعد از خود تا فرمان بعدی را می‌گیرد
fn split_command_groups(words: &[LexedWord]) -> Vec<(&LexedWord, Vec<LexedWord>)> {
    let mut groups: Vec<(&LexedWord, Vec<LexedWord>)> = Vec::new();
    for w in words {
        let is_command = match w.letter {
            'G' | 'M' => w.number.is_some(),
            'T' => true,
            'N' => false, // شمارهٔ خط است، نه فرمان
            _ => false,
        };
        if is_command {
            groups.push((w, Vec::new()));
        } else if let Some(last) = groups.last_mut() {
            last.1.push(w.clone());
        }
    }
    groups
}

/// یافتن مقدار عددی یک حرف پارامتر
fn find_number(params: &[LexedWord], letter: char) -> Option<f64> {
    params
        .iter()
        .find(|p| p.letter == letter)
        .and_then(|p| p.number)
        .filter(|v| v.is_finite())
}

/// ساخت جدول پارامترهای فارسی برای inspector
fn param_rows(params: &[LexedWord], state: &MachineState) -> Vec<ParamRow> {
    params
        .iter()
        .filter(|p| p.letter != 'N')
        .map(|p| ParamRow {
            letter: p.letter,
            raw_value: p
                .number
                .map(|v| format!("{v}"))
                .unwrap_or_else(|| p.text.clone().unwrap_or_default()),
            normalized: p.number.map(|v| state.to_mm(v)),
            meaning_fa: knowledge::param_meaning(p.letter).to_string(),
        })
        .collect()
}

/// نگاشت کد فرمان به LineType برای خطوط غیرحرکتی
fn family_to_line_type(code: &str) -> LineType {
    match knowledge::lookup(code.chars().next().unwrap_or('G'), code[1..].parse().unwrap_or(0)) {
        Some(info) => match info.family {
            CommandFamily::Temperature => LineType::Temperature,
            CommandFamily::Fan => LineType::Fan,
            CommandFamily::Wait => LineType::Wait,
            CommandFamily::Tool => LineType::ToolChange,
            CommandFamily::Display => LineType::Display,
            CommandFamily::Motors => LineType::Motors,
            CommandFamily::Homing => LineType::Homing,
            CommandFamily::Mesh => LineType::Homing,
            CommandFamily::RetractMacro => LineType::Retract,
            _ => LineType::Setting,
        },
        None => LineType::UnknownCommand,
    }
}

// ============================================================================
// آزمون‌های واحد مفسر — اجرای fixtureهای سند
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcode::lexer::lex_line;

    /// تفسیر متن چندخطی راحت
    fn interp(text: &str) -> Interpretation {
        let lexed: Vec<LexedLine> = text
            .lines()
            .enumerate()
            .map(|(i, l)| lex_line(l, i))
            .collect();
        interpret(&lexed, &InterpretOptions::default())
    }

    #[test]
    fn test_fixture1_basic_flow() {
        // fixture ۱ سند: مسیر پایه، E مطلق، retract و قوس
        let text = "; TEST FIXTURE ONLY - NOT A VERIFIED PRINT FILE\n\
                    ;LAYER_COUNT:2\n\
                    G21 ; millimeters\n\
                    G90 ; absolute XYZ\n\
                    M82 ; absolute E\n\
                    G28 ; home\n\
                    G92 E0\n\
                    M140 S60\n\
                    M104 S200\n\
                    M190 S60\n\
                    M109 S200\n\
                    G1 Z0.2 F300\n\
                    G1 X10 Y10 F6000\n\
                    G1 X20 Y10 E0.5 F1200\n\
                    G1 X20 Y20 E1.0\n\
                    G1 E0.2 F1800 ; retract in absolute-E mode\n\
                    G1 X30 Y30 F6000 ; travel\n\
                    G1 E1.0 F1800 ; unretract\n\
                    G1 Z0.4 F300\n\
                    G1 X30 Y40 E1.8 F1200\n\
                    G2 X40 Y40 I5 J0 E2.8 F1200 ; semicircle, clockwise in XY\n\
                    G3 X30 Y40 I-5 J0 E3.8 F1200 ; semicircle, counter-clockwise in XY\n\
                    M106 S128\n\
                    M117 Hello printer\n\
                    M107\n\
                    M104 S0\n\
                    M140 S0\n\
                    M84";
        let r = interp(text);

        // کامنت‌ها هیچ فرمانی تولید نکردند
        assert_eq!(r.lines[0].line_type, LineType::Comment);
        assert!(r.lines[0].commands.is_empty());

        // تعداد خطوط preserved
        assert_eq!(r.lines.len(), 28);

        // G28: هیچ segment؛ وضعیت نامعلوم
        let g28 = &r.lines[5];
        assert_eq!(g28.line_type, LineType::Homing);
        assert!(g28.segment_indices.is_empty());

        // دما: state تغییر کرد ولی segment جعلی نه
        // (خط ۷ = M140 بستر، خط ۸ = M104 نازل)
        let m140 = &r.lines[7];
        assert_eq!(m140.line_type, LineType::Temperature);
        assert!(m140.segment_indices.is_empty());
        assert_eq!(m140.state_after.bed_temp_c, Some(60.0));
        let m104 = &r.lines[8];
        assert_eq!(m104.line_type, LineType::Temperature);
        assert!(m104.segment_indices.is_empty());
        assert_eq!(m104.state_after.nozzle_temp_c, Some(200.0));

        // حرکت اول (Z0.2): travel — G28 موقعیت را نامعلوم کرد → certainty Unknown
        let first_move = &r.lines[11];
        assert_eq!(first_move.line_type, LineType::Travel);
        let fm_seg = &r.segments[first_move.segment_indices[0]];
        assert_eq!(fm_seg.certainty, Certainty::Unknown);

        // حرکت چاپی اول: از (10,10,0.2) به (20,10,0.2) با ΔE=0.5
        let print1 = &r.lines[13];
        assert_eq!(print1.line_type, LineType::Extrusion);
        let seg = &r.segments[print1.segment_indices[0]];
        assert_eq!(seg.start_mm, Some([10.0, 10.0, 0.2]));
        assert_eq!(seg.end_mm, Some([20.0, 10.0, 0.2]));
        assert_eq!(seg.e_delta_mm, Some(0.5));
        assert_eq!(seg.feedrate_mm_min, Some(1200.0));

        // retract مطلق: E از 1.0 به 0.2 → ΔE=-0.8 → Retract
        let retract = &r.lines[15];
        assert_eq!(retract.line_type, LineType::Retract);
        assert_eq!(retract.e_delta, Some(-0.8));

        // travel بدون E
        let travel = &r.lines[16];
        assert_eq!(travel.line_type, LineType::Travel);

        // unretract: E خالص مثبت
        let unretract = &r.lines[17];
        assert_eq!(unretract.line_type, LineType::Unretract);
        assert_eq!(unretract.e_delta, Some(0.8));

        // قوس‌ها: نوع Arc و نگاشت به همان خط
        let arc1 = &r.lines[20];
        assert_eq!(arc1.line_type, LineType::Arc);
        assert!(arc1.segment_indices.len() > 1); // tessellated
        // همهٔ قطعات قوس به خط مبدأ اشاره می‌کنند
        for &si in &arc1.segment_indices {
            assert_eq!(r.segments[si].source_line_index, 20);
        }
        // مجموع ΔE قطعات قوس ≈ 1.0
        let e_sum: f64 = arc1.segment_indices.iter().map(|&si| r.segments[si].e_delta_mm.unwrap()).sum();
        assert!((e_sum - 1.0).abs() < 1e-6);

        // لایه‌ها: ۲ لایه از Z=0.2 و 0.4 (مطابق ;LAYER_COUNT:2)
        assert_eq!(r.layers.len(), 2);
        assert!((r.layers[0].z_mm.unwrap() - 0.2).abs() < 1e-9);
        assert!((r.layers[1].z_mm.unwrap() - 0.4).abs() < 1e-9);

        // فن: 128 → حدود ۵۰٪ (خط ۲۲)
        let fan = &r.lines[22];
        assert_eq!(fan.state_after.fan_255, Some(128));
        assert_eq!(fan.state_after.fan_percent, Some(50));

        // M117: payload متنی بدون تفسیر پارامتری (خط ۲۳)
        let m117 = &r.lines[23];
        assert_eq!(m117.line_type, LineType::Display);
        assert!(m117.commands[0].action_fa.contains("Hello printer"));

        // M107: فن خاموش (خط ۲۴)
        assert_eq!(r.lines[24].state_after.fan_255, Some(0));

        // آمار: مجموع E مثبت
        assert!(r.stats.total_e_mm > 3.0);
    }

    #[test]
    fn test_fixture2_relative() {
        // fixture ۲: مختصات و اکستروژن نسبی
        let text = "; TEST FIXTURE ONLY\n\
                    G21\n\
                    G91\n\
                    M83\n\
                    G1 X1 Y0 Z0.2 E0.05 F600\n\
                    G1 X-1 Y1 E0.10 F600\n\
                    G1 X0 Y0 Z0.2 E-0.20 F1200 ; relative retract\n\
                    G1 X1 E0.20 F600 ; relative unretract/positive extrusion";
        let r = interp(text);

        // حالت‌های modal (خط ۲ = G91، خط ۳ = M83)
        assert!(!r.lines[2].state_after.absolute_xyz); // G91
        assert!(!r.lines[3].state_after.absolute_e); // M83

        // حرکت نسبی اول: از (0,0,0) تقریبی به (1,0,0.2)
        let m1 = &r.lines[4];
        let seg1 = &r.segments[m1.segment_indices[0]];
        assert_eq!(seg1.end_mm, Some([1.0, 0.0, 0.2]));
        // تقریبی چون مبنای اولیه فرض نمایش است
        assert_eq!(seg1.certainty, Certainty::Approximate);
        assert_eq!(seg1.e_delta_mm, Some(0.05));

        // حرکت دوم: (1,0,0.2) → (0,1,0.2) — این خط Z ندارد
        let m2 = &r.lines[5];
        let seg2 = &r.segments[m2.segment_indices[0]];
        assert_eq!(seg2.start_mm, Some([1.0, 0.0, 0.2]));
        assert_eq!(seg2.end_mm, Some([0.0, 1.0, 0.2]));

        // retract نسبی: E=-0.20 همراه Z0.2 نسبی → نوع Retract، z به 0.4
        let m3 = &r.lines[6];
        assert_eq!(m3.line_type, LineType::Retract);
        assert_eq!(m3.e_delta, Some(-0.20));
        assert_eq!(m3.state_after.z_mm, Some(0.4));

        // unretract نسبی: E=+0.20 همراه حرکت X → Extrusion
        let m4 = &r.lines[7];
        assert_eq!(m4.line_type, LineType::Extrusion);
        assert_eq!(m4.e_delta, Some(0.20));
    }

    #[test]
    fn test_fixture3_inches() {
        // fixture ۳: تغییر واحد به اینچ
        let text = "; TEST FIXTURE ONLY\n\
                    G20\n\
                    G90\n\
                    M82\n\
                    G92 X0 Y0 Z0 E0\n\
                    G1 X1 Y0 Z0 E0.05 F60\n\
                    G21\n\
                    G1 X25.4 Y0 Z0 E0.10 F1524";
        let r = interp(text);

        // X1 در حالت G20 معادل 25.4 mm است
        let m1 = &r.lines[5];
        assert_eq!(m1.state_after.units_fa, "اینچ (G20)");
        assert_eq!(m1.state_after.x_mm, Some(25.4));
        // F60 اینچ/min → 1524 mm/min
        assert_eq!(m1.state_after.feedrate_mm_min, Some(1524.0));

        // پس از G21 مقادیر دوباره mm تفسیر می‌شوند
        let m2 = &r.lines[7];
        assert_eq!(m2.state_after.units_fa, "میلی‌متر (G21)");
        assert_eq!(m2.state_after.x_mm, Some(25.4));
        assert_eq!(m2.state_after.feedrate_mm_min, Some(1524.0));

        // نمایش مقدار خام در params: X1 و X25.4
        assert_eq!(m1.commands[0].params[0].raw_value, "1");
        assert_eq!(m1.commands[0].params[0].normalized, Some(25.4));
        assert_eq!(m2.commands[0].params[0].raw_value, "25.4");
        assert_eq!(m2.commands[0].params[0].normalized, Some(25.4));
    }

    #[test]
    fn test_fixture4_compact_and_checksum() {
        // fixture ۴: فرمت فشرده، کامنت پرخطرنما، checksum
        let text = "; G1 X999 Y999 E999 -- this is only a comment\n\
                    G1X10Y-2E0.42F1200\n\
                    N1 G21*27\n\
                    N2 G90*18\n\
                    N3 M82*26\n\
                    N4 G1 X10 Y10 E0.5 F1200*6";
        let r = interp(text);

        // کامنت شبیه‌فرمان هیچ حرکتی نساخت
        assert!(r.lines[0].segment_indices.is_empty());
        assert_eq!(r.lines[0].line_type, LineType::Comment);

        // فرمت فشرده parse شد و حرکت تولید کرد
        assert_eq!(r.lines[1].line_type, LineType::Extrusion);
        let seg = &r.segments[r.lines[1].segment_indices[0]];
        assert_eq!(seg.end_mm, Some([10.0, -2.0, 0.0]));
        assert_eq!(seg.e_delta_mm, Some(0.42));
        assert_eq!(seg.feedrate_mm_min, Some(1200.0));
    }

    #[test]
    fn test_fixture5_errors() {
        // fixture ۵: خطای نحوی و فرمان ناشناخته
        let text = "; TEST FIXTURE ONLY\n\
                    G21\n\
                    G90\n\
                    G1 X10 Y5 E0.2 F900\n\
                    G1 XNaN Y6 E0.3 F900\n\
                    M900 K0.05\n\
                    M117 Unknown values: X999 E999";
        let r = interp(text);

        // XNaN → diagnostic خطا با حفظ متن
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code == "E-BAD-NUMBER" && d.line_index == Some(4)));
        // خط نامعتبر حرکت تولید نکرد ولی بقیهٔ خطوط سالم ماندند
        assert!(r.lines[4].segment_indices.is_empty());
        assert_eq!(r.lines[3].line_type, LineType::Extrusion);

        // M900 → شناخته‌شده اما وابسته به firmware (اثر دقیق مدل نمی‌شود)
        // طبق fixture سند: «اگر در knowledge base دقیق پیاده‌سازی نشده،
        // ناشناخته/وابسته به firmware باشد؛ معنی حدسی نساز» — M900 در
        // دانشنامه با برچسب firmware_dependent ثبت شده است.
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code == "I-FW-CMD" && d.line_index == Some(5)));
        assert_eq!(r.lines[5].line_type, LineType::Setting);
        // توضیح فرمان باید وابستگی به firmware را نشان دهد
        assert!(r.lines[5].commands[0].action_fa.contains("فریم‌ور"));

        // M117 payload: اعداد داخل پیام پارامتر نشدند
        let m117 = &r.lines[6];
        assert_eq!(m117.commands[0].params.len(), 0);
        assert!(m117.commands[0].action_fa.contains("X999 E999"));
    }

    #[test]
    fn test_spiral_mode_detection() {
        // حالت مارپیچی: Z در همهٔ حرکت‌های چاپی پیوسته بالا می‌رود
        let mut text = String::from("G21\nG90\nM82\nG28\nG92 E0\nG1 X0 Y0 Z0.2 F1200\n");
        // ۶۰ دور مارپیچ با Z صعودی
        for i in 0..60 {
            let z = 0.2 + i as f64 * 0.01;
            let e = 0.5 + i as f64 * 0.1;
            text.push_str(&format!("G1 X10 Y0 Z{z:.3} E{e:.3}\n"));
            text.push_str(&format!("G1 X-10 Y0 Z{z:.3} E{e:.3}\n"));
        }
        let r = interp(&text);
        assert!(r.spiral_mode, "باید مارپیچی تشخیص داده شود");
        assert!(r.layers.is_empty(), "در حالت مارپیچی لایهٔ گسسته ساخته نمی‌شود");
    }

    #[test]
    fn test_macro_syntax_flagged() {
        // سینتکس ماکرو باید علامت «پشتیبانی‌نشده» بخورد
        let r = interp("#10 = 5\nIF [#10 EQ 5] G1 X10\nG1 X5");
        assert!(r.diagnostics.iter().any(|d| d.code == "I-MACRO-SYNTAX"));
    }

    #[test]
    fn test_bounds_checking_with_profile() {
        // بازرسی bounds فقط وقتی پروفایل (build volume) داده شده
        let text = "G21\nG90\nG28\nG1 X10 Y10 Z0.2 F3000\nG1 X400 Y10 E1.0 F1200\n";
        let lexed: Vec<LexedLine> = text.lines().enumerate().map(|(i, l)| lex_line(l, i)).collect();

        // بدون پروفایل: هیچ هشدار bounds
        let r1 = interpret(&lexed, &InterpretOptions::default());
        assert!(!r1.diagnostics.iter().any(|d| d.code == "W-BOUNDS"));

        // با پروفایل ۲۲۰×۲۲۰×۲۵۰: X=400 خارج است
        let r2 = interpret(
            &lexed,
            &InterpretOptions { build_volume_mm: Some([220.0, 220.0, 250.0]), ..Default::default() },
        );
        assert!(r2.diagnostics.iter().any(|d| d.code == "W-BOUNDS"));
    }

    #[test]
    fn test_arc_r_form() {
        // قوس با R به‌جای I/J
        let text = "G21\nG90\nG28\nG1 X30 Y40 Z0.2 F1200\nG2 X40 Y40 R5 E1.0\n";
        let r = interp(text);
        let arc_line = &r.lines[4];
        assert_eq!(arc_line.line_type, LineType::Arc);
        // شعاع مؤثر 5: قوس باید از حدود (35, 40±5) بگذرد
        let segs: Vec<&ToolpathSegment> = arc_line
            .segment_indices
            .iter()
            .map(|&i| &r.segments[i])
            .collect();
        // کمان نیم‌دور ساعت‌گرد از (30,40): از بالای مرکز (35,45) می‌گذرد
        let max_y = segs
            .iter()
            .filter_map(|s| s.end_mm.map(|p| p[1]))
            .fold(f64::MIN, f64::max);
        let min_y = segs
            .iter()
            .filter_map(|s| s.end_mm.map(|p| p[1]))
            .fold(f64::MAX, f64::min);
        assert!(max_y > 44.5 && max_y < 45.5, "max_y = {max_y}");
        // سر و ته روی y=40 هستند
        assert!(min_y > 39.9 && min_y < 40.1, "min_y = {min_y}");
    }

    #[test]
    fn test_invalid_arc_diagnostics() {
        // قوس با شعاع نامعتبر → diagnostic خطا نه crash
        let text = "G21\nG90\nG28\nG1 X0 Y0 Z0.2 F1200\nG2 X10 Y0 R2 E1.0\n";
        let r = interp(text);
        assert!(r.diagnostics.iter().any(|d| d.code == "E-ARC-INVALID"));
        // فایل ادامه یافت؛ خط بعدی (نداریم) — فقط crash نشدن مهم است
    }

    #[test]
    fn test_multiple_commands_one_line() {
        // چند فرمان در یک خط به ترتیب اجرا می‌شوند
        let r = interp("G21 G90 M82\nG1 X10 E0.5 F1200");
        assert_eq!(r.lines[0].commands.len(), 3);
        assert_eq!(r.lines[1].line_type, LineType::Extrusion);
    }

    #[test]
    fn test_g28_partial_axes() {
        // G28 X فقط محور X را نامعلوم می‌کند
        let text = "G21\nG90\nG1 X10 Y20 Z0.2 F3000\nG28 X\nG1 Y25 F3000\n";
        let r = interp(text);
        // پس از G28 X: x نامعلوم، y و z معلوم مانده‌اند
        let after_home = &r.lines[3].state_after;
        assert_eq!(after_home.x_mm, None);
        assert_eq!(after_home.y_mm, Some(20.0));
        assert_eq!(after_home.z_mm, Some(0.2));
    }

    #[test]
    fn test_tool_change() {
        let r = interp("T0\nT1\nM104 S200");
        assert_eq!(r.lines[0].state_after.tool, 0);
        assert_eq!(r.lines[1].state_after.tool, 1);
        assert_eq!(r.lines[1].line_type, LineType::ToolChange);
    }

    #[test]
    fn test_state_snapshots_before_after() {
        // snapshot قبل/بعد هر خط برای inspector
        let r = interp("M104 S200\nG1 X10 F1200");
        let l0 = &r.lines[0];
        assert_eq!(l0.state_before.nozzle_temp_c, None);
        assert_eq!(l0.state_after.nozzle_temp_c, Some(200.0));
        let l1 = &r.lines[1];
        assert_eq!(l1.state_before.feedrate_mm_min, None);
        assert_eq!(l1.state_after.feedrate_mm_min, Some(1200.0));
    }
}
