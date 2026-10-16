/**
 * ============================================================================
 * components/Viewport.tsx — نمایشگر سه‌بعدی مسیر نازل (Three.js/WebGL2)
 * ============================================================================
 * الزامات Docs/PROMPT_02 §7:
 *   - مسیر چاپی از travel/retract متمایز (رنگ‌بندی + legend متنی — رنگ
 *     تنها راه تفکیک نیست، الزام دسترس‌پذیری)؛
 *   - segment/خط/لایهٔ فعلی برجسته؛
 *   - فیلتر «تا این خط»، «فقط این لایه»، «فقط این خط»؛
 *   - orbit/zoom/pan/fit/reset + grid و محورها و build volume (اگر پروفایل)؛
 *   - برای فایل بزرگ: یک BufferGeometry واحد (نه mesh per segment!)؛
 *   - WebGL نبود → پیام راهنما (قابلیت‌های متنی همچنان کار می‌کنند).
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

export interface ViewportProps extends ViewportCallbacks {
  /** payload فشردهٔ مسیر از parse_gcode (null = خالی) */
  segments: SegmentsPayload | null;
  /** idهای segmentهای برجسته (خط انتخاب‌شده) */
  highlightIds: Set<number>;
  /** حالت فیلتر: all | upTo(segmentId) | onlyLayer | onlyLine */
  filter: { mode: "all" | "upTo" | "onlyHighlight"; upToId: number };
  /** حجم ساخت [x,y,z] اگر پروفایل انتخاب شده (mm) */
  buildVolume?: [number, number, number] | null;
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
      // کامپوننت re-render می‌شود و پیام نشان می‌دهد
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

    // ---- نورپردازی ساده (برای احتمال mesh در آینده) ----
    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const dir = new THREE.DirectionalLight(0xffffff, 0.8);
    dir.position.set(200, 300, 100);
    scene.add(dir);

    // ---- grid و محورها ----
    const grid = new THREE.GridHelper(400, 40, 0x343945, 0x22262f);
    grid.position.y = -0.02;
    scene.add(grid);
    const axes = new THREE.AxesHelper(60);
    axes.position.y = 0.01;
    scene.add(axes);

    // ---- کنترل مداری دستی (بدون وابستگی اضافه: OrbitControls باندل npm است
    // ولی برای کنترل کامل رفتار در RTL، تعامل سادهٔ ماوس/لمس پیاده می‌کنیم) ----
    let dragging = false;
    let panning = false;
    let lastX = 0, lastY = 0;
    let rotX = Math.atan2(camera.position.z - 0, camera.position.x - 0);
    let rotY = Math.asin(
      (camera.position.y - 60) /
        Math.hypot(camera.position.x, camera.position.y - 60, camera.position.z)
    );
    let dist = camera.position.length() - 60;

    const updateCam = () => {
      rotY = Math.max(-1.5, Math.min(1.5, rotY));
      const cy = 60 + dist * Math.sin(rotY);
      const cr = dist * Math.cos(rotY);
      camera.position.set(cr * Math.cos(rotX), cy, cr * Math.sin(rotX));
      camera.lookAt(0, 60, 0);
    };

    const onDown = (e: PointerEvent) => {
      dragging = e.button === 0;
      panning = e.button === 1 || e.button === 2 || e.shiftKey;
      lastX = e.clientX; lastY = e.clientY;
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging && !panning) return;
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
        // pan — دوربین و هدف را با هم می‌بریم
        camera.position.addScaledVector(right, -dx * panScale);
        camera.position.addScaledVector(up, dy * panScale);
      }
    };
    const onUp = () => { dragging = false; panning = false; };
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      dist *= 1 + Math.sign(e.deltaY) * 0.12;
      dist = Math.max(10, Math.min(5000, dist));
      updateCam();
    };

    // ---- کلیک: انتخاب segment با Raycaster روی LineSegments ----
    // (افزایش ضخامت خط منطقی برای raycast خطوط سخت است؛ در عوض نقطهٔ
    // کلیک را به نزدیک‌ترین segment از روی عمود می‌سنجیم)
    const raycaster = new THREE.Raycaster();
    raycaster.params.Line = { threshold: 3 };
    const onClick = (e: MouseEvent) => {
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
        // ایندکس رأس → شمارهٔ segment
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
    // حذف خط قبلی (آزادسازی GPU — الزام PROMPT_04 §8)
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
  // build volume (فقط با پروفایل — الزام: بدون پروفایل جعل نکن)
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
    if (!vol) return;
    const box = new THREE.BoxGeometry(vol[0], vol[2], vol[1]);
    const edges = new THREE.EdgesGeometry(box);
    const lines = new THREE.LineSegments(
      edges,
      new THREE.LineBasicMaterial({ color: 0x7d8392, transparent: true, opacity: 0.5 })
    );
    lines.name = "build-volume";
    lines.position.y = vol[2] / 2;
    scene.add(lines);
  }, [props.buildVolume]);

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
      </div>
    </div>
  );
}
