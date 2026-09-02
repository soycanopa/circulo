import { useEffect } from "react";

import { AppShell } from "@/components/layout/AppShell";
import Transcript from "@/features/chat/Transcript";
import { Composer } from "@/features/chat/Composer";
import { ProjectBanner, ReconnectBar } from "@/features/connection/Banners";
import {
  NewChatButton,
  ProjectsSection,
  SidebarFooter,
} from "@/features/projects/SidebarSections";
import { SessionsSection } from "@/features/sessions/SessionsSection";
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
    <AppShell
      sidebar={
        <>
          <NewChatButton />
          <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
            <ProjectsSection />
            <SessionsSection />
          </div>
          <SidebarFooter />
        </>
      }
    >
      <ReconnectBar />
      {!activeProject ? (
        <EmptyState />
      ) : (
        <>
          {/* Banners are informational strips: the chat stays mounted while
              the adapter starts, retries or recovers (docs/flow.md §9). */}
          <ProjectBanner project={activeProject} />
          <div className="flex min-h-0 flex-1 flex-col">
            {session ? (
              <Transcript session={session} />
            ) : (
              <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-1">
                <div className="text-[15px] font-medium">New chat</div>
                <div className="text-[13px] text-muted-foreground">{activeProject.path}</div>
              </div>
            )}
            <Composer />
          </div>
        </>
      )}
    </AppShell>
  );
}
