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

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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

function ModelPicker() {
  const models = useAppStore((s) => s.metaModels);
  const selected = useAppStore((s) => s.selectedModel);
  const setSelected = useAppStore((s) => s.setSelectedModel);
  const current = models.find((m) => `${m.provider}:${m.id}` === selected);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="flex h-[28px] items-center gap-[6px] rounded-md border border-border bg-bg-code px-[10px] text-sm text-text-primary hover:bg-muted"
        >
          {current?.name || current?.id || "model"}
          <ChevronDown className="size-3 text-text-tertiary" />
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
  const selected = useAppStore((s) => s.selectedAgent);
  const setSelected = useAppStore((s) => s.setSelectedAgent);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11.5px] text-muted-foreground hover:bg-muted"
        >
          agent: {selected || "build"}
          <ChevronDown className="size-3" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuLabel>Agent</DropdownMenuLabel>
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

export function Composer() {
  const [text, setText] = useState("");
  const send = useAppStore((s) => s.sendPrompt);
  const abort = useAppStore((s) => s.abort);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const chat = useAppStore((s) => s.chat);
  const taRef = useRef<HTMLTextAreaElement>(null);

  const projects = useAppStore((s) => s.projects);
  const activeProject = projects.find((p) => p.id === activeProjectId);
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

  const models = useAppStore((s) => s.metaModels);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const currentModel = models.find((m) => `${m.provider}:${m.id}` === selectedModel);

  return (
    <div className="bg-background/80 px-6 pb-3 pt-2 backdrop-blur">
      <div className="mx-auto max-w-3xl space-y-2">
        {session && session.permissions.length > 0 && (
          <div className="space-y-2">
            {session.permissions.map((p) => (
              <PermissionCard key={p.id} perm={p} />
            ))}
          </div>
        )}
        <div className="mx-auto flex w-full max-w-[768px] flex-col rounded-xl border border-border-strong bg-bg-main shadow-[#00000059_0px_8px_24px] focus-within:border-ring">
          <textarea
            ref={taRef}
            data-selectable
            rows={1}
            value={text}
            placeholder={
              activeSessionId
                ? "Ask a follow-up…"
                : "Write anything — Circulo does the rest"
            }
            disabled={!activeProjectId}
            className="w-full resize-none bg-transparent px-4 pt-4 pb-2 text-md text-text-primary outline-none placeholder:text-text-tertiary disabled:cursor-not-allowed"
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                submit();
              }
            }}
          />
          <div className="flex items-center gap-1.5 px-3 pb-[10px] pt-2">
            <ModelPicker />
            <AgentPicker />
            <span className="flex-1" />
            {busy ? (
              <Button size="icon-sm" variant="secondary" aria-label="Stop" onClick={() => void abort()}>
                <Square className="size-3.5 fill-current" />
              </Button>
            ) : (
              <Button
                size="icon-sm"
                aria-label="Send"
                disabled={!text.trim() || !activeProjectId}
                onClick={submit}
              >
                <ArrowUp className="size-4" />
              </Button>
            )}
          </div>
        </div>
        <div className="mx-auto mt-2 flex max-w-[768px] items-center gap-3 text-xs text-text-tertiary">
          <span className="flex items-center gap-[6px]">
            <span
              className={cn(
                "size-1.5 rounded-full",
                activeProject && activeProject.status === "running" ? "bg-success" : "bg-text-tertiary",
              )}
            />
            {activeProject
              ? activeProject.path.split("/").filter(Boolean).pop()
              : "no project"}
          </span>
          <span>•</span>
          <span>Local environment</span>
          <span>•</span>
          <span className="truncate">
            {currentModel ? `${currentModel.provider}/${currentModel.id}` : ""}
          </span>
        </div>
        {session?.status === "retry" && session.retry && (
          <Badge variant="outline" className="text-warning">
            retrying (attempt {session.retry.attempt})
          </Badge>
        )}
      </div>
    </div>
  );
}
