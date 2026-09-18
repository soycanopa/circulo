import { useCallback, useMemo, useState, type ReactNode } from "react";

/* ─────────────────────────────────────────────────────────
 * CODE PANEL (owner's CodeBlock design) — markdown code and
 * diff blocks rendered as one editor-style panel:
 *   · Code — line-numbered listing with light syntax coloring
 *   · Diff — unified diff: old/new gutters, green/red accent
 *     bar + row tint, word-level add/del highlights
 * The agent emits ```lang and ```diff fences; MarkdownView
 * routes them here.
 * ───────────────────────────────────────────────────────── */

/* A single run of code within a diff row; `change` tints it as an add/del. */
export type CodePiece = { text: string; change?: "add" | "del" };
/* One row of a unified diff: old/new line numbers, its kind, and its pieces. */
export type DiffRow = {
  old: number | null;
  cur: number | null;
  type: "ctx" | "add" | "del";
  pieces: CodePiece[];
};

const HATCH =
  "repeating-linear-gradient(45deg, var(--danger) 0, var(--danger) 1.5px, transparent 1.5px, transparent 3px)";

/* light syntax coloring — keywords, function calls, strings & numbers */
const KEYWORDS = new Set([
  "import", "from", "export", "default", "async", "function", "const", "let",
  "var", "await", "return", "if", "else", "for", "while", "new", "throw",
  "try", "catch", "null", "true", "false", "undefined", "func", "def", "class",
  "struct", "interface", "type", "package", "public", "private", "fn", "use",
  "match", "switch", "case", "break", "continue", "go", "defer", "select",
]);
const TOKEN =
  /("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`[^`]*`|\b\d+(?:\.\d+)?\b|\b(?:import|from|export|default|async|function|const|let|var|await|return|if|else|for|while|new|throw|try|catch|null|true|false|undefined|func|def|class|struct|interface|type|package|public|private|fn|use|match|switch|case|break|continue|go|defer|select)\b|[A-Za-z_$][\w$]*(?=\s*\())/g;

/** Light tokenizer: strings/numbers amber, keywords indigo, calls emphasized. */
export function highlight(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let last = 0;
  let k = 0;
  for (const m of text.matchAll(TOKEN)) {
    const idx = m.index ?? 0;
    const t = m[0];
    if (idx > last) nodes.push(<span key={k++}>{text.slice(last, idx)}</span>);
    let color: string;
    let weight: number | undefined;
    if (/^["'`]/.test(t) || /^\d/.test(t)) color = "var(--warning)";
    else if (KEYWORDS.has(t)) color = "var(--accent-cir)";
    else {
      color = "var(--text-primary)";
      weight = 500;
    }
    nodes.push(
      <span key={k++} style={{ color, fontWeight: weight }}>
        {t}
      </span>,
    );
    last = idx + t.length;
  }
  if (last < text.length) nodes.push(<span key={k++}>{text.slice(last)}</span>);
  return nodes;
}

function Pieces({ pieces }: { pieces: CodePiece[] }) {
  return (
    <>
      {pieces.map((p, i) => {
        if (p.change) {
          const add = p.change === "add";
          return (
            <span
              key={i}
              className="rounded-[3px]"
              style={{
                background: `color-mix(in srgb, var(--${add ? "success" : "danger"}) 18%, transparent)`,
                padding: "0 2px",
                margin: "0 -1px",
                boxDecorationBreak: "clone",
                WebkitBoxDecorationBreak: "clone",
              }}
            >
              {highlight(p.text)}
            </span>
          );
        }
        return <span key={i}>{highlight(p.text)}</span>;
      })}
    </>
  );
}

function FileIcon() {
  return (
    <svg aria-hidden width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="shrink-0 text-text-tertiary">
      <path d="M17.25 6.75 22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3-4.5 16.5" />
    </svg>
  );
}

/* word-level diff of one replaced line pair (split keeps whitespace) */
function wordDiff(oldText: string, newText: string): { oldPieces: CodePiece[]; newPieces: CodePiece[] } {
  const ow = oldText.split(/(\s+)/);
  const nw = newText.split(/(\s+)/);
  let start = 0;
  while (start < ow.length && start < nw.length && ow[start] === nw[start]) start++;
  let endO = ow.length;
  let endN = nw.length;
  while (endO > start && endN > start && ow[endO - 1] === nw[endN - 1]) {
    endO--;
    endN--;
  }
  const join = (words: string[], change?: "add" | "del"): CodePiece[] => {
    const text = words.join("");
    return text ? [{ text, change }] : [];
  };
  return {
    oldPieces: [
      ...join(ow.slice(0, start)),
      ...join(ow.slice(start, endO), "del"),
      ...join(ow.slice(endO)),
    ],
    newPieces: [
      ...join(nw.slice(0, start)),
      ...join(nw.slice(start, endN), "add"),
      ...join(nw.slice(endN)),
    ],
  };
}

/**
 * Parses a ```diff fence into render rows: +/- prefixes become add/del
 * (hunk headers, ---/+++ and "no newline" markers are skipped), gutter
 * numbers are synthesized from 1, and a del row directly followed by an
 * add row gets word-level pieces for the changed span.
 */
export function parseDiffLines(text: string): DiffRow[] {
  const lines = text.replace(/\n$/, "").split("\n");
  const plain: { type: "ctx" | "add" | "del"; text: string }[] = [];
  let old = 1;
  let cur = 1;
  for (const line of lines) {
    if (/^(diff |index |@@|[+]{3} |[-]{3} |\\ No newline)/.test(line)) continue;
    if (line.startsWith("+")) {
      plain.push({ type: "add", text: line.slice(1) });
      cur++;
    } else if (line.startsWith("-")) {
      plain.push({ type: "del", text: line.slice(1) });
      old++;
    } else {
      plain.push({ type: "ctx", text: line.startsWith(" ") ? line.slice(1) : line });
      old++;
      cur++;
    }
  }

  const rows: DiffRow[] = [];
  let o = 1;
  let c = 1;
  for (let i = 0; i < plain.length; i++) {
    const row = plain[i];
    const next = plain[i + 1];
    if (row.type === "del" && next?.type === "add") {
      const { oldPieces, newPieces } = wordDiff(row.text, next.text);
      rows.push({ old: o++, cur: null, type: "del", pieces: oldPieces });
      rows.push({ old: null, cur: c++, type: "add", pieces: newPieces });
      i++; // consume the paired add
      continue;
    }
    if (row.type === "del") {
      rows.push({ old: o++, cur: null, type: "del", pieces: [{ text: row.text }] });
      continue;
    }
    if (row.type === "add") {
      rows.push({ old: null, cur: c++, type: "add", pieces: [{ text: row.text }] });
      continue;
    }
    rows.push({ old: o++, cur: c++, type: "ctx", pieces: [{ text: row.text }] });
  }
  return rows;
}

/* ── the panel ── */
export function CodePanel({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const copy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [code]);

  const isDiff = lang === "diff";
  const lines = useMemoLines(isDiff ? "" : code);
  const diff = useMemoDiff(isDiff ? code : "");
  const added = diff.filter((r) => r.type === "add").length;
  const removed = diff.filter((r) => r.type === "del").length;

  return (
    <div className="w-full overflow-hidden rounded-xl border border-border bg-bg-popover">
      {/* header — file · (diff stat | copy) */}
      <div className="flex h-11 items-center gap-2 border-b border-border px-4 text-[12.5px]">
        <span className="inline-flex min-w-0 items-center gap-[7px]">
          <FileIcon />
          <span className="truncate font-mono leading-none text-text-primary">{lang}</span>
        </span>

        {isDiff ? (
          <span className="ml-auto inline-flex items-center gap-2 font-mono text-[12px] leading-none tabular-nums">
            <span className="text-success">+{added}</span>
            <span className="text-danger">-{removed}</span>
          </span>
        ) : (
          <button
            type="button"
            aria-label="Copy code"
            onClick={copy}
            className={`-mr-1 ml-auto flex h-6 items-center gap-1 rounded-[6px] px-1.5 text-[12px]
              font-medium transition-colors duration-100 hover:bg-bg-hover
              ${copied ? "text-success" : "text-text-tertiary hover:text-text-primary"}`}
          >
            {copied ? (
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6L9 17l-5-5" /></svg>
            ) : (
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2.5" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>
            )}
            {copied ? "Copied" : "Copy"}
          </button>
        )}
      </div>

      {/* body — equal 12px inset on top / left / right; lines wrap */}
      <div className="py-3 font-mono text-[12.5px] leading-[1.65] text-text-secondary">
        {isDiff ? (
          <div className="relative">
            <span className="pointer-events-none absolute inset-y-0 left-5 w-px bg-border" />
            {diff.map((r, i) => {
              const add = r.type === "add";
              const del = r.type === "del";
              // one gutter column: removals keep the old number,
              // additions/context show the new one
              const num = del ? r.old : r.cur;
              return (
                <div
                  key={i}
                  className={`relative grid grid-cols-[20px_minmax(0,1fr)] items-start ${
                    add ? "bg-success/10" : del ? "bg-danger/10" : ""
                  }`}
                >
                  {(add || del) && (
                    <span
                      className="absolute inset-y-0 left-0 w-[3px]"
                      style={{ background: add ? "var(--success)" : HATCH }}
                    />
                  )}
                  <span className={`select-none text-center text-[11px] ${add ? "text-success" : del ? "text-danger" : "text-text-tertiary"}`}>
                    {num ?? ""}
                  </span>
                  <code data-selectable className="whitespace-pre-wrap break-words pl-1 pr-3">
                    <Pieces pieces={r.pieces} />
                  </code>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="relative">
            <span className="pointer-events-none absolute inset-y-0 left-5 w-px bg-border" />
            {lines.map((line, i) => (
              <div key={i} className="grid grid-cols-[20px_minmax(0,1fr)] items-start">
                <span className="select-none text-center text-[11px] text-text-tertiary">{i + 1}</span>
                <code data-selectable className="whitespace-pre-wrap break-words pl-1 pr-3">
                  {highlight(line)}
                </code>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function useMemoLines(code: string): string[] {
  return useMemo(() => code.replace(/\n$/, "").split("\n"), [code]);
}

function useMemoDiff(code: string): DiffRow[] {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useMemo(() => parseDiffLines(code), [code]);
}
