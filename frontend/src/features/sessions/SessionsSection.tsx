/**
 * Session list — 1:1 replica of the Circulo Paper sidebar: one flat list
 * across all projects grouped Today / Earlier (collapsible), two-line rows
 * (title, then folder + project name · relative time), busy spinner on the
 * running session, hover rename/delete (FR-7).
 */

import { useMemo, useState } from "react";
import { ChevronDown, FolderPlus, Folder, Loader2, Pencil, Trash2 } from "lucide-react";

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
import { api } from "@/lib/agent/api";
import { useAppStore } from "@/lib/agent/store";
import { groupByDate, type SessionGroup } from "./groupByDate";
import { timeAgo } from "./timeAgo";
import { cn } from "@/lib/utils";
import type { Session } from "@/lib/agent/protocol";

/** One row = a session plus the project it belongs to. */
interface Row extends Session {
  projectID: string;
  projectName: string;
}

function basename(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}

function SectionHeader({
  group,
  collapsed,
  onToggle,
  onNewSession,
}: {
  group: SessionGroup<Row>;
  collapsed: boolean;
  onToggle: () => void;
  onNewSession?: () => void;
}) {
  return (
    <div className="flex w-full items-center py-2">
      <button
        type="button"
        className="flex items-center gap-1 rounded-md text-left"
        onClick={onToggle}
        aria-expanded={!collapsed}
      >
        <div className="text-xs font-medium leading-[14px] text-text-secondary">{group.label}</div>
        <ChevronDown
          className={cn(
            "size-3 shrink-0 text-text-secondary transition-transform",
            collapsed && "-rotate-90",
          )}
          strokeWidth={2}
        />
      </button>
      <div className="grow" />
      {group.key === "today" && onNewSession && (
        <button
          type="button"
          aria-label="New session"
          title="New session"
          className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-xs text-text-secondary hover:bg-bg-hover"
          onClick={onNewSession}
        >
          <FolderPlus className="size-3.5" strokeWidth={2} />
        </button>
      )}
    </div>
  );
}

function SessionRow({
  row,
  active,
  busy,
}: {
  row: Row;
  active: boolean;
  busy: boolean;
}) {
  const openSession = useAppStore((s) => s.openSession);
  const setActiveProject = useAppStore((s) => s.setActiveProject);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(row.title);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const saveTitle = async () => {
    setEditing(false);
    if (draft.trim() && draft !== row.title) {
      await api.renameSession(row.projectID, row.id, draft.trim()).catch(() => undefined);
      await useAppStore.getState().refreshSessions(row.projectID);
    }
  };

  const open = () => {
    // Cross-project row: switching project clears the active session, then
    // openSession hydrates the target one.
    setActiveProject(row.projectID);
    void openSession(row.projectID, row.id);
  };

  return (
    <div
      role="button"
      tabIndex={0}
      className={cn(
        "group flex cursor-pointer flex-col gap-[2px] rounded-md px-2 py-1 text-left outline-none",
        active && "bg-bg-hover",
      )}
      onClick={() => void open()}
      onKeyDown={(e) => {
        if (e.key === "Enter") void open();
      }}
    >
      {editing ? (
        <input
          autoFocus
          data-selectable
          className="min-w-0 flex-1 rounded border border-border-strong bg-background px-1 py-0.5 text-[13px] outline-none"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            if (e.key === "Enter") void saveTitle();
            if (e.key === "Escape") setEditing(false);
          }}
          onBlur={() => void saveTitle()}
        />
      ) : (
        <>
          <div className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-base font-medium leading-[18px] text-text-primary line-clamp-1">
              {row.title || "Untitled"}
            </span>
            <span className="hidden shrink-0 items-center gap-0.5 group-hover:flex">
              <button
                aria-label="Rename session"
                className="rounded p-0.5 text-muted-foreground hover:text-foreground"
                onClick={(e) => {
                  e.stopPropagation();
                  setDraft(row.title);
                  setEditing(true);
                }}
              >
                <Pencil className="size-3" />
              </button>
              <button
                aria-label="Delete session"
                className="rounded p-0.5 text-muted-foreground hover:text-destructive"
                onClick={(e) => {
                  e.stopPropagation();
                  setConfirmDelete(true);
                }}
              >
                <Trash2 className="size-3" />
              </button>
            </span>
            {busy && (
              <Loader2 className="size-3 shrink-0 animate-spin text-text-secondary" />
            )}
          </div>
          <div className="flex items-center justify-between gap-[6px]">
            <span className="flex min-w-0 items-center gap-[6px]">
              <Folder className="size-3 shrink-0 text-text-tertiary" />
              <span className="min-w-0 truncate text-sm/tight text-text-tertiary">
                {row.projectName}
              </span>
            </span>
            <span className="shrink-0 text-xs leading-[14px] text-text-tertiary">
              {timeAgo(row.timeUpdated || row.timeCreated)}
            </span>
          </div>
        </>
      )}
      <AlertDialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete session?</AlertDialogTitle>
            <AlertDialogDescription>
              “{row.title}” and its history on the agent server will be removed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={(e) => e.stopPropagation()}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={(e) => {
                e.stopPropagation();
                setConfirmDelete(false);
                void useAppStore.getState().deleteSession(row.projectID, row.id);
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

export function SessionsSection() {
  const sessionsByProject = useAppStore((s) => s.sessionsByProject);
  const projects = useAppStore((s) => s.projects);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const chat = useAppStore((s) => s.chat);
  const search = useAppStore((s) => s.sessionSearch);
  const closeSession = useAppStore((s) => s.closeSession);
  const [collapsed, setCollapsed] = useState<Record<"today" | "earlier", boolean>>({
    today: false,
    earlier: false,
  });

  // One flat list across every project (Circulo sidebar has no project
  // switcher: each row carries its project name).
  const rows = useMemo(() => {
    const nameOf = new Map(projects.map((p) => [p.id, basename(p.path)]));
    const out: Row[] = [];
    for (const [pid, list] of Object.entries(sessionsByProject)) {
      const projectName = nameOf.get(pid) ?? pid;
      for (const session of list) {
        out.push({ ...session, projectID: pid, projectName });
      }
    }
    const q = search.trim().toLowerCase();
    return out
      .filter((r) => !q || (r.title || "").toLowerCase().includes(q))
      .sort((a, b) => (b.timeUpdated || b.timeCreated) - (a.timeUpdated || a.timeCreated));
  }, [sessionsByProject, projects, search]);

  const groups = useMemo(() => groupByDate(rows), [rows]);

  // New-session mode (owner call): close the session so the composer's
  // project/branch target strip unlocks and the user picks the destination.
  const newChat = () => closeSession();

  if (!activeProjectId && projects.length === 0) return null;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-3">
      {groups.length === 0 && (
        <div className="px-2 py-2 text-sm/tight text-text-tertiary">
          No sessions yet — start a chat.
        </div>
      )}
      {groups.map((g) => (
        <div key={g.key} className="flex flex-col">
          <SectionHeader
            group={g}
            collapsed={collapsed[g.key]}
            onToggle={() => setCollapsed((c) => ({ ...c, [g.key]: !c[g.key] }))}
            onNewSession={newChat}
          />
          {!collapsed[g.key] &&
            g.items.map((row) => (
              <SessionRow
                key={row.id}
                row={row}
                active={activeSessionId === row.id}
                busy={
                  chat.sessions[row.id]?.status === "busy" ||
                  chat.sessions[row.id]?.status === "retry"
                }
              />
            ))}
        </div>
      ))}
    </div>
  );
}
