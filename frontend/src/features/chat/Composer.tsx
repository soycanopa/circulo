/**
 * Composer per docs/ux.md §7: pinned bottom, auto-grow, permission and
 * question cards floating directly above, model/agent pickers, Send ↔ Stop
 * swap.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowUp,
  Check,
  ChevronDown,
  HelpCircle,
  Pencil,
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
import type {
  FormInfo,
  FormField,
  ModelInfo,
  PermissionRequest,
} from "@/lib/agent/protocol";

/** Shared floating-card elevation (matches the composer box shadow). */
const cardFloat =
  "rounded-xl border bg-bg-popover p-3 text-[13px] [box-shadow:#0E0E0E59_0px_8px_24px]";

function PermissionCard({ perm }: { perm: PermissionRequest }) {
  const replyPermission = useAppStore((s) => s.replyPermission);
  const command =
    perm.metadata && typeof perm.metadata === "object"
      ? String((perm.metadata as Record<string, unknown>).command ?? "")
      : "";

  return (
    <div className={cn(cardFloat, "border-warning/50")}>
      <div className="flex items-center gap-2 font-medium">
        <AlertTriangle className="size-4 text-warning" />
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

/** Floating card for a pending form (question tool): option chips plus an
 *  optional custom answer per field; one Answer action submits them all. */
function QuestionCard({ form }: { form: FormInfo }) {
  const replyForm = useAppStore((s) => s.replyForm);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const complete = form.fields.every((f) => (answers[f.key] ?? "").trim() !== "");
  const set = (key: string, value: string) =>
    setAnswers((prev) => ({ ...prev, [key]: value }));

  return (
    <div className={cn(cardFloat, "border-accent-cir/50")}>
      <div className="flex items-center gap-2 font-medium">
        <HelpCircle className="size-4 text-accent-cir" />
        <span>{form.title || "Question"}</span>
      </div>
      {form.fields.map((field) => (
        <QuestionField
          key={field.key}
          field={field}
          value={answers[field.key] ?? ""}
          onChange={(v) => set(field.key, v)}
        />
      ))}
      <div className="mt-2.5 flex justify-end">
        <Button
          size="sm"
          disabled={!complete}
          onClick={() => void replyForm(form.id, answers)}
        >
          Answer
        </Button>
      </div>
    </div>
  );
}

function QuestionField({
  field,
  value,
  onChange,
}: {
  field: FormField;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="mt-2.5 first:mt-2">
      {field.description && (
        <div data-selectable className="break-words text-[13px]">
          {field.description}
        </div>
      )}
      {field.title && field.title !== field.description && (
        <div className="mt-0.5 text-[11.5px] text-text-tertiary">{field.title}</div>
      )}
      {field.options && field.options.length > 0 && (
        <div className="mt-1.5 flex flex-wrap gap-1.5">
          {field.options.map((o) => (
            <button
              key={o.value}
              type="button"
              title={o.description}
              onClick={() => onChange(o.value)}
              className={cn(
                "rounded-full border px-2.5 py-1 text-[12px] transition-colors duration-150",
                value === o.value
                  ? "border-accent-cir bg-accent-cir/15 text-text-primary"
                  : "border-border text-text-secondary hover:border-border-strong hover:bg-bg-hover",
              )}
            >
              {o.label}
            </button>
          ))}
        </div>
      )}
      {(field.custom || !field.options || field.options.length === 0) && (
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.options?.length ? "Custom answer…" : "Your answer…"}
          className="mt-1.5 w-full rounded-md border border-border bg-bg-code px-2.5 py-1.5 text-[12.5px] outline-none placeholder:text-text-tertiary focus:border-ring"
        />
      )}
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
  const selectedVariant = useAppStore((s) => s.selectedVariant);
  const setVariant = useAppStore((s) => s.setSelectedVariant);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const selectedRef = useRef<HTMLButtonElement>(null);

  const current = models.find((m) => `${m.provider}:${m.id}` === selectedModel);
  const q = query.trim().toLowerCase();

  // One flat list sectioned by internal provider (owner spec); opencode
  // first, then alphabetical. The rail holds agent-level tabs (future
  // providers); internal providers are sections, never tabs.
  const sections = useMemo(() => {
    const seen = new Set(models.map((m) => m.provider));
    const order = [...seen].sort((a, b) => {
      if (a === "opencode") return -1;
      if (b === "opencode") return 1;
      return a.localeCompare(b);
    });
    return order
      .map((provider) => ({
        provider,
        models: models.filter(
          (m) =>
            m.provider === provider &&
            (!q ||
              (m.name || m.id).toLowerCase().includes(q) ||
              m.id.toLowerCase().includes(q)),
        ),
      }))
      .filter((g) => g.models.length > 0);
  }, [models, q]);

  // Opening lands on the selected model's section.
  useEffect(() => {
    if (open) queueMicrotask(() => selectedRef.current?.scrollIntoView({ block: "nearest" }));
  }, [open]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {/* div (not button): keeps layout; selection happens in the popover. */}
        <div className={cn("cursor-pointer select-none", chipBtn)} data-slot="model-chip">
          <span className="max-w-40 truncate">{current?.name || current?.id || "Model"}</span>
          {selectedVariant && (
            <span
              className={cn(
                "rounded-full border px-1 py-px text-xs leading-[14px] font-medium",
                variantTagClass(selectedVariant),
              )}
            >
              {selectedVariant}
            </span>
          )}
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </div>
      </PopoverTrigger>
      <PopoverContent align="start" side="top" sideOffset={8} className="h-[320px] w-[344px]">
        <div className="flex min-h-0 flex-1">
          {/* Provider tabs — left rail. Today only opencode exists; future
              agent providers get their own tab here (owner decision). */}
          <div className="flex w-[44px] shrink-0 flex-col gap-0.5 border-r border-border p-1">
            <button
              type="button"
              title="opencode"
              aria-label="opencode"
              className="flex items-center justify-center rounded-md bg-bg-hover py-2"
            >
              <ProviderIcon provider="opencode" size={12} />
            </button>
          </div>
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
              {sections.map((g) => (
                <div key={g.provider} className="mb-1">
                  <div className="flex items-center gap-1.5 px-2 pb-1 pt-2 text-[11px] font-medium uppercase leading-[14px] tracking-wider text-text-tertiary">
                    <ProviderIcon provider={g.provider} size={10} />
                    {g.provider}
                  </div>
                  {g.models.map((m) => {
                    const key = `${m.provider}:${m.id}`;
                    const selected = key === selectedModel;
                    const hasVariants = Boolean(m.reasoning && m.variants?.length);
                    return (
                      <div key={key} className="flex items-center gap-1">
                        <button
                          ref={selected ? selectedRef : undefined}
                          type="button"
                          onClick={() => {
                            setSelected(key);
                            setOpen(false);
                          }}
                          className={cn(
                            "flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm/tight",
                            selected
                              ? "bg-bg-hover text-text-primary"
                              : "text-text-primary hover:bg-bg-hover/60",
                          )}
                        >
                          <span className="min-w-0 truncate">{m.name || m.id}</span>
                          {selected && (
                            <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
                          )}
                        </button>
                        {hasVariants && (
                          <ModelVariantsPopover
                            model={m}
                            selected={selected}
                            selectedVariant={selectedVariant}
                            onPick={(variant) => {
                              setSelected(key);
                              setVariant(variant);
                              setOpen(false);
                            }}
                          />
                        )}
                      </div>
                    );
                  })}
                </div>
              ))}
              {sections.length === 0 && (
                <div className="px-2 py-1.5 text-xs text-text-tertiary">No models found</div>
              )}
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

/** Side popover to pick a model's reasoning effort (Circulo subpopover). */
function ModelVariantsPopover({
  model,
  selected,
  selectedVariant,
  onPick,
}: {
  model: ModelInfo;
  selected: boolean;
  selectedVariant: string;
  onPick: (variant: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const option =
    "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm/tight";
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          title="Reasoning effort"
          aria-label={`Reasoning effort of ${model.name || model.id}`}
          className={cn(
            "shrink-0 rounded-md p-1 text-text-tertiary hover:bg-bg-hover/60",
            open && "bg-bg-hover text-text-primary",
          )}
          onClick={(e) => e.stopPropagation()}
        >
          <Pencil className="size-3" strokeWidth={2} />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" side="right" sideOffset={4} className="w-[164px] p-1.5">
        <div className="px-2 py-1 text-xs leading-[14px] text-text-tertiary">Reasoning effort</div>
        <button
          type="button"
          onClick={() => onPick("")}
          className={cn(option, !selected || selectedVariant === "" ? "bg-bg-hover text-text-primary" : "hover:bg-bg-hover/60")}
        >
          None
          {(!selected || selectedVariant === "") && (
            <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />
          )}
        </button>
        {model.variants?.map((v) => (
          <button
            key={v}
            type="button"
            onClick={() => onPick(v)}
            className={cn(option, selected && selectedVariant === v ? "bg-bg-hover text-text-primary" : "hover:bg-bg-hover/60")}
          >
            {v}
            {selected && selectedVariant === v && (
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
        {session && (session.permissions.length > 0 || session.forms.length > 0) && (
          <div className="space-y-2">
            {session.permissions.map((p) => (
              <PermissionCard key={p.id} perm={p} />
            ))}
            {session.forms.map((f) => (
              <QuestionCard key={f.id} form={f} />
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
