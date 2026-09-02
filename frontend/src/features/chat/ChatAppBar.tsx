/**
 * ChatAppBar — replica of the Circulo design's App Bar: breadcrumb
 * (project › session title) on the left, live turn status and session
 * actions (close) on the right.
 */

import { memo } from "react";
import { Loader2, X } from "lucide-react";

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
        <span className="truncate text-xs font-medium leading-[14px] text-text-secondary">
          {session ? `${title} · ${projectName}` : `New session · ${projectName}`}
        </span>
      }
      right={
        <>
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
        </>
      }
    />
  );
});
