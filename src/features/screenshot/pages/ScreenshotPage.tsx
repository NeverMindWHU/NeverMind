/**
 * ScreenshotPage – fullscreen screenshot overlay rendered per physical monitor.
 *
 * Interaction state machine:
 *   idle       → user starts dragging → drawing
 *   drawing    → mouseUp (≥10×10)      → adjusting
 *   drawing    → mouseUp (<10×10)      → idle
 *   adjusting  → Enter                 → confirm (crop + copy + close)
 *   adjusting  → Escape                → close all
 *   adjusting  → drag handle           → resize
 *   adjusting  → drag interior         → move
 *   adjusting  → drag outside          → drawing (re-draw)
 *   any        → Escape                → close all
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { GENERATION_FROM_SCREENSHOT_EVENT } from "@/lib/generation-from-screenshot";
import { isTauri } from "@/lib/tauri";

// ─── Types ──────────────────────────────────────────────────────────────────

type Phase = "idle" | "drawing" | "adjusting";

/** Normalised selection rectangle – x,y always top-left corner, w/h always positive */
interface Rect { x: number; y: number; w: number; h: number; }

type HandleId = "nw" | "n" | "ne" | "w" | "e" | "sw" | "s" | "se";
interface Handle { id: HandleId; x: number; y: number; cursor: string; }

// ─── Constants ───────────────────────────────────────────────────────────────

const HANDLE_SIZE = 8;
const HANDLE_HALF = HANDLE_SIZE / 2;
const MIN_SELECTION = 10;
const HANDLE_HIT_RADIUS = 12; // slightly larger than rendered size for easier clicking

// ─── Helpers ─────────────────────────────────────────────────────────────────

function getHandles(r: Rect): Handle[] {
  const mx = r.x + r.w / 2;
  const my = r.y + r.h / 2;
  const r2 = r.x + r.w;
  const b2 = r.y + r.h;
  return [
    { id: "nw", x: r.x, y: r.y,  cursor: "nwse-resize" },
    { id: "n",  x: mx,  y: r.y,  cursor: "ns-resize"   },
    { id: "ne", x: r2,  y: r.y,  cursor: "nesw-resize"  },
    { id: "w",  x: r.x, y: my,   cursor: "ew-resize"   },
    { id: "e",  x: r2,  y: my,   cursor: "ew-resize"   },
    { id: "sw", x: r.x, y: b2,   cursor: "nesw-resize"  },
    { id: "s",  x: mx,  y: b2,   cursor: "ns-resize"   },
    { id: "se", x: r2,  y: b2,   cursor: "nwse-resize" },
  ];
}

function hitTestHandle(handles: Handle[], px: number, py: number): Handle | null {
  for (const h of handles) {
    if (Math.abs(px - h.x) <= HANDLE_HIT_RADIUS && Math.abs(py - h.y) <= HANDLE_HIT_RADIUS) {
      return h;
    }
  }
  return null;
}

function pointInRect(r: Rect, px: number, py: number): boolean {
  return px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h;
}

function clampRect(r: Rect, maxW: number, maxH: number): Rect {
  const x = Math.max(0, Math.min(r.x, maxW - r.w));
  const y = Math.max(0, Math.min(r.y, maxH - r.h));
  return { ...r, x, y };
}

