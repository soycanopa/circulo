/**
 * Session list — replica of the Circulo Paper design: two-row cards
 * (title + folder/relative-time), selected = bg-accent, busy spinner,
 * hover actions (rename / delete), date-grouped.
 */

import { useMemo, useState } from "react";
import { Loader2, Pencil, Trash2 } from "lucide-react";

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
import { groupByDate } from "./groupByDate";
import { timeAgo } from "./timeAgo";
import type { Session } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

function SessionRow({
  projectId,
  projectName,
  session,
}: {
  projectId: string;
  projectName: string;
  session: Session;
}) {
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const openSession = useAppStore((s) => s.openSession);
  const chat = useAppStore((s) => s.chat);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(session.title);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const state = chat.sessions[session.id];
  const busy = state?.status === "busy" || state?.status === "retry";
  const active = activeSessionId === session.id;

  const saveTitle = async () => {
    setEditing(false);
    if (draft.trim() && draft !== session.title) {
      await api.renameSession(projectId, session.id, draft.trim()).catch(() => undefined);
      await useAppStore.getState().refreshSessions(projectId);
    }
  };

  return (
    <div
      role="button"
      tabIndex={0}
      className={cn(
        "group flex w-full cursor-pointer flex-col gap-[2px] rounded-md px-[10px] py-2 text-left outline-none hover:bg-muted/60 focus-visible:ring-1 focus-visible:ring-ring",
        active && "bg-bg-hover",
      )}
      onClick={() => void openSession(projectId, session.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter") void openSession(projectId, session.id);
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
              {session.title || "Untitled"}
            </span>
            <span className="shrink-0 text-xs text-text-tertiary">
          {timeAgo(session.timeUpdated || session.timeCreated)}
        </span>
        <span className="hidden shrink-0 items-center gap-0.5 group-hover:flex">
              <button
                aria-label="Rename session"
                className="rounded p-0.5 text-muted-foreground hover:text-foreground"
                onClick={(e) => {
                  e.stopPropagation();
                  setDraft(session.title);
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
            {busy && <Loader2 className="size-3 shrink-0 animate-spin text-muted-foreground" />}
          </div>
          <div className="flex items-center gap-[6px]">
            <span className="min-w-0 flex-1 truncate text-sm text-text-tertiary">
              {projectName}
            </span>
          </div>
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

export function SessionsSection() {
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const sessionsByProject = useAppStore((s) => s.sessionsByProject);
  const search = useAppStore((s) => s.sessionSearch);
  const activeSessions = useMemo(() => {
    const list = activeProjectId ? sessionsByProject[activeProjectId] ?? [] : [];
    const q = search.trim().toLowerCase();
    if (!q) return list;
    return list.filter((x) => (x.title || "").toLowerCase().includes(q));
  }, [activeProjectId, sessionsByProject, search]);
  const groups = useMemo(() => groupByDate(activeSessions), [activeSessions]);
  const activeProject = useAppStore((s) => s.projects.find((p) => p.id === activeProjectId));
  const activeProjectName = activeProject
    ? activeProject.path.split("/").filter(Boolean).pop() ?? activeProject.path
    : "";

  if (!activeProjectId) return null;
  return (
    <div>
      <div className="px-2 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        Sessions
      </div>
      {groups.length === 0 && (
        <div className="px-2 py-2 text-[12.5px] text-muted-foreground">
          No sessions yet — start a chat.
        </div>
      )}
      {groups.map((g) => (
        <div key={g.label}>
          <div className="px-2 pb-0.5 pt-2 text-[11px] text-text-tertiary">{g.label}</div>
          {g.items.map((s) => (
            <SessionRow
              key={s.id}
              projectId={activeProjectId}
              projectName={activeProjectName}
              session={s}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
