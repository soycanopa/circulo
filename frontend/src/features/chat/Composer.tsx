/**
 * Composer per docs/ux.md §7: pinned bottom, auto-grow, permission cards
 * stacked directly above, model/agent pickers, Send ↔ Stop swap.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowUp,
  Check,
  ChevronDown,
  Search,
  Square,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { ProviderIcon } from "@/components/ProviderIcon";
import { useAppStore } from "@/lib/agent/store";
import { cn } from "@/lib/utils";
import type { PermissionRequest } from "@/lib/agent/protocol";

function PermissionCard({ perm }: { perm: PermissionRequest }) {
  const replyPermission = useAppStore((s) => s.replyPermission);
  const command =
    perm.metadata && typeof perm.metadata === "object"
      ? String((perm.metadata as Record<string, unknown>).command ?? "")
      : "";

  return (
    <div className="rounded-xl border border-amber-500/50 bg-amber-500/5 p-3 text-[13px]">
      <div className="flex items-center gap-2 font-medium">
        <AlertTriangle className="size-4 text-amber-500" />
        <span>Permission · {perm.kind ?? "action"}</span>
      </div>
      <div data-selectable className="mt-1 break-words font-mono text-[12px] text-muted-foreground">
        {perm.title || command || perm.callID || perm.id}
      </div>
      <div className="mt-2.5 flex gap-2">
        <Button
          size="sm"
          onClick={() => void replyPermission(perm.id, "once")}
        >
          Allow once
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => void replyPermission(perm.id, "always")}
        >
          Always allow
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="text-red-600 hover:text-red-600 dark:text-red-400"
          onClick={() => void replyPermission(perm.id, "reject")}
        >
          Deny
        </Button>
      </div>
    </div>
  );
}

/** Tag pill per the Circulo reasoning-tags spec (color keyed by effort). */
function variantTagClass(v: string): string {
  switch (v) {
    case "low":
      return "bg-[#243D2E] border-diff-add text-success";
    case "medium":
      return "bg-[#47381A] border-[#8C661F] text-[#EBB847]";
    case "high":
      return "bg-[#522014] border-[#9E5214] text-[#FA9E47]";
    case "max":
    case "xhigh":
      return "bg-[#472438] border-[#733366] text-danger";
    default:
      return "bg-bg-hover border-border text-text-secondary";
  }
}

const chipBtn =
  "flex items-center gap-[6px] rounded-md px-2 py-1 text-sm/tight text-text-secondary hover:bg-muted";

