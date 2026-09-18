/**
 * AppShell: the only place that knows the window layout — a full-height
 * sidebar slot plus a main column, each with a 40px top strip that together
 * form the unified macOS titlebar (traffic lights live over the sidebar's
 * strip; `InvisibleTitleBarHeight: 44` in main.go).
 */

import type { ReactNode } from "react";

import { useAppStore } from "@/lib/agent/store";

export function AppShell({ sidebar, children }: { sidebar: ReactNode; children: ReactNode }) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      {sidebarOpen && (
        <aside className="flex h-full w-[260px] shrink-0 flex-col border-r border-border bg-sidebar">
          <div className="h-[40px] shrink-0" />
          {sidebar}
        </aside>
      )}
      <main className="flex min-w-0 flex-1 flex-col">
        <div className="h-[40px] shrink-0 select-none" />
        {children}
      </main>
    </div>
  );
}
