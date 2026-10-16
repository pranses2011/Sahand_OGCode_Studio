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
 * معماری رندر: LineSegments با BufferGeometry واحد + رنگ per-vertex.
 * برجسته‌سازی: بازنویسی رنگ رأس‌های segmentهای انتخاب‌شده (بدون ساخت
 * شیء جدید → سریع حتی با صدها هزار segment).
 * ============================================================================
 */

import { useEffect, useMemo, useRef } from "react";
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
  /** زاویهٔ فعلی فاز (درجه) — فقط برای phaseRotation (نسخهٔ ۱.۳) */
  phaseDeg?: number;
}

/** رویداد تغییر اسپات با درگ */
export interface SpotChange {
  zStart?: number;
  zEnd?: number | null;
  holeDiameter?: number;
  /** چرخش فاز θ (درجه ۰..۳۶۰) — نسخهٔ ۱.۳ */
  phaseDeg?: number;
}

export interface ViewportProps extends ViewportCallbacks {
  /** payload فشردهٔ مسیر از parse_gcode (null = خالی) */
  segments: SegmentsPayload | null;
  /** idهای segmentهای برجسته (خط انتخاب‌شده) */
  highlightIds: Set<number>;
  /** حالت فیلتر: all | upTo(segmentId) | onlyLayer | onlyLine */
  filter: { mode: "all" | "upTo" | "onlyHighlight"; upToId: number };
  /** حجم ساخت [x,y,z] اگر پروفایل انتخاب شده (mm) — مختصات واقعی 0..vol */
  buildVolume?: [number, number, number] | null;
  /** اسپات جابه‌جایی (نسخهٔ ۱.۲) — null = بدون اسپات */
  spot?: SpotHandle | null;
  /** تغییر اسپات با درگ */
  onSpotChange?: (change: SpotChange) => void;
  /** پیام حالت خالی */
  emptyMessage: string;
  /** overlay خطای WebGL */
  webglMessage: string;
  /** رنگ رشتهٔ چاپ (اکستروژن) — انتخاب کاربر؛ پیش‌فرض سبز */
  extrusionColor?: string;
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
  const spotDragRef = useRef<null | { which: "start" | "end" | "hole" | "phase"; lastX: number; lastY: number }>(null);

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
    renderer.setPixelRatio(window.devicePixelRatio);
    holder.appendChild(renderer.domElement);
    rendererRef.current = renderer;

