/**
 * Transcript per docs/ux.md §5: single scroll column, pin-to-bottom with
 * jump pill, parts render in server order, working timer at 1 Hz.
 */

import { memo, useEffect, useMemo, useRef, useState } from "react";
import { ArrowDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  ErrorBlock,
  PatchCard,
  ReasoningPart,
  SubtaskPill,
  ToolCard,
  TurnFooter,
} from "./parts";
import { MarkdownView } from "./Markdown";
import type { MessageRecord, SessionState } from "@/lib/agent/reducer";
import { cn } from "@/lib/utils";

const BOTTOM_EPSILON = 24;

function usePinnedScroll(deps: unknown[]) {
  const ref = useRef<HTMLDivElement>(null);
  const [pinned, setPinned] = useState(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = () => {
      const atBottom =
        el.scrollHeight - el.scrollTop - el.clientHeight < BOTTOM_EPSILON;
      setPinned(atBottom);
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    const el = ref.current;
    if (el && pinned) el.scrollTop = el.scrollHeight;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const jump = () => {
    const el = ref.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  };
  return { ref, pinned, jump };
}

const UserMessage = memo(function UserMessage({ m }: { m: MessageRecord }) {
  const text = m.parts
    .filter((p) => p.type === "text")
    .map((p) => p.text ?? "")
    .join("\n");
  return (
    <div className="flex justify-end">
      <div className="max-w-[85%] whitespace-pre-wrap break-words rounded-xl rounded-br-sm border border-border bg-card px-3.5 py-2.5 text-[15px] leading-relaxed">
        {text}
      </div>
    </div>
  );
});

const AssistantMessage = memo(function AssistantMessage({
  m,
  streaming,
}: {
  m: MessageRecord;
  streaming: boolean;
}) {
  const lastTextId = useMemo(() => {
    let id = "";
    for (const p of m.parts) if (p.type === "text") id = p.id;
    return id;
  }, [m.parts]);

  return (
    <div className="space-y-2">
      {m.parts.map((p) => {
        switch (p.type) {
          case "reasoning":
            return <ReasoningPart key={p.id} part={p} streaming={streaming && p.id === lastStreamingPartId(m)} />;
          case "tool":
            return <ToolCard key={p.id} part={p} />;
          case "patch":
            return <PatchCard key={p.id} part={p} />;
          case "agent":
          case "subtask":
            return <SubtaskPill key={p.id} part={p} />;
          case "text":
            return <MarkdownView key={p.id} text={p.text ?? ""} />;
          case "step-start":
          case "step-finish":
          case "file":
          default:
            return null; // step markers fold into the footer
        }
      })}
      {streaming && lastTextId === "" && (
        <div className="flex items-center gap-2 text-[13px] text-muted-foreground">
          <span className="inline-flex gap-1">
            <Dot delay="0ms" />
            <Dot delay="150ms" />
            <Dot delay="300ms" />
          </span>
        </div>
      )}
      {!streaming && (
        <TurnFooter
          tokens={aggregateTokens(m)}
          cost={m.info.cost || aggregateCost(m)}
        />
      )}
    </div>
  );
});

function lastStreamingPartId(m: MessageRecord): string {
  for (let i = m.parts.length - 1; i >= 0; i--) {
    if (m.parts[i].type === "reasoning") return m.parts[i].id;
  }
  return "";
}

function aggregateTokens(m: MessageRecord) {
  if (m.info.tokens) return m.info.tokens;
  let tokens;
  for (const p of m.parts) {
    if (p.type === "step-finish" && p.tokens) {
      tokens = p.tokens; // last step carries cumulative totals on most turns
    }
  }
  return tokens;
}

function aggregateCost(m: MessageRecord): number {
  let cost = 0;
  for (const p of m.parts) if (p.type === "step-finish" && p.cost) cost += p.cost;
  return cost;
}

function Dot({ delay }: { delay: string }) {
  return (
    <span
      className="size-1.5 animate-bounce rounded-full bg-muted-foreground/70"
      style={{ animationDelay: delay }}
    />
  );
}

/** Optimistic rows (client_*) are hidden once the server echo of the same
 * text exists — otherwise every prompt renders as two bubbles (flow.md §3). */
function visibleMessages(messages: MessageRecord[]): MessageRecord[] {
  const serverUserTexts = new Set(
    messages
      .filter((m) => m.info.role === "user" && !m.info.id.startsWith("client_"))
      .map((m) => m.parts.find((p) => p.type === "text")?.text ?? ""),
  );
  return messages.filter(
    (m) =>
      !(m.info.id.startsWith("client_") &&
        serverUserTexts.has(m.parts.find((p) => p.type === "text")?.text ?? "")),
  );
}

export function Transcript({ session }: { session: SessionState }) {
  const shown = visibleMessages(session.messages);
  const messageIds = shown.map((m) => m.info.id).join(",");
  const partsLens = shown.map((m) => m.parts.length).join(",");
  const lastTextLens = shown
    .map((m) => {
      let len = 0;
      for (const p of m.parts) if (p.type === "text" || p.type === "reasoning") len = (p.text ?? "").length;
      return len;
    })
    .join(",");
  const busy = session.status === "busy" || session.status === "retry";
  const { ref, pinned, jump } = usePinnedScroll([
    messageIds,
    partsLens,
    lastTextLens,
    session.permissions.length,
  ]);

  // Turn timer: record when busy starts; reset on idle. 1 Hz tick (UX §5).
  const [now, setNow] = useState(Date.now());
  const busyStartRef = useRef<number>(0);
  const [busySince, setBusySince] = useState<number>(0);
  useEffect(() => {
    if (busy) {
      if (!busyStartRef.current) {
        busyStartRef.current = Date.now();
        setBusySince(busyStartRef.current);
      }
      const t = setInterval(() => setNow(Date.now()), 1000);
      return () => clearInterval(t);
    }
    busyStartRef.current = 0;
    setBusySince(0);
  }, [busy]);

  return (
    <div className="relative min-h-0 flex-1">
      <div ref={ref} className="h-full overflow-y-auto" role="log" aria-label="Chat transcript">
        <div className="mx-auto flex max-w-3xl flex-col gap-4 px-6 py-6">
          {shown.map((m) =>
            m.info.role === "user" ? (
              <UserMessage key={m.info.id} m={m} />
            ) : (
              <AssistantMessage key={m.info.id} m={m} streaming={busy} />
            ),
          )}
          {session.lastError && (
            <ErrorBlock name={session.lastError.name} message={session.lastError.message} />
          )}
          {busy && (
            <div className="flex items-center gap-2 text-[13px] text-muted-foreground">
              <span className="inline-flex gap-1">
                <Dot delay="0ms" />
                <Dot delay="150ms" />
                <Dot delay="300ms" />
              </span>
              {session.status === "retry" && session.retry ? (
                <span>
                  Provider retrying (attempt {session.retry.attempt}): {session.retry.message}
                </span>
              ) : (
                <span>
                  Working…
                  {busySince ? ` ${Math.max(1, Math.round((now - busySince) / 1000))}s` : ""}
                </span>
              )}
            </div>
          )}
        </div>
      </div>
      {!pinned && (
        <Button
          size="sm"
          variant="secondary"
          className={cn(
            "absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full shadow-sm",
          )}
          onClick={jump}
        >
          <ArrowDown className="size-3.5" /> Jump to latest
        </Button>
      )}
    </div>
  );
}