function ModelPicker() {
  const models = useAppStore((s) => s.metaModels);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const setSelected = useAppStore((s) => s.setSelectedModel);
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const current = models.find((m) => `${m.provider}:${m.id}` === selectedModel);

  // Provider tabs; "opencode" first (owner spec), then alphabetical.
  const providers = useMemo(() => {
    const seen = new Set(models.map((m) => m.provider));
    return [...seen].sort((a, b) => {
      if (a === "opencode") return -1;
      if (b === "opencode") return 1;
      return a.localeCompare(b);
    });
  }, [models]);

  // Open on the selected model's provider tab; opencode-first order only
  // applies when nothing is selected yet.
  const currentProvider = current?.provider;
  const active = tab ?? (currentProvider && providers.includes(currentProvider) ? currentProvider : providers[0]) ?? "";
  const q = query.trim().toLowerCase();
  const filtered = models.filter(
    (m) =>
      m.provider === active &&
      (!q || (m.name || m.id).toLowerCase().includes(q) || m.id.toLowerCase().includes(q)),
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {/* div (not button): the effort tag nested inside is its own trigger. */}
        <div className={cn("cursor-pointer select-none", chipBtn)} data-slot="model-chip">
          <span className="max-w-40 truncate">{current?.name || current?.id || "Model"}</span>
          <VariantPicker />
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </div>
      </PopoverTrigger>
      <PopoverContent align="start" side="top" sideOffset={8} className="h-[280px] w-[380px]">
        <div className="flex min-h-0 flex-1">
          {/* Provider tabs — left rail, icon only (owner spec) */}
          <div className="flex w-[56px] shrink-0 flex-col gap-0.5 border-r border-border p-1.5">
            {providers.map((p) => (
              <button
                key={p}
                type="button"
                title={p}
                aria-label={p}
                onClick={() => setTab(p)}
                className={cn(
                  "flex items-center justify-center rounded-md py-2",
                  active === p ? "bg-bg-hover" : "opacity-60 hover:bg-bg-hover/60 hover:opacity-100",
                )}
              >
                <ProviderIcon provider={p} size={12} />
              </button>
            ))}
          </div>
          {/* Models — right column with search */}
          <div className="flex min-w-0 flex-1 flex-col">
            <div className="shrink-0 border-b border-border p-2">
              <div className="flex items-center gap-2 rounded-md bg-bg-code px-2 py-1.5">
                <Search className="size-3 shrink-0 text-text-tertiary" />
                <input
                  data-selectable
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="Search models"
                  className="w-full bg-transparent text-xs text-text-primary outline-none placeholder:text-text-tertiary"
                />
              </div>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {filtered.map((m) => {
                const key = `${m.provider}:${m.id}`;
                const selected = key === selectedModel;
                return (
                  <button
                    key={key}
                    type="button"
                    onClick={() => {
                      setSelected(key);
                      setOpen(false);
                    }}
                    className={cn(
                      "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm/tight",
                      selected
                        ? "bg-bg-hover text-text-primary"
                        : "text-text-primary hover:bg-bg-hover/60",
                    )}
                  >
                    <span className="flex min-w-0 items-center gap-2">
                      <ProviderIcon provider={active} size={10} />
                      <span className="min-w-0 truncate">{m.name || m.id}</span>
                    </span>
                    {selected && (
                      <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
                    )}
                  </button>
                );
              })}
              {filtered.length === 0 && (
                <div className="px-2 py-1.5 text-xs text-text-tertiary">No models found</div>
              )}
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

/** Reasoning-effort selector: opens from the tag inside the model chip. */
function VariantPicker() {
  const models = useAppStore((s) => s.metaModels);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const selectedVariant = useAppStore((s) => s.selectedVariant);
  const setVariant = useAppStore((s) => s.setSelectedVariant);
  const [open, setOpen] = useState(false);

  const current = models.find((m) => `${m.provider}:${m.id}` === selectedModel);
  if (!current?.reasoning || !current.variants?.length) return null;
  // The tag is always rendered for capable models: it is the trigger (gray
  // "none" per the design when no effort is set).
  const tag = (
    <span
      className={cn(
        "rounded-full border px-1 py-px text-xs leading-[14px] font-medium",
        selectedVariant
          ? variantTagClass(selectedVariant)
          : "bg-bg-hover border-border text-text-secondary",
      )}
    >
      {selectedVariant || "none"}
    </span>
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label="Reasoning effort"
          className="flex items-center"
          onClick={(e) => e.stopPropagation()}
        >
          {tag}
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" side="top" sideOffset={8} className="w-[164px] p-1.5">
        <div className="px-2 py-1 text-xs leading-[14px] text-text-tertiary">Reasoning effort</div>
        <button
          type="button"
          onClick={() => {
            setVariant("");
            setOpen(false);
          }}
          className={cn(
            "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm/tight",
            !selectedVariant ? "bg-bg-hover text-text-primary" : "hover:bg-bg-hover/60",
          )}
        >
          None
          {!selectedVariant && (
            <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
          )}
        </button>
        {current.variants.map((v) => (
          <button
            key={v}
            type="button"
            onClick={() => {
              setVariant(v);
              setOpen(false);
            }}
            className={cn(
              "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm/tight",
              selectedVariant === v ? "bg-bg-hover text-text-primary" : "hover:bg-bg-hover/60",
            )}
          >
            {v}
            {selectedVariant === v && (
              <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
            )}
          </button>
        ))}
      </PopoverContent>
    </Popover>
  );
}

function AgentPicker() {
  const agents = useAppStore((s) => s.metaAgents);
  const selected = useAppStore((s) => s.selectedAgent);
  const setSelected = useAppStore((s) => s.setSelectedAgent);
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button type="button" className={chipBtn}>
          Mode
          {selected && (
            <span className="max-w-24 truncate text-text-primary">{selected}</span>
          )}
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" side="top" sideOffset={8} className="w-[240px] p-1.5">
        <div className="px-2 py-1 text-xs leading-[14px] text-text-tertiary">Mode</div>
        {agents.map((a) => (
          <button
            key={a.name}
            type="button"
            onClick={() => {
              setSelected(a.name);
              setOpen(false);
            }}
            className={cn(
              "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm/tight",
              selected === a.name ? "bg-bg-hover text-text-primary" : "hover:bg-bg-hover/60",
            )}
          >
            <span className="min-w-0 flex-1 truncate">{a.name}</span>
            {a.mode === "subagent" && (
              <span className="shrink-0 text-[11px] text-text-tertiary">subagent</span>
            )}
            {selected === a.name && (
              <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
            )}
          </button>
        ))}
        {agents.length === 0 && (
          <div className="px-2 py-1.5 text-xs text-text-tertiary">No modes available</div>
        )}
      </PopoverContent>
    </Popover>
  );
}

export function Composer() {
  const [text, setText] = useState("");
  const send = useAppStore((s) => s.sendPrompt);
  const abort = useAppStore((s) => s.abort);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const chat = useAppStore((s) => s.chat);
  const taRef = useRef<HTMLTextAreaElement>(null);

  const session =
    activeProjectId && activeSessionId ? chat.sessions[activeSessionId] : undefined;
  const busy = session?.status === "busy" || session?.status === "retry";

  // Auto-grow up to ~6 lines.
  useEffect(() => {
    const el = taRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 168)}px`;
  }, [text]);

  const submit = () => {
    const t = text.trim();
    if (!t || !activeProjectId) return;
    setText("");
    void send(t);
  };

  return (
    <div className="shrink-0 px-6 pb-8 pt-2">
      <div className="mx-auto max-w-3xl space-y-2">
        {session && session.permissions.length > 0 && (
          <div className="space-y-2">
            {session.permissions.map((p) => (
              <PermissionCard key={p.id} perm={p} />
            ))}
          </div>
        )}
        <div className="mx-auto flex w-full max-w-[768px] flex-col rounded-xl border border-border-strong bg-bg-main [box-shadow:#0E0E0E59_0px_8px_24px] focus-within:border-ring">
          <textarea
            ref={taRef}
            id="composer"
            data-selectable
            rows={1}
            value={text}
            placeholder={
              activeSessionId
                ? "Ask a follow-up…"
                : "Write anything — Circulo does the rest"
            }
            disabled={!activeProjectId}
            className="w-full resize-none bg-transparent px-4 pt-4 pb-2 text-md/relaxed text-text-primary outline-none placeholder:text-text-tertiary disabled:cursor-not-allowed"
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                submit();
              }
            }}
          />
          <div className="flex items-end px-[10px] pb-[10px] pt-2">
            <ModelPicker />
            <AgentPicker />
            <span className="flex-1" />
            {busy ? (
              <button
                type="button"
                aria-label="Stop"
                className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md bg-track-off hover:brightness-110"
                onClick={() => void abort()}
              >
                <Square className="size-3 fill-current text-text-tertiary" />
              </button>
            ) : (
              <button
                type="button"
                aria-label="Send"
                disabled={!text.trim() || !activeProjectId}
                className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md bg-track-off hover:brightness-110 disabled:opacity-40"
                onClick={submit}
              >
                <ArrowUp className="size-[13px] text-text-tertiary" strokeWidth={2.2} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
