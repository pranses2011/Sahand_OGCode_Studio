//! ============================================================================
//! toolpath/mod.rs — Toolpath IR: بازنمایی میانی مسیر ابزار
//! ============================================================================
//! «IR» یعنی Intermediate Representation (بازنمایی میانی).
//! این ساختار «منبع واحد حقیقت» بین سه مصرف‌کننده است:
//!   ۱) viewport سه‌بعدی (پیش‌نمایش)؛
//!   ۲) خروجی‌ساز G-code طراح مارپیچی؛
//!   ۳) جدول وضعیت/شبیه‌سازی.
//! طبق Docs/PROMPT_03 §5: «همان مسیر برای 3D preview و خروجی G-code» —
//! یعنی چیزی که کاربر می‌بیند همان چیزی است که صادر می‌شود.
//!
//! طراحی برای IPC کارآمد: segmentها با Option فشرده سریال می‌شوند و در لایهٔ
//! commands به آرایه‌های عددی فشرده برای WebView تبدیل می‌شوند (نه JSON
//! میلیونی — الزام Docs/PROMPT_04 §2).
//! ============================================================================

use serde::{Deserialize, Serialize};

/// زیرماژول tessellation قوس‌های G2/G3
pub mod arcs;

// re-export برای راحتی import
pub use arcs::{tessellate_arc, ArcError, ArcParams, Plane};

/// نوع segment از نظر فیزیکی چاپ (رنگ‌بندی viewport و inspector)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    /// حرکت با اکستروژن مثبت → چاپ واقعی
    Extrusion,
    /// حرکت بدون اکستروژن → travel
    Travel,
    /// کاهش E (عقب‌کشیدن فیلامنت) → retract
    Retract,
    /// بازگرداندن E بدون حرکت XY → unretract
    Unretract,
    /// segment تولیدشده از قوس G2/G3
    Arc,
    /// حرکت با موقعیت ناشناخته (مثلاً بعد از G28 بدون پروفایل)
    Unknown,
}

/// درجهٔ اطمینان به مختصات segment
/// الزام: «از unknown/approximate به‌جای تبدیل ابهام به عدد ظاهراً دقیق»
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Certainty {
    /// مختصات از فرمان‌های معلوم محاسبه شده
    Known,
    /// تقریبی: مثلاً بعد از homing بدون پروفایل یا قوس با پارامتر مرزی
    Approximate,
    /// ناشناخته: موقعیت فعلی ماشین معلوم نیست
    Unknown,
}

/// یک بازهٔ حرکت نازل — واحد همیشه میلی‌متر است
/// (تبدیل اینچ→mm در مفسر انجام می‌شود، نه در viewport)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolpathSegment {
    /// شناسهٔ یکتا برای raycast/انتخاب در viewport
    pub id: u64,
    /// ایندکس خط مبدأ در سند (چند segment می‌توانند به یک خط اشاره کنند،
    /// مثلاً قطعات tessellation یک قوس)
    pub source_line_index: usize,
    /// نوع فیزیکی حرکت
    pub kind: SegmentKind,
    /// نقطهٔ شروع [x,y,z] به mm — None یعنی نامعلوم
    pub start_mm: Option<[f64; 3]>,
    /// نقطهٔ پایان [x,y,z] به mm — None یعنی نامعلوم
    pub end_mm: Option<[f64; 3]>,
    /// تغییر E در این segment (mm فیلامنت) — مثبت=اکستروژن، منفی=retract
    pub e_delta_mm: Option<f64>,
    /// سرعت درخواستی (mm/min)
    pub feedrate_mm_min: Option<f64>,
    /// ایندکس لایه اگر قابل تعیین بود
    pub layer_index: Option<u32>,
    /// درجهٔ اطمینان مختصات
    pub certainty: Certainty,
}

/// اطلاعات یک لایه برای نوار انتخاب لایه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub index: u32,
    /// z شروع لایه (mm) اگر معلوم بود
    pub z_mm: Option<f64>,
    /// بازهٔ ایندکس segmentهای لایه [شروع، پایان)
    pub first_segment: u64,
    pub last_segment: u64,
    /// شمارهٔ خط اولین تغییر Z مربوط به این لایه (برای jump-to-line)
    pub source_line_index: usize,
}

/// خلاصهٔ آماری مسیر برای نوار وضعیت
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolpathStats {
    pub segment_count: usize,
    pub extrusion_segments: usize,
    pub travel_segments: usize,
    pub retract_count: usize,
    /// طول کل مسیر چاپی (mm) — تخمینی
    pub extrusion_length_mm: f64,
    /// طول کل travel (mm) — تخمینی
    pub travel_length_mm: f64,
    /// مجموع فیلامنت مصرفی (mm) — تخمینی
    pub total_e_mm: f64,
}

impl ToolpathStats {
    /// به‌روزرسانی آمار بر اساس یک segment — از interpreter صدا زده می‌شود
    pub fn accumulate(&mut self, seg: &ToolpathSegment) {
        self.segment_count += 1;
        // طول مسیر فقط وقتی هر دو سر معلوم‌اند
        let length = match (seg.start_mm, seg.end_mm) {
            (Some(a), Some(b)) => {
                let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            }
            _ => 0.0,
        };
        match seg.kind {
            SegmentKind::Extrusion | SegmentKind::Arc => {
                self.extrusion_segments += 1;
                self.extrusion_length_mm += length;
                if let Some(e) = seg.e_delta_mm {
                    self.total_e_mm += e.max(0.0);
                }
            }
            SegmentKind::Travel | SegmentKind::Unknown => {
                self.travel_segments += 1;
                self.travel_length_mm += length;
            }
            SegmentKind::Retract => {
                self.retract_count += 1;
            }
            SegmentKind::Unretract => {}
        }
    }
}

// ============================================================================
// آزمون‌های واحد
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_accumulate() {
        let mut stats = ToolpathStats::default();
        // یک segment چاپی ۳-۴-۵ (طول ۵)
        stats.accumulate(&ToolpathSegment {
            id: 0,
            source_line_index: 0,
            kind: SegmentKind::Extrusion,
            start_mm: Some([0.0, 0.0, 0.0]),
            end_mm: Some([3.0, 4.0, 0.0]),
            e_delta_mm: Some(1.5),
            feedrate_mm_min: Some(1200.0),
            layer_index: Some(0),
            certainty: Certainty::Known,
        });
        // یک travel
        stats.accumulate(&ToolpathSegment {
            id: 1,
            source_line_index: 1,
            kind: SegmentKind::Travel,
            start_mm: Some([3.0, 4.0, 0.0]),
            end_mm: Some([13.0, 4.0, 0.0]),
            e_delta_mm: None,
            feedrate_mm_min: Some(6000.0),
            layer_index: Some(0),
            certainty: Certainty::Known,
        });
        assert_eq!(stats.segment_count, 2);
        assert_eq!(stats.extrusion_segments, 1);
        assert_eq!(stats.travel_segments, 1);
        assert!((stats.extrusion_length_mm - 5.0).abs() < 1e-9);
        assert!((stats.travel_length_mm - 10.0).abs() < 1e-9);
        assert!((stats.total_e_mm - 1.5).abs() < 1e-9);
    }
}
