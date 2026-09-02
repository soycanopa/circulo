/** Tool call card (docs/ux.md §4): one-line collapsed, expandable detail,
 * state chip; auto-expands while running. */

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
import type { Part, ToolState } from "@/lib/agent/protocol";

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
    state.title ||
    (typeof state.input === "object" && state.input !== null
      ? String(
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
