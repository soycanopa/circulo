/**
 * Rewind — restore the workspace to the start of one agent turn.
 *
 * The turns are git refs the orchestrator wrote at the edges of every turn
 * (see internal/checkpoint), so this is a project capability rather than an
 * adapter one: it still works when the backend is down, which is when you are
 * most likely to want it.
 *
 * Two things drive the shape of this component:
 *
 *   - A rewind is destructive, so it is never one click. The dialog fetches
 *     the preview first and shows the actual diff of what would be undone,
 *     because "rewind to turn 4" means nothing until you see what it removes.
 *   - It is not irreversible: the orchestrator checkpoints the current state
 *     before restoring, so the dialog says so rather than implying otherwise.
 *
 * The whole control is hidden when the project is not a git repository. That
 * is a normal state, not an error worth a disabled button and a tooltip.
 */

import { memo, useCallback, useEffect, useState } from "react";
import { History, Loader2 } from "lucide-react";

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
import type { CheckpointView, RewindPreview } from "@/lib/agent/protocol";

/** Turns worth offering in the control. The backend keeps more; older ones are noise. */
const MAX_TURNS_SHOWN = 10;

/**
 * Which turns can be rewound to, newest first.
 *
 * A turn counts only when it both started and finished. A start without a
 * matching end is a turn that is still running, or that died mid-flight —
 * rewinding to it would discard work the user is still watching. The guard
 * refs written by a previous rewind are excluded: they exist to undo a
 * rewind, not to be a rewind target of their own.
 */
export function rewindableTurns(checkpoints: CheckpointView["checkpoints"]): number[] {
  const started = new Set<number>();
  const finished = new Set<number>();
  for (const cp of checkpoints) {
    if (cp.kind === "start") started.add(cp.turn);
    if (cp.kind === "end") finished.add(cp.turn);
  }
  return [...started]
    .filter((t) => finished.has(t))
    .sort((a, b) => b - a)
    .slice(0, MAX_TURNS_SHOWN);
}

export const RewindControl = memo(function RewindControl({
  projectID,
  sessionID,
  onRewound,
}: {
  projectID: string;
  sessionID: string;
  onRewound: () => void;
}) {
  const [view, setView] = useState<CheckpointView | null>(null);
  const [open, setOpen] = useState(false);
  const [target, setTarget] = useState<number | null>(null);
  const [preview, setPreview] = useState<RewindPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Checkpoints appear as turns complete, so poll rather than refetching the
  // whole app: this control must not drive a resync just to stay current.
  useEffect(() => {
    let alive = true;
    const load = () => {
      api
        .checkpoints(projectID, sessionID)
        .then((v) => alive && setView(v))
        .catch(() => alive && setView(null));
    };
    load();
    const timer = setInterval(load, 5000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [projectID, sessionID]);

  const offerable = rewindableTurns(view?.checkpoints ?? []);

  const choose = useCallback(
    (turn: number) => {
      setTarget(turn);
      setPreview(null);
      setError(null);
      setLoading(true);
      api
        .rewindPreview(projectID, sessionID, turn)
        .then((p) => {
          setPreview(p);
          setOpen(true);
        })
        .catch((e: Error) => setError(e.message))
        .finally(() => setLoading(false));
    },
    [projectID, sessionID],
  );

  const confirm = useCallback(() => {
    if (target === null) return;
    setWorking(true);
    setError(null);
    api
      .rewind(projectID, sessionID, target)
      .then((v) => {
        setView(v);
        setOpen(false);
        onRewound();
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setWorking(false));
  }, [projectID, sessionID, target, onRewound]);

  if (!view?.available || offerable.length === 0) return null;

  return (
    <>
      <div className="flex items-center gap-1">
        {offerable.map((turn) => (
          <button
            key={turn}
            type="button"
            disabled={loading}
            onClick={() => choose(turn)}
            title={`Rewind the workspace to before turn ${turn}`}
            aria-label={`Rewind to before turn ${turn}`}
            className="flex size-6 items-center justify-center rounded-full text-text-tertiary hover:bg-bg-code hover:text-text-primary disabled:opacity-40"
          >
            {loading && target === turn ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <span className="text-[10px] leading-none tabular-nums">{turn}</span>
            )}
          </button>
        ))}
        <span className="flex items-center gap-1 text-[10px] text-text-tertiary" aria-hidden>
          <History className="size-3" />
          rewind
        </span>
      </div>

      {error && !open && (
        <span className="text-[10px] text-danger" role="alert">
          {error}
        </span>
      )}

      <AlertDialog open={open} onOpenChange={setOpen}>
        <AlertDialogContent className="max-w-[560px]">
          <AlertDialogHeader>
            <AlertDialogTitle>Rewind to before turn {preview?.turn ?? target}?</AlertDialogTitle>
            <AlertDialogDescription>
              {preview?.willLose
                ? "The workspace has uncommitted changes right now. The current state is saved first, so this can be undone."
                : "The workspace will be restored to how it was before that turn."}
            </AlertDialogDescription>
          </AlertDialogHeader>

          {preview && (
            <div className="max-h-[240px] overflow-auto rounded-md border border-border bg-bg-code p-2">
              <pre className="whitespace-pre-wrap break-words text-[11px] leading-[15px] text-text-secondary">
                {preview.diff.trim() || "No file changes."}
              </pre>
            </div>
          )}

          {error && (
            <p className="text-[11px] text-danger" role="alert">
              {error}
            </p>
          )}

          <AlertDialogFooter>
            <AlertDialogCancel disabled={working}>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirm} disabled={working || !preview}>
              {working ? (
                <Loader2 className="size-3 animate-spin" />
              ) : (
                "Rewind workspace"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
});
