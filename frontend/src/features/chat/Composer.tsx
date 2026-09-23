/**
 * Composer per docs/ux.md §7: pinned bottom, auto-grow, permission and
 * question cards floating directly above, model/agent pickers, Send ↔ Stop
 * swap.
 */

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowUp,
  Check,
  ChevronDown,
  FolderGit2,
  Folder,
  GitBranch,
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
import { ContextGauge } from "@/features/chat/parts/ContextGauge";
import { useAppStore } from "@/lib/agent/store";
import { cn } from "@/lib/utils";
import type {
  FormInfo,
  ModelInfo,
  PermissionRequest,
  ProjectVcs,
} from "@/lib/agent/protocol";
import { api } from "@/lib/agent/api";

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

/** One question of the approval flow, mapped from a wire form field. */
type ApprovalQuestion = {
  key: string;
  q: string;
  sub?: string;
  check: boolean;
  options: { label: string; value: string }[];
  custom: boolean;
};

/* ─────────────────────────────────────────────────────────
 * QUESTION CARD (owner's ApprovalCard design)
 * One wire field at a time; the stack slides vertically and the
 * card height animates; the step counter rolls like an odometer;
 * radio choices auto-advance, multi-select waits. Custom answers
 * ride the same reply ({key: value | values}).
 * ───────────────────────────────────────────────────────── */

const ROLL_MS = 400;
const SLIDE = "360ms cubic-bezier(0.22, 1, 0.36, 1)";

function RollingDigits({ value }: { value: string }) {
  const prevRef = useRef(value);
  const [oldVal, setOldVal] = useState(value);
  const [chars, setChars] = useState(value);
  const [rolling, setRolling] = useState(false);
  const [shifted, setShifted] = useState(false);
  const [dir, setDir] = useState<"up" | "down">("up");

  useEffect(() => {
    if (prevRef.current === value) return;
    const from = prevRef.current;
    prevRef.current = value;
    const fromN = parseInt(from, 10);
    const toN = parseInt(value, 10);
    setDir(Number.isFinite(fromN) && Number.isFinite(toN) && toN < fromN ? "down" : "up");
    setOldVal(from);
    setChars(value);
    setRolling(true);
    setShifted(false);

    let raf2 = 0;
    const raf1 = requestAnimationFrame(() => {
      raf2 = requestAnimationFrame(() => setShifted(true));
    });
    const done = setTimeout(() => {
      setRolling(false);
      setOldVal(value);
      setShifted(false);
    }, ROLL_MS);

    return () => {
      cancelAnimationFrame(raf1);
      cancelAnimationFrame(raf2);
      clearTimeout(done);
    };
  }, [value]);

  const shown = rolling ? chars : oldVal;

  return (
    <>
      {Array.from({ length: shown.length }, (_, i) => {
        const o = oldVal[i] ?? "";
        const n = shown[i] ?? "";
        if (!rolling || o === n) {
          return <span key={`${i}-${n}`}>{n}</span>;
        }
        const top = dir === "down" ? n : o;
        const bottom = dir === "down" ? o : n;
        const restY = dir === "down" ? "0" : "-1em";
        const startY = dir === "down" ? "-1em" : "0";
        return (
          <span
            key={`${i}-${o}-${n}-${dir}`}
            style={{ display: "inline-block", position: "relative", overflow: "hidden", height: "1em", lineHeight: "1em", verticalAlign: "-0.05em" }}
          >
            <span
              style={{
                display: "flex",
                flexDirection: "column",
                transition: "transform 350ms cubic-bezier(0.4, 0, 0.2, 1)",
                transform: `translateY(${shifted ? restY : startY})`,
              }}
            >
              <span style={{ height: "1em", lineHeight: "1em" }}>{top}</span>
              <span style={{ height: "1em", lineHeight: "1em" }}>{bottom}</span>
            </span>
          </span>
        );
      })}
    </>
  );
}

