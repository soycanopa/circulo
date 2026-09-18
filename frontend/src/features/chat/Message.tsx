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

function lastReasoningId(m: MessageRecord): string {
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

export const AssistantMessage = memo(function AssistantMessage({
  m,
  streaming,
}: {
  m: MessageRecord;
  streaming: boolean;
}) {
  // Reasoning + tool parts collapse into one ThinkingState trace (ux.md §4);
  // it renders at the position of the first of them, in server part order.
  const traceParts = m.parts.filter((p) => p.type === "reasoning" || p.type === "tool");
  const sources = useMemo(() => sourcesFromParts(m.parts), [m.parts]);
  const lastTextId = useMemo(() => {
    let id = "";
    for (const p of m.parts) if (p.type === "text") id = p.id;
    return id;
  }, [m.parts]);
  let traceRendered = false;
  return (
    <div className="flex w-full flex-col gap-2">
      {m.parts.map((p) => {
        if (p.type === "reasoning" || p.type === "tool") {
          if (traceRendered) return null;
          traceRendered = true;
          return (
            <ThinkingState
              key="trace"
              parts={traceParts}
              streaming={streaming}
              liveReasoningId={lastReasoningId(m)}
            />
          );
        }
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
      })}
      {/* Nothing streamed yet: the transcript-level LoadingState below is the
          single busy indicator — keep the bubble clean. */}
      {!streaming && (
        <TurnFooter tokens={aggregateTokens(m)} cost={m.info.cost || aggregateCost(m)} />
      )}
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
