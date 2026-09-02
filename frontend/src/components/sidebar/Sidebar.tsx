/**
 * Sidebar per docs/ui.md: project switcher (status dots), sessions grouped by
 * date, new chat, settings footer. Rename/delete via context menu.
 */

import { useMemo, useState } from "react";
import {
  Check,
  Circle,
  FolderOpen,
  Loader2,
  Plus,
  SquareTerminal,
  Trash2,
  X,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { PickFolder } from "@/bindings/circulogo/internal/appservice/dialog";
import { api } from "@/lib/agent/api";
import { useAppStore } from "@/lib/agent/store";
import type { AdapterState, Session } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

function StatusDot({ status, detail }: { status: AdapterState; detail?: string }) {
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

function groupLabel(ts: number): string {
  const today = new Date();
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  if (ts >= startOfToday) return "Today";
  if (ts >= startOfToday - 86400000) return "Yesterday";
  if (ts >= startOfToday - 7 * 86400000) return "This week";
  return "Older";
}

function SessionRow({ projectId, session }: { projectId: string; session: Session }) {
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const openSession = useAppStore((s) => s.openSession);
  const chat = useAppStore((s) => s.chat);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(session.title);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const state = chat.sessions[session.id];

  const saveTitle = async () => {
    setEditing(false);
    if (draft.trim() && draft !== session.title) {
      await api.renameSession(projectId, session.id, draft.trim()).catch(() => undefined);
      await useAppStore.getState().refreshSessions(projectId);
    }
  };

  return (
    <div
      className={cn(
        "group flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-[13px] hover:bg-muted/60",
        activeSessionId === session.id && "bg-muted",
      )}
      onClick={() => void openSession(projectId, session.id)}
    >
      {editing ? (
        <>
          <input
            autoFocus
            data-selectable
            className="min-w-0 flex-1 rounded border border-border bg-background px-1 py-0.5 text-[13px] outline-none"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Enter") void saveTitle();
              if (e.key === "Escape") setEditing(false);
            }}
            onBlur={() => void saveTitle()}
          />
        </>
      ) : (
        <>
          <span className="min-w-0 flex-1 truncate">{session.title || "Untitled"}</span>
          {state?.status === "busy" && (
            <Loader2 className="size-3 shrink-0 animate-spin text-muted-foreground" />
          )}
          <span className="hidden shrink-0 items-center gap-1 group-hover:flex">
            <button
              aria-label="Rename session"
              className="rounded p-0.5 hover:bg-muted"
              onClick={(e) => {
                e.stopPropagation();
                setDraft(session.title);
                setEditing(true);
              }}
            >
              <SquareTerminal className="size-3.5" />
            </button>
            <button
              aria-label="Delete session"
              className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-red-500"
              onClick={(e) => {
                e.stopPropagation();
                setConfirmDelete(true);
              }}
            >
              <Trash2 className="size-3.5" />
            </button>
          </span>
        </>
      )}
      <AlertDialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete session?</AlertDialogTitle>
            <AlertDialogDescription>
              “{session.title}” and its history on the agent server will be removed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={(e) => e.stopPropagation()}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={(e) => {
                e.stopPropagation();
                setConfirmDelete(false);
                void useAppStore.getState().deleteSession(projectId, session.id);
              }}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

export function Sidebar() {
  const projects = useAppStore((s) => s.projects);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const setActiveProject = useAppStore((s) => s.setActiveProject);
  const sessionsByProject = useAppStore((s) => s.sessionsByProject);
  const addProject = useAppStore((s) => s.addProject);
  const newSession = useAppStore((s) => s.newSession);
  const [attachMode, setAttachMode] = useState(false);
  const [attachUrl, setAttachUrl] = useState("");
  const [error, setError] = useState("");

  const activeSessions = useMemo(
    () => (activeProjectId ? sessionsByProject[activeProjectId] ?? [] : []),
    [activeProjectId, sessionsByProject],
  );
  const groups = useMemo(() => {
    const out: { label: string; items: Session[] }[] = [];
    for (const s of activeSessions) {
      const label = groupLabel(s.timeUpdated || s.timeCreated);
      const g = out.find((x) => x.label === label);
      if (g) g.items.push(s);
      else out.push({ label, items: [s] });
    }
    return out;
  }, [activeSessions]);

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
    <aside className="flex h-full w-72 shrink-0 flex-col border-r border-border bg-sidebar">
      <div className="p-3">
        <Button
          className="w-full justify-start gap-2"
          disabled={!activeProjectId}
          onClick={() => activeProjectId && void newSession(activeProjectId)}
        >
          <Plus className="size-4" /> New chat
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        {/* Projects */}
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
        {error && (
          <div className="px-2 py-1 text-[11.5px] text-red-500">{error}</div>
        )}
        <label className="mt-1 flex items-center gap-2 px-2 text-[11.5px] text-muted-foreground">
          <input
            type="checkbox"
            checked={attachMode}
            onChange={(e) => setAttachMode(e.target.checked)}
            className="accent-violet-500"
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

        {/* Sessions */}
        {activeProjectId && (
          <>
            <div className="px-2 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
              Sessions
            </div>
            {groups.length === 0 && (
              <div className="px-2 py-2 text-[12.5px] text-muted-foreground">
                No sessions yet — start a chat.
              </div>
            )}
            {groups.map((g) => (
              <div key={g.label}>
                <div className="px-2 pb-0.5 pt-2 text-[11px] text-muted-foreground/80">
                  {g.label}
                </div>
                {g.items.map((s) => (
                  <SessionRow key={s.id} projectId={activeProjectId} session={s} />
                ))}
              </div>
            ))}
          </>
        )}
      </div>

      <div className="border-t border-border p-3">
        <div className="flex items-center gap-2 text-[12px] text-muted-foreground">
          <Check className="size-3.5 text-emerald-500" />
          local-only · no ACP
          <Badge variant="outline" className="ml-auto text-[10px]">v0</Badge>
        </div>
      </div>
    </aside>
  );
}
