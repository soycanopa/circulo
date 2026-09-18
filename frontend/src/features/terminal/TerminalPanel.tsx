/**
 * TerminalPanel (owner call): project-scoped interactive shells in a
 * tabbed panel below the chat zone, above the composer. Tabs open/close
 * in place; the panel grows and shrinks with a grid-rows animation.
 * I/O: SSE stream (base64 chunks) + write/resize POSTs per terminal.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { Plus, X } from "lucide-react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import { api } from "@/lib/agent/api";
import { cn } from "@/lib/utils";

type Tab = {
  termId: string; // backend PTY id
  name: string;
  es: EventSource;
  term: Terminal;
  fit: FitAddon;
  dead: boolean;
};

let counter = 0;

export function TerminalPanel({ projectID }: { projectID: string }) {
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const hostRef = useRef<HTMLDivElement>(null);
  const tabsRef = useRef<Tab[]>([]);
  tabsRef.current = tabs;

  const openTab = useCallback(async () => {
    counter += 1;
    const name = `Term ${counter}`;
    const { id: termId } = await api.openTerminal(projectID);

    const term = new Terminal({
      fontSize: 12,
      fontFamily: "ui-monospace, Menlo, 'SF Mono', monospace",
      theme: {
        background: "#1C1C1F",
        foreground: "#EBEBED",
        cursor: "#EBEBED",
        selectionBackground: "rgba(115, 120, 242, 0.35)",
      },
      cursorBlink: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);

    const es = new EventSource(
      `/agent/projects/${projectID}/terminals/${termId}/stream`,
    );
    es.onmessage = (msg) => {
      if (msg.data === "closed") {
        setTabs((current) =>
          current.map((t) => (t.termId === termId ? { ...t, dead: true } : t)),
        );
        return;
      }
      const bytes = Uint8Array.from(atob(msg.data), (c) => c.charCodeAt(0));
      term.write(bytes);
    };

    const tab: Tab = { termId, name, es, term, fit, dead: false };
    setTabs((current) => {
      setActiveId((currentActive) => currentActive ?? termId);
      return [...current, tab];
    });
  }, [projectID]);

  // first tab on mount
  useEffect(() => {
    void openTab();
    return () => {
      // leaving the project/page: close every shell
      for (const t of tabsRef.current) {
        t.es.close();
        t.term.dispose();
        void api.closeTerminal(projectID, t.termId).catch(() => undefined);
      }
    };
  }, [openTab, projectID]);

  // attach the active terminal to its host div once mounted
  const hostRefs = useRef(new Map<string, HTMLDivElement>());
  const ensureAttached = useCallback(
    (tab: Tab) => {
      const host = hostRefs.current.get(tab.termId);
      if (!host || host.childElementCount > 0) return;
      tab.term.open(host);
      tab.fit.fit();
      void api
        .resizeTerminal(projectID, tab.termId, Math.round(tab.fit.proposeDimensions()?.cols ?? 80), Math.round(tab.fit.proposeDimensions()?.rows ?? 24))
        .catch(() => undefined);
      tab.term.onData((data) => {
        void api.writeTerminal(projectID, tab.termId, data).catch(() => undefined);
      });
    },
    [projectID],
  );

  useEffect(() => {
    if (!activeId) return;
    const tab = tabs.find((t) => t.termId === activeId);
    if (tab) ensureAttached(tab);
  }, [activeId, tabs, ensureAttached]);

  const closeTab = (termId: string) => {
    const tab = tabs.find((t) => t.termId === termId);
    if (tab) {
      tab.es.close();
      tab.term.dispose();
      void api.closeTerminal(projectID, termId).catch(() => undefined);
    }
    setTabs((current) => {
      const next = current.filter((t) => t.termId !== termId);
      if (activeId === termId) setActiveId(next[next.length - 1]?.termId ?? null);
      return next;
    });
  };

  return (
    <div className="flex h-[240px] flex-col overflow-hidden rounded-xl border border-border bg-[#1C1C1F]">
      {/* tab bar */}
      <div className="flex h-8 shrink-0 items-center gap-1 border-b border-border px-2">
        {tabs.map((t, i) => (
          <button
            key={t.termId}
            type="button"
            onClick={() => setActiveId(t.termId)}
            className={cn(
              "group flex h-6 items-center gap-1.5 rounded-md px-2 text-[12px] transition-colors duration-100",
              t.dead && "opacity-50",
              t.termId === activeId
                ? "bg-bg-hover text-text-primary"
                : "text-text-secondary hover:bg-bg-hover/60",
            )}
          >
            <span>{t.name}</span>
            <span
              role="button"
              aria-label={`Close ${t.name}`}
              className="flex size-3.5 items-center justify-center rounded-[4px] text-text-tertiary opacity-0 transition-opacity duration-100 hover:bg-bg-hover hover:text-text-primary group-hover:opacity-100"
              onClick={(e) => {
                e.stopPropagation();
                closeTab(t.termId);
              }}
            >
              <X className="size-2.5" strokeWidth={2.5} />
            </span>
            <span className="sr-only">{i}</span>
          </button>
        ))}
        <button
          type="button"
          aria-label="New terminal"
          title="New terminal"
          onClick={() => void openTab()}
          className="flex size-5 items-center justify-center rounded-md text-text-tertiary transition-colors duration-100 hover:bg-bg-hover hover:text-text-primary"
        >
          <Plus className="size-3.5" strokeWidth={2} />
        </button>
        <div className="grow" />
      </div>

      {/* bodies — every tab stays mounted; hidden ones keep their output */}
      <div ref={hostRef} className="relative min-h-0 flex-1 p-1.5">
        {tabs.map((t) => (
          <div
            key={t.termId}
            ref={(el) => {
              if (el) hostRefs.current.set(t.termId, el);
              else hostRefs.current.delete(t.termId);
            }}
            className={cn(
              "absolute inset-1.5 overflow-hidden",
              t.termId === activeId ? "visible" : "invisible",
            )}
          >
            {t.dead && (
              <div className="absolute inset-0 z-10 flex items-center justify-center bg-[#1C1C1F]/80 text-[12.5px] text-text-tertiary">
                Process exited
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