function QuestionCard({ form }: { form: FormInfo }) {
  const replyForm = useAppStore((s) => s.replyForm);
  const questions: ApprovalQuestion[] = useMemo(
    () =>
      form.fields.map((f) => ({
        key: f.key,
        q: f.description || f.title || "Question",
        sub: f.title && f.title !== f.description ? f.title : undefined,
        check: f.type === "multiselect",
        options: (f.options ?? []).map((o) => ({ label: o.label, value: o.value })),
        custom: f.custom ?? !f.options?.length,
      })),
    [form],
  );

  const [qi, setQi] = useState(0);
  const [answers, setAnswers] = useState<Record<number, number[]>>({});
  const [custom, setCustom] = useState<Record<number, string>>({});
  const [sent, setSent] = useState(false);
  const [open, setOpen] = useState(true);

  const advanceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const questionRefs = useRef<(HTMLDivElement | null)[]>([]);
  const measured = useRef(false);
  const [viewportH, setViewportH] = useState<number | undefined>(undefined);
  const [trackY, setTrackY] = useState(0);
  const [animate, setAnimate] = useState(false);
  const [ready, setReady] = useState(false);

  const last = qi === questions.length - 1;
  const selected = answers[qi] ?? [];
  const hasAnswer = selected.length > 0 || Boolean(custom[qi]?.trim());

  const sync = (withAnim: boolean) => {
    const item = questionRefs.current[qi];
    if (!item) return;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    setViewportH(item.offsetHeight);
    setTrackY(item.offsetTop);
    setAnimate(withAnim && !reduce);
  };

  useLayoutEffect(() => {
    const withAnim = measured.current;
    measured.current = true;
    sync(withAnim);
    setReady(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [qi, answers, custom, open, sent]);

  useEffect(() => {
    const id = requestAnimationFrame(() => sync(measured.current));
    return () => cancelAnimationFrame(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [qi]);

  useEffect(() => () => { if (advanceTimer.current) clearTimeout(advanceTimer.current); }, []);

  const goTo = (next: number) => {
    if (advanceTimer.current) clearTimeout(advanceTimer.current);
    setQi(Math.min(Math.max(next, 0), questions.length - 1));
  };

  const send = () => {
    if (advanceTimer.current) clearTimeout(advanceTimer.current);
    setSent(true);
    // Map UI picks onto the wire answer: radio → value or custom text;
    // check → selected values. Unanswered fields are simply omitted.
    const answer: Record<string, string | string[]> = {};
    questions.forEach((question, qIdx) => {
      const picked = answers[qIdx] ?? [];
      const text = custom[qIdx]?.trim();
      if (question.check) {
        if (picked.length > 0) answer[question.key] = picked.map((i) => question.options[i].value);
      } else if (picked.length > 0 && !text) {
        answer[question.key] = question.options[picked[0]]?.value ?? text ?? "";
      } else if (text) {
        answer[question.key] = text;
      }
    });
    void replyForm(form.id, answer);
  };

  const advance = () => {
    if (last) send();
    else goTo(qi + 1);
  };

  const toggle = (index: number) => {
    const check = questions[qi].check;
    setAnswers((current) => {
      const picked = current[qi] ?? [];
      const next = check
        ? picked.includes(index)
          ? picked.filter((item) => item !== index)
          : [...picked, index]
        : [index];
      return { ...current, [qi]: next };
    });
    if (!check) {
      setCustom((current) => ({ ...current, [qi]: "" }));
      if (advanceTimer.current) clearTimeout(advanceTimer.current);
      advanceTimer.current = setTimeout(() => {
        if (last) send();
        else setQi((current) => Math.min(questions.length - 1, current + 1));
      }, 480);
    }
  };

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={cn(cardFloat, "w-fit border-border text-[12.5px] font-medium text-text-primary transition-colors duration-150 hover:bg-bg-hover")}
      >
        Answer question
      </button>
    );
  }

  if (sent) {
    return (
      <div className="flex w-full items-center gap-3" style={{ animation: "pop-in 260ms cubic-bezier(0.23,1,0.32,1) both" }}>
        <span className="inline-flex items-center gap-1.5 rounded-full bg-success/10 py-1 pr-2.5 pl-1 text-[12.5px] font-medium text-success">
          <span className="flex size-4 items-center justify-center rounded-full bg-success text-white">
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6L9 17l-5-5" /></svg>
          </span>
          Answers sent
        </span>
      </div>
    );
  }

  return (
    <div className="w-full">
      <div
        className="relative overflow-hidden rounded-xl border border-border bg-bg-popover [box-shadow:#0E0E0E59_0px_8px_24px]"
        style={{ animation: "fade-up 380ms cubic-bezier(0.23,1,0.32,1) both" }}
      >
        <button
          type="button"
          aria-label="Dismiss"
          onClick={() => setOpen(false)}
          className="absolute right-2.5 top-2.5 z-10 flex size-6 items-center justify-center rounded-md text-text-tertiary transition-colors duration-100 hover:bg-bg-hover hover:text-text-primary"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden><path d="M18 6L6 18M6 6l12 12" /></svg>
        </button>
        <div className="p-3.5">
          {/* the question itself is the heading */}
          <div
            className="overflow-hidden"
            style={{ height: viewportH, transition: animate ? `height ${SLIDE}` : undefined }}
            aria-live="polite"
          >
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 26,
                transform: `translate3d(0, ${-trackY}px, 0)`,
                transition: animate ? `transform ${SLIDE}` : undefined,
                willChange: "transform",
              }}
            >
              {questions.map((question, qIdx) => {
                const active = qIdx === qi;
                // Before the first measure, mount only the active question so
                // the card opens at its real height, not the full stack.
                if (!ready && !active) return null;
                const picked = answers[qIdx] ?? [];
                const questionStyle = {
                  opacity: active ? 1 : 0,
                  transition: animate ? `opacity ${SLIDE}` : undefined,
                  pointerEvents: active ? undefined : "none" as const,
                };
                return (
                  <div
                    key={question.key}
                    ref={(el) => { questionRefs.current[qIdx] = el; }}
                    aria-hidden={active ? undefined : true}
                    style={questionStyle}
                  >
                    <div className="pr-7 text-[14px] font-medium text-text-primary">{question.q}</div>
                    {question.sub && (
                      <div className="mt-0.5 text-[11.5px] text-text-tertiary">{question.sub}</div>
                    )}
                    <div className="mt-2.5 flex flex-col gap-1">
                      {question.options.map((option, i) => {
                        const on = picked.includes(i);
                        return (
                          <button
                            key={option.value}
                            type="button"
                            aria-pressed={on}
                            tabIndex={active ? 0 : -1}
                            onClick={() => { if (active) toggle(i); }}
                            className="relative z-10 flex items-center gap-1.5 rounded-md pl-1 pr-2 py-1 text-left transition-colors duration-100 hover:bg-bg-hover"
                          >
                            <span
                              className={`flex size-4 shrink-0 items-center justify-center transition-colors duration-200
                                ${question.check ? "rounded-[5px]" : "rounded-full"}
                                ${on ? "bg-text-primary text-bg-popover" : "text-transparent [box-shadow:inset_0_0_0_1.5px_var(--border-strong)]"}`}
                            >
                              {question.check ? (
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6L9 17l-5-5" /></svg>
                              ) : (
                                <span className="size-1.5 rounded-full bg-bg-popover transition-transform duration-200" style={{ transform: on ? "scale(1)" : "scale(0)" }} />
                              )}
                            </span>
                            <span className={`text-[13px] leading-none transition-colors duration-200 ${on ? "text-text-primary" : "text-text-secondary"}`}>
                              {option.label}
                            </span>
                          </button>
                        );
                      })}
                      {question.custom && (
                        <label className="relative z-10 flex items-center gap-1.5 rounded-md pl-1 pr-2 py-1 transition-colors duration-100 hover:bg-bg-hover">
                          <input
                            value={custom[qi] ?? ""}
                            tabIndex={active ? 0 : -1}
                            onChange={(event) => {
                              if (!active) return;
                              setCustom((current) => ({ ...current, [qIdx]: event.target.value }));
                              if (!question.check) setAnswers((current) => ({ ...current, [qIdx]: [] }));
                            }}
                            onKeyDown={(event) => {
                              if (event.key === "Enter" && hasAnswer) {
                                event.preventDefault();
                                advance();
                              }
                            }}
                            placeholder="Something else…"
                            aria-label="Custom answer"
                            className="min-w-0 flex-1 bg-transparent pl-1.5 text-[13px] text-text-primary outline-none placeholder:text-text-tertiary"
                          />
                        </label>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        {/* footer — step nav (rolling counter) + pill actions */}
        <div className="flex items-center justify-between gap-3 border-t border-border px-3.5 py-2">
          <div className="flex items-center gap-1 text-text-tertiary">
            <button
              type="button"
              aria-label="Previous question"
              disabled={qi <= 0}
              onClick={() => goTo(qi - 1)}
              className="flex size-[18px] items-center justify-center rounded-[5px] transition-colors duration-100 enabled:hover:text-text-primary disabled:opacity-30"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden><path d="M18 15l-6-6-6 6" /></svg>
            </button>
            <span className="inline-flex items-center text-[12px] font-medium tabular-nums text-text-tertiary" style={{ letterSpacing: "-0.1px", lineHeight: 1 }}>
              <RollingDigits value={`${qi + 1} / ${questions.length}`} />
            </span>
            <button
              type="button"
              aria-label="Next question"
              disabled={last}
              onClick={() => goTo(qi + 1)}
              className="flex size-[18px] items-center justify-center rounded-[5px] transition-colors duration-100 enabled:hover:text-text-primary disabled:opacity-30"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden><path d="M6 9l6 6 6-6" /></svg>
            </button>
          </div>

          <div className="flex items-center gap-1.5">
            <Button variant="ghost" size="sm" onClick={() => (last ? setOpen(false) : goTo(qi + 1))}>
              Skip
            </Button>
            <Button size="sm" disabled={!hasAnswer} onClick={advance}>
              {last ? "Send" : "Continue"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

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
  const projects = useAppStore((s) => s.projects);
  const metaByProject = useAppStore((s) => s.metaByProject);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const selectedModelProject = useAppStore((s) => s.selectedModelProject);
  const setSelected = useAppStore((s) => s.setSelectedModel);
  const selectedVariant = useAppStore((s) => s.selectedVariant);
  const setVariant = useAppStore((s) => s.setSelectedVariant);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [tabOverride, setTabOverride] = useState<string | null>(null);
  const selectedRef = useRef<HTMLButtonElement>(null);

  // One tab per project that can serve models (backend per project): the
  // picker aggregates every provider so opencode and omp are both reachable.
  const tabs = useMemo(() => {
    const live = projects.filter(
      (p) => p.status === "running" || p.status === "starting" || metaByProject[p.id],
    );
    return live
      .map((p) => ({ project: p, meta: metaByProject[p.id] ?? { models: [], defaultKey: "" } }))
      .sort((a, b) => {
        if (a.project.id === activeProjectId) return -1;
        if (b.project.id === activeProjectId) return 1;
        return a.project.provider.localeCompare(b.project.provider);
      });
  }, [projects, metaByProject, activeProjectId]);

  // Default to a tab that actually has models — the active project may be
  // an erroring backend that can never serve a catalog.
  const fallbackTab = tabs.find((t) => t.meta.models.length > 0)?.project.id ?? tabs[0]?.project.id ?? null;
  const effectiveTab =
    tabOverride ??
    selectedModelProject ??
    (activeProjectId && metaByProject[activeProjectId] ? activeProjectId : fallbackTab);
  const models = metaByProject[effectiveTab ?? ""]?.models ?? [];
  // The chip shows the actual selection (which may live in another tab);
  // the list below shows the tab being browsed.
  const current = metaByProject[selectedModelProject ?? effectiveTab ?? ""]?.models.find(
    (m) => `${m.provider}:${m.id}` === selectedModel,
  );
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
          {/* Provider tabs — one per project backend; picking a model from a
              tab targets that project for the next message. */}
          <div className="flex w-[44px] shrink-0 flex-col gap-0.5 border-r border-border p-1">
            {tabs.map(({ project }) => (
              <button
                key={project.id}
                type="button"
                title={`${project.provider} — ${project.path.split("/").pop()}`}
                aria-label={`${project.provider} (${project.path})`}
                onClick={() => setTabOverride(project.id)}
                className={cn(
                  "flex items-center justify-center rounded-md py-2",
                  project.id === effectiveTab ? "bg-bg-hover" : "hover:bg-bg-hover/60",
                )}
              >
                <ProviderIcon provider={project.provider} size={12} />
              </button>
            ))}
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
                      <div key={key} className="group flex items-center gap-1">
                        <button
                          ref={selected ? selectedRef : undefined}
                          type="button"
                          onClick={() => {
                            setSelected(key, effectiveTab ?? "");
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
                              setSelected(key, effectiveTab ?? "");
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
        {/* Text affordance instead of a tiny pencil: revealed on row hover
            via opacity (no layout shift, so the list never jumps/scrolls
            under the cursor), pinned while the popover is open and on
            keyboard focus. */}
        <button
          type="button"
          title="Reasoning effort"
          aria-label={`Edit reasoning effort of ${model.name || model.id}`}
          className={cn(
            "shrink-0 rounded-md px-1.5 py-1 text-[11px] leading-none text-text-tertiary opacity-0 transition-opacity hover:bg-bg-hover/60 hover:text-text-primary focus-visible:opacity-100 group-hover:opacity-100",
            open && "bg-bg-hover text-text-primary opacity-100",
          )}
          onClick={(e) => e.stopPropagation()}
        >
          Edit
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

/** Icons for the access rows (Waku pattern: lock/pencil/sparkle). */
function AccessIcon({ id, className }: { id: string; className?: string }) {
  const common = {
    className,
    width: 14,
    height: 14,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    "aria-hidden": true,
  } as const;
  if (id === "supervised") return <svg {...common}><rect x="4" y="11" width="16" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>;
  if (id === "edits") return <svg {...common}><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" /></svg>;
  return <svg {...common}><path d="M12 3l1.9 5.8a2 2 0 0 0 1.3 1.3L21 12l-5.8 1.9a2 2 0 0 0-1.3 1.3L12 21l-1.9-5.8a2 2 0 0 0-1.3-1.3L3 12l5.8-1.9a2 2 0 0 0 1.3-1.3Z" /></svg>;
}

/** One Mode row: label + value + (access rows) Waku-style subtitle. */
function ModeRow({
  label,
  value,
  active,
  onClick,
  trailing,
}: {
  label: string;
  value: string;
  active: boolean;
  onClick: () => void;
  trailing?: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={() => {
        onClick();
      }}
      className={cn(
        "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm/tight",
        active ? "bg-bg-hover text-text-primary" : "text-text-primary hover:bg-bg-hover/60",
      )}
    >
      <span className="min-w-0 flex-1 truncate">
        {label}
        <span className={cn("ml-1.5", active ? "text-text-secondary" : "text-text-tertiary")}>
          {value}
        </span>
      </span>
      {trailing}
      {active && <Check className="size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />}
    </button>
  );
}

/** Mode picker — one popover, two contexts (owner: no duplicate chips):
 *  - omp projects: the Waku-style access selector (Supervised / Auto-accept
 *    edits / Full access) with icon + title + subtitle rows.
 *  - opencode projects: the agent list (build/plan/…), same rows without
 *    subtitles (no access surface). */
function ModePicker() {
  const agents = useAppStore((s) => s.metaAgents);
  const selectedAgent = useAppStore((s) => s.selectedAgent);
  const setSelectedAgent = useAppStore((s) => s.setSelectedAgent);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const meta = useAppStore((s) => (activeProjectId ? s.metaByProject[activeProjectId] : undefined));
  const setAccess = useAppStore((s) => s.setAccess);
  const [open, setOpen] = useState(false);

  const modes = meta?.accessModes ?? [];
  const current = modes.find((m) => m.id === meta?.accessMode) ?? modes.find((m) => m.id === "full");

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button type="button" className={chipBtn} aria-expanded={open}>
          {modes.length > 0 ? (
            <>
              <AccessIcon id={current?.id ?? "full"} className="shrink-0 text-text-tertiary" />
              <span className="max-w-32 truncate">{current?.title ?? "Mode"}</span>
            </>
          ) : (
            <>
              Mode
              {selectedAgent && (
                <span className="max-w-24 truncate text-text-primary">{selectedAgent}</span>
              )}
            </>
          )}
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" side="top" sideOffset={8} className="w-[320px] p-1.5">
        {modes.length > 0 ? (
          <>
            <div className="px-2 py-1 text-xs leading-[14px] text-text-tertiary">Mode</div>
            {modes.map((m) => {
              const selected = m.id === meta?.accessMode;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => {
                    setAccess(m.id);
                    setOpen(false);
                  }}
                  className={cn(
                    "flex w-full items-start gap-2.5 rounded-md px-2 py-1.5 text-left",
                    selected ? "bg-bg-hover" : "hover:bg-bg-hover/60",
                  )}
                >
                  <AccessIcon id={m.id} className="mt-0.5 shrink-0 text-text-secondary" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm/tight font-medium text-text-primary">{m.title}</span>
                    <span className="block text-[11.5px]/[16px] text-text-tertiary">{m.description}</span>
                  </span>
                  {selected && <Check className="mt-1 size-3.5 shrink-0 text-accent-cir" strokeWidth={2} />}
                </button>
              );
            })}
          </>
        ) : (
          <>
            <div className="px-2 py-1 text-xs leading-[14px] text-text-tertiary">Mode</div>
            {agents.map((a) => (
              <ModeRow
                key={a.name}
                label={a.name}
                value={a.mode === "subagent" ? "subagent" : ""}
                active={selectedAgent === a.name}
                onClick={() => {
                  setSelectedAgent(a.name);
                  setOpen(false);
                }}
              />
            ))}
            {agents.length === 0 && (
              <div className="px-2 py-1.5 text-xs text-text-tertiary">No modes available</div>
            )}
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}

/** Project/branch strip under the composer (owner call): always visible.
 *  In an existing session the project is fixed (read-only chip) and picking
 *  a branch pins it live on that session; while creating a session the
 *  project itself is what gets selected, and the branch rides the create. */
function SessionTargetStrip({
  projectID,
  onProjectSelect,
  sessionID,
  branch,
  onBranch,
}: {
  projectID: string;
  /** absent in existing sessions: the project cannot change there */
  onProjectSelect?: (projectID: string) => void;
  /** present in existing sessions: branch picks pin live via the API */
  sessionID?: string;
  branch: string;
  onBranch: (branch: string) => void;
}) {
  const projects = useAppStore((s) => s.projects);
  const [vcs, setVcs] = useState<ProjectVcs | null | undefined>(undefined);
  const [branches, setBranches] = useState<string[]>([]);
  const [projOpen, setProjOpen] = useState(false);
  const [branchOpen, setBranchOpen] = useState(false);

  const selected = projects.find((p) => p.id === projectID);

  useEffect(() => {
    let alive = true;
    setVcs(undefined);
    setBranches([]);
    onBranch(""); // reset any pinned override when the target changes
    api
      .vcs(projectID)
      .then((v) => {
        if (!alive) return;
        setVcs(v);
        if (v.isRepo) api.branches(projectID).then((b) => alive && setBranches(b)).catch(() => undefined);
      })
      .catch(() => alive && setVcs(null));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectID, sessionID]);

  const pickBranch = (b: string) => {
    onBranch(b);
    setBranchOpen(false);
    if (sessionID) void api.setBranch(projectID, sessionID, b).catch((e) => console.error(e));
  };

  const chip =
    "inline-flex h-6 min-w-0 items-center gap-1 rounded-[6px] px-1.5 text-[12px] font-medium text-text-primary transition-colors duration-100 bg-bg-code hover:bg-bg-hover";
  const chipStatic =
    "inline-flex h-6 min-w-0 items-center gap-1 rounded-[6px] px-1.5 text-[12px] font-medium text-text-primary bg-bg-code";

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {onProjectSelect ? (
        <Popover open={projOpen} onOpenChange={setProjOpen}>
          <PopoverTrigger asChild>
            <button type="button" aria-expanded={projOpen} className={chip}>
              <Folder className="size-3 shrink-0 text-text-tertiary" />
              <span className="max-w-48 truncate">
                {selected ? selected.path.split("/").filter(Boolean).pop() : "Project"}
              </span>
              <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
            </button>
          </PopoverTrigger>
          <PopoverContent align="start" side="top" sideOffset={8} className="w-[320px] p-1">
            {projects.map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => {
                  onProjectSelect(p.id);
                  setProjOpen(false);
                }}
                className="flex w-full items-center gap-2 rounded-[6px] px-2 py-1.5 text-left transition-colors duration-100 hover:bg-bg-hover"
              >
                <FolderGit2 className="size-3.5 shrink-0 text-text-tertiary" />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[12.5px] font-medium text-text-primary">
                    {p.path.split("/").filter(Boolean).pop()}
                  </span>
                  <span className="block truncate text-[11px] text-text-tertiary">{p.path}</span>
                </span>
                {p.id === projectID && <Check className="size-3.5 shrink-0 text-text-primary" />}
              </button>
            ))}
          </PopoverContent>
        </Popover>
      ) : (
        <span className={chipStatic} title={selected?.path}>
          <FolderGit2 className="size-3 shrink-0 text-text-tertiary" />
          <span className="max-w-48 truncate">
            {selected ? selected.path.split("/").filter(Boolean).pop() : "Project"}
          </span>
        </span>
      )}

      {vcs === undefined ? null : vcs?.isRepo ? (
        <>
          <span
            className="inline-flex h-6 items-center gap-1 rounded-[6px] bg-bg-code px-1.5 text-[12px] font-medium text-text-secondary"
            title="Local git repository"
          >
            <GitBranch className="size-3 shrink-0 text-success" />
            Local repo
          </span>
          <Popover open={branchOpen} onOpenChange={setBranchOpen}>
            <PopoverTrigger asChild>
              <button type="button" aria-expanded={branchOpen} className={chip}>
                <span className="max-w-40 truncate">branch: {branch || vcs.branch || "default"}</span>
                <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
              </button>
            </PopoverTrigger>
            <PopoverContent align="start" side="top" sideOffset={8} className="max-h-[280px] w-[240px] overflow-y-auto p-1">
              {branches.map((b) => (
                <button
                  key={b}
                  type="button"
                  onClick={() => pickBranch(b)}
                  className="flex w-full items-center gap-2 rounded-[6px] px-2 py-1.5 text-left transition-colors duration-100 hover:bg-bg-hover"
                >
                  <GitBranch className="size-3 shrink-0 text-text-tertiary" />
                  <span className="min-w-0 flex-1 truncate text-[12.5px] font-medium text-text-primary">{b}</span>
                  {(branch || vcs.branch) === b && <Check className="size-3.5 shrink-0 text-text-primary" />}
                </button>
              ))}
            </PopoverContent>
          </Popover>
        </>
      ) : (
        <span className="text-[12px] text-text-tertiary">No git repository</span>
      )}
    </div>
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

  // New-session targeting (owner call): with multiple linked projects, the
  // composer picks which folder — and which branch — the session lands in.
  const projects = useAppStore((s) => s.projects);
  const isNewSession = !activeSessionId;
  const [targetProjectId, setTargetProjectId] = useState(activeProjectId ?? "");
  const [targetBranch, setTargetBranch] = useState("");
  useEffect(() => {
    setTargetBranch("");
  }, [activeSessionId]);
  // Entering new-session mode defaults the target to the last-added project
  // (owner call); the user re-picks from the unlocked strip.
  useEffect(() => {
    if (isNewSession) {
      const last = projects[projects.length - 1];
      setTargetProjectId(last?.id ?? activeProjectId ?? "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isNewSession]);

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
    const targeting =
      isNewSession && ((targetProjectId && targetProjectId !== activeProjectId) || targetBranch)
        ? { projectID: targetProjectId || activeProjectId, branch: targetBranch || undefined }
        : undefined;
    void send(t, targeting);
  };

  return (
    <div className="shrink-0 px-6 pb-3 pt-2">
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
            <ModePicker />
            <span className="flex-1" />
            {session?.context && <ContextGauge used={session.context.used} window={session.context.window} />}
            {busy ? (
              <button
                type="button"
                aria-label="Stop"
                className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md bg-white hover:bg-white/90"
                onClick={() => void abort()}
              >
                <Square className="size-3 fill-current text-black" />
              </button>
            ) : (
              <button
                type="button"
                aria-label="Send"
                disabled={!text.trim() || !activeProjectId}
                className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md bg-white hover:bg-white/90 disabled:opacity-40"
                onClick={submit}
              >
                <ArrowUp className="size-[13px] text-black" strokeWidth={2.2} />
              </button>
            )}
          </div>
        </div>
        {activeProjectId && (
          <div className="pt-2">
            <SessionTargetStrip
              projectID={isNewSession ? targetProjectId || activeProjectId : activeProjectId}
              onProjectSelect={isNewSession ? setTargetProjectId : undefined}
              sessionID={activeSessionId ?? undefined}
              branch={targetBranch}
              onBranch={setTargetBranch}
            />
          </div>
        )}
      </div>
    </div>
  );
}
