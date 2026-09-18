import { memo, useMemo, useState } from "react";
import { Check, Copy, Globe } from "lucide-react";

import { MarkdownView } from "@/features/chat/markdown/MarkdownView";
import type { Part } from "@/lib/agent/protocol";
import { useSmoothText } from "./useSmoothText";

/* ─────────────────────────────────────────────────────────
 * ASSISTANT TEXT — streaming text per the owner's StreamingText
 * design: the words resolve one by one behind a caret while the
 * part streams; when it settles, an actions row appears with a
 * working copy button and (when the turn used web tools) a
 * sources toggle with an avatar stack and an expandable list.
 *
 * Real-data notes: the wire has no inline citation markers, so no
 * inline chips; sources derive from the turn's websearch/webfetch
 * tool parts. Follow-up prompts need a producer on the wire and
 * are intentionally absent.
 * ───────────────────────────────────────────────────────── */

export type SourceRef = { name: string; domain: string; href: string };

const AVATAR_TONES = [
  "bg-accent-cir",
  "bg-success",
  "bg-warning",
  "bg-danger",
  "bg-streaming",
];

/** Domain-keyed tone so a domain always wears the same color. */
function LetterAvatar({ domain }: { domain: string }) {
  let hash = 0;
  for (let i = 0; i < domain.length; i++) hash = (hash * 31 + domain.charCodeAt(i)) | 0;
  const tone = AVATAR_TONES[Math.abs(hash) % AVATAR_TONES.length];
  return (
    <span
      className={`flex size-3.5 shrink-0 items-center justify-center rounded-full ${tone} font-mono text-[8px] font-semibold text-white`}
    >
      {(domain[0] ?? "?").toUpperCase()}
    </span>
  );
}

function hostOf(url: string): string {
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url.slice(0, 40);
  }
}

/** Sources from the turn's websearch/webfetch tool parts, deduped by URL. */
export function sourcesFromParts(parts: Part[]): SourceRef[] {
  const out: SourceRef[] = [];
  const seen = new Set<string>();
  const push = (url: string, name?: string) => {
    if (!url || seen.has(url)) return;
    seen.add(url);
    out.push({ name: name || hostOf(url), domain: hostOf(url), href: url });
  };
  for (const p of parts) {
    if (p.type !== "tool") continue;
    const input = (p.state?.input ?? {}) as Record<string, unknown>;
    const meta = (p.state?.metadata ?? {}) as Record<string, unknown>;
    if (Array.isArray(meta.sources)) {
      for (const s of meta.sources as Record<string, unknown>[]) {
        if (s && typeof s.url === "string") push(s.url, typeof s.title === "string" ? s.title : undefined);
      }
    }
    if (typeof input.url === "string") push(input.url);
    // websearch queries carry no visited URL — they stay in the ThinkingState
    // trace, not fabricated into the sources list.
  }
  return out.slice(0, 10);
}

export const AssistantText = memo(function AssistantText({
  text,
  partKey,
  streaming,
  sources,
  showActions,
}: {
  text: string;
  /** stable part id — restarts the reveal when a new part starts */
  partKey: string;
  /** the part is still receiving content — reveal + caret */
  streaming: boolean;
  sources: SourceRef[];
  /** actions row shows once the text settles (last text part of the turn) */
  showActions: boolean;
}) {
  const shown = useSmoothText(text, streaming, partKey);
  const [copied, setCopied] = useState(false);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const done = !streaming;
  const body = useMemo(() => (streaming ? shown : text), [streaming, shown, text]);

  return (
    <div className="w-full">
      <MarkdownView text={body + (streaming && body ? " ▍" : "")} />

      {/* actions row — appears when the text settles */}
      {showActions && done && (
        <div
          className="mt-1 flex items-center gap-0.5 transition-opacity duration-300"
          style={{ animation: "fade-in 300ms ease-out both" }}
        >
          <button
            type="button"
            aria-label="Copy"
            className="flex size-6 items-center justify-center rounded-[6px] text-text-tertiary
              transition-colors duration-100 hover:bg-bg-hover hover:text-text-secondary"
            onClick={() => {
              void navigator.clipboard.writeText(text).then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 2000);
              });
            }}
          >
            {copied ? (
              <Check className="size-3.5 text-success" />
            ) : (
              <Copy className="size-3.5" />
            )}
          </button>

          {sources.length > 0 && (
            <button
              type="button"
              aria-expanded={sourcesOpen}
              onClick={() => setSourcesOpen((current) => !current)}
              className="ml-1 flex items-center gap-1.5 rounded-[6px] px-1 py-0.5 text-left transition-colors duration-150 hover:bg-bg-hover"
            >
              <span className="flex -space-x-1">
                {sources.slice(0, 4).map((s) => (
                  <span key={s.href} className="rounded-full ring-2 ring-bg-main">
                    <LetterAvatar domain={s.domain} />
                  </span>
                ))}
              </span>
              <span className="text-[12px] text-text-secondary">
                {sources.length} source{sources.length === 1 ? "" : "s"}
              </span>
            </button>
          )}
        </div>
      )}

      {/* expandable sources list */}
      {sources.length > 0 && (
        <div
          className="grid transition-[grid-template-rows,opacity] duration-300"
          style={{
            gridTemplateRows: done && sourcesOpen ? "1fr" : "0fr",
            opacity: done && sourcesOpen ? 1 : 0,
            transitionTimingFunction: "cubic-bezier(0.23, 1, 0.32, 1)",
          }}
        >
          <div className="overflow-hidden">
            <div className="mt-1 flex flex-col rounded-[10px] border border-border bg-bg-code p-1">
              {sources.map((s) => (
                <a
                  key={s.href}
                  href={s.href}
                  target="_blank"
                  rel="noreferrer"
                  className="flex items-center gap-2 rounded-[6px] px-1.5 py-1 text-[12px] text-text-secondary transition-colors duration-150 hover:bg-bg-hover hover:text-text-primary"
                >
                  <LetterAvatar domain={s.domain} />
                  <Globe className="size-3 shrink-0 text-text-tertiary" />
                  <span className="animated-underline truncate">{s.name}</span>
                  <span className="ml-auto shrink-0 font-mono text-[10.5px] text-text-tertiary">
                    {s.domain}
                  </span>
                </a>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
