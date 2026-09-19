/**
 * Transcript (docs/ux.md §5): single scroll column, pin-to-bottom with jump
 * pill, parts in server order, pixel-grid loader while the agent works.
 */

import { memo, useEffect } from "react";
import { ArrowDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import { usePinnedScroll } from "./hooks/usePinnedScroll";
import { AssistantTurn, UserMessage, visibleMessages } from "./Message";
import { ErrorBlock } from "./parts/MiscParts";
import { TaskList } from "./parts/TaskList";
import LoadingState from "./parts/LoadingState";
import type { MessageRecord, SessionState } from "@/lib/agent/reducer";
import { cn } from "@/lib/utils";

const Transcript = memo(function Transcript({ session }: { session: SessionState }) {
  const shown = visibleMessages(session.messages);
  // Consecutive assistant messages form one turn: one collapsible trace, one
  // usage footer (ux.md §4).
  type Item = { kind: "user"; m: MessageRecord } | { kind: "turn"; msgs: MessageRecord[] };
  const items: Item[] = [];
  for (const m of shown) {
    const last = items[items.length - 1];
    if (m.info.role === "assistant") {
      if (last?.kind === "turn") last.msgs.push(m);
      else items.push({ kind: "turn", msgs: [m] });
    } else {
      items.push({ kind: "user", m });
    }
  }
  const lastItem = items[items.length - 1];
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
  // The turn trace carries its own working header — the pixel loader below is
  // only for turns with nothing to trace yet.
  const hasTrace =
    lastItem?.kind === "turn" &&
    lastItem.msgs.some((m) => m.parts.some((p) => p.type === "reasoning" || p.type === "tool"));
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
          {items.map((item) =>
            item.kind === "user" ? (
              <div key={item.m.info.id} className="flex w-full flex-col items-end">
                <UserMessage m={item.m} />
              </div>
            ) : (
              <AssistantTurn
                key={item.msgs[0].info.id}
                messages={item.msgs}
                streaming={busy && item === lastItem}
              />
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
              <div className="w-full">
                <LoadingState />
              </div>
            ))}
        </div>
      </div>
      {!pinned && (
        <Button
          size="sm"
          variant="secondary"
          className={cn("absolute bottom-4 left-1/2 -translate-x-1/2 z-20 rounded-full shadow-sm")}
          onClick={jump}
        >
          <ArrowDown className="size-3.5" /> Jump to latest
        </Button>
      )}
      {/* Edge fades (owner call): content dissolves instead of clipping hard
          at the viewport top and where the composer takes over. Purely
          visual — pointer-events keep the transcript interactive. */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-0 top-0 z-10 h-7 bg-gradient-to-b from-bg-main to-transparent"
      />
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-0 bottom-0 z-10 h-12 bg-gradient-to-t from-bg-main via-bg-main/70 to-transparent"
      />
    </div>
  );
});

export default Transcript;
