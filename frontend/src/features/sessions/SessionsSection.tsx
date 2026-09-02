/**
 * Session list (docs/ui.md): date-grouped rows with inline rename and
 * confirm-delete; busy spinner from live session status.
 */

import { useMemo, useState } from "react";
import { Loader2, SquareTerminal, Trash2 } from "lucide-react";

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
import type { Session } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

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

export function SessionsSection() {
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const sessionsByProject = useAppStore((s) => s.sessionsByProject);
  const activeSessions = useMemo(
    () => (activeProjectId ? sessionsByProject[activeProjectId] ?? [] : []),
    [activeProjectId, sessionsByProject],
  );
  const groups = useMemo(() => groupByDate(activeSessions), [activeSessions]);

  if (!activeProjectId) return null;
  return (
    <div>
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
          <div className="px-2 pb-0.5 pt-2 text-[11px] text-muted-foreground/80">{g.label}</div>
          {g.items.map((s) => (
            <SessionRow key={s.id} projectId={activeProjectId} session={s} />
          ))}
        </div>
      ))}
    </div>
  );
}

