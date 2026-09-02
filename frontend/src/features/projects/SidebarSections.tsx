/**
 * Sidebar sections — replica of the Circulo Paper design: New chat (⌘N),
 * project rows with adapter status dots, session items as cards, footer with
 * Settings + server indicator.
 */

import { useState } from "react";
import {
  Circle,
  FolderOpen,
  Loader2,
  Plus,
  Settings,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { PickFolder } from "@/bindings/circulogo/internal/appservice/dialog";
import { useAppStore } from "@/lib/agent/store";
import type { AdapterState } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

export function NewChatButton() {
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const newSession = useAppStore((s) => s.newSession);
  return (
    <div className="px-3 pt-0 pb-2">
      <Button
        className="w-full justify-between gap-2"
        disabled={!activeProjectId}
        onClick={() => activeProjectId && void newSession(activeProjectId)}
      >
        <span className="flex items-center gap-2">
          <Plus className="size-4" /> New chat
        </span>
        <kbd className="rounded-sm border border-primary/20 px-1 font-sans text-[11px] text-primary/60">
          ⌘N
        </kbd>
      </Button>
    </div>
  );
}

export function StatusDot({ status, detail }: { status: AdapterState; detail?: string }) {
  const title = detail ? `${status}: ${detail}` : status;
  return (
    <span title={title} aria-label={status}>
      {status === "running" && <Circle className="size-2 fill-success text-success" />}
      {status === "starting" && <Loader2 className="size-2.5 animate-spin text-warning" />}
      {status === "error" && <Circle className="size-2 fill-destructive text-destructive" />}
      {status === "stopped" && <Circle className="size-2 fill-text-tertiary text-text-tertiary" />}
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
      <div className="px-2 pb-1 pt-3 text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        Projects
      </div>
      {projects.map((p) => {
        const name = p.path.split("/").filter(Boolean).pop() ?? p.path;
        return (
          <button
            key={p.id}
            className={cn(
              "group flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] hover:bg-muted/60",
              activeProjectId === p.id && "bg-muted",
            )}
            onClick={() => setActiveProject(p.id)}
          >
            <StatusDot status={p.status} detail={p.detail} />
            <span className="min-w-0 flex-1 truncate font-medium">{name}</span>
            <span className="truncate text-[11px] text-text-tertiary">{p.mode}</span>
            <span
              role="button"
              aria-label="Remove project"
              className="hidden rounded p-0.5 text-muted-foreground hover:text-destructive group-hover:flex"
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
      {error && <div className="px-2 py-1 text-[11.5px] text-destructive">{error}</div>}
      <label className="mt-1 flex items-center gap-2 px-2 text-[11.5px] text-muted-foreground">
        <input
          type="checkbox"
          checked={attachMode}
          onChange={(e) => setAttachMode(e.target.checked)}
          className="accent-accent-indigo"
        />
        attach to running server
      </label>
      {attachMode && (
        <input
          data-selectable
          placeholder="http://127.0.0.1:4096"
          value={attachUrl}
          onChange={(e) => setAttachUrl(e.target.value)}
          className="mt-1 w-full rounded-md border border-border bg-card px-2 py-1 text-[12px] outline-none"
        />
      )}
    </div>
  );
}

export function SidebarFooter() {
  return (
    <div className="border-t border-border px-3 py-2">
      <div className="flex items-center gap-2">
        <button
          className="flex items-center gap-2 rounded-md px-1.5 py-1 text-[12px] text-muted-foreground hover:bg-muted/60"
          title="Settings — coming soon"
        >
          <Settings className="size-3.5" /> Settings
        </button>
        <span className="ml-auto flex items-center gap-[6px] text-[11px] text-text-tertiary">
          <span className="size-2 rounded-full bg-success" />
          circulo
        </span>
      </div>
    </div>
  );
}