/** Apply a handle drag delta to a rect. Returns the new rect (normalised). */
function applyHandleResize(
  orig: Rect,
  handle: HandleId,
  dx: number,
  dy: number,
  maxW: number,
  maxH: number,
): Rect {
  let { x, y, w, h } = orig;

  switch (handle) {
    case "nw": x += dx; y += dy; w -= dx; h -= dy; break;
    case "n":              y += dy;          h -= dy; break;
    case "ne":             y += dy; w += dx; h -= dy; break;
    case "w":  x += dx;            w -= dx;           break;
    case "e":                       w += dx;           break;
    case "sw": x += dx;  y += dy; w -= dx; h -= dy; break; // handled below
    case "s":                                h += dy; break;
    case "se":                      w += dx; h += dy; break;
  }
  // sw special case
  if (handle === "sw") { x = orig.x + dx; y = orig.y; w = orig.w - dx; h = orig.h + dy; }

  // Enforce minimum size
  if (w < MIN_SELECTION) {
    if (handle === "nw" || handle === "w" || handle === "sw") x = orig.x + orig.w - MIN_SELECTION;
    w = MIN_SELECTION;
  }
  if (h < MIN_SELECTION) {
    if (handle === "nw" || handle === "n" || handle === "ne") y = orig.y + orig.h - MIN_SELECTION;
    h = MIN_SELECTION;
  }

  // Clamp to window
  x = Math.max(0, x);
  y = Math.max(0, y);
  w = Math.min(w, maxW - x);
  h = Math.min(h, maxH - y);

  return { x, y, w, h };
}

// ─── Drawing ─────────────────────────────────────────────────────────────────

function drawOverlay(
  canvas: HTMLCanvasElement,
  rect: Rect | null,
  phase: Phase,
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  canvas.width  = window.innerWidth;
  canvas.height = window.innerHeight;

  // Dim everything
  ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  if (!rect || rect.w < 1 || rect.h < 1) return;

  // Reveal selected area
  ctx.clearRect(rect.x, rect.y, rect.w, rect.h);

  // Selection border
  ctx.strokeStyle = "#007aff";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(rect.x, rect.y, rect.w, rect.h);

  // Size label
  const label = `${Math.round(rect.w)} × ${Math.round(rect.h)}`;
  ctx.font = "bold 11px system-ui, sans-serif";
  const lw = ctx.measureText(label).width + 8;
  const lx = rect.x + 4;
  const ly = rect.y > 20 ? rect.y - 6 : rect.y + rect.h + 16;
  ctx.fillStyle = "#007aff";
  ctx.fillRect(lx - 2, ly - 13, lw, 16);
  ctx.fillStyle = "#fff";
  ctx.fillText(label, lx + 2, ly);

  // Resize handles (only in adjusting phase)
  if (phase === "adjusting") {
    const handles = getHandles(rect);
    for (const h of handles) {
      ctx.fillStyle = "#fff";
      ctx.fillRect(h.x - HANDLE_HALF, h.y - HANDLE_HALF, HANDLE_SIZE, HANDLE_SIZE);
      ctx.strokeStyle = "#007aff";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(h.x - HANDLE_HALF, h.y - HANDLE_HALF, HANDLE_SIZE, HANDLE_SIZE);
    }
  }
}

// ─── Component ───────────────────────────────────────────────────────────────

