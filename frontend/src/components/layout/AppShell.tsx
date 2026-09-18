/**
 * AppShell per the Circulo Paper frame ("Circulo / General"): an 8px bg-app
 * margin wraps a full-width 40px app bar — toggle-sidebar sits beside the
 * macOS traffic lights (InvisibleTitleBarHeight 44 in main.go), then the
 * context title — above the sidebar and main surfaces, each washed with the
 * bottom indigo glow (circulo-glow). The sidebar width is draggable (owner
 * call) and persists in localStorage.
 */

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { PanelLeft } from "lucide-react";

import { useAppStore } from "@/lib/agent/store";
import { cn } from "@/lib/utils";

const SIDEBAR_MIN = 200;
const SIDEBAR_MAX = 480;
const SIDEBAR_DEFAULT = 260;

function loadSidebarWidth(): number {
  const raw = Number(localStorage.getItem("circulogo.sidebarWidth"));
  return Number.isFinite(raw) && raw > 0
    ? Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, raw))
    : SIDEBAR_DEFAULT;
}

/** Animated pixel texture for the bottom glow (owner call): pixels pop in
 *  and out in random order, dissolving toward the top of the strip so the
 *  texture only reads inside the glow. Reduced motion draws the static
 *  texture. Canvas keeps per-pixel randomness cheap (~2k dots at ~20fps). */
function GlowPixels() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const PITCH = 9;
    const BASE = 0.35;
    let dots: { x: number; y: number; phase: number; cycle: number }[] = [];
    let w = 0;
    let h = 0;
    let raf = 0;
    let last = 0;

    const build = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = canvas.clientWidth;
      h = canvas.clientHeight;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      dots = [];
      for (let y = PITCH / 2; y < h; y += PITCH)
        for (let x = PITCH / 2; x < w; x += PITCH)
          dots.push({ x, y, phase: Math.random(), cycle: 3800 + Math.random() * 5200 });
    };

    const paint = (twinkle: boolean, now: number) => {
      ctx.clearRect(0, 0, w, h);
      for (const d of dots) {
        const fade = 1 - d.y / h; // dissolve upward into the surface
        const a = BASE * fade * (twinkle ? Math.sin(((now / d.cycle + d.phase) % 1) * Math.PI) ** 2 : 1);
        if (a < 0.02) continue;
        ctx.fillStyle = `rgba(115, 120, 242, ${a})`;
        ctx.fillRect(d.x - 1, d.y - 1, 2, 2);
      }
    };

    const loop = (now: number) => {
      raf = requestAnimationFrame(loop);
      if (now - last < 50) return; // ~20fps is plenty for a slow twinkle
      last = now;
      paint(true, now);
    };

    build();
    if (reduce) paint(false, 0);
    else raf = requestAnimationFrame(loop);

    const ro = new ResizeObserver(build);
    ro.observe(canvas);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, []);

  return (
    <canvas
      ref={ref}
      aria-hidden
      className="pointer-events-none absolute inset-x-0 bottom-0 h-[150px] w-full"
    />
  );
}

export function AppShell({
  sidebar,
  title,
  children,
}: {
  sidebar: ReactNode;
  title?: ReactNode;
  children: ReactNode;
}) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const [sidebarWidth, setSidebarWidth] = useState(loadSidebarWidth);
  const [dragging, setDragging] = useState(false);
  const drag = useRef({ startX: 0, startWidth: 0 });

  const onHandleDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      drag.current = { startX: e.clientX, startWidth: sidebarWidth };
      setDragging(true);
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    [sidebarWidth],
  );

  const onHandleMove = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragging) return;
    const next = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, drag.current.startWidth + (e.clientX - drag.current.startX)));
    setSidebarWidth(next);
  }, [dragging]);

  const onHandleUp = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    setDragging(false);
    e.currentTarget.releasePointerCapture(e.pointerId);
    setSidebarWidth((w) => {
      localStorage.setItem("circulogo.sidebarWidth", String(w));
      return w;
    });
  }, []);

  return (
    <div className={cn("flex h-screen w-screen flex-col gap-2 overflow-hidden bg-bg-app p-2 text-foreground", dragging && "select-none")}>
      <div className="flex h-10 shrink-0 items-center">
        {/* Slot aligned with the sidebar width; the system traffic lights
            overlay its start, so the toggle button clears them. Collapses to
            the button when the sidebar is hidden so the title slides next to
            it (owner: ~16px gap). */}
        <div
          className={cn("flex h-10 shrink-0 items-center pl-[74px]", sidebarOpen && "shrink-0")}
          style={sidebarOpen ? { width: sidebarWidth - 8 } : undefined}
        >
          <button
            type="button"
            aria-label="Toggle sidebar"
            title="Toggle sidebar (⌘B)"
            className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md text-text-secondary hover:bg-bg-hover"
            onClick={toggleSidebar}
          >
            <PanelLeft className="size-4" strokeWidth={2} />
          </button>
        </div>
        <div className="flex h-10 min-w-0 flex-1 items-center px-4">{title}</div>
      </div>
      <div className="flex min-h-0 flex-1 gap-2">
        {sidebarOpen && (
          <>
            <aside
              className="circulo-glow relative flex shrink-0 flex-col overflow-hidden rounded-[18px] bg-bg-sidebar"
              style={{ width: sidebarWidth }}
            >
              <GlowPixels />
              <div className="relative flex min-h-0 flex-1 flex-col">{sidebar}</div>
            </aside>
            {/* invisible drag handle living in the 8px gap; double-click
                resets to the default width */}
            <div
              role="separator"
              aria-orientation="vertical"
              className={cn(
                "group relative z-20 -mx-3 w-4 shrink-0 cursor-col-resize",
                dragging && "cursor-col-resizing",
              )}
              onPointerDown={onHandleDown}
              onPointerMove={onHandleMove}
              onPointerUp={onHandleUp}
              onDoubleClick={() => {
                setSidebarWidth(SIDEBAR_DEFAULT);
                localStorage.setItem("circulogo.sidebarWidth", String(SIDEBAR_DEFAULT));
              }}
            >
              <div
                className={cn(
                  "absolute inset-y-0 left-1/2 w-px -translate-x-1/2 transition-colors duration-150",
                  dragging ? "bg-accent-cir" : "bg-transparent group-hover:bg-border",
                )}
              />
            </div>
          </>
        )}
        <main className="circulo-glow relative flex min-w-0 flex-1 flex-col overflow-hidden rounded-[18px] bg-bg-main">
          <GlowPixels />
          <div className="relative flex min-h-0 flex-1 flex-col">{children}</div>
        </main>
      </div>
    </div>
  );
}
