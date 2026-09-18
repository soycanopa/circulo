import { useEffect, useLayoutEffect, useRef, useState } from "react";

/* ─────────────────────────────────────────────────────────
 * THINKING — expandable agent trace (owner's ThinkingState design),
 * driven by real protocol parts instead of staged demo timers.
 *
 *   Reasoning rows  prose that expands while streaming, settles muted
 *   Search rows     websearch/webfetch: query + sources, colored dots
 *   Coding rows     read/write/edit/patch/shell trace with ±counts
 *
 * The header shimmers while any row runs, auto-expands the trace, then
 * settles to a summary and stays manually expandable. Palette maps onto
 * the Circulo tokens: ink→text-primary, ink-2→text-secondary,
 * ink-3→text-tertiary, accent/orange/green→accent-cir/warning/success.
 * ───────────────────────────────────────────────────────── */

import { DEFAULT_LABELS, LoaderGrid, PATTERNS, useElapsed, useRotatingLabel } from "./LoadingState";
import type { Part, ToolStatus } from "@/lib/agent/protocol";

type Row = {
  key: string;
  kind: "reasoning" | "search" | "code";
  primary: string;
  secondary?: string;
  mono?: boolean;
  add?: number;
  del?: number;
  href?: string;
  status: ToolStatus;
  /** tool result payloads — shown when the row is toggled open */
  detail?: { output?: string; error?: string; input?: unknown };
  /** websearch query, lifted to the trace's magnifier line */
  query?: string;
};

/** v2 tool names (shell is v2's bash); fallback capitalizes the raw name. */
const TOOL_LABELS: Record<string, string> = {
  shell: "Run",
  read: "Read",
  edit: "Edit",
  write: "Write",
  patch: "Patch",
  grep: "Grep",
  glob: "Glob",
  list: "List",
  webfetch: "Fetch",
  websearch: "Search",
  todowrite: "Todo",
  todoread: "Todo",
};

const SEARCH_TOOLS = new Set(["websearch", "webfetch"]);

function toolLabel(tool: string): string {
  return TOOL_LABELS[tool] ?? tool.charAt(0).toUpperCase() + tool.slice(1);
}

/** Mono subtitle from the tool input (path · command · pattern · query). */
function toolSubtitle(state: NonNullable<Part["state"]>, tool: string): string {
  if (state.title) return state.title;
  if (typeof state.input === "object" && state.input !== null) {
    const i = state.input as Record<string, unknown>;
    const raw = (i.file_path ?? i.path ?? i.command ?? i.pattern ?? i.query ?? i.q ?? i.url ?? "") as string;
    if (raw) return String(raw);
  }
  return tool;
}

function diffStat(meta: unknown): { add?: number; del?: number } {
  if (!meta || typeof meta !== "object") return {};
  const m = meta as Record<string, unknown>;
  const add = m.additions ?? m.add;
  const del = m.deletions ?? m.del;
  if (typeof add === "number" && typeof del === "number") return { add, del };
  return {};
}

function buildRows(parts: Part[], streaming: boolean, liveReasoningId: string): Row[] {
  const rows: Row[] = [];
  for (const p of parts) {
    if (p.type === "reasoning") {
      const running = streaming && p.id === liveReasoningId;
      rows.push({
        key: p.id,
        kind: "reasoning",
        primary: p.text ?? "",
        status: running ? "running" : "completed",
      });
      continue;
    }
    if (p.type !== "tool") continue; // unreachable: parts are pre-filtered
    const state = p.tool && p.state ? p.state : { status: "pending" as ToolStatus };
    const tool = p.tool ?? "";
    const status = state.status;
    const search = SEARCH_TOOLS.has(tool);
    const i = (typeof state.input === "object" && state.input !== null
      ? state.input
      : {}) as Record<string, unknown>;
    const stat = diffStat(state.metadata);
    rows.push({
      key: p.id,
      kind: search ? "search" : "code",
      primary: toolLabel(tool),
      secondary: toolSubtitle(state, tool),
      mono: !search,
      href: search && typeof i.url === "string" ? i.url : undefined,
      query: search && typeof (i.q ?? i.query) === "string" ? (i.q ?? i.query) as string : undefined,
      ...stat,
      status,
      detail:
        status === "pending" || status === "running"
          ? undefined
          : { output: state.output, error: state.error, input: state.input },
    });
  }
  return rows;
}

const TONES = ["bg-accent-cir", "bg-warning", "bg-success"];

/** Trace categories (owner call): one collapsible per kind of agent work. */
export type TraceVariant = "Reasoning" | "Search" | "Coding" | "Tools";

const PHRASE_SETS: Record<TraceVariant, string[]> = {
  Reasoning: DEFAULT_LABELS,
  Search: ["Searching the web", "Reading results", "Comparing sources"],
  Coding: ["Working on files", "Reading files", "Applying edits"],
  Tools: ["Running tools", "Making calls", "Gathering context"],
};

