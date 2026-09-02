/** Reasoning part (docs/ux.md §4): auto-open while streaming showing the
 * tail; collapsed single row after completion. */

import { memo, useEffect, useState } from "react";
import { ChevronDown, ChevronRight, Loader2 } from "lucide-react";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import type { Part } from "@/lib/agent/protocol";

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
