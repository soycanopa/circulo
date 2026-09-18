/**
 * Composer per docs/ux.md §7: pinned bottom, auto-grow, permission cards
 * stacked directly above, model/agent pickers, Send ↔ Stop swap.
 */

import { useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowUp,
  ChevronDown,
  Square,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useAppStore } from "@/lib/agent/store";
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

function ModelPicker() {
  const models = useAppStore((s) => s.metaModels);
  const setSelected = useAppStore((s) => s.setSelectedModel);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="flex items-center gap-[6px] rounded-md px-2 py-1 text-sm/tight text-text-secondary hover:bg-muted"
        >
          Model
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-80 overflow-auto">
        <DropdownMenuLabel>Model</DropdownMenuLabel>
        {models.map((m) => (
          <DropdownMenuItem
            key={`${m.provider}:${m.id}`}
            onClick={() => setSelected(`${m.provider}:${m.id}`)}
          >
            <span>{m.name || m.id}</span>
            <span className="ml-auto text-muted-foreground">{m.provider}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function AgentPicker() {
  const agents = useAppStore((s) => s.metaAgents);
  const setSelected = useAppStore((s) => s.setSelectedAgent);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="flex items-center gap-[6px] rounded-md px-2 py-1 text-sm/tight text-text-secondary hover:bg-muted"
        >
          Mode
          <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuLabel>Mode</DropdownMenuLabel>
        {agents.map((a) => (
          <DropdownMenuItem key={a.name} onClick={() => setSelected(a.name)}>
            <span>{a.name}</span>
            {a.description && (
              <span className="ml-auto max-w-48 truncate text-muted-foreground">
                {a.description}
              </span>
            )}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Design chip only: the protocol does not carry a reasoning-effort yet. */
function ReasoningChip() {
  return (
    <div className="flex items-center gap-[6px] rounded-md px-2 py-1 text-sm/tight text-text-secondary">
      Rasoning
      <ChevronDown className="size-[11px] shrink-0 text-text-tertiary" strokeWidth={2} />
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
            <ReasoningChip />
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
