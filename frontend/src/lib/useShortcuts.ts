/**
 * Global keyboard shortcuts (docs/ux.md §6): Cmd+N new chat, Cmd+B toggle
 * sidebar, Cmd+K focus composer, Cmd+↓ jump to latest, Esc aborts the turn
 * (500 ms hold-to-confirm while a tool is running).
 */

import { useEffect } from "react";

import { useAppStore } from "./agent/store";

const ABORT_HOLD_MS = 500;

// One-line inputs keep Esc for "cancel edit" (rename, search); the composer
// textarea intentionally lets Esc through so it can abort a turn.
function isSingleLineInput(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement;
}

function toolRunning(): boolean {
  const { activeSessionId, chat } = useAppStore.getState();
  if (!activeSessionId) return false;
  const st = chat.sessions[activeSessionId];
  return (
    st?.messages.some((m) =>
      m.parts.some(
        (p) => p.type === "tool" && (p.state?.status === "running" || p.state?.status === "pending"),
      ),
    ) ?? false
  );
}

export function useShortcuts() {
  useEffect(() => {
    let holdTimer: number | undefined;

    const abortNow = () => void useAppStore.getState().abort();

    const onKeyDown = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && !e.shiftKey && !e.altKey) {
        switch (e.key.toLowerCase()) {
          case "n": {
            e.preventDefault();
            const { activeProjectId, newSession } = useAppStore.getState();
            if (activeProjectId) void newSession(activeProjectId);
            return;
          }
          case "b":
            e.preventDefault();
            useAppStore.getState().toggleSidebar();
            return;
          case "k":
            e.preventDefault();
            document.getElementById("composer")?.focus();
            return;
          case "arrowdown":
            e.preventDefault();
            window.dispatchEvent(new CustomEvent("circulogo:jump-to-latest"));
            return;
        }
      }
      if (e.key === "Escape" && !e.repeat && !isSingleLineInput(e.target)) {
        const { activeSessionId, chat } = useAppStore.getState();
        const st = activeSessionId ? chat.sessions[activeSessionId] : undefined;
        if (st?.status !== "busy" && st?.status !== "retry") return;
        if (toolRunning()) {
          if (holdTimer === undefined) holdTimer = window.setTimeout(abortNow, ABORT_HOLD_MS);
        } else {
          abortNow();
        }
      }
    };

    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === "Escape" && holdTimer !== undefined) {
        window.clearTimeout(holdTimer);
        holdTimer = undefined;
      }
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      if (holdTimer !== undefined) window.clearTimeout(holdTimer);
    };
  }, []);
}