    const scene = new THREE.Scene();
    scene.fog = new THREE.Fog(0x14161c, 600, 3000);
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 10000);
    camera.position.set(160, 140, 160);
    camera.lookAt(0, 60, 0);
    cameraRef.current = camera;

    // ---- نورپردازی ساده (برای mesh و حلقه‌های اسپات) ----
    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const dir = new THREE.DirectionalLight(0xffffff, 0.8);
    dir.position.set(200, 300, 100);
    scene.add(dir);

    // ---- grid و محورها (جابه‌جایی به مرکز صفحه در افکت buildVolume) ----
    const grid = new THREE.GridHelper(400, 40, 0x343945, 0x22262f);
    grid.name = "floor-grid";
    grid.position.y = -0.02;
    scene.add(grid);
    const axes = new THREE.AxesHelper(60);
    axes.name = "floor-axes";
    axes.position.y = 0.01;
    scene.add(axes);

    // ---- کنترل مداری دستی ----
    let dragging = false;
    let panning = false;
    let lastX = 0, lastY = 0;
    let rotX = Math.atan2(camera.position.z - 0, camera.position.x - 0);
    let rotY = Math.asin(
      (camera.position.y - 60) /
        Math.hypot(camera.position.x, camera.position.y - 60, camera.position.z)
    );
    let dist = camera.position.length() - 60;
    // هدف مدار (با build volume به مرکز صفحه می‌رود)
    let orbitTarget = new THREE.Vector3(0, 60, 0);

    const updateCam = () => {
      rotY = Math.max(-1.5, Math.min(1.5, rotY));
      const cy = orbitTarget.y + dist * Math.sin(rotY);
      const cr = dist * Math.cos(rotY);
      camera.position.set(
        orbitTarget.x + cr * Math.cos(rotX),
        cy,
        orbitTarget.z + cr * Math.sin(rotX)
      );
      camera.lookAt(orbitTarget);
    };

    const raycaster = new THREE.Raycaster();
    raycaster.params.Line = { threshold: 3 };

    const onDown = (e: PointerEvent) => {
      // اول: برخورد با حلقه‌های اسپات؟ (اولویت روی orbit)
      const spotGroup = spotGroupRef.current;
      if (spotGroup && spotGroup.children.length > 0) {
        const rect = renderer.domElement.getBoundingClientRect();
        const ndc = new THREE.Vector2(
          ((e.clientX - rect.left) / rect.width) * 2 - 1,
          -((e.clientY - rect.top) / rect.height) * 2 + 1
        );
        raycaster.setFromCamera(ndc, camera);
        const hits = raycaster.intersectObjects(spotGroup.children, false);
        if (hits.length > 0 && e.button === 0) {
          const name = hits[0].object.name || "";
          if (name === "spot-start" || name === "spot-end" || name === "spot-hole" || name === "spot-phase") {
            spotDragRef.current = {
              which: name === "spot-start" ? "start"
                : name === "spot-end" ? "end"
                : name === "spot-hole" ? "hole"
                : "phase",
              lastX: e.clientX,
              lastY: e.clientY,
            };
            renderer.domElement.style.cursor = "grabbing";
            e.preventDefault();
            return;
          }
        }
      }
      dragging = e.button === 0;
      panning = e.button === 1 || e.button === 2 || e.shiftKey;
      lastX = e.clientX; lastY = e.clientY;
    };

    const onMove = (e: PointerEvent) => {
      // ---- درگ اسپات: جابه‌جایی بازه/شعاع با حرکت نسبی ماوس ----
      const drag = spotDragRef.current;
      if (drag) {
        const dx = e.clientX - drag.lastX;
        const dy = e.clientY - drag.lastY;
        drag.lastX = e.clientX;
        drag.lastY = e.clientY;
        const spot = stateRef.current.spot;
        const cb = stateRef.current.onSpotChange;
        if (spot && cb) {
          // مقیاس حرکت نسبت به فاصلهٔ دوربین (همان ضریب pan)
          const k = Math.max(dist, 40) * 0.0016;
          if (drag.which === "hole") {
            // افقی → قطر سوراخ (mm)
            const d = Math.max(0, spot.radius * 2 + dx * k * 2);
            cb({ holeDiameter: d });
          } else if (drag.which === "phase") {
            // ---- دیال چرخش فاز θ (نسخهٔ ۱.۳) ----
            // درگ مطلق: برخورد پرتو ماوس با صفحهٔ افقی دیال → زاویه
            const sg = spotGroupRef.current;
            const ud = (sg?.userData ?? {}) as { phaseCenter?: [number, number]; phaseZ?: number };
            if (ud.phaseCenter && typeof ud.phaseZ === "number") {
              const rect = renderer.domElement.getBoundingClientRect();
              const ndc = new THREE.Vector2(
                ((e.clientX - rect.left) / rect.width) * 2 - 1,
                -((e.clientY - rect.top) / rect.height) * 2 + 1
              );
              raycaster.setFromCamera(ndc, camera);
              const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -(ud.phaseZ as number));
              const hit = new THREE.Vector3();
              if (raycaster.ray.intersectPlane(plane, hit)) {
                const relX = hit.x - ud.phaseCenter[0];
                const relZ = hit.z - ud.phaseCenter[1];
                if (Math.hypot(relX, relZ) > 2.5) {
                  let deg = dialAngleDeg(relX, relZ);
                  if (e.shiftKey) deg = snapDeg(deg, 15); // چفت ۱۵°
                  cb({ phaseDeg: deg });
                }
              }
            }
          } else {
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
        return;
      }
      if (!dragging && !panning) {
        // نشانگر grab روی حلقه‌ها
        const spotGroup = spotGroupRef.current;
        if (spotGroup && spotGroup.children.length > 0) {
          const rect = renderer.domElement.getBoundingClientRect();
          const ndc = new THREE.Vector2(
            ((e.clientX - rect.left) / rect.width) * 2 - 1,
            -((e.clientY - rect.top) / rect.height) * 2 + 1
          );
          raycaster.setFromCamera(ndc, camera);
          const hits = raycaster.intersectObjects(spotGroup.children, false);
          renderer.domElement.style.cursor =
            hits.length > 0 ? "grab" : "default";
        }
        return;
      }
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      lastX = e.clientX; lastY = e.clientY;
      if (dragging && !panning) {
        rotX -= dx * 0.006;
        rotY += dy * 0.006;
        updateCam();
      } else if (panning) {
        const panScale = dist * 0.0016;
        const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0);
        const up = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 1);
        camera.position.addScaledVector(right, -dx * panScale);
        camera.position.addScaledVector(up, dy * panScale);
        orbitTarget.addScaledVector(right, -dx * panScale);
        orbitTarget.addScaledVector(up, dy * panScale);
      }
    };
    const onUp = () => {
      dragging = false;
      panning = false;
      if (spotDragRef.current) {
        spotDragRef.current = null;
        if (rendererRef.current) {
          rendererRef.current.domElement.style.cursor = "default";
        }
      }
    };
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      dist *= 1 + Math.sign(e.deltaY) * 0.12;
      dist = Math.max(10, Math.min(5000, dist));
      updateCam();
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

    const el = renderer.domElement;
    el.style.touchAction = "none";
    el.addEventListener("pointerdown", onDown);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("click", onClick);
    el.addEventListener("contextmenu", (e) => e.preventDefault());

    // ---- resize ----
    const ro = new ResizeObserver(() => {
      const w = holder.clientWidth || 1;
      const h = holder.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    });
    ro.observe(holder);
    const w0 = holder.clientWidth || 640;
    const h0 = holder.clientHeight || 480;
    renderer.setSize(w0, h0, false);
    camera.aspect = w0 / h0;
    camera.updateProjectionMatrix();

    // ---- حلقهٔ رندر ----
    let raf = 0;
    const loop = () => {
      renderer.render(scene, camera);
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
      renderer.dispose();
      if (el.parentElement === holder) holder.removeChild(el);
    };
  }, []);

  // ------------------------------------------------------------------------
  // به‌روزرسانی هندسه با segments جدید
  // ------------------------------------------------------------------------
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    if (linesRef.current) {
      scene.remove(linesRef.current);
      linesRef.current.geometry.dispose();
      (linesRef.current.material as THREE.Material).dispose();
      linesRef.current = null;
      colorsAttrRef.current = null;
    }
    const segs = props.segments;
    if (!segs || segs.ids.length === 0) return;

    const n = segs.ids.length;
    const positions = new Float32Array(n * 6);
    const colors = new Float32Array(n * 6);
    for (let i = 0; i < n; i++) {
      const p = segs.positions;
      positions[i * 6 + 0] = p[i * 6 + 0];
      positions[i * 6 + 1] = p[i * 6 + 2]; // z بالا (Three: Y up)
      positions[i * 6 + 2] = -p[i * 6 + 1]; // y → -z (چپ‌دست G-code → راست‌دست Three)
      positions[i * 6 + 3] = p[i * 6 + 3];
      positions[i * 6 + 4] = p[i * 6 + 5];
      positions[i * 6 + 5] = -p[i * 6 + 4];
      const c = kindColor(segs.kinds[i], COLORS);
      colors.set([c.r, c.g, c.b, c.r, c.g, c.b], i * 6);
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    const colorsAttr = new THREE.BufferAttribute(colors, 3);
    geo.setAttribute("color", colorsAttr);
    colorsAttrRef.current = colorsAttr;

    const mat = new THREE.LineBasicMaterial({ vertexColors: true });
    const lines = new THREE.LineSegments(geo, mat);
    scene.add(lines);
    linesRef.current = lines;

    // fit-to-view خودکار در بار اول
    geo.computeBoundingSphere();
    if (geo.boundingSphere) {
      const c = geo.boundingSphere.center;
      const r = Math.max(geo.boundingSphere.radius, 30);
      const cam = cameraRef.current;
      if (cam) {
        cam.position.set(c.x + r * 1.6, c.y + r * 1.2, c.z + r * 1.6);
        cam.lookAt(c.x, c.y, c.z);
      }
    }
  }, [props.segments]);

  // ------------------------------------------------------------------------
  // برجسته‌سازی + فیلتر — فقط بازنویسی رنگ‌ها (سریع)
  // ------------------------------------------------------------------------
  useEffect(() => {
    const attr = colorsAttrRef.current;
    const segs = props.segments;
    if (!attr || !segs) return;
    const n = segs.ids.length;
    const arr = attr.array as Float32Array;

    const upToIdx = props.filter.mode === "upTo"
      ? (idToIndex.get(props.filter.upToId) ?? n - 1)
      : n - 1;

    for (let i = 0; i < n; i++) {
      const visible =
        props.filter.mode === "upTo" ? i <= upToIdx : true;
      const isHl = props.highlightIds.has(segs.ids[i]);
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
  }, [props.highlightIds, props.filter, props.segments, idToIndex, COLORS]);

  // ------------------------------------------------------------------------
  // build volume — مختصات واقعی G-code: صفحه از (0,0) تا (vol_x, vol_y)
  // (نسخهٔ ۱.۲: قبلاً حول مبدأ جهانی رسم می‌شد که با G-code واقعی
  //  و طراحِ مرکز‌گذاری‌شده ناسازگار بود)
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
  }, [props.buildVolume]);

  // ------------------------------------------------------------------------
  // اسپات جابه‌جایی الگو/بافت/کف — حلقه‌های قابل‌درگ (نسخهٔ ۱.۲)
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
    const center = toThree([ax, ay, 0]); // مرکز محور در Three
    // ضخامت لوله: به‌اندازهٔ کافی کلفت تا با ماوس راحت گرفته شود (۹٪ شعاع)
    const tube = Math.min(Math.max(spot.radius * 0.09, 1.8), 4.0);

    const makeRing = (
      z: number,
      radius: number,
      color: number,
      name: string
    ) => {
      const geo = new THREE.TorusGeometry(Math.max(radius, 2), tube, 10, 64);
      const mat = new THREE.MeshBasicMaterial({
        color,
        transparent: true,
        opacity: 0.85,
      });
      const ring = new THREE.Mesh(geo, mat);
      ring.rotation.x = Math.PI / 2; // افقی
      ring.position.set(center[0], z, center[2]);
      ring.name = name;
      return ring;
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
      // حلقهٔ شروع (نارنجی) — با نشانگر کروی «اسپات»
      group.add(makeRing(spot.zStart, spot.radius, 0xf0883e, "spot-start"));
      const dotGeo = new THREE.SphereGeometry(Math.max(2.4, tube * 1.7), 12, 12);
      const dotMat = new THREE.MeshBasicMaterial({ color: 0xf0883e });
      const dot = new THREE.Mesh(dotGeo, dotMat);
      // نقطهٔ اسپات روی حلقهٔ شروع (سمت جلو)
      const a = 0.9;
      dot.position.set(
        center[0] + spot.radius * Math.cos(a),
        spot.zStart,
        center[2] - spot.radius * Math.sin(a) * 0.4
      );
      dot.name = "spot-start";
      group.add(dot);
      // حلقهٔ پایان (آبی) — اگر بازهٔ محدود است
      if (spot.zEnd !== null) {
        group.add(makeRing(spot.zEnd, spot.radius, 0x5a87d6, "spot-end"));
      }
    }
    scene.add(group);
    spotGroupRef.current = group;

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

  return (
    <div className="viewport" ref={holderRef}>
      {(!props.segments || props.segments.ids.length === 0) && (
        <div className="overlay-msg">{props.emptyMessage}</div>
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
              : "اسپات: بکشید و جابه‌جا کنید"}
          </span>
        )}
      </div>
    </div>
  );
}