function doneLabel(variant: TraceVariant, rows: Row[], thought: string | null): string {
  const n = rows.filter((r) => r.kind !== "reasoning").length;
  const calls = `${n} call${n === 1 ? "" : "s"}`;
  switch (variant) {
    case "Reasoning":
      return thought ?? "Thought";
    case "Search":
      return n > 0 ? `Searched the web · ${calls}` : "Searched the web";
    case "Coding":
      return n > 0 ? `Ran ${n} file op${n === 1 ? "" : "s"}` : "Worked on files";
    case "Tools":
      return n > 0 ? calls : "No tool calls";
  }
}

export default function ThinkingState({
  parts,
  variant,
  streaming,
  liveReasoningId,
}: {
  parts: Part[];
  variant: TraceVariant;
  streaming: boolean;
  /** id of the reasoning part currently streaming (drives its spinner) */
  liveReasoningId?: string;
}) {
  const rows = buildRows(parts, streaming, liveReasoningId ?? "");
  // Working requires the turn to still be streaming: a settled turn must
  // never keep the loader header spinning over a stale running row (a lost
  // tool frame heals via the idle history refetch instead).
  const working = streaming && rows.some((r) => r.status === "running" || r.status === "pending");
  const query = rows.find((r) => r.query)?.query;

  // Reasoning duration only when the wire carried start+end (hydrated
  // history rarely does — honest fallback is no duration).
  let thought: string | null = null;
  if (variant === "Reasoning") {
    for (const p of parts) {
      if (p.type === "reasoning" && p.time && p.time.end && p.time.end > p.time.start) {
        thought = `Thought for ${Math.max(1, Math.round((p.time.end - p.time.start) / 1000))} seconds`;
      }
    }
  }
  const done = doneLabel(variant, rows, thought);

  // While the turn works, the header is the pixel-grid loader with rotating
  // phrases (owner design) for this category.
  const phrase = useRotatingLabel(PHRASE_SETS[variant]);
  const elapsed = useElapsed();

  const [manualExpanded, setManualExpanded] = useState<boolean | null>(null);
  const [selectedTool, setSelectedTool] = useState<string | null>(null);
  // Traces always start folded — work stays out of the way until asked for.
  const expanded = manualExpanded ?? false;
  const traceRef = useRef<HTMLDivElement>(null);
  const [lineHeight, setLineHeight] = useState(0);
  useLayoutEffect(() => {
    if (traceRef.current) setLineHeight(traceRef.current.offsetHeight);
  }, [rows.length, expanded, working]);

  // A fresh trace re-arms the fold; manual override survives only within
  // one trace.
  useEffect(() => {
    setManualExpanded(null);
    setSelectedTool(null);
  }, [parts[0]?.id]);

  return (
    <div className="flex w-full flex-col items-center">
      {/* centered cluster: header, trace and detail travel together */}
      <div className="flex w-fit max-w-full flex-col items-start">
      {/* header — pixel-grid loader with rotating phrases while working,
          sparkle + settled summary after */}
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setManualExpanded((current) => !(current ?? false))}
        className="-mx-1.5 flex w-fit items-center gap-2 rounded-md px-1.5 py-1
          transition-colors duration-100 hover:bg-bg-hover"
      >
        <span role="status" className="contents">
          {working ? (
            <>
              <LoaderGrid {...PATTERNS.Drive} />
              <span
                key={phrase}
                className="bg-clip-text text-[13px] font-medium whitespace-nowrap text-transparent"
                style={{
                  backgroundImage:
                    "linear-gradient(90deg, var(--text-tertiary) 35%, var(--text-primary) 50%, var(--text-tertiary) 65%)",
                  backgroundSize: "200% 100%",
                  animation:
                    "shimmer-text 1.4s linear infinite, phrase-in 300ms cubic-bezier(0.16,1,0.3,1) both",
                }}
              >
                {phrase}
              </span>
              <span className="font-mono text-[12px] text-text-tertiary tabular-nums">
                {elapsed}
              </span>
            </>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--text-tertiary)">
                <path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z" />
              </svg>
              <span
                className="text-[13px] font-medium whitespace-nowrap text-text-secondary"
                style={{ animation: "fade-in 350ms ease-out both" }}
              >
                {done}
              </span>
            </>
          )}
        </span>
        <svg
          width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"
          className="transition-transform duration-300"
          style={{ transform: expanded ? "rotate(180deg)" : "rotate(0)" }}
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      {/* expandable trace */}
      <div
        className="grid transition-[grid-template-rows,opacity] duration-400"
        style={{
          gridTemplateRows: expanded ? "1fr" : "0fr",
          opacity: expanded ? 1 : 0,
          transitionTimingFunction: "cubic-bezier(0.23, 1, 0.32, 1)",
        }}
      >
        <div className="overflow-hidden">
          <div className="relative mt-1 ml-[5px] pl-4">
            <span
              aria-hidden
              className="absolute left-[3px] w-px bg-border"
              style={{ top: -8, height: lineHeight ? lineHeight - 2 : 0, transition: "height 500ms cubic-bezier(0.23,1,0.32,1)" }}
            />
            <div ref={traceRef} className="flex flex-col gap-1 py-1">
              {query && (
                <div className="flex h-6 items-center gap-2 px-1.5" style={{ animation: expanded ? "fade-up 300ms cubic-bezier(0.23,1,0.32,1) both" : undefined }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" strokeWidth="2" strokeLinecap="round" className="shrink-0">
                    <circle cx="11" cy="11" r="7" />
                    <path d="M21 21l-4.3-4.3" />
                  </svg>
                  <span className="text-[12.5px] text-text-secondary">{query}</span>
                </div>
              )}
              {rows.map((row, i) => {
                const last = i === rows.length - 1;
                const spinning = working && last && (row.status === "running" || row.status === "pending");
                const checked = row.status === "completed" || (row.status === "running" && !spinning);
                const content = (
                  <>
                    {row.kind === "search" && (
                      <span className={`flex size-3.5 shrink-0 items-center justify-center rounded-full text-white ${TONES[i % 3]}`}>
                        <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                          <circle cx="12" cy="12" r="9" />
                          <path d="M3.5 12h17M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
                        </svg>
                      </span>
                    )}
                    {row.kind === "reasoning" ? null : spinning ? (
                      <span className="size-3 shrink-0 rounded-full border-[1.5px] border-border border-t-text-secondary" style={{ animation: "spin 700ms linear infinite" }} />
                    ) : row.status === "error" ? (
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
                        <path d="M18 6L6 18M6 6l12 12" />
                      </svg>
                    ) : checked ? (
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
                        <path d="M20 6L9 17l-5-5" />
                      </svg>
                    ) : null}
                    <span
                      className={`min-w-0 truncate text-[12.5px] ${
                        row.kind === "reasoning"
                          ? "whitespace-normal leading-relaxed text-text-secondary"
                          : "font-medium text-text-primary"
                      } ${row.kind === "search" ? "animated-underline" : ""}`}
                    >
                      {row.primary}
                    </span>
                    {row.secondary && (
                      <span className={`shrink-0 text-[11.5px] text-text-tertiary ${row.mono ? "font-mono" : ""}`}>
                        {row.secondary}
                      </span>
                    )}
                    {row.add !== undefined && (
                      <span className="shrink-0 font-mono text-[11px] tabular-nums">
                        <span className="text-success">+{row.add}</span>{" "}
                        <span className="text-danger">−{row.del}</span>
                      </span>
                    )}
                  </>
                );
                const rowClass = "flex min-h-7 w-full items-center gap-2 rounded-[6px] px-1.5 py-0.5 text-left";
                const animation = { animation: `fade-up 320ms cubic-bezier(0.23,1,0.32,1) ${Math.min(i, 6) * 120}ms both` };

                if (row.kind === "search") {
                  return row.href ? (
                    <a
                      key={row.key}
                      href={row.href}
                      target="_blank"
                      rel="noreferrer"
                      className={`${rowClass} transition-colors duration-150 hover:bg-bg-hover`}
                      style={animation}
                    >
                      {content}
                    </a>
                  ) : (
                    <div key={row.key} className={rowClass} style={animation}>
                      {content}
                    </div>
                  );
                }

                if (row.kind === "code") {
                  const selected = selectedTool === row.key;
                  return (
                    <button
                      key={row.key}
                      type="button"
                      aria-pressed={selected}
                      onClick={() => setSelectedTool(selected ? null : row.key)}
                      disabled={!row.detail}
                      className={`${rowClass} transition-colors duration-150 ${
                        selected ? "bg-bg-code" : row.detail ? "hover:bg-bg-hover" : ""
                      }`}
                      style={animation}
                    >
                      {content}
                    </button>
                  );
                }

                return (
                  <div key={row.key} className={rowClass} style={animation}>
                    {content}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* selected tool detail — output / error / raw input, mono and bounded */}
      {selectedTool && (() => {
        const row = rows.find((r) => r.key === selectedTool);
        const body = row?.detail;
        if (!body) return null;
        const text = row?.status === "error" ? body.error || body.output : body.output;
        return (
          <div className="ml-[5px] mt-1 max-w-[520px] rounded-md border border-border bg-bg-code px-2.5 py-2">
            {text ? (
              <pre data-selectable className="max-h-72 overflow-auto whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-foreground/90">
                {text}
              </pre>
            ) : null}
            {body.input != null && Object.keys(body.input as object).length > 0 && (
              <details className="mt-1 text-[12px] text-muted-foreground">
                <summary className="cursor-pointer select-none">input</summary>
                <pre data-selectable className="mt-1 overflow-auto font-mono text-[12px]">
                  {JSON.stringify(body.input, null, 2)}
                </pre>
              </details>
            )}
          </div>
        );
      })()}
      </div>
    </div>
  );
}
