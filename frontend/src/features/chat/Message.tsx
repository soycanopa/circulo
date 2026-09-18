/**
 * Message rows: user bubbles and assistant turns. Assistant parts render in
 * strict server order; step markers fold into the usage footer; optimistic
 * user rows are hidden once the server echo of the same text exists
 * (flow.md §3).
 */

import { memo, useMemo } from "react";

import ThinkingState from "./parts/ThinkingState";
import { AssistantText, sourcesFromParts } from "./parts/AssistantText";
import { PatchCard, SubtaskPill, TurnFooter } from "./parts/MiscParts";
import type { MessageRecord } from "@/lib/agent/reducer";

export const UserMessage = memo(function UserMessage({ m }: { m: MessageRecord }) {
  const text = m.parts
    .filter((p) => p.type === "text")
    .map((p) => p.text ?? "")
    .join("\n");
  return (
    <div className="flex flex-col items-end">
      <div className="max-w-[560px] whitespace-pre-wrap break-words rounded-l-lg rounded-br-lg rounded-tr-xs bg-bg-hover px-4 py-3 text-md font-medium text-text-primary">
        {text}
      </div>
    </div>
  );
});

function lastReasoningIdOf(parts: MessageRecord["parts"]): string {
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i].type === "reasoning") return parts[i].id;
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

export const AssistantTurn = memo(function AssistantTurn({
  messages,
  streaming,
}: {
  /** every assistant message of one turn (consecutive server messages) */
  messages: MessageRecord[];
  streaming: boolean;
}) {
  // One collapsible per TURN: reasoning + tool parts of every step collapse
  // into a single ThinkingState; texts and cards follow in server order, and
  // the usage footer covers the whole turn once it settles.
  const allParts = useMemo(() => messages.flatMap((m) => m.parts), [messages]);
  const traceParts = useMemo(
    () => allParts.filter((p) => p.type === "reasoning" || p.type === "tool"),
    [allParts],
  );
  const sources = useMemo(() => sourcesFromParts(allParts), [allParts]);
  const lastTextId = useMemo(() => {
    let id = "";
    for (const p of allParts) if (p.type === "text") id = p.id;
    return id;
  }, [allParts]);
  const tokens = useMemo(() => {
    const sum = { input: 0, output: 0, reasoning: 0, cacheRead: 0, cacheWrite: 0 };
    let any = false;
    for (const m of messages) {
      const t = aggregateTokens(m);
      if (!t) continue;
      any = true;
      sum.input += t.input;
      sum.output += t.output;
      sum.reasoning += t.reasoning;
      sum.cacheRead += t.cacheRead;
      sum.cacheWrite += t.cacheWrite;
    }
    return any ? sum : undefined;
  }, [messages]);
  const cost = useMemo(
    () => messages.reduce((acc, m) => acc + (m.info.cost || aggregateCost(m)), 0),
    [messages],
  );
  return (
    <div className="flex w-full flex-col gap-2">
      {traceParts.length > 0 && (
        <ThinkingState
          parts={traceParts}
          streaming={streaming}
          liveReasoningId={lastReasoningIdOf(allParts)}
        />
      )}
      {messages.map((m) =>
        m.parts.map((p) => {
          switch (p.type) {
            case "patch":
              return <PatchCard key={p.id} part={p} />;
            case "agent":
            case "subtask":
              return <SubtaskPill key={p.id} part={p} />;
            case "text":
              return (
                <AssistantText
                  key={p.id}
                  partKey={p.id}
                  text={p.text ?? ""}
                  streaming={streaming && p.id === lastTextId}
                  sources={sources}
                  showActions={p.id === lastTextId}
                />
              );
            case "step-start":
            case "step-finish":
            case "file":
            default:
              return null; // step markers fold into the footer
          }
        }),
      )}
      {/* Nothing streamed yet: the transcript-level LoadingState below is the
          single busy indicator — keep the turn clean. */}
      {!streaming && <TurnFooter tokens={tokens} cost={cost} />}
    </div>
  );
});

/** Optimistic rows (client_*) are hidden once the server echo of the same
 * text exists — otherwise every prompt renders as two bubbles. */
export function visibleMessages(messages: MessageRecord[]): MessageRecord[] {
  const serverUserTexts = new Set(
    messages
      .filter((m) => m.info.role === "user" && !m.info.id.startsWith("client_"))
      .map((m) => m.parts.find((p) => p.type === "text")?.text ?? ""),
  );
  return messages.filter(
    (m) =>
      !(
        m.info.id.startsWith("client_") &&
        serverUserTexts.has(m.parts.find((p) => p.type === "text")?.text ?? "")
      ),
  );
}
