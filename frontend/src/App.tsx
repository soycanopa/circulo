import { useEffect } from "react";

import { AppShell } from "@/components/layout/AppShell";
import Transcript from "@/features/chat/Transcript";
import { ChatAppBar } from "@/features/chat/ChatAppBar";
import { Composer } from "@/features/chat/Composer";
import { ProjectBanner, ReconnectBar } from "@/features/connection/Banners";
import { SidebarHeader, SidebarFooter } from "@/features/projects/SidebarSections";
import { SessionsSection } from "@/features/sessions/SessionsSection";
import { Button } from "@/components/ui/button";
import { TerminalPanel } from "@/features/terminal/TerminalPanel";
import { connectSse, type SseHandle } from "@/lib/agent/sse";
import { useAppStore } from "@/lib/agent/store";
import { useShortcuts } from "@/lib/useShortcuts";
import { SquareTerminal } from "lucide-react";
import type { Envelope } from "@/lib/agent/protocol";
import { cn } from "@/lib/utils";

/**
 * Empty-state per the Circulo Paper frame: sparkle chip, title and subtext.
 * With zero projects an Add project CTA is appended (the design assumes a
 * project exists).
 */
function GeneralEmptyState({ hasProjects }: { hasProjects: boolean }) {
  const addProject = useAppStore((s) => s.addProject);
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 pb-6">
      <div className="flex size-[44px] items-center justify-center rounded-[14px] border border-border-strong bg-bg-code">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
          <path
            d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z"
            stroke="var(--color-text-secondary)"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </div>
      <div className="text-xl/loose font-semibold tracking-tight text-text-primary">
        What are we making today?
      </div>
      <div className="text-md/relaxed text-text-tertiary">
        Describe it in your own words — Circulo handles the rest.
      </div>
      {!hasProjects && (
        <Button
          variant="secondary"
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
      )}
    </div>
  );
}

export default function App() {
  const projects = useAppStore((s) => s.projects);
  const activeProjectId = useAppStore((s) => s.activeProjectId);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const chat = useAppStore((s) => s.chat);
  useShortcuts();

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
  const terminalOpen = useAppStore((s) => s.terminalOpen);
  const toggleTerminal = useAppStore((s) => s.toggleTerminal);

  return (
    <AppShell
      sidebar={
        <>
          <SidebarHeader />
          <SessionsSection />
          <SidebarFooter />
        </>
      }
      title={
        session ? (
          <ChatAppBar project={activeProject!} session={session} />
        ) : (
          <span className="text-xs leading-[14px] text-text-secondary">Circulo</span>
        )
      }
      actions={
        activeProject ? (
          <button
            aria-label="Toggle terminal"
            title="Terminal"
            className={cn(
              "flex size-7 shrink-0 items-center justify-center rounded-full border text-muted-foreground hover:text-foreground",
              terminalOpen
                ? "border-accent-cir/60 bg-accent-cir/15 text-text-primary"
                : "border-border-strong bg-bg-code",
            )}
            onClick={toggleTerminal}
          >
            <SquareTerminal className="size-3.5" />
          </button>
        ) : undefined
      }
    >
      <ReconnectBar />
      {/* The app bar carries session context; banners below it are
          informational strips only — the chat stays mounted while the
          adapter starts, retries or recovers (docs/flow.md §9). */}
      {activeProject && <ProjectBanner project={activeProject} />}
      <div className="flex min-h-0 flex-1 flex-col">
        {session ? (
          <Transcript session={session} />
        ) : (
          <GeneralEmptyState hasProjects={projects.length > 0} />
        )}
        {/* Terminal zone: below the chat area, ABOVE the composer (owner
            call). Grows/shrinks with a grid-rows animation. */}
        {activeProject && (
          <div
            className={cn(
              "grid shrink-0 transition-[grid-template-rows] duration-300 ease-out",
              !terminalOpen && "invisible",
            )}
            style={{
              gridTemplateRows: terminalOpen ? "minmax(0, 1fr)" : "0fr",
            }}
          >
            <div className="min-h-0 overflow-hidden">
              <TerminalPanel projectID={activeProject.id} />
            </div>
          </div>
        )}
        <Composer />
      </div>
    </AppShell>
  );
}
