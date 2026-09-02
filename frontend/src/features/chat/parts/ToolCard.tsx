/**
 * Tool call card — replica of the Circulo design: full-width rounded card on
 * the code surface, 22px circular state icon (indigo spinner / green check /
 * red X / muted pending), 13px title + mono 12px subtitle, expandable detail.
 */

import { memo, useEffect, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  CircleDashed,
  Loader2,
  X,
} from "lucide-react";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import type { Part, ToolState } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

function StateBadge({ state }: { state: ToolState }) {
  const base = "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-full";
  switch (state.status) {
    case "running":
      return (
        <div className={cn(base, "bg-accent-indigo")}>
          <Loader2 className="size-3 animate-spin text-white" />
        </div>
      );
    case "completed":
      return (
        <div className={cn(base, "bg-success")}>
          <Check className="size-3 text-white" strokeWidth={3} />
        </div>
      );
    case "error":
      return (
        <div className={cn(base, "bg-destructive")}>
          <X className="size-3 text-white" strokeWidth={3} />
        </div>
      );
    default:
      return (
        <div className={cn(base, "bg-muted")}>
          <CircleDashed className="size-3 text-text-tertiary" />
        </div>
      );
  }
}

/** Mono subtitle from the tool input, per the design (path · command · pattern). */
function subtitle(state: ToolState, tool?: string): string {
  if (state.title) return state.title;
  if (typeof state.input === "object" && state.input !== null) {
    const i = state.input as Record<string, unknown>;
    const raw = (i.file_path ?? i.path ?? i.command ?? i.pattern ?? i.query ?? "") as string;
    if (raw) return String(raw);
  }
  return tool ?? "";
}

function formatInput(state: ToolState): string {
  if (state.input == null) return "";
  try {
    return JSON.stringify(state.input, null, 2);
  } catch {
    return String(state.input);
  }
}

export const ToolCard = memo(function ToolCard({ part }: { part: Part }) {
  const [open, setOpen] = useState(false);
  const state = part.state ?? { status: "pending" as const };
  // Auto-expand while running (live feedback), re-collapsible by the user.
  useEffect(() => {
    if (state.status === "running") setOpen(true);
  }, [state.status]);

  const sub = subtitle(state, part.tool);

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="group/tool">
      <div className="rounded-lg border border-border bg-bg-code">
        <CollapsibleTrigger className="flex w-full items-center gap-[10px] px-3 py-[10px] text-left">
          <StateBadge state={state} />
          <span className="flex min-w-0 flex-col grow gap-px">
            <span className="truncate text-[13px] font-medium leading-[18px] text-foreground">
              {part.tool}
              {state.status === "running" ? "…" : ""}
            </span>
            {sub && (
              <span className="truncate font-mono text-[12px] leading-[14px] text-text-tertiary">
                {sub}
              </span>
            )}
          </span>
          {open ? (
            <ChevronDown className="size-3.5 shrink-0 text-text-tertiary" />
          ) : (
            <ChevronRight className="size-3.5 shrink-0 text-text-tertiary" />
          )}
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="space-y-2 border-t border-border px-3 py-2">
            {state.output && (
              <pre
                data-selectable
                className="max-h-96 overflow-auto whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-foreground/90"
              >
                {state.status === "error" ? state.error || state.output : state.output}
              </pre>
            )}
            {state.status === "error" && !state.output && state.error && (
              <pre className="whitespace-pre-wrap break-words font-mono text-[12px] text-destructive">
                {state.error}
              </pre>
            )}
            {state.input != null && Object.keys(state.input as object).length > 0 && (
              <details className="text-[12px] text-muted-foreground">
                <summary className="cursor-pointer select-none">input</summary>
                <pre data-selectable className="mt-1 overflow-auto font-mono text-[12px]">
                  {formatInput(state)}
                </pre>
              </details>
            )}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
});
