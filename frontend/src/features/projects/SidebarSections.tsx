/**
 * Sidebar chrome — 1:1 replica of the Circulo dark design:
 * New session button (h-34, indigo) → search (h-8, bg-bg-main) →
 * [projects rows] → [sessions list] → footer (Settings row, border-t).
 * Width 260px lives in AppShell.
 */

import { useState } from "react";
import {
  Circle,
  FolderOpen,
  Loader2,
  Plus,
  Search,
  Settings,
  X,
} from "lucide-react";

import { PickFolder } from "@/bindings/circulogo/internal/appservice/dialog";
import { useAppStore } from "@/lib/agent/store";
import type { AdapterState } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

export function NewSessionButton() {
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const newSession = useAppStore((s) => s.newSession);
  return (
    <div className="flex flex-col shrink-0 pt-2 pb-3 gap-2 px-3">
      <button
        className="flex items-center justify-center h-[34px] rounded-md gap-[6px] shrink-0 bg-accent-cir hover:bg-accent-cir-hover disabled:opacity-50"
        disabled={!activeProjectId}
        onClick={() => activeProjectId && void newSession(activeProjectId)}
      >
        <Plus className="size-3.5 text-white" strokeWidth={2.5} />
        <span className="text-sm font-medium text-white">New session</span>
      </button>
    </div>
  );
}

export function SearchInput() {
  const setSearch = useAppStore((s) => s.setSessionSearch);
  return (
    <div className="flex flex-col shrink-0 px-3 pb-1">
      <div className="flex items-center h-8 px-[10px] rounded-md gap-2 bg-bg-main border border-border">
        <Search className="size-3.5 shrink-0 text-text-tertiary" />
        <input
          data-selectable
          placeholder="Search sessions"
          onChange={(e) => setSearch(e.target.value)}
          className="w-full bg-transparent text-sm text-text-primary outline-none placeholder:text-text-tertiary"
        />
      </div>
    </div>
  );
}

export function StatusDot({ status, detail }: { status: AdapterState; detail?: string }) {
  const title = detail ? `${status}: ${detail}` : status;
  return (
    <span title={title} aria-label={status}>
      {status === "running" && <Circle className="size-2 fill-success text-success" />}
      {status === "starting" && <Loader2 className="size-2.5 animate-spin text-warning" />}
      {status === "error" && <Circle className="size-2 fill-danger text-danger" />}
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
      <div className="flex items-center pt-3 pb-[6px] px-2">
        <span className="text-xs font-medium tracking-wider uppercase leading-[14px] text-text-tertiary">
          Projects
        </span>
      </div>
      {projects.map((p) => {
        const name = p.path.split("/").filter(Boolean).pop() ?? p.path;
        return (
          <button
            key={p.id}
            className={cn(
              "group flex w-full items-center gap-2 rounded-md px-[10px] py-2 text-left text-sm hover:bg-muted/60",
              activeProjectId === p.id && "bg-muted",
            )}
            onClick={() => setActiveProject(p.id)}
          >
            <StatusDot status={p.status} detail={p.detail} />
            <span className="min-w-0 flex-1 truncate font-medium text-text-primary">{name}</span>
            <span className="truncate text-xs text-text-tertiary">{p.mode}</span>
            <span
              role="button"
              aria-label="Remove project"
              className="hidden rounded p-0.5 text-muted-foreground hover:text-destructive group-hover:flex"
              onClick={(e) => {
                e.stopPropagation();
                void useAppStore.getState().removeProject(p.id);
              }}
            >
              <X className="size-3" />
            </span>
          </button>
        );
      })}
      <button
        className="flex w-full items-center gap-2 rounded-md px-[10px] py-2 text-left text-sm text-text-secondary hover:bg-muted/60"
        onClick={() => void pickFolder()}
      >
        <FolderOpen className="size-3.5" /> Add project…
      </button>
      {error && <div className="px-2 py-1 text-xs text-danger">{error}</div>}
      <label className="mt-1 flex items-center gap-2 px-[10px] text-xs text-text-tertiary">
        <input
          type="checkbox"
          checked={attachMode}
          onChange={(e) => setAttachMode(e.target.checked)}
          className="accent-accent-cir"
        />
        attach to running server
      </label>
      {attachMode && (
        <input
          data-selectable
          placeholder="http://127.0.0.1:4096"
          value={attachUrl}
          onChange={(e) => setAttachUrl(e.target.value)}
          className="mt-1 w-full rounded-md border border-border bg-bg-main px-2 py-1 text-xs outline-none"
        />
      )}
    </div>
  );
}

export function SidebarFooter() {
  return (
    <div className="flex flex-col shrink-0 py-2 px-3 border-t border-border">
      <div className="flex items-center h-8 px-2 rounded-md gap-2">
        <Settings className="size-3.5 text-text-secondary" />
        <span className="text-sm text-text-secondary">Settings</span>
        <span className="ml-auto flex items-center gap-[6px] text-xs text-text-tertiary">
          <span
            className="size-2 rounded-full bg-success"
            title="circulo — local agent running"
          />
          circulo
        </span>
      </div>
    </div>
  );
}
