/**
 * ChatAppBar — replica of the Circulo design's App Bar: breadcrumb
 * (project › session title) on the left, live turn status and session
 * actions (close) on the right.
 */

import { memo } from "react";
import { ChevronRight, Loader2, X } from "lucide-react";

import { AppBar } from "@/components/layout/AppBar";
import { useAppStore } from "@/lib/agent/store";
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
  const projectName = project.path.split("/").filter(Boolean).pop() ?? project.path;
  const title = session?.session.title || "New chat";
  const busy = session?.status === "busy";
  const retrying = session?.status === "retry";

  return (
    <AppBar
      left={
        <>
          <span className="truncate text-[13px] text-muted-foreground">{projectName}</span>
          <ChevronRight className="size-3 shrink-0 text-text-tertiary" />
          <span className="truncate text-[13px] font-semibold text-foreground">{title}</span>
        </>
      }
      right={
        <>
          {busy && (
            <span className="flex items-center gap-1.5 text-[12px] text-muted-foreground">
              <Loader2 className="size-3.5 animate-spin" /> working
            </span>
          )}
          {retrying && (
            <span className="flex items-center gap-1.5 text-[12px] text-warning">
              <Loader2 className="size-3.5 animate-spin" />
              retrying{session?.retry ? ` (attempt ${session.retry.attempt})` : ""}
            </span>
          )}
          {session && (
            <button
              aria-label="Close session"
              title="Close session"
              className="flex h-7 w-7 items-center justify-center rounded-md border border-border text-muted-foreground hover:bg-muted hover:text-foreground"
              onClick={closeSession}
            >
              <X className="size-3.5" />
            </button>
          )}
        </>
      }
    />
  );
});
