/**
 * ============================================================================
 * components/Viewport.tsx — نمایشگر سه‌بعدی مسیر نازل (Three.js/WebGL2)
 * ============================================================================
 * الزامات Docs/PROMPT_02 §7:
 *   - مسیر چاپی از travel/retract متمایز (رنگ‌بندی + legend متنی — رنگ
 *     تنها راه تفکیک نیست، الزام دسترس‌پذیری)؛
 *   - segment/خط/لایهٔ فعلی برجسته؛
 *   - فیلتر «تا این خط»، «فقط این لایه»، «فقط این خط»؛
 *   - orbit/zoom/pan/fit + grid و محورها و build volume (اگر پروفایل)؛
 *   - برای فایل بزرگ: یک BufferGeometry واحد (نه mesh per segment!)؛
 *   - WebGL نبود → پیام راهنما (قابلیت‌های متنی همچنان کار می‌کنند).
 *
 * نسخهٔ ۱.۲:
 *   - جعبهٔ build volume در «مختصات واقعی G-code» (0..vol) رسم می‌شود —
 *     نه حول مبدأ جهانی؛ طراح هم‌اکنون طرح را مرکز صفحه می‌گذارد؛
 *   - «اسپات» قابل‌درگ: حلقه‌های نارنجی/آبی روی فرم برای جابه‌جایی
 *     بازهٔ z الگو/بافت و شعاع سوراخ کف با کشیدن مستقیم در نمای 3D.
 *
 * نسخهٔ ۱.۳:
 *   - اسپات «دیال چرخش فاز θ»: حلقهٔ بنفش با نشانگر و پیکان مماسی؛
 *     کشیدن نشانگر حول محور، زاویهٔ درز/جهت‌گیری فرم را می‌چرخاند
 *     (درگ مطلق روی صفحهٔ افقی؛ Shift = چفت ۱۵ درجه).
 *
 * نسخهٔ ۱.۴ — کارایی (درخواست کاربر: «نمایشگر خیلی کنده»):
 *   - رندر «به‌اقتضا»: حلقهٔ rAF فقط وقتی چیزی تغییر کرده یا کاربر در
 *     تعامل است رندر می‌کند — نه ۶۰fps دائم روی GPU؛
 *   - رفع پرش دوربین: fit خودکار فقط در «بار اول» یا تغییر دامنهٔ
 *     هندسه (>۱٫۶×)؛ در تغییرات اسلایدر/درگِ اسپات، دوربین کاربر می‌ماند؛
 *   - بازاستفادهٔ بافر: position/color درجا به‌روز می‌شوند (setDrawRange)
 *     و فقط با رشد ظرفیت بازسازی می‌شوند — بدون dispose/recreate پیوسته؛
 *   - سقف devicePixelRatio = ۲ (نمایش HiDPI بدون هزینهٔ بی‌ پایان)؛
 *   - دوربین/مدار در ref مشترک تا fit بیرونی با orbit ناسازگار نشود.
 *
 * نسخهٔ ۱.۴ — اسپات مثل مرجع (دست‌کاری مستقیم روی مدل):
 *   - الگو/بافت: دستهٔ نارنجی/آبی «روی سطح فرم» در زاویهٔ فاز الگو؛
 *     درگ عمودی = جابه‌جایی ارتفاع بازه، درگ افقی (حول محور) = چرخش
 *     زاویهٔ فاز الگو روی فرم — همان تجربهٔ درگ مستقیم اپ مرجع
 *     (بر پایهٔ مستندات تور ویدئویی ۵ گامی؛ جزئیات داخل اپ مرجع
 *     پشت دیوار مجوز است و «کپی» ادعا نمی‌شود — Docs/REFERENCE_AUDIT.md).
 *
 * معماری رندر: LineSegments با BufferGeometry واحد + رنگ per-vertex.
 * برجسته‌سازی: بازنویسی رنگ رأس‌های segmentهای انتخاب‌شده (بدون ساخت
 * شیء جدید → سریع حتی با صدها هزار segment).
 * ============================================================================
 */

import { useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import type { SegmentsPayload } from "../lib/tauri";

/** رنگ‌های نوع segment — معادل CSS vars؛ رنگ اکستروژن از props (رشتهٔ چاپ) */
const BASE_COLORS = {
  extrusion: new THREE.Color("#46c878"),
  travel: new THREE.Color("#5a87d6"),
  retract: new THREE.Color("#d65a5a"),
  unretract: new THREE.Color("#c87846"),
  arc: new THREE.Color("#46a8c8"),
  unknown: new THREE.Color("#9a8fd6"),
  highlight: new THREE.Color("#ff7a28"),
  dimmed: new THREE.Color("#2e333d"),
};

/** خروجی انتخاب segment با کلیک */
export interface ViewportCallbacks {
  onSegmentClick?: (segmentId: number) => void;
}

/** اسپات جابه‌جایی الگو/بافت/کف روی نمای 3D — نسخهٔ ۱.۲ */
export interface SpotHandle {
  /** چه چیزی را جابه‌جا می‌کند */
  kind: "patternRange" | "textureFade" | "floorHole" | "phaseRotation";
  /** بازهٔ فعلی (mm) — zEnd null یعنی تا بالای فرم */
  zStart: number;
  zEnd: number | null;
  /** شعاع مرجع (mm) — شعاع فرم در zStart یا شعاع سوراخ کف */
  radius: number;
  /** ارتفاع کل فرم (mm) برای clamp */
  height: number;
  /** مرکز محور فرم روی صفحه (mm) — از center_on_bed */
  bedOffset: [number, number];
  /**
   * زاویهٔ فاز (درجه) — برای phaseRotation = فاز کل فرم (۱.۳)؛
   * برای patternRange/textureFade = فاز «خودِ الگو/بافت» (۱.۴):
   * دسته روی سطح فرم در همین زاویه می‌نشیند و درگ افقی آن می‌چرخاند.
   */
  phaseDeg?: number;
}

/** رویداد تغییر اسپات با درگ */
export interface SpotChange {
  zStart?: number;
  zEnd?: number | null;
  holeDiameter?: number;
  /** چرخش فاز (درجه ۰..۳۶۰) — ۱.۳: کل فرم؛ ۱.۴: الگو/بافت فعال */
  phaseDeg?: number;
}

export interface ViewportProps extends ViewportCallbacks {
  /** payload فشردهٔ مسیر (null = خالی) — ids می‌تواند خالی باشد (طراح) */
  segments: SegmentsPayload | null;
  /** idهای segmentهای برجسته (خط انتخاب‌شده) */
  highlightIds: Set<number>;
  /** حالت فیلتر: all | upTo(segmentId) | onlyLayer | onlyLine */
  filter: { mode: "all" | "upTo" | "onlyHighlight"; upToId: number };
  /** حجم ساخت [x,y,z] اگر پروفایل انتخاب شده (mm) — مختصات واقعی 0..vol */
  buildVolume?: [number, number, number] | null;
  /** اسپات جابه‌جایی — null = بدون اسپات */
  spot?: SpotHandle | null;
  /** تغییر اسپات با درگ */
  onSpotChange?: (change: SpotChange) => void;
  /** پیام حالت خالی */
  emptyMessage: string;
  /** overlay خطای WebGL */
  webglMessage: string;
  /** رنگ رشتهٔ چاپ (اکستروژن) — انتخاب کاربر؛ پیش‌فرض سبز */
  extrusionColor?: string;
  /** نمونه‌های شعاع (z, r) فرم — نسخهٔ ۱.۵: نمای جامد سایه‌دار.
   *  با این داده، سطح انقلاب با MeshStandardMaterial ساخته می‌شود تا
   *  «طرح» مثل اپ مرجع کاملاً قابل‌ مشاهده باشد (نه فقط خطوط مسیر).
   *  null = بدون سطح جامد (ویرایشگر G-code). */
  surfaceSamples?: number[][] | null;
  /** دیسک کف — نسخهٔ ۱.۵: نمایش کف در نمای جامد.
   *  null = بدون کف؛ { holeDiameter } = دیسک با سوراخ مرکزی. */
  floorDisk?: { holeDiameter: number } | null;
}

/** تبدیل شمارهٔ نوع به رنگ پایه (از مجموعهٔ رنگ نمونه) */
function kindColor(kind: number, colors: typeof BASE_COLORS): THREE.Color {
  switch (kind) {
    case 0: return colors.extrusion;
    case 1: return colors.travel;
    case 2: return colors.retract;
    case 3: return colors.unretract;
    case 4: return colors.arc;
    default: return colors.unknown;
  }
}

/** G-code (x,y,z) → Three.js (x, z→Y, −y→Z) */
function toThree(p: [number, number, number]): [number, number, number] {
  return [p[0], p[2], -p[1]];
}

/** زاویهٔ فاز از مختصات نسبی صفحهٔ افقی Three — خالص و قابل‌آزمون.
 * Three: (x, z) = (x_gc, −y_gc) → θ_gc = atan2(−z, x)؛ خروجی [0..۳۶۰) */
export function dialAngleDeg(relX: number, relZ: number): number {
  let deg = (Math.atan2(-relZ, relX) * 180) / Math.PI;
  deg = ((deg % 360) + 360) % 360;
  return deg;
}

/** چفت زاویه به گام‌های مساوی (مثلاً ۱۵°) — Shift هنگام درگ */
export function snapDeg(deg: number, step: number): number {
  if (step <= 0) return deg;
  const snapped = Math.round(deg / step) * step;
  return ((snapped % 360) + 360) % 360;
}

/** وضعیت درگ اسپات — ۱.۴: قفل جهت ژست (z یا θ) پس از ۸px حرکت */
type SpotDrag = {
  which: "start" | "end" | "hole" | "phase";
  lastX: number;
  lastY: number;
  startX: number;
  startY: number;
  /** حالت قفل‌شدهٔ ژست — تا وقتی null است تصمیم نگرفته‌ایم */
  mode: "z" | "theta" | null;
  /** z صفحهٔ درگ زاویه‌ای (z دسته در آغاز ژست) */
  planeZ: number;
};

export function Viewport(props: ViewportProps) {
  const holderRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const linesRef = useRef<THREE.LineSegments | null>(null);
  const colorsAttrRef = useRef<THREE.BufferAttribute | null>(null);
  const webglFailed = useRef(false);
  const stateRef = useRef(props);
  stateRef.current = props;
  // حلقه‌های اسپات (برای درگ و پاک‌سازی)
  const spotGroupRef = useRef<THREE.Group | null>(null);
  const spotDragRef = useRef<SpotDrag | null>(null);

  // ---- کارایی ۱.۴: رندر به‌اقتضا + دوربین پایدار ----
  /** تغییر صحنه → رندر در فریم بعد (rAF همیشه فعال، رندر فقط هنگام نیاز) */
  const dirtyRef = useRef(true);
  /** شمارندهٔ تعامل فعال (orbit/pan/درگ اسپات) — رندر پیوسته هنگام آن */
  const interactingRef = useRef(0);
  /** کنترل مدار مشترک — تا fit بیرونی با orbit ناسازگار نشود (رفع پرش) */
  const camCtlRef = useRef({
    rotX: 0.75,
    rotY: 0.42,
    dist: 300,
    target: new THREE.Vector3(110, 60, -110),
  });
  /** اعمال دوربین از camCtlRef — در setup effect پر می‌شود */
  const applyCamRef = useRef<(() => void) | null>(null);
  /** آخرین fit — فقط تغییر دامنهٔ بزرگ دوباره fit می‌کند (رفع پرش) */
  const lastFitRef = useRef<{ x: number; y: number; z: number; r: number } | null>(null);
  // ---- نسخهٔ ۱.۵: نمای جامد ----
  const surfaceRef = useRef<THREE.Mesh | null>(null);
  const floorDiskRef = useRef<THREE.Mesh | null>(null);
  /** حالت نما: solid = فقط سطح؛ both = سطح + مسیر؛ path = فقط مسیر */
  const [viewMode, setViewMode] = useState<"solid" | "both" | "path">("both");
  const viewModeRef = useRef(viewMode);
  viewModeRef.current = viewMode;

  const markDirty = () => {
    dirtyRef.current = true;
  };

  // رنگ اکستروژن قابل‌انتخاب (رشتهٔ چاپ) — بازسازی فقط با تغییر مقدار
  const extrusionColor = useMemo(
    () => new THREE.Color(props.extrusionColor ?? "#46c878"),
    [props.extrusionColor]
  );
  // مجموعهٔ رنگ‌های مؤثر این نمونه
  const COLORS = useMemo(
    () => ({ ...BASE_COLORS, extrusion: extrusionColor }),
    [extrusionColor]
  );

  // نگاشت id→ایندکس برای برجسته‌سازی سریع
  const idToIndex = useMemo(() => {
    const m = new Map<number, number>();
    if (props.segments) {
      props.segments.ids.forEach((id, i) => m.set(id, i));
    }
    return m;
  }, [props.segments]);

  // ------------------------------------------------------------------------
  // راه‌اندازی صحنه (یک‌بار)
  // ------------------------------------------------------------------------
  useEffect(() => {
    const holder = holderRef.current;
    if (!holder) return;

    // WebGL در دسترس است؟
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    } catch {
      webglFailed.current = true;
      holder.dataset.webgl = "failed";
      holder.dispatchEvent(new CustomEvent("webgl-failed"));
      return;
    }
    renderer.setClearColor(0x14161c, 1);
    // ۱.۴: سقف DPR=۲ — کیفیت HiDPI بدون رندر ۳× سنگین
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    holder.appendChild(renderer.domElement);
    rendererRef.current = renderer;

    const scene = new THREE.Scene();
    scene.fog = new THREE.Fog(0x14161c, 600, 3000);
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 10000);
    cameraRef.current = camera;

    // ---- نورپردازی (نسخهٔ ۱.۵: غنی‌تر برای نمای جامد سایه‌دار) ----
    // Ambient ملایم + Hemisphere (آسمان/زمین) + Directional اصلی + rim
    // نتیجه: سطح فرم با سایه‌روشن خوانا می‌شود — «طرح دیده شود».
    scene.add(new THREE.AmbientLight(0xffffff, 0.42));
    const hemi = new THREE.HemisphereLight(0xdfe8ff, 0x2a2f3a, 0.55);
    scene.add(hemi);
    const dir = new THREE.DirectionalLight(0xffffff, 0.9);
    dir.position.set(200, 300, 100);
    scene.add(dir);
    const rim = new THREE.DirectionalLight(0x88aaff, 0.35);
    rim.position.set(-250, 120, -180);
    scene.add(rim);

    // ---- grid و محورها (جابه‌جایی به مرکز صفحه در افکت buildVolume) ----
    const grid = new THREE.GridHelper(400, 40, 0x343945, 0x22262f);
    grid.name = "floor-grid";
    grid.position.y = -0.02;
    scene.add(grid);
    const axes = new THREE.AxesHelper(60);
    axes.name = "floor-axes";
    axes.position.y = 0.01;
    scene.add(axes);

    // ---- کنترل مداری دستی (وضعیت در camCtlRef — مشترک با fit) ----
    let dragging = false;
    let panning = false;
    let lastX = 0, lastY = 0;

    const updateCam = () => {
      const ctl = camCtlRef.current;
      ctl.rotY = Math.max(-1.5, Math.min(1.5, ctl.rotY));
      const cy = ctl.target.y + ctl.dist * Math.sin(ctl.rotY);
      const cr = ctl.dist * Math.cos(ctl.rotY);
      camera.position.set(
        ctl.target.x + cr * Math.cos(ctl.rotX),
        cy,
        ctl.target.z + cr * Math.sin(ctl.rotX)
      );
      camera.lookAt(ctl.target);
    };
    applyCamRef.current = () => {
      updateCam();
      markDirty();
    };
    updateCam();

    const raycaster = new THREE.Raycaster();
    raycaster.params.Line = { threshold: 3 };

    /** برخورد ماوس با گروه اسپات (اولویت روی orbit) */
    const spotHit = (e: PointerEvent | MouseEvent): THREE.Object3D | null => {
      const spotGroup = spotGroupRef.current;
      if (!spotGroup || spotGroup.children.length === 0) return null;
      const rect = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const hits = raycaster.intersectObjects(spotGroup.children, false);
      return hits.length > 0 ? hits[0].object : null;
    };

    /** برخورد پرتو ماوس با صفحهٔ افقی z — برای درگ زاویه‌ای اسپات */
    const planeHit = (
      e: PointerEvent | MouseEvent,
      planeZ: number
    ): THREE.Vector3 | null => {
      const rect = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -planeZ);
      const hit = new THREE.Vector3();
      if (raycaster.ray.intersectPlane(plane, hit)) return hit;
      return null;
    };

    const onDown = (e: PointerEvent) => {
      // اول: برخورد با دسته‌های اسپات؟ (اولویت روی orbit)
      const hitObj = spotHit(e);
      if (hitObj && e.button === 0) {
        const name = hitObj.name || "";
        if (name === "spot-start" || name === "spot-end" || name === "spot-hole" || name === "spot-phase") {
          const which: SpotDrag["which"] =
            name === "spot-start" ? "start"
              : name === "spot-end" ? "end"
                : name === "spot-hole" ? "hole"
                  : "phase";
          const spot = stateRef.current.spot;
          // z صفحهٔ درگ زاویه‌ای: z دسته (یا دیال) در آغاز ژست
          const planeZ = which === "phase"
            ? Math.max(spot?.zStart ?? 1.2, 1.2)
            : which === "start" ? Math.max(spot?.zStart ?? 1, 1)
              : which === "end" ? Math.max(spot?.zEnd ?? spot?.zStart ?? 1, 1)
                : 1.0;
          spotDragRef.current = {
            which,
            lastX: e.clientX,
            lastY: e.clientY,
            startX: e.clientX,
            startY: e.clientY,
            mode: null, // تصمیم جهت پس از ۸px
            planeZ,
          };
          interactingRef.current += 1;
          renderer.domElement.style.cursor = "grabbing";
          e.preventDefault();
          return;
        }
      }
      dragging = e.button === 0;
      panning = e.button === 1 || e.button === 2 || e.shiftKey;
      if (dragging || panning) interactingRef.current += 1;
      lastX = e.clientX; lastY = e.clientY;
    };

    const onMove = (e: PointerEvent) => {
      // ---- درگ اسپات ----
      const drag = spotDragRef.current;
      if (drag) {
        const dxTotal = e.clientX - drag.startX;
        const dyTotal = e.clientY - drag.startY;
        // ۱.۴: قفل جهت ژست پس از ۸px — عمودی = ارتفاع، افقی = زاویه
        if (drag.mode === null && (Math.abs(dxTotal) > 8 || Math.abs(dyTotal) > 8)) {
          drag.mode = Math.abs(dyTotal) >= Math.abs(dxTotal) ? "z" : "theta";
        }
        const spot = stateRef.current.spot;
        const cb = stateRef.current.onSpotChange;
        if (spot && cb) {
          // مقیاس حرکت نسبت به فاصلهٔ دوربین (همان ضریب pan)
          const k = Math.max(camCtlRef.current.dist, 40) * 0.0016;
          if (drag.which === "hole") {
            const dx = e.clientX - drag.lastX;
            const d = Math.max(0, spot.radius * 2 + dx * k * 2);
            cb({ holeDiameter: d });
          } else if (drag.which === "phase") {
            // ---- دیال چرخش فاز θ (۱.۳): درگ مطلق روی صفحهٔ دیال ----
            const hit = planeHit(e, drag.planeZ);
            const sg = spotGroupRef.current;
            const ud = (sg?.userData ?? {}) as { phaseCenter?: [number, number] };
            if (hit && ud.phaseCenter) {
              const relX = hit.x - ud.phaseCenter[0];
              const relZ = hit.z - ud.phaseCenter[1];
              if (Math.hypot(relX, relZ) > 2.5) {
                let deg = dialAngleDeg(relX, relZ);
                if (e.shiftKey) deg = snapDeg(deg, 15); // چفت ۱۵°
                cb({ phaseDeg: deg });
              }
            }
          } else if (drag.mode === "theta") {
            // ---- ۱.۴: درگ افقی دسته = چرخش زاویهٔ فاز الگو/بافت ----
            // برخورد پرتو با صفحهٔ افقی در z دسته → زاویهٔ حول محور فرم
            const hit = planeHit(e, drag.planeZ);
            const [ax, ay] = spot.bedOffset;
            const cx = ax;
            const cz = -ay; // Three: y_gc → −z
            if (hit) {
              const relX = hit.x - cx;
              const relZ = hit.z - cz;
              if (Math.hypot(relX, relZ) > 2.0) {
                let deg = dialAngleDeg(relX, relZ);
                if (e.shiftKey) deg = snapDeg(deg, 15);
                cb({ phaseDeg: deg });
              }
            }
          } else {
            // ---- درگ عمودی = ارتفاع (رفتار ۱.۲) ----
            const dy = e.clientY - drag.lastY;
            const dz = -dy * k;
            if (drag.which === "start") {
              const nz = Math.min(
                Math.max(0, spot.zStart + dz),
                spot.zEnd !== null ? spot.zEnd - 1 : spot.height - 1
              );
              cb({ zStart: nz });
            } else if (spot.zEnd !== null) {
              const nz = Math.max(spot.zStart + 1, spot.zEnd + dz);
              cb({ zEnd: Math.min(nz, spot.height) });
            }
          }
        }
        drag.lastX = e.clientX;
        drag.lastY = e.clientY;
        return;
      }
      if (!dragging && !panning) {
        // نشانگر grab روی دسته‌ها
        const hitObj = spotHit(e);
        renderer.domElement.style.cursor = hitObj ? "grab" : "default";
        return;
      }
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      lastX = e.clientX; lastY = e.clientY;
      const ctl = camCtlRef.current;
      if (dragging && !panning) {
        ctl.rotX -= dx * 0.006;
        ctl.rotY += dy * 0.006;
        updateCam();
        markDirty();
      } else if (panning) {
        const panScale = ctl.dist * 0.0016;
        const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0);
        const up = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 1);
        camera.position.addScaledVector(right, -dx * panScale);
        camera.position.addScaledVector(up, dy * panScale);
        ctl.target.addScaledVector(right, -dx * panScale);
        ctl.target.addScaledVector(up, dy * panScale);
        markDirty();
      }
    };
    const onUp = () => {
      if (dragging || panning) interactingRef.current = Math.max(0, interactingRef.current - 1);
      dragging = false;
      panning = false;
      if (spotDragRef.current) {
        spotDragRef.current = null;
        interactingRef.current = Math.max(0, interactingRef.current - 1);
        if (rendererRef.current) {
          rendererRef.current.domElement.style.cursor = "default";
        }
      }
    };
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      camCtlRef.current.dist *= 1 + Math.sign(e.deltaY) * 0.12;
      camCtlRef.current.dist = Math.max(10, Math.min(5000, camCtlRef.current.dist));
      updateCam();
      markDirty();
    };

    // ---- کلیک: انتخاب segment با Raycaster روی LineSegments ----
    const onClick = (e: MouseEvent) => {
      if (spotDragRef.current) return; // درگ اسپات انتخاب نیست
      const lines = linesRef.current;
      const cb = stateRef.current.onSegmentClick;
      if (!lines || !cb) return;
      const rect = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const hits = raycaster.intersectObject(lines);
      if (hits.length > 0 && hits[0].index !== undefined) {
        const segIndex = Math.floor(hits[0].index / 2);
        const segs = stateRef.current.segments;
        if (segs && segIndex < segs.ids.length) {
          cb(segs.ids[segIndex]);
        }
      }
    };

    // ---- دابل‌کلیک: fit دوباره به هندسهٔ جاری (کنترل دستی کاربر) ----
    const onDblClick = () => {
      const lines = linesRef.current;
      if (!lines) return;
      const geo = lines.geometry;
      geo.computeBoundingSphere();
      if (geo.boundingSphere) {
        const c = geo.boundingSphere.center;
        const r = Math.max(geo.boundingSphere.radius, 30);
        const ctl = camCtlRef.current;
        ctl.target.copy(c);
        ctl.dist = r * 2.6;
        ctl.rotX = 0.75;
        ctl.rotY = 0.42;
        lastFitRef.current = { x: c.x, y: c.y, z: c.z, r };
        updateCam();
        markDirty();
      }
    };

    const el = renderer.domElement;
    el.style.touchAction = "none";
    el.addEventListener("pointerdown", onDown);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("click", onClick);
    el.addEventListener("dblclick", onDblClick);
    el.addEventListener("contextmenu", (e) => e.preventDefault());

    // ---- resize ----
    const ro = new ResizeObserver(() => {
      const w = holder.clientWidth || 1;
      const h = holder.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      markDirty();
    });
    ro.observe(holder);
    const w0 = holder.clientWidth || 640;
    const h0 = holder.clientHeight || 480;
    renderer.setSize(w0, h0, false);
    camera.aspect = w0 / h0;
    camera.updateProjectionMatrix();

    // ---- ۱.۴: حلقهٔ رندر به‌اقتضا ----
    // rAF همیشه فعال است ولی «رندر» فقط وقتی صحنه کثیف است یا کاربر در
    // تعامل (orbit/pan/درگ) است انجام می‌شود — بیکاری = صفر بار GPU.
    let raf = 0;
    const loop = () => {
      if (dirtyRef.current || interactingRef.current > 0) {
        renderer.render(scene, camera);
        dirtyRef.current = false;
      }
      raf = requestAnimationFrame(loop);
    };
    loop();

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      el.removeEventListener("pointerdown", onDown);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("click", onClick);
      el.removeEventListener("dblclick", onDblClick);
      renderer.dispose();
      if (el.parentElement === holder) holder.removeChild(el);
    };
  }, []);

  // ------------------------------------------------------------------------
  // به‌روزرسانی هندسه با segments جدید — ۱.۴: بازاستفادهٔ بافر + دوربین پایدار
  // ------------------------------------------------------------------------
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    const segs = props.segments;
    const n = segs ? Math.floor(segs.positions.length / 6) : 0;

    // ---- fit هوشمند: فقط بار اول یا تغییر دامنهٔ بزرگ (رفع پرش دوربین) ----
    // دوربین کاربر در به‌روزرسانی‌های پیوسته (اسلایدر/درگ اسپات) ثابت می‌ماند
    const maybeFit = (cx: number, cy: number, cz: number, r: number) => {
      const last = lastFitRef.current;
      const bigChange =
        !last ||
        r > last.r * 1.6 ||
        r < last.r * 0.4 ||
        Math.hypot(cx - last.x, cy - last.y, cz - last.z) > Math.max(last.r, r);
      if (bigChange && interactingRef.current === 0) {
        const cam = cameraRef.current;
        if (cam) {
          cam.position.set(cx + r * 1.6, cy + r * 1.2, cz + r * 1.6);
          cam.lookAt(cx, cy, cz);
          // همگام‌سازی کنترل مدار (رفع پرش پس از fit هنگام orbit بعدی):
          // جهت دوربین از هدف = (1.6, 1.2, 1.6)·r
          const ctl = camCtlRef.current;
          ctl.target.set(cx, cy, cz);
          ctl.dist = r * Math.hypot(1.6, 1.2, 1.6); // فاصلهٔ واقعی دوربین
          ctl.rotX = Math.atan2(1.6, 1.6); // ≈ π/4
          ctl.rotY = Math.asin(
            Math.max(-0.99, Math.min(0.99, 1.2 / Math.hypot(1.6, 1.2, 1.6)))
          );
        }
        lastFitRef.current = { x: cx, y: cy, z: cz, r };
      }
    };

    // ---- بازاستفادهٔ LineSegments و بافرها (بدون dispose/recreate) ----
    // ۱.۴: بازپیوند به صحنهٔ جاری — در StrictMode (dev) صحنه پس از remount
    // دوباره ساخته می‌شود؛ شیء بازاستفاده‌شده باید به صحنهٔ «فعلی» برگردد
    // (three.js add() خودش از والد قبلی خارج می‌کند)
    let lines = linesRef.current;
    if (lines && lines.parent !== scene) {
      scene.add(lines);
    }
    if (!lines) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.BufferAttribute(new Float32Array(0), 3));
      geo.setAttribute("color", new THREE.BufferAttribute(new Float32Array(0), 3));
      const mat = new THREE.LineBasicMaterial({ vertexColors: true });
      lines = new THREE.LineSegments(geo, mat);
      lines.frustumCulled = false; // setDrawRange داینامیک — culling دستی
      scene.add(lines);
      linesRef.current = lines;
    }
    const geo = lines.geometry;
    const posAttr = geo.getAttribute("position") as THREE.BufferAttribute;
    const colAttr = geo.getAttribute("color") as THREE.BufferAttribute;
    const capacity = posAttr.count; // تعداد «رأس» (۲ برابر segment)

    if (n === 0) {
      geo.setDrawRange(0, 0);
      markDirty();
      return;
    }

    // رشد ظرفیت فقط وقتی لازم است (×۱٫۵ حاشیه)
    if (capacity < n * 2) {
      const newCap = Math.ceil(n * 2 * 1.5);
      const positions = new Float32Array(newCap * 3);
      const colors = new Float32Array(newCap * 3);
      positions.set(posAttr.array as Float32Array, 0);
      colors.set(colAttr.array as Float32Array, 0);
      geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
      const newColAttr = new THREE.BufferAttribute(colors, 3);
      geo.setAttribute("color", newColAttr);
      colorsAttrRef.current = newColAttr;
    } else {
      colorsAttrRef.current = colAttr;
    }

    const posArr = (geo.getAttribute("position") as THREE.BufferAttribute).array as Float32Array;
    const colArr = (colorsAttrRef.current!.array as Float32Array);
    const p = segs!.positions;
    // محدوده برای fit دستی (بدون computeBoundingSphere سنگین)
    let minX = Infinity, minY = Infinity, minZ = Infinity;
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;

    for (let i = 0; i < n; i++) {
      // G-code → Three: (x, y, z) → (x, z, −y)
      const x0 = p[i * 6 + 0], y0 = p[i * 6 + 1], z0 = p[i * 6 + 2];
      const x1 = p[i * 6 + 3], y1 = p[i * 6 + 4], z1 = p[i * 6 + 5];
      const ax = x0, ay = z0, az = -y0;
      const bx = x1, by = z1, bz = -y1;
      posArr[i * 6 + 0] = ax; posArr[i * 6 + 1] = ay; posArr[i * 6 + 2] = az;
      posArr[i * 6 + 3] = bx; posArr[i * 6 + 4] = by; posArr[i * 6 + 5] = bz;
      if (ax < minX) minX = ax; if (ax > maxX) maxX = ax;
      if (ay < minY) minY = ay; if (ay > maxY) maxY = ay;
      if (az < minZ) minZ = az; if (az > maxZ) maxZ = az;
      if (bx < minX) minX = bx; if (bx > maxX) maxX = bx;
      if (by < minY) minY = by; if (by > maxY) maxY = by;
      if (bz < minZ) minZ = bz; if (bz > maxZ) maxZ = bz;
      // رنگ نوع — نوشتن مستقیم بدون تخصیص آرایه (کارایی)
      const c = kindColor(segs!.kinds[i], COLORS);
      colArr[i * 6 + 0] = c.r; colArr[i * 6 + 1] = c.g; colArr[i * 6 + 2] = c.b;
      colArr[i * 6 + 3] = c.r; colArr[i * 6 + 4] = c.g; colArr[i * 6 + 5] = c.b;
    }
    (geo.getAttribute("position") as THREE.BufferAttribute).needsUpdate = true;
    colorsAttrRef.current!.needsUpdate = true;
    geo.setDrawRange(0, n * 2);

    // fit هوشمند (۱.۴ — رفع پرش دوربین در هر تغییر اسلایدر)
    const cx = (minX + maxX) / 2, cy = (minY + maxY) / 2, cz = (minZ + maxZ) / 2;
    const r = Math.max(
      Math.max(maxX - minX, maxY - minY, maxZ - minZ) / 2,
      30
    );
    // bounding sphere دستی از محدودهٔ دادهٔ «پرشده» — computeBoundingSphere
    // روی کل ظرفیت (شامل دنبالهٔ صفر) حساب می‌شود و کرهٔ غلط می‌داد
    geo.boundingSphere = new THREE.Sphere(new THREE.Vector3(cx, cy, cz), r);
    maybeFit(cx, cy, cz, r);
    markDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.segments, COLORS]);

  // ------------------------------------------------------------------------
  // برجسته‌سازی + فیلتر — فقط بازنویسی رنگ‌ها (سریع) + رندر به‌اقتضا
  // ------------------------------------------------------------------------
  useEffect(() => {
    const attr = colorsAttrRef.current;
    const segs = props.segments;
    if (!attr || !segs) return;
    const n = Math.floor(segs.positions.length / 6);
    const arr = attr.array as Float32Array;

    const upToIdx = props.filter.mode === "upTo"
      ? (idToIndex.get(props.filter.upToId) ?? n - 1)
      : n - 1;

    for (let i = 0; i < n; i++) {
      const visible =
        props.filter.mode === "upTo" ? i <= upToIdx : true;
      const isHl = segs.ids.length > 0 && props.highlightIds.has(segs.ids[i]);
      let c: THREE.Color;
      if (!visible) {
        c = COLORS.dimmed;
      } else if (isHl) {
        c = COLORS.highlight;
      } else if (props.filter.mode === "onlyHighlight") {
        c = COLORS.dimmed;
      } else {
        c = kindColor(segs.kinds[i], COLORS);
      }
      arr[i * 6 + 0] = c.r; arr[i * 6 + 1] = c.g; arr[i * 6 + 2] = c.b;
      arr[i * 6 + 3] = c.r; arr[i * 6 + 4] = c.g; arr[i * 6 + 5] = c.b;
    }
    attr.needsUpdate = true;
    markDirty();
  }, [props.highlightIds, props.filter, props.segments, idToIndex, COLORS]);

  // ------------------------------------------------------------------------
  // build volume — مختصات واقعی G-code: صفحه از (0,0) تا (vol_x, vol_y)
  // ------------------------------------------------------------------------
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    // حذف قبلی
    const old = scene.getObjectByName("build-volume");
    if (old) {
      scene.remove(old);
      (old as THREE.LineSegments).geometry.dispose();
    }
    const vol = props.buildVolume;
    // grid/axes به مرکز صفحه (اگر حجم ساخت معلوم است)
    const grid = scene.getObjectByName("floor-grid");
    const axes = scene.getObjectByName("floor-axes");
    if (vol) {
      const bedCx = vol[0] / 2;
      const bedCy = vol[1] / 2;
      if (grid) {
        const size = Math.ceil(Math.max(vol[0], vol[1]) / 20) * 20;
        // بازسازی grid با اندازهٔ صفحه (GridHelper اندازه ثابت دارد)
        const g = grid as THREE.GridHelper;
        g.scale.setScalar(size / 400);
        g.position.set(bedCx, -0.02, -bedCy);
      }
      if (axes) axes.position.set(0, 0.01, 0);
      // جعبهٔ حجم ساخت: x∈[0,vol_x], y∈[0,vol_z], z∈[-vol_y,0]
      const box = new THREE.BoxGeometry(vol[0], vol[2], vol[1]);
      const edges = new THREE.EdgesGeometry(box);
      const lines = new THREE.LineSegments(
        edges,
        new THREE.LineBasicMaterial({ color: 0x7d8392, transparent: true, opacity: 0.55 })
      );
      lines.name = "build-volume";
      lines.position.set(vol[0] / 2, vol[2] / 2, -vol[1] / 2);
      scene.add(lines);
    } else {
      if (grid) {
        (grid as THREE.GridHelper).scale.setScalar(1);
        grid.position.set(0, -0.02, 0);
      }
      if (axes) axes.position.set(0, 0.01, 0);
    }
    markDirty();
  }, [props.buildVolume]);

  // ------------------------------------------------------------------------
  // نمای جامد — نسخهٔ ۱.۵: سطح انقلاب از radius_samples + دیسک کف
  // ------------------------------------------------------------------------
  // radius_samples = ۱۲۹ نمونهٔ (z, r) «با الگوها» از هستهٔ Rust؛ چرخش
  // حول محور ⇒ سطح دقیق فرم. تراکم: ۴۸ ستون زاویه‌ای (~۱۲k مثلث) — سبک.
  // z پایه از کمینهٔ z اکستروژن‌ها محاسبه می‌شود تا سطح با مسیر هم‌راستا
  // شود (پس از کفِ چندلایه، فرم از بالای کف شروع می‌شود).
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    // حذف قبلی
    for (const ref of [surfaceRef, floorDiskRef]) {
      if (ref.current) {
        scene.remove(ref.current);
        ref.current.geometry.dispose();
        (ref.current.material as THREE.Material).dispose();
        ref.current = null;
      }
    }
    const samples = props.surfaceSamples;
    if (!samples || samples.length < 2) return;

    // z پایه: کمینهٔ z اکستروژن‌ها (شروع واقعی فرم روی صفحه)
    const segs = props.segments;
    let zBase = 0;
    if (segs) {
      const n = Math.floor(segs.positions.length / 6);
      let minZ = Infinity;
      for (let i = 0; i < n; i++) {
        if (segs.kinds[i] === 0) {
          minZ = Math.min(minZ, segs.positions[i * 6 + 2], segs.positions[i * 6 + 5]);
        }
      }
      if (Number.isFinite(minZ)) zBase = minZ;
    }

    // ---- سطح انقلاب ----
    const rows = samples.length;
    const cols = 48;
    const positions = new Float32Array(rows * (cols + 1) * 3);
    const normals = new Float32Array(rows * (cols + 1) * 3);
    const indices: number[] = [];
    // مرکز فرم روی صفحه (bedOffset اسپات — پیش‌فرض مرکز صفحهٔ چاپ)
    const [ax, ay] = props.spot?.bedOffset ?? [110, 110];
    const center3x = ax;
    const center3z = -ay;
    for (let i = 0; i < rows; i++) {
      const z = zBase + samples[i][0];
      const r = Math.max(samples[i][1], 0.05);
      for (let j = 0; j <= cols; j++) {
        const th = (j / cols) * Math.PI * 2;
        const k = (i * (cols + 1) + j) * 3;
        positions[k] = center3x + r * Math.cos(th);
        positions[k + 1] = z;
        positions[k + 2] = center3z - r * Math.sin(th);
      }
    }
    // نرمال‌ها: هر رأس → از سطح بیرون (تقریب شعاعی + شیب z)
    for (let i = 0; i < rows; i++) {
      const r = Math.max(samples[i][1], 0.05);
      const rPrev = i > 0 ? Math.max(samples[i - 1][1], 0.05) : r;
      const rNext = i < rows - 1 ? Math.max(samples[i + 1][1], 0.05) : r;
      const dr = (rNext - rPrev) / 2;
      // نرمال در صفحهٔ (r,z): (1, -dr) نرمال‌شده (بیرون‌رونده)
      const len = Math.hypot(1, dr) || 1;
      const nr = 1 / len;
      const nz = -dr / len;
      for (let j = 0; j <= cols; j++) {
        const th = (j / cols) * Math.PI * 2;
        const k = (i * (cols + 1) + j) * 3;
        normals[k] = nr * Math.cos(th);
        normals[k + 1] = nz;
        normals[k + 2] = -nr * Math.sin(th);
      }
    }
    // مثلث‌ها — دو نیمهٔ هر quad
    for (let i = 0; i < rows - 1; i++) {
      for (let j = 0; j < cols; j++) {
        const a = i * (cols + 1) + j;
        const b = a + 1;
        const c = (i + 1) * (cols + 1) + j;
        const d = c + 1;
        indices.push(a, c, d, a, d, b);
      }
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geo.setAttribute("normal", new THREE.BufferAttribute(normals, 3));
    geo.setIndex(indices);
    const mat = new THREE.MeshStandardMaterial({
      color: extrusionColor,
      side: THREE.DoubleSide,
      roughness: 0.5,
      metalness: 0.08,
      // کمی درخشش داخلی تا فرم روی پس‌زمینهٔ تیره «جان» بگیرد
      emissive: new THREE.Color(extrusionColor).multiplyScalar(0.06),
    });
    const surface = new THREE.Mesh(geo, mat);
    surface.name = "solid-surface";
    surface.renderOrder = 1;
    scene.add(surface);
    surfaceRef.current = surface;

    // ---- دیسک کف (اگر کف فعال است) ----
    const fd = props.floorDisk;
    if (fd) {
      const rOuter = Math.max(samples[0][1], 1);
      const rHole = Math.max(fd.holeDiameter / 2, 0);
      const diskGeo = rHole > 0.4
        ? new THREE.RingGeometry(rHole, rOuter, 64)
        : new THREE.CircleGeometry(rOuter, 64);
      // RingGeometry در صفحهٔ XY → چرخش به افقی
      const diskMat = new THREE.MeshStandardMaterial({
        color: extrusionColor,
        side: THREE.DoubleSide,
        roughness: 0.7,
        metalness: 0.02,
      });
      const disk = new THREE.Mesh(diskGeo, diskMat);
      disk.rotation.x = -Math.PI / 2;
      disk.position.set(center3x, Math.max(zBase - 0.4, 0.05), center3z);
      disk.name = "solid-floor";
      disk.renderOrder = 1;
      scene.add(disk);
      floorDiskRef.current = disk;
    }

    // bounding دستی برای fit اولیهٔ سطح (وقتی مسیر هنوز خالی است)
    markDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.surfaceSamples, props.floorDisk, extrusionColor]);

  // ------------------------------------------------------------------------
  // تغییر حالت نما (جامد/هر دو/مسیر) — فقط visible سوییچ می‌شود
  // ------------------------------------------------------------------------
  useEffect(() => {
    const solid = viewMode !== "path";
    const path = viewMode !== "solid";
    if (surfaceRef.current) surfaceRef.current.visible = solid;
    if (floorDiskRef.current) floorDiskRef.current.visible = solid;
    if (linesRef.current) linesRef.current.visible = path;
    markDirty();
  }, [viewMode]);

  // تغییر رنگ رشتهٔ چاپ → به‌روزرسانی متریال سطح
  useEffect(() => {
    if (surfaceRef.current) {
      (surfaceRef.current.material as THREE.MeshStandardMaterial).color.copy(extrusionColor);
      (surfaceRef.current.material as THREE.MeshStandardMaterial).emissive.copy(
        new THREE.Color(extrusionColor).multiplyScalar(0.06)
      );
    }
    if (floorDiskRef.current) {
      (floorDiskRef.current.material as THREE.MeshStandardMaterial).color.copy(extrusionColor);
    }
    markDirty();
  }, [extrusionColor]);

  // ------------------------------------------------------------------------
  // اسپات — ۱.۴: دستهٔ مستقیم روی سطح فرم (مثل مرجع) + حلقه‌های ظریف
  // ------------------------------------------------------------------------
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    // حذف گروه قبلی
    if (spotGroupRef.current) {
      scene.remove(spotGroupRef.current);
      spotGroupRef.current.traverse((o) => {
        const m = o as THREE.Mesh;
        if (m.geometry) m.geometry.dispose();
      });
      spotGroupRef.current = null;
    }
    const spot = props.spot;
    if (!spot) return;

    const group = new THREE.Group();
    group.name = "spot-group";
    const [ax, ay] = spot.bedOffset;
    const center = toThree([ax, ay, 0]); // مرکز محور فرم در Three
    // ضخامت لوله: به‌اندازهٔ کافی کلفت تا با ماوس راحت گرفته شود (۹٪ شعاع)
    const tube = Math.min(Math.max(spot.radius * 0.09, 1.8), 4.0);

    const makeRing = (
      z: number,
      radius: number,
      color: number,
      name: string,
      opacity = 0.85
    ) => {
      const geo = new THREE.TorusGeometry(Math.max(radius, 2), tube, 10, 64);
      // ۱.۵: depthTest خاموش + renderOrder — حلقه روی سطح جامد هم پیدا/دیده می‌شود
      const mat = new THREE.MeshBasicMaterial({
        color,
        transparent: true,
        opacity,
        depthTest: false,
      });
      const ring = new THREE.Mesh(geo, mat);
      ring.rotation.x = Math.PI / 2; // افقی
      ring.renderOrder = 900; // بالای سطح جامد
      ring.position.set(center[0], z, center[2]);
      ring.name = name;
      return ring;
    };

    /**
     * دستهٔ اسپات «روی سطح فرم» — ۱.۴ (مثل مرجع).
     * کرهٔ همیشه‌رو در زاویهٔ فاز روی شعاع فرم + هاله؛ درگ عمودی = z،
     * درگ افقی = چرخش فاز. depthTest:false تا پشت فرم هم قابل‌گرفتن باشد.
     */
    const makeSurfaceHandle = (
      z: number,
      radius: number,
      phaseDeg: number,
      color: number,
      name: string
    ) => {
      const th = (phaseDeg * Math.PI) / 180;
      // G-code: x = cx + r·cosθ, y = cy + r·sinθ → Three: (x, z, −y)
      const hx = center[0] + radius * Math.cos(th);
      const hz = center[2] - radius * Math.sin(th);
      const dotGeo = new THREE.SphereGeometry(Math.max(3.2, tube * 2.2), 14, 14);
      const dotMat = new THREE.MeshBasicMaterial({ color, depthTest: false });
      const dot = new THREE.Mesh(dotGeo, dotMat);
      dot.renderOrder = 1000;
      dot.position.set(hx, z, hz);
      dot.name = name;
      // هالهٔ روشن — دیده‌شدن روی هر پس‌زمینه
      const haloGeo = new THREE.TorusGeometry(Math.max(4.8, tube * 3.4), tube * 0.4, 8, 24);
      const haloMat = new THREE.MeshBasicMaterial({
        color: 0xffffff,
        transparent: true,
        opacity: 0.55,
        depthTest: false,
      });
      const halo = new THREE.Mesh(haloGeo, haloMat);
      halo.renderOrder = 1001;
      halo.rotation.x = Math.PI / 2;
      halo.position.set(hx, z, hz);
      halo.name = name;
      group.add(halo);
      return dot;
    };

    if (spot.kind === "floorHole") {
      // حلقهٔ سوراخ کف — درگ افقی شعاع را عوض می‌کند
      if (spot.radius > 0.5) {
        group.add(makeRing(1.0, spot.radius, 0xf0883e, "spot-hole"));
      }
      // نشانگر مرکز (نقطهٔ اسپات) — همان رفتار درگ حلقهٔ سوراخ
      const dotGeo = new THREE.SphereGeometry(Math.max(2.2, tube * 1.6), 12, 12);
      const dotMat = new THREE.MeshBasicMaterial({ color: 0xf0883e });
      const dot = new THREE.Mesh(dotGeo, dotMat);
      dot.position.set(center[0], 1.0, center[2]);
      dot.name = "spot-hole";
      group.add(dot);
    } else if (spot.kind === "phaseRotation") {
      // ---- دیال چرخش فاز θ (نسخهٔ ۱.۳) ----
      // حلقهٔ بنفش کم‌رنگ + نشانگر روی زاویهٔ فعلی + پیکان مماسی
      // (جهت افزایش θ). درگ نشانگر حول محور = چرخش فرم.
      const dialR = Math.max(spot.radius * 1.18 + 4, 12);
      const zDial = Math.max(spot.zStart, 1.2);
      const ring = new THREE.Mesh(
        new THREE.TorusGeometry(dialR, tube * 0.5, 8, 72),
        new THREE.MeshBasicMaterial({ color: 0xb06ce0, transparent: true, opacity: 0.4 })
      );
      ring.rotation.x = Math.PI / 2;
      ring.position.set(center[0], zDial, center[2]);
      ring.name = "spot-phase";
      group.add(ring);
      // نشانگر روی زاویهٔ فعلی θ (G-code: x=cosθ, y=sinθ → سه: z=−sinθ)
      // همیشه‌رو (depthTest=false): حتی وقتی پشت فرم است دیده/گرفته می‌شود
      const th = ((spot.phaseDeg ?? 0) * Math.PI) / 180;
      const mx = center[0] + dialR * Math.cos(th);
      const mz = center[2] - dialR * Math.sin(th);
      const dotGeo = new THREE.SphereGeometry(Math.max(3.4, tube * 2.4), 14, 14);
      const dotMat = new THREE.MeshBasicMaterial({ color: 0xb06ce0, depthTest: false });
      const dot = new THREE.Mesh(dotGeo, dotMat);
      dot.renderOrder = 1000;
      dot.position.set(mx, zDial, mz);
      dot.name = "spot-phase";
      group.add(dot);
      // حلقهٔ هالهٔ نشانگر — دیده‌شدن بهتر روی هر پس‌زمینه
      const haloGeo = new THREE.TorusGeometry(Math.max(5.2, tube * 3.6), tube * 0.42, 8, 24);
      const haloMat = new THREE.MeshBasicMaterial({ color: 0xe8d5ff, transparent: true, opacity: 0.85, depthTest: false });
      const halo = new THREE.Mesh(haloGeo, haloMat);
      halo.renderOrder = 1001;
      halo.rotation.x = Math.PI / 2;
      halo.position.set(mx, zDial, mz);
      halo.name = "spot-phase";
      group.add(halo);
      // پیکان مماسی — جهت افزایش θ (پادساعت‌گرد G-code)
      const tangent = new THREE.Vector3(-Math.sin(th), 0, -Math.cos(th)).normalize();
      const cone = new THREE.Mesh(
        new THREE.ConeGeometry(tube * 1.3, tube * 3.2, 10),
        new THREE.MeshBasicMaterial({ color: 0xb06ce0, depthTest: false })
      );
      cone.renderOrder = 1000;
      cone.position.set(
        mx + tangent.x * tube * 3.0,
        zDial,
        mz + tangent.z * tube * 3.0
      );
      cone.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), tangent);
      cone.name = "spot-phase";
      group.add(cone);
      // مرکز دیال برای درگ مطلق (برخورد پرتو با صفحهٔ افقی)
      group.userData.phaseCenter = [center[0], center[2]];
      group.userData.phaseZ = zDial;
    } else {
      // ---- الگو/بافت — ۱.۴: حلقه‌های ظریف + دسته‌های روی سطح فرم ----
      const phase = spot.phaseDeg ?? 0;
      // حلقهٔ شروع (نارنجی ظریف) + دستهٔ سطحی در زاویهٔ فاز
      group.add(makeRing(spot.zStart, spot.radius, 0xf0883e, "spot-start", 0.55));
      group.add(
        makeSurfaceHandle(spot.zStart, spot.radius, phase, 0xf0883e, "spot-start")
      );
      // حلقهٔ پایان (آبی) + دسته — اگر بازهٔ محدود است
      if (spot.zEnd !== null) {
        group.add(makeRing(spot.zEnd, spot.radius, 0x5a87d6, "spot-end", 0.55));
        group.add(
          makeSurfaceHandle(spot.zEnd, spot.radius, phase, 0x5a87d6, "spot-end")
        );
      }
    }
    scene.add(group);
    spotGroupRef.current = group;
    markDirty();

    return () => {
      const sc = sceneRef.current;
      if (sc && spotGroupRef.current) {
        sc.remove(spotGroupRef.current);
        spotGroupRef.current.traverse((o) => {
          const m = o as THREE.Mesh;
          if (m.geometry) m.geometry.dispose();
        });
        spotGroupRef.current = null;
      }
    };
  }, [props.spot]);

  const segCount = props.segments ? Math.floor(props.segments.positions.length / 6) : 0;

  return (
    <div className="viewport" ref={holderRef}>
      {segCount === 0 && (
        <div className="overlay-msg">{props.emptyMessage}</div>
      )}
      {props.surfaceSamples && props.surfaceSamples.length > 1 && (
        <div className="viewmode-toggle" role="radiogroup" aria-label="حالت نمای سه‌بعدی">
          {([
            ["solid", "جامد"],
            ["both", "جامد + مسیر"],
            ["path", "فقط مسیر"],
          ] as const).map(([mode, label]) => (
            <button
              key={mode}
              className={`vm-btn ${viewMode === mode ? "active" : ""}`}
              role="radio"
              aria-checked={viewMode === mode}
              onClick={() => setViewMode(mode)}
            >
              {label}
            </button>
          ))}
        </div>
      )}
      <div className="legend">
        <span><span className="sw" style={{ background: "#" + extrusionColor.getHexString() }} />چاپ (اکستروژن)</span>
        <span><span className="sw" style={{ background: "#5a87d6" }} />حرکت خالی</span>
        <span><span className="sw" style={{ background: "#d65a5a" }} />عقب‌کشیدن</span>
        <span><span className="sw" style={{ background: "#46a8c8" }} />قوس</span>
        <span><span className="sw" style={{ background: "#ff7a28" }} />انتخاب‌شده</span>
        {props.spot && (
          <span className="spot-hint">
            <span className="sw" style={{ background: props.spot.kind === "phaseRotation" ? "#b06ce0" : "#f0883e" }} />
            {props.spot.kind === "phaseRotation"
              ? "اسپات θ: نشانگر بنفش را حول محور بچرخانید (Shift = ۱۵°)"
              : props.spot.kind === "floorHole"
                ? "اسپات: حلقه را بکشید تا قطر سوراخ کف تغییر کند"
                : "اسپات: دسته را عمودی بکشید (ارتفاع) یا افقی (چرخش زاویه — Shift = ۱۵°)"}
          </span>
        )}
      </div>
    </div>
  );
}
