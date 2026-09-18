/**
 * AppShell per the Circulo Paper frame ("Circulo / General"): an 8px bg-app
 * margin wraps a full-width 40px app bar — toggle-sidebar sits beside the
 * macOS traffic lights (InvisibleTitleBarHeight 44 in main.go), then the
 * context title — above the sidebar and main surfaces, each washed with the
 * bottom indigo glow (circulo-glow).
 */

import type { ReactNode } from "react";
import { PanelLeft } from "lucide-react";

import { useAppStore } from "@/lib/agent/store";
import { cn } from "@/lib/utils";

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
  return (
    <div className="flex h-screen w-screen flex-col gap-2 overflow-hidden bg-bg-app p-2 text-foreground">
      <div className="flex h-10 shrink-0 items-center">
        {/* Slot aligned with the sidebar width; the system traffic lights
            overlay its start, so the toggle button clears them. Collapses to
            the button when the sidebar is hidden so the title slides next to
            it (owner: ~16px gap). */}
        <div
          className={cn(
            "flex h-10 shrink-0 items-center pl-[74px]",
            sidebarOpen && "w-[252px]",
          )}
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
          <aside className="circulo-glow flex w-[260px] shrink-0 flex-col overflow-hidden rounded-[18px] bg-bg-sidebar">
            {sidebar}
          </aside>
        )}
        <main className="circulo-glow flex min-w-0 flex-1 flex-col overflow-hidden rounded-[18px] bg-bg-main">
          {children}
        </main>
      </div>
    </div>
  );
}
