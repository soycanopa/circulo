/**
 * Part renderers per docs/ux.md §4: reasoning collapses (auto-open while
 * streaming), tool calls are one-line cards with expandable detail, patches
 * list changed files, steps fold into a turn footer.
 */

import { memo, useEffect, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  CircleDashed,
  FilePen,
  FileText,
  FolderTree,
  GitBranch,
  Globe,
  Loader2,
  Search,
  SquareTerminal,
  Wrench,
  X,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import type { Part, ToolState, TokenUsage } from "@/lib/agent/protocol";

function ToolIcon({ tool }: { tool?: string }) {
  const t = tool ?? "";
  const cls = "size-3.5 shrink-0";
  if (/bash|shell|terminal|command/.test(t)) return <SquareTerminal className={cls} />;
  if (/edit|write|patch|multiedit/.test(t)) return <FilePen className={cls} />;
  if (/read|view|cat/.test(t)) return <FileText className={cls} />;
  if (/grep|search|find/.test(t)) return <Search className={cls} />;
  if (/glob|list|ls/.test(t)) return <FolderTree className={cls} />;
  if (/web|fetch|http/.test(t)) return <Globe className={cls} />;
  if (/task|agent|subtask/.test(t)) return <GitBranch className={cls} />;
  return <Wrench className={cls} />;
}

function StateChip({ state }: { state: ToolState }) {
  switch (state.status) {
    case "running":
      return (
        <Badge variant="outline" className="gap-1 text-[11px]">
          <Loader2 className="size-3 animate-spin" /> running
        </Badge>
      );
    case "completed":
      return (
        <Badge variant="outline" className="gap-1 text-[11px] text-emerald-600 dark:text-emerald-400">
          <Check className="size-3" /> done
        </Badge>
      );
    case "error":
      return (
        <Badge variant="outline" className="gap-1 text-[11px] text-red-600 dark:text-red-400">
          <X className="size-3" /> error
        </Badge>
      );
    default:
      return (
        <Badge variant="outline" className="gap-1 text-[11px] text-muted-foreground">
          <CircleDashed className="size-3" /> pending
        </Badge>
      );
  }
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

  const label =
    state.title || (typeof state.input === "object" && state.input !== null
      ? // Common shapes: {command}, {file_path}, {pattern}
        String(
          (state.input as Record<string, unknown>).command ??
            (state.input as Record<string, unknown>).file_path ??
            (state.input as Record<string, unknown>).pattern ??
            part.tool,
        )
      : (part.tool ?? "tool"));

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="group/tool">
      <div className="rounded-lg border border-border bg-card/50">
        <CollapsibleTrigger className="flex w-full items-center gap-2 px-3 py-2 text-left text-[13px]">
          <ToolIcon tool={part.tool} />
          <span className="font-medium text-foreground/90">{part.tool}</span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">{label}</span>
          <StateChip state={state} />
          {open ? (
            <ChevronDown className="size-3.5 text-muted-foreground" />
          ) : (
            <ChevronRight className="size-3.5 text-muted-foreground" />
          )}
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="space-y-2 border-t border-border/60 px-3 py-2">
            {state.output && (
              <pre
                data-selectable
                className="max-h-96 overflow-auto whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-foreground/90"
              >
                {state.status === "error" ? state.error || state.output : state.output}
              </pre>
            )}
            {state.status === "error" && !state.output && state.error && (
              <pre className="whitespace-pre-wrap break-words font-mono text-[12px] text-red-600 dark:text-red-400">
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

/** Reasoning: open while streaming (showing the tail), collapsed after. */
export const ReasoningPart = memo(function ReasoningPart({
  part,
  streaming,
}: {
  part: Part;
  streaming: boolean;
}) {
  const [open, setOpen] = useState(streaming);
  useEffect(() => {
    if (streaming) setOpen(true);
  }, [streaming]);

  const text = part.text ?? "";
  const tail = streaming && open ? text.split("\n").slice(-10).join("\n") : text;

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <div className="rounded-lg border border-dashed border-border bg-muted/30">
        <CollapsibleTrigger className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[13px] text-muted-foreground">
          {streaming ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : open ? (
            <ChevronDown className="size-3.5" />
          ) : (
            <ChevronRight className="size-3.5" />
          )}
          <span className="font-medium">Thinking{streaming ? "…" : ""}</span>
          <span className="min-w-0 flex-1" />
        </CollapsibleTrigger>
        <CollapsibleContent>
          <pre
            data-selectable
            className="max-h-72 overflow-auto whitespace-pre-wrap break-words px-3 pb-2 font-mono text-[12.5px] leading-relaxed text-muted-foreground"
          >
            {tail}
          </pre>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
});

/** Patch part: changed-files summary. (Diff text arrives via tool output in
 *  v0; the file list is what the server guarantees here.) */
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
        {part.type === "subtask" ? `subtask: ${part.agent ?? ""} — ${part.description ?? part.prompt ?? ""}` : `agent: ${part.name ?? ""}`}
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

export const ErrorBlock = memo(function ErrorBlock({ name, message }: { name: string; message: string }) {
  return (
    <div className="rounded-lg border border-red-500/40 bg-red-500/5 px-3 py-2 text-[13px]">
      <div className="font-medium text-red-600 dark:text-red-400">{name || "Error"}</div>
      <div className="text-foreground/80">{message}</div>
    </div>
  );
});

/** Group consecutive non-answer parts of a finished assistant message. */
export function partIsVisibleAnswer(p: Part): boolean {
  return p.type === "text";
}