export function ScreenshotPage() {
  const [searchParams] = useSearchParams();
  const monitorIndex = parseInt(searchParams.get("monitor") ?? "0", 10);

  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [isCapturing, setIsCapturing] = useState(true);

  // State machine
  const [phase, setPhase]   = useState<Phase>("idle");
  const [rect, setRect]     = useState<Rect | null>(null);
  const [cursor, setCursor] = useState("crosshair");

  // Drag bookkeeping (kept in refs to avoid stale closures in mousemove)
  const dragRef = useRef<{
    startX: number; startY: number;
    origRect: Rect | null;
    activeHandle: HandleId | null;
    isMove: boolean;
  } | null>(null);

  const phaseRef    = useRef<Phase>("idle");
  const rectRef     = useRef<Rect | null>(null);
  const canvasRef   = useRef<HTMLCanvasElement>(null);
  const imageRef    = useRef<HTMLImageElement>(null);
  const rootRef     = useRef<HTMLDivElement>(null);

  // Keep refs in sync
  phaseRef.current = phase;
  rectRef.current  = rect;

  // ── Close all overlay windows ─────────────────────────────────────────────
  const closeAll = useCallback(() => {
    void invoke("close_screenshot_windows").catch(console.error);
  }, []);

  // ── Confirm: 主窗口后台生成卡片 + 剪贴板 + 关闭 overlay ───────────────────
  const confirm = useCallback(() => {
    const r   = rectRef.current;
    const img = imageRef.current;
    if (!r || !img || r.w < MIN_SELECTION || r.h < MIN_SELECTION) return;

    const offCanvas = document.createElement("canvas");
    offCanvas.width  = r.w;
    offCanvas.height = r.h;
    const offCtx = offCanvas.getContext("2d");
    if (!offCtx) return;

    offCtx.drawImage(img, r.x, r.y, r.w, r.h, 0, 0, r.w, r.h);
    const dataUrl = offCanvas.toDataURL("image/png");

    void (async () => {
      if (isTauri()) {
        try {
          await emitTo("main", GENERATION_FROM_SCREENSHOT_EVENT, {
            sourceText: "",
            sourceType: "image",
            imageUrls: [dataUrl],
          });
        } catch (err) {
          console.error("Failed to dispatch screenshot to card generation:", err);
        }
      }
      try {
        const res = await fetch(dataUrl);
        const blob = await res.blob();
        await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      } catch (err) {
        console.error("Clipboard write failed:", err);
      }
      closeAll();
    })();
  }, [closeAll]);

  // ── Keyboard：capture 阶段监听，避免未聚焦或子层拦截时 idle 下 Esc 无法退出 ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeAll();
        return;
      }
      if (e.key === "Enter" && phaseRef.current === "adjusting") {
        e.preventDefault();
        confirm();
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [closeAll, confirm]);

  // 主动聚焦根节点，配合 Rust set_focus，保证未框选时也能收到 Esc
  useEffect(() => {
    rootRef.current?.focus({ preventScroll: true });
  }, []);
  useEffect(() => {
    if (!isCapturing) {
      rootRef.current?.focus({ preventScroll: true });
    }
  }, [isCapturing]);

  // ── Capture this monitor's pre-captured screenshot ────────────────────────
  useEffect(() => {
    async function loadScreen() {
      try {
        setIsCapturing(true);
        const bytes = await invoke<number[]>("get_captured_monitor", { index: monitorIndex });
        const blob  = new Blob([new Uint8Array(bytes)], { type: "image/png" });
        setImageSrc(URL.createObjectURL(blob));
      } catch (err) {
        console.error("Failed to load captured monitor:", err);
      } finally {
        setIsCapturing(false);
      }
    }
    void loadScreen();
  }, [monitorIndex]);

  // ── Redraw canvas whenever rect/phase changes ─────────────────────────────
  useEffect(() => {
    if (canvasRef.current) {
      drawOverlay(canvasRef.current, rect, phase);
    }
  }, [rect, phase, imageSrc]);

  // ── Mouse event handlers ──────────────────────────────────────────────────
  const handleMouseDown = (e: React.MouseEvent) => {
    const px = e.clientX;
    const py = e.clientY;
    const currentPhase = phaseRef.current;
    const currentRect  = rectRef.current;

    if (currentPhase === "idle" || currentPhase === "drawing") {
      // Start a new selection
      dragRef.current = { startX: px, startY: py, origRect: null, activeHandle: null, isMove: false };
      setPhase("drawing");
      setRect({ x: px, y: py, w: 0, h: 0 });
      return;
    }

    if (currentPhase === "adjusting" && currentRect) {
      const handles = getHandles(currentRect);
      const hit     = hitTestHandle(handles, px, py);

      if (hit) {
        // Resize via handle
        dragRef.current = { startX: px, startY: py, origRect: { ...currentRect }, activeHandle: hit.id, isMove: false };
        return;
      }

      if (pointInRect(currentRect, px, py)) {
        // Move interior
        dragRef.current = { startX: px, startY: py, origRect: { ...currentRect }, activeHandle: null, isMove: true };
        return;
      }

      // Outside → start a new draw
      dragRef.current = { startX: px, startY: py, origRect: null, activeHandle: null, isMove: false };
      setPhase("drawing");
      setRect({ x: px, y: py, w: 0, h: 0 });
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    const px = e.clientX;
    const py = e.clientY;
    const drag = dragRef.current;

    // Update cursor based on hover position (adjusting phase)
    if (phaseRef.current === "adjusting" && rectRef.current) {
      const handles = getHandles(rectRef.current);
      const hit     = hitTestHandle(handles, px, py);
      if (hit) { setCursor(hit.cursor); }
      else if (pointInRect(rectRef.current, px, py)) { setCursor("move"); }
      else { setCursor("crosshair"); }
    }

    if (!drag) return;

    const dx = px - drag.startX;
    const dy = py - drag.startY;
    const mw = window.innerWidth;
    const mh = window.innerHeight;

    if (phaseRef.current === "drawing") {
      // Normalised rect while drawing
      const x = Math.min(drag.startX, px);
      const y = Math.min(drag.startY, py);
      const w = Math.abs(dx);
      const h = Math.abs(dy);
      setRect({ x: Math.max(0, x), y: Math.max(0, y), w: Math.min(w, mw - x), h: Math.min(h, mh - y) });
      return;
    }

    if (drag.origRect) {
      if (drag.activeHandle) {
        // Resize
        setRect(applyHandleResize(drag.origRect, drag.activeHandle, dx, dy, mw, mh));
      } else if (drag.isMove) {
        // Move
        setRect(clampRect({ ...drag.origRect, x: drag.origRect.x + dx, y: drag.origRect.y + dy }, mw, mh));
      }
    }
  };

  const handleMouseUp = () => {
    dragRef.current = null;

    if (phaseRef.current === "drawing") {
      const r = rectRef.current;
      if (r && r.w >= MIN_SELECTION && r.h >= MIN_SELECTION) {
        setPhase("adjusting");
        setCursor("move");
      } else {
        setPhase("idle");
        setRect(null);
        setCursor("crosshair");
      }
    }
    // In adjusting phase a drag was a resize or move – no phase change needed
  };

  // ─── Render ────────────────────────────────────────────────────────────────

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      className="fixed inset-0 w-screen h-screen overflow-hidden select-none outline-none"
      style={{ cursor }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {isCapturing && (
        <div className="absolute inset-0 bg-black/50 flex items-center justify-center text-white z-50 text-sm">
          Capturing screen…
        </div>
      )}

      {imageSrc && (
        <>
          <img
            ref={imageRef}
            src={imageSrc}
            className="absolute inset-0 w-full h-full object-fill pointer-events-none"
            alt="Screen capture"
            draggable={false}
          />
          <canvas
            ref={canvasRef}
            className="absolute inset-0 w-full h-full pointer-events-none"
          />
        </>
      )}

      {(phase === "adjusting" || phase === "idle") && (
        <div
          className="absolute pointer-events-none"
          style={{ bottom: 16, left: "50%", transform: "translateX(-50%)" }}
        >
          <div className="flex flex-col gap-1 items-center bg-black/60 text-white text-xs px-3 py-1.5 rounded-full backdrop-blur max-w-[min(90vw,24rem)] text-center">
            {phase === "adjusting" ? (
              <>
                <div className="flex flex-wrap justify-center gap-x-2 gap-y-1">
                  <span>按 <kbd className="font-mono bg-white/20 px-1 rounded">Enter</kbd> 确认</span>
                  <span className="opacity-40">·</span>
                  <span>按 <kbd className="font-mono bg-white/20 px-1 rounded">Esc</kbd> 取消</span>
                </div>
                <span className="text-[10px] text-white/75 leading-snug">
                  确认后：复制选区到剪贴板，并在主窗口开始生成知识卡片
                </span>
              </>
            ) : (
              <span>
                拖拽框选区域 · 按 <kbd className="font-mono bg-white/20 px-1 rounded">Esc</kbd> 退出
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
