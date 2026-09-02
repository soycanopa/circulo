/** Agent task list (todo.updated) — Circulo design: Done / In progress / Pending. */

import { memo } from "react";
import { Check, Loader2 } from "lucide-react";

import type { Task } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

function TaskRow({ task }: { task: Task }) {
  const done = task.status === "completed";
  const inProgress = task.status === "in_progress" || task.status === "in-progress";
  return (
    <div className="flex items-center gap-2.5 px-3 py-2">
      {done ? (
        <span className="flex size-[18px] shrink-0 items-center justify-center rounded-full bg-success">
          <Check className="size-2.5 text-white" strokeWidth={3} />
        </span>
      ) : inProgress ? (
        <span className="flex size-[18px] shrink-0 items-center justify-center rounded-sm bg-accent-indigo">
          <Loader2 className="size-2.5 animate-spin text-white" />
        </span>
      ) : (
        <span className="size-[18px] shrink-0 rounded-full border border-border-strong" />
      )}
      <span
        className={cn(
          "min-w-0 flex-1 truncate text-[13px]",
          done ? "text-muted-foreground line-through" : "text-foreground",
        )}
      >
        {task.content}
      </span>
      <span
        className={cn(
          "shrink-0 text-[12px]",
          done && "text-success",
          inProgress && "text-accent-indigo",
          !done && !inProgress && "text-text-tertiary",
        )}
      >
        {done ? "Done" : inProgress ? "In progress" : "Pending"}
      </span>
    </div>
  );
}

export const TaskList = memo(function TaskList({ tasks }: { tasks: Task[] }) {
  if (tasks.length === 0) return null;
  return (
    <div className="rounded-lg border border-border bg-bg-code py-1">
      {tasks.map((t, i) => (
        <TaskRow key={t.id ?? i} task={t} />
      ))}
    </div>
  );
});
