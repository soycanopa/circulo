/** Patch, subtask and turn-footer parts. */

import { memo } from "react";
import { FilePen, GitBranch } from "lucide-react";

import type { Part, TokenUsage } from "@/lib/agent/protocol";

export const PatchCard = memo(function PatchCard({ part }: { part: Part }) {
  return (
    <div className="rounded-lg border border-border bg-card/50 px-3 py-2 text-[13px]">
      <div className="flex items-center gap-2 font-medium text-foreground/90">
        <FilePen className="size-3.5" /> Changed files
      </div>
      <ul className="mt-1 space-y-0.5">
        {(part.files ?? []).map((f) => (
          <li key={f} className="truncate font-mono text-[12px] text-muted-foreground">
            {f}
          </li>
        ))}
      </ul>
    </div>
  );
});

export const SubtaskPill = memo(function SubtaskPill({ part }: { part: Part }) {
  return (
    <div className="inline-flex max-w-full items-center gap-1.5 rounded-full border border-border px-2.5 py-1 text-[12px] text-muted-foreground">
      <GitBranch className="size-3" />
      <span className="truncate">
        {part.type === "subtask"
          ? `subtask: ${part.agent ?? ""} — ${part.description ?? part.prompt ?? ""}`
          : `agent: ${part.name ?? ""}`}
      </span>
    </div>
  );
});

export const TurnFooter = memo(function TurnFooter({
  tokens,
  cost,
}: {
  tokens?: TokenUsage;
  cost?: number;
}) {
  if (!tokens && !cost) return null;
  const total = tokens ? tokens.input + tokens.output + tokens.reasoning : 0;
  return (
    <div className="flex items-center gap-2 text-[11.5px] text-muted-foreground">
      <span className="h-px flex-1 bg-border" />
      <span>
        {total > 0 ? `${(total / 1000).toFixed(1)}k tok` : ""}
        {tokens && tokens.cacheRead > 0 ? ` · ${tokens.cacheRead} cached` : ""}
        {cost ? ` · $${cost.toFixed(4)}` : ""}
      </span>
      <span className="h-px w-8 bg-border" />
    </div>
  );
});

export const ErrorBlock = memo(function ErrorBlock({
  name,
  message,
}: {
  name: string;
  message: string;
}) {
  return (
    <div className="rounded-lg border border-red-500/40 bg-red-500/5 px-3 py-2 text-[13px]">
      <div className="font-medium text-red-600 dark:text-red-400">{name || "Error"}</div>
      <div className="text-foreground/80">{message}</div>
    </div>
  );
});
