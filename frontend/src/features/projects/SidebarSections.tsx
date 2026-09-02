/** Sidebar sections (docs/ui.md §2): New chat, projects, sessions, footer. */

import { useState } from "react";
import { Check, Circle, FolderOpen, Loader2, Plus, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { PickFolder } from "@/bindings/circulogo/internal/appservice/dialog";
import { useAppStore } from "@/lib/agent/store";
import type { AdapterState } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

export function NewChatButton() {
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const newSession = useAppStore((s) => s.newSession);
  return (
    <div className="px-3 pt-0 pb-1">
      <Button
        className="w-full justify-start gap-2"
        disabled={!activeProjectId}
        onClick={() => activeProjectId && void newSession(activeProjectId)}
      >
        <Plus className="size-4" /> New chat
      </Button>
    </div>
  );
}

export function StatusDot({ status, detail }: { status: AdapterState; detail?: string }) {
  const title = detail ? `${status}: ${detail}` : status;
  return (
    <span title={title} aria-label={status}>
      {status === "running" && <Circle className="size-2 fill-emerald-500 text-emerald-500" />}
      {status === "starting" && <Loader2 className="size-2.5 animate-spin text-amber-500" />}
      {status === "error" && <Circle className="size-2 fill-red-500 text-red-500" />}
      {status === "stopped" && <Circle className="size-2 fill-zinc-500 text-zinc-500" />}
    </span>
  );
}

export function ProjectsSection() {
  const projects = useAppStore((s) => s.projects);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const setActiveProject = useAppStore((s) => s.setActiveProject);
  const addProject = useAppStore((s) => s.addProject);
  const [attachMode, setAttachMode] = useState(false);
  const [attachUrl, setAttachUrl] = useState("");
  const [error, setError] = useState("");

  const pickFolder = async () => {
    setError("");
    const path = await PickFolder().catch(() => "");
    if (!path) return;
    try {
      await addProject(path, attachMode ? "attach" : "managed", attachUrl || undefined);
      setAttachMode(false);
      setAttachUrl("");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <div className="px-2 pb-1 pt-2 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
        Projects
      </div>
      {projects.map((p) => {
        const name = p.path.split("/").filter(Boolean).pop() ?? p.path;
        return (
          <button
            key={p.id}
            className={cn(
              "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] hover:bg-muted/60",
              activeProjectId === p.id && "bg-muted",
            )}
            onClick={() => setActiveProject(p.id)}
          >
            <StatusDot status={p.status} detail={p.detail} />
            <span className="min-w-0 flex-1 truncate font-medium">{name}</span>
            <span className="truncate text-[11px] text-muted-foreground">{p.mode}</span>
            <span
              role="button"
              aria-label="Remove project"
              className="hidden rounded p-0.5 text-muted-foreground hover:text-red-500 group-hover:flex"
              onClick={(e) => {
                e.stopPropagation();
                void useAppStore.getState().removeProject(p.id);
              }}
            >
              <X className="size-3.5" />
            </span>
          </button>
        );
      })}
      <button
        className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] text-muted-foreground hover:bg-muted/60"
        onClick={() => void pickFolder()}
      >
        <FolderOpen className="size-3.5" /> Add project…
      </button>
      {error && <div className="px-2 py-1 text-[11.5px] text-red-500">{error}</div>}
      <label className="mt-1 flex items-center gap-2 px-2 text-[11.5px] text-muted-foreground">
        <input
          type="checkbox"
          checked={attachMode}
          onChange={(e) => setAttachMode(e.target.checked)}
          className="accent-zinc-500"
        />
        attach to running server
      </label>
      {attachMode && (
        <input
          data-selectable
          placeholder="http://127.0.0.1:4096"
          value={attachUrl}
          onChange={(e) => setAttachUrl(e.target.value)}
          className="mt-1 w-full rounded-md border border-border bg-background px-2 py-1 text-[12px] outline-none"
        />
      )}
    </div>
  );
}

export function SidebarFooter() {
  return (
    <div className="border-t border-border p-3">
      <div className="flex items-center gap-2 text-[12px] text-muted-foreground">
        <Check className="size-3.5 text-emerald-500" />
        local-only · no ACP
        <Badge variant="outline" className="ml-auto text-[10px]">v0</Badge>
      </div>
    </div>
  );
}
