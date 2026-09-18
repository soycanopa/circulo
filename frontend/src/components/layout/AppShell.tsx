/**
 * AppShell per the Circulo Paper frame ("Circulo / General"): an 8px bg-app
 * margin wraps a full-width 40px app bar — toggle-sidebar sits beside the
 * macOS traffic lights (InvisibleTitleBarHeight 44 in main.go), then the
 * context title — above the sidebar and main surfaces, each washed with the
 * bottom indigo glow (circulo-glow). The sidebar width is draggable (owner
 * call) and persists in localStorage.
 */

import { useCallback, useRef, useState, type ReactNode } from "react";
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
              className="circulo-glow flex shrink-0 flex-col overflow-hidden rounded-[18px] bg-bg-sidebar"
              style={{ width: sidebarWidth }}
            >
              {sidebar}
            </aside>
            {/* invisible drag handle living in the 8px gap; double-click
                resets to the default width */}
            <div
              role="separator"
              aria-orientation="vertical"
              className={cn(
                "group relative z-20 -mx-1 w-2 shrink-0 cursor-col-resize",
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
        <main className="circulo-glow flex min-w-0 flex-1 flex-col overflow-hidden rounded-[18px] bg-bg-main">
          {children}
        </main>
      </div>
    </div>
  );
}
