/**
 * ChatAppBar: the app bar content for the chat area — session context on the
 * left, live turn status on the right. Future chat-area controls go into the
 * AppBar's remaining slots (center / more right items).
 */

import { memo } from "react";
import { Loader2 } from "lucide-react";

import { AppBar } from "@/components/layout/AppBar";
import type { ProjectView } from "@/lib/agent/protocol";
import type { SessionState } from "@/lib/agent/reducer";

export const ChatAppBar = memo(function ChatAppBar({
  project,
  session,
}: {
  project: ProjectView;
  session?: SessionState;
}) {
  const title = session?.session.title || "New chat";
  const busy = session?.status === "busy";
  const retrying = session?.status === "retry";

  return (
    <AppBar
      left={
        <>
          <span className="truncate text-[13.5px] font-medium">{title}</span>
          <span className="truncate text-[12px] text-muted-foreground">{project.path}</span>
        </>
      }
      right={
        busy ? (
          <span className="flex items-center gap-1.5 text-[12px] text-muted-foreground">
            <Loader2 className="size-3.5 animate-spin" /> working
          </span>
        ) : retrying ? (
          <span className="flex items-center gap-1.5 text-[12px] text-amber-600 dark:text-amber-400">
            <Loader2 className="size-3.5 animate-spin" />
            retrying{session?.retry ? ` (attempt ${session.retry.attempt})` : ""}
          </span>
        ) : null
      }
    />
  );
});
