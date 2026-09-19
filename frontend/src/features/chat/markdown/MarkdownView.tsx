/**
 * MarkdownView per docs/ui.md §4: react-markdown + remark-gfm, memoized per
 * message; code blocks wrap (no horizontal scroll) with a language label and
 * copy button. No syntax highlighting in v0 — quiet mono keeps streaming cheap.
 */

import { memo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Check, Copy } from "lucide-react";

import { cn } from "@/lib/utils";
import { Flowchart, parseFlowBlock } from "./Flowchart";
import { CodePanel } from "./CodePanel";

function CopyButton({ getText }: { getText: () => string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      aria-label="Copy code"
      className="absolute right-2 top-2 rounded-md border border-border/60 bg-background/80 p-1.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground"
      onClick={() => {
        void navigator.clipboard.writeText(getText()).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        });
      }}
    >
      {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
    </button>
  );
}

export const MarkdownView = memo(function MarkdownView({ text }: { text: string }) {
  return (
    <div
      data-selectable
      className="prose-sm text-[14px] leading-relaxed text-foreground [&_a]:text-primary [&_a]:underline [&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_h1]:text-base [&_h1]:font-semibold [&_h2]:text-base [&_h2]:font-semibold [&_h3]:text-sm [&_h3]:font-semibold [&_hr]:border-border [&_li]:my-0.5 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_p]:my-2 [&_p:first-child]:mt-0 [&_p:last-child]:mb-0 [&_strong]:font-semibold [&_table]:my-2 [&_table]:w-full [&_table]:border-collapse [&_table]:text-sm [&_td]:border [&_td]:border-border [&_td]:px-2 [&_td]:py-1 [&_th]:border [&_th]:border-border [&_th]:bg-muted/50 [&_th]:px-2 [&_th]:py-1 [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5"
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code({ className, children, ...props }) {
            const text = String(children ?? "");
            const isBlock = /language-/.test(className ?? "") || text.includes("\n");
            if (!isBlock) {
              return (
                <code
                  className="rounded-[4px] bg-muted px-1 py-0.5 font-mono text-[13px]"
                  {...props}
                >
                  {children}
                </code>
              );
            }
            const lang = /language-(\S+)/.exec(className ?? "")?.[1] ?? "text";
            // Agent flowcharts: a circulogo-flow block carries the workflow
            // JSON taught by the session format instruction; invalid JSON
            // falls back to the plain code block.
            if (lang === "circulogo-flow") {
              const doc = parseFlowBlock(text.replace(/\n$/, ""));
              if (doc) return <Flowchart doc={doc} />;
            }
            // Code + diff fences share the owner's editor-panel design:
            // line numbers and light syntax coloring, or the unified diff
            // with gutters, accent bars and word-level highlights.
            if (text.includes("\n")) {
              return <CodePanel lang={lang} code={text.replace(/\n$/, "")} />;
            }
            return (
              <div className="group relative my-2 overflow-hidden rounded-lg border border-border bg-bg-code">
                <div className="flex h-7 items-center justify-between border-b border-border/60 px-3">
                  <span className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">
                    {lang}
                  </span>
                </div>
                <CopyButton getText={() => text.replace(/\n$/, "")} />
                <pre className="overflow-x-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-[13px] leading-relaxed">
                  <code className={className} {...props}>
                    {children}
                  </code>
                </pre>
              </div>
            );
          },
          pre({ children }) {
            // Handled inside code(); render children directly.
            return <>{children}</>;
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
});

/** Minimal unified-diff renderer for patch parts (v0: mono, no highlight). */
export const DiffView = memo(function DiffView({ diff }: { diff: string }) {
  return (
    <pre
      data-selectable
      className="max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md bg-bg-code p-2 font-mono text-[12px] leading-relaxed"
    >
      {diff.split("\n").map((line, i) => (
        <div
          key={i}
          className={cn(
            line.startsWith("+") && "bg-diff-add text-foreground",
            line.startsWith("-") && "bg-diff-del text-foreground",
            line.startsWith("@@") && "text-accent-indigo",
          )}
        >
          {line}
        </div>
      ))}
    </pre>
  );
});
