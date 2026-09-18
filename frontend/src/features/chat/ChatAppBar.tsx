/**
 * ChatAppBar — the app-bar title row content while a session is open
 * (Circulo Paper): breadcrumb (session · project) on the left, live turn
 * status and session actions (close) on the right.
 */

import { memo } from "react";
import { Folder, Loader2, SquareTerminal, X } from "lucide-react";

import { useAppStore } from "@/lib/agent/store";
import { cn } from "@/lib/utils";
import type { ProjectView } from "@/lib/agent/protocol";
import type { SessionState } from "@/lib/agent/reducer";

export const ChatAppBar = memo(function ChatAppBar({
  project,
  session,
}: {
  project: ProjectView;
  session?: SessionState;
}) {
  const closeSession = useAppStore((s) => s.closeSession);
  const terminalOpen = useAppStore((s) => s.terminalOpen);
  const toggleTerminal = useAppStore((s) => s.toggleTerminal);
  const title = session?.session.title || "New chat";
  const busy = session?.status === "busy";
  const retrying = session?.status === "retry";

  return (
    <div className="flex w-full min-w-0 items-center justify-between gap-3">
      <span className="flex min-w-0 items-center gap-1.5">
        <Folder className="size-3.5 shrink-0 text-text-tertiary" strokeWidth={2} />
        <span className="truncate text-xs font-medium leading-[14px] text-text-secondary">
          {title}
        </span>
      </span>
      <div className="flex shrink-0 items-center gap-2">
        {busy && (
          <span className="flex items-center gap-1.5 text-xs text-text-tertiary">
            <Loader2 className="size-3.5 animate-spin" /> working
          </span>
        )}
        {retrying && (
          <span className="flex items-center gap-1.5 text-xs text-warning">
            <Loader2 className="size-3.5 animate-spin" />
            retrying{session?.retry ? ` (attempt ${session.retry.attempt})` : ""}
          </span>
        )}
        <button
          aria-label="Toggle terminal"
          title="Terminal"
          className={cn(
            "flex size-7 shrink-0 items-center justify-center rounded-full border text-muted-foreground hover:text-foreground",
            terminalOpen
              ? "border-accent-cir/60 bg-accent-cir/15 text-text-primary"
              : "border-border-strong bg-bg-code",
          )}
          onClick={toggleTerminal}
        >
          <SquareTerminal className="size-3.5" />
        </button>
        {session && (
          <button
            aria-label="Close session"
            title="Close session"
            className="flex size-7 items-center justify-center shrink-0 rounded-full bg-bg-code border border-border-strong text-muted-foreground hover:text-foreground"
            onClick={closeSession}
          >
            <X className="size-3.5" />
          </button>
        )}
      </div>
    </div>
  );
});
