import { useEffect } from "react";

import { Composer } from "@/components/chat/Composer";
import { Transcript } from "@/components/chat/Transcript";
import { Sidebar } from "@/components/sidebar/Sidebar";
import { Button } from "@/components/ui/button";
import { connectSse, type SseHandle } from "@/lib/agent/sse";
import { useAppStore } from "@/lib/agent/store";
import type { Envelope } from "@/lib/agent/protocol";

function EmptyState() {
  const addProject = useAppStore((s) => s.addProject);
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3">
      <h1 className="text-2xl font-semibold tracking-tight">circuloGo</h1>
      <p className="text-sm text-muted-foreground">
        One window for your coding agents — projects, live sessions, permissions.
      </p>
      <Button
        className="mt-2"
        onClick={() => {
          // Same flow as the sidebar's Add project (native folder picker).
          void import("@/bindings/circulogo/internal/appservice/dialog").then(
            async ({ PickFolder }) => {
              const path = await PickFolder().catch(() => "");
              if (path) await addProject(path, "managed");
            },
          );
        }}
      >
        Add project
      </Button>
      <p className="mt-4 max-w-md text-center text-[12px] text-muted-foreground/70">
        Local-first: agents run on 127.0.0.1 over HTTP+SSE. No ACP, no cloud.
      </p>
    </div>
  );
}

function ReconnectBar() {
  const connection = useAppStore((s) => s.connection);
  if (connection !== "reconnecting") return null;
  return (
    <div className="border-b border-amber-500/40 bg-amber-500/10 px-4 py-1 text-center text-[12px] text-amber-600 dark:text-amber-400">
      Reconnecting… showing last known state
    </div>
  );
}

export default function App() {
  const projects = useAppStore((s) => s.projects);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const chat = useAppStore((s) => s.chat);

  useEffect(() => {
    const store = useAppStore.getState();
    const handle: SseHandle = connectSse(
      (env: Envelope) => store.dispatch(env),
      (state) => useAppStore.getState().setConnection(state),
      () => void useAppStore.getState().resync(),
    );
    void store.refreshProjects().catch((e) => console.error("initial load", e));
    return () => handle.close();
  }, []);

  const activeProject = projects.find((p) => p.id === activeProjectId);
  const session = activeSessionId ? chat.sessions[activeSessionId] : undefined;

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <Sidebar />
      <main className="flex min-w-0 flex-1 flex-col">
        <div
          className="h-11 shrink-0 select-none"
          style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
        />
        <ReconnectBar />
        {!activeProject ? (
          <EmptyState />
        ) : activeProject.status === "error" ? (
          <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3">
            <div className="text-[15px] font-medium text-red-500">
              Agent server failed
            </div>
            <div className="max-w-lg break-words text-center text-[13px] text-muted-foreground">
              {activeProject.detail || "Unknown error"}
            </div>
          </div>
        ) : activeProject.status === "starting" ? (
          <div className="flex min-h-0 flex-1 items-center justify-center text-[13px] text-muted-foreground">
            Starting OpenCode…
          </div>
        ) : (
          <>
            {session ? (
              <Transcript session={session} />
            ) : (
              <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-1">
                <div className="text-[15px] font-medium">New chat</div>
                <div className="text-[13px] text-muted-foreground">
                  {activeProject.path}
                </div>
              </div>
            )}
            <Composer />
          </>
        )}
      </main>
    </div>
  );
}
