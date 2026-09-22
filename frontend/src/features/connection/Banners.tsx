/**
 * Project/connection banners. These are informationals only: they render as a
 * strip ABOVE the workspace and never unmount the chat (the old behavior
 * replaced the transcript with the banner, which read as "the chat left and
 * came back").
 */

import { useAppStore } from "@/lib/agent/store";
import type { ProjectView } from "@/lib/agent/protocol";

export function ProjectBanner({ project }: { project: ProjectView }) {
  if (project.status === "running") return null;
  if (project.status === "starting") {
    return (
      <div className="border-b border-border bg-muted/40 px-4 py-1.5 text-center text-[12px] text-muted-foreground">
        Starting {project.provider === "omp" ? "omp" : "OpenCode"}…
      </div>
    );
  }
  if (project.status === "error") {
    return (
      <div className="border-b border-red-500/40 bg-red-500/5 px-4 py-1.5 text-center text-[12px] text-red-600 dark:text-red-400">
        Agent server problem: {project.detail || "unknown error"} — retrying automatically
      </div>
    );
  }
  return null;
}

export function ReconnectBar() {
  const connection = useAppStore((s) => s.connection);
  if (connection !== "reconnecting") return null;
  return (
    <div className="border-b border-amber-500/40 bg-amber-500/10 px-4 py-1 text-center text-[12px] text-amber-600 dark:text-amber-400">
      Reconnecting… showing last known state
    </div>
  );
}
