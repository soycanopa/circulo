/**
 * Transcript (docs/ux.md §5): single scroll column, pin-to-bottom with jump
 * pill, parts in server order, pixel-grid loader while the agent works.
 */

import { memo, useEffect } from "react";
import { ArrowDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import { usePinnedScroll } from "./hooks/usePinnedScroll";
import { AssistantMessage, UserMessage, visibleMessages } from "./Message";
import { ErrorBlock } from "./parts/MiscParts";
import { TaskList } from "./parts/TaskList";
import LoadingState from "./parts/LoadingState";
import type { SessionState } from "@/lib/agent/reducer";
import { cn } from "@/lib/utils";

const Transcript = memo(function Transcript({ session }: { session: SessionState }) {
  const shown = visibleMessages(session.messages);
  const messageIds = shown.map((m) => m.info.id).join(",");
  const partsLens = shown.map((m) => m.parts.length).join(",");
  const lastTextLens = shown
    .map((m) => {
      let len = 0;
      for (const p of m.parts)
        if (p.type === "text" || p.type === "reasoning") len = (p.text ?? "").length;
      return len;
    })
    .join(",");
  const busy = session.status === "busy" || session.status === "retry";
  // The trace (reasoning/tools) carries its own working header — the pixel
  // loader below is only for turns with nothing to trace yet.
  const last = shown[shown.length - 1];
  const hasTrace =
    !!last && last.info.role === "assistant" && last.parts.some((p) => p.type === "reasoning" || p.type === "tool");
  const { ref, pinned, jump } = usePinnedScroll([
    messageIds,
    partsLens,
    lastTextLens,
    session.permissions.length,
    session.tasks.length,
  ]);

  // Cmd+↓ (ux.md §6) lands here via the global shortcut hook.
  useEffect(() => {
    window.addEventListener("circulogo:jump-to-latest", jump);
    return () => window.removeEventListener("circulogo:jump-to-latest", jump);
  }, [jump]);

  return (
    <div className="relative min-h-0 flex-1">
      <div ref={ref} className="h-full overflow-y-auto" role="log" aria-label="Chat transcript">
        <div className="mx-auto flex w-full max-w-[768px] flex-col items-center gap-6 px-0 pt-6 pb-2">
          {shown.map((m) =>
            m.info.role === "user" ? (
              <div key={m.info.id} className="flex w-full flex-col items-end">
                <UserMessage m={m} />
              </div>
            ) : (
              <AssistantMessage key={m.info.id} m={m} streaming={busy} />
            ),
          )}
          {session.tasks.length > 0 && <TaskList tasks={session.tasks} />}
          {session.lastError && (
            <ErrorBlock name={session.lastError.name} message={session.lastError.message} />
          )}
          {busy && !hasTrace &&
            (session.status === "retry" && session.retry ? (
              <div className="text-[13px] text-muted-foreground">
                Provider retrying (attempt {session.retry.attempt}): {session.retry.message}
              </div>
            ) : (
              <LoadingState />
            ))}
        </div>
      </div>
      {!pinned && (
        <Button
          size="sm"
          variant="secondary"
          className={cn("absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full shadow-sm")}
          onClick={jump}
        >
          <ArrowDown className="size-3.5" /> Jump to latest
        </Button>
      )}
    </div>
  );
});

export default Transcript;
