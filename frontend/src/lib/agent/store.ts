/**
 * Global store: projects, active selection, session lists, and chat state.
 * One EventSource dispatches into this store; components subscribe by slice.
 */

import { create } from "zustand";

import { api } from "./api";
import {
  applyEvent,
  emptyChatState,
  mergeHydrated,
  addOptimisticUserMessage,
  type ChatState,
} from "./reducer";
import type {
  AdapterStatus,
  Envelope,
  ProjectView,
  Session,
  SessionRemovedEvent,
  SessionStatusEvent,
  SessionUpdatedEvent,
} from "./protocol";

export type ConnectionState = "connecting" | "open" | "reconnecting";

interface AppStore {
  // Connection (webview ↔ relay bridge)
  connection: ConnectionState;

  // Projects
  projects: ProjectView[];
  activeProjectId: string | null;
  sessionsByProject: Record<string, Session[]>;

  // Active session
  activeSessionId: string | null;
  chat: ChatState;

  // Composer picker data
  metaAgents: { name: string; description?: string; mode?: string }[];
  metaModels: { id: string; name?: string; provider: string }[];
  selectedAgent: string;
  selectedModel: string; // "provider:model"

  setConnection: (s: ConnectionState) => void;
  refreshProjects: () => Promise<void>;
  refreshSessions: (projectID: string) => Promise<void>;
  addProject: (path: string, mode: "managed" | "attach", url?: string) => Promise<void>;
  removeProject: (projectID: string) => Promise<void>;
  setActiveProject: (projectID: string | null) => void;
  newSession: (projectID: string, title?: string) => Promise<string | null>;
  openSession: (projectID: string, sessionID: string) => Promise<void>;
  deleteSession: (projectID: string, sessionID: string) => Promise<void>;
  loadMeta: (projectID: string) => Promise<void>;
  setSelectedAgent: (a: string) => void;
  setSelectedModel: (m: string) => void;
  sendPrompt: (text: string) => Promise<void>;
  abort: () => Promise<void>;
  replyPermission: (permissionID: string, response: "once" | "always" | "reject") => Promise<void>;
  /** Feed one neutral envelope into the store (SSE or tests). */
  dispatch: (env: Envelope) => void;
  /** Full resync after (re)connect (flow.md §7). */
  resync: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  connection: "connecting",
  projects: [],
  activeProjectId: null,
  sessionsByProject: {},
  activeSessionId: null,
  chat: emptyChatState,
  metaAgents: [],
  metaModels: [],
  selectedAgent: "build",
  selectedModel: "",

  setConnection: (connection) => set({ connection }),

  refreshProjects: async () => {
    const projects = await api.listProjects();
    set({ projects });
    // Load sessions for each known project (sidebar lists).
    await Promise.all(
      projects.map((p) =>
        get()
          .refreshSessions(p.id)
          .catch(() => undefined),
      ),
    );
  },

  refreshSessions: async (projectID) => {
    const sessions = await api.listSessions(projectID);
    set((s) => ({
      sessionsByProject: { ...s.sessionsByProject, [projectID]: sessions },
    }));
  },

  addProject: async (path, mode, url) => {
    const pv = await api.addProject(path, mode, url);
    set((s) => ({
      projects: [...s.projects.filter((p) => p.id !== pv.id), pv],
      activeProjectId: s.activeProjectId ?? pv.id,
    }));
    get().setActiveProject(pv.id);
  },

  removeProject: async (projectID) => {
    await api.removeProject(projectID);
    set((s) => {
      const projects = s.projects.filter((p) => p.id !== projectID);
      const sessionsByProject = { ...s.sessionsByProject };
      delete sessionsByProject[projectID];
      return {
        projects,
        sessionsByProject,
        activeProjectId: s.activeProjectId === projectID ? projects[0]?.id ?? null : s.activeProjectId,
        activeSessionId: null,
      };
    });
  },

  setActiveProject: (projectID) => {
    set({ activeProjectId: projectID, activeSessionId: null });
    if (projectID) {
      get()
        .refreshSessions(projectID)
        .catch(() => undefined);
      get()
        .loadMeta(projectID)
        .catch(() => undefined);
    }
  },

  newSession: async (projectID, title) => {
    try {
      const session = await api.createSession(projectID, title);
      await get().refreshSessions(projectID);
      await get().openSession(projectID, session.id);
      return session.id;
    } catch (e) {
      console.error("newSession failed", e);
      return null;
    }
  },

  openSession: async (projectID, sessionID) => {
    set({ activeSessionId: sessionID });
    // Hydrate history into the chat reducer (flow.md §5).
    try {
      const history = await api.messages(projectID, sessionID, 0);
      set((s) => ({ chat: mergeHydrated(s.chat, sessionID, history) }));
    } catch (e) {
      console.error("hydrate failed", e);
    }
  },

  deleteSession: async (projectID, sessionID) => {
    await api.deleteSession(projectID, sessionID);
    set((s) => {
      const chat = { ...s.chat };
      delete chat.sessions[sessionID];
      return {
        chat: { sessions: chat.sessions },
        activeSessionId: s.activeSessionId === sessionID ? null : s.activeSessionId,
      };
    });
    await get().refreshSessions(projectID);
  },

  loadMeta: async (projectID) => {
    try {
      const meta = await api.meta(projectID);
      const primary = meta.agents.find((a) => a.mode === "primary") ?? meta.agents[0];
      set((s) => ({
        metaAgents: meta.agents,
        metaModels: meta.models,
        selectedAgent: s.selectedAgent || primary?.name || "build",
        selectedModel: s.selectedModel || firstModelKey(meta.models),
      }));
    } catch (e) {
      console.error("loadMeta failed", e);
    }
  },

  setSelectedAgent: (selectedAgent) => set({ selectedAgent }),
  setSelectedModel: (selectedModel) => set({ selectedModel }),

  sendPrompt: async (text) => {
    const { activeProjectId, activeSessionId, selectedAgent, selectedModel } = get();
    if (!activeProjectId || !text.trim()) return;
    let sessionID = activeSessionId;
    if (!sessionID) {
      sessionID = await get().newSession(activeProjectId);
      if (!sessionID) return;
    }
    const clientID = `client_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    set((s) => ({
      chat: addOptimisticUserMessage(s.chat, sessionID!, clientID, text),
    }));
    const [provider, model] = selectedModel.split(":");
    await api
      .prompt(activeProjectId, sessionID, {
        text,
        agent: selectedAgent,
        provider: provider || undefined,
        model: model || undefined,
      })
      .catch((e) => console.error("prompt failed", e));
  },

  abort: async () => {
    const { activeProjectId, activeSessionId } = get();
    if (!activeProjectId || !activeSessionId) return;
    await api.abort(activeProjectId, activeSessionId).catch((e) => console.error(e));
  },

  replyPermission: async (permissionID, response) => {
    const { activeProjectId, activeSessionId } = get();
    if (!activeProjectId || !activeSessionId) return;
    await api
      .replyPermission(activeProjectId, activeSessionId, permissionID, response)
      .catch((e) => console.error(e));
  },

  dispatch: (env) => {
    // Session bookkeeping outside the chat reducer.
    switch (env.type) {
      case "adapter.status": {
        const p = env.payload as AdapterStatus;
        const known = get().projects.some((x) => x.id === p.projectID);
        if (!known) {
          // Project added by another client (or before a reconnect): refetch.
          void get().refreshProjects().catch(() => undefined);
          break;
        }
        set((s) => ({
          projects: s.projects.map((x) =>
            x.id === p.projectID ? { ...x, status: p.state, detail: p.detail } : x,
          ),
        }));
        break;
      }
      case "session.updated": {
        const p = env.payload as SessionUpdatedEvent;
        set((s) => {
          const list = s.sessionsByProject[p.projectID] ?? [];
          const idx = list.findIndex((x) => x.id === p.session.id);
          const next =
            idx === -1
              ? [p.session, ...list]
              : list.map((x, i) => (i === idx ? p.session : x));
          return { sessionsByProject: { ...s.sessionsByProject, [p.projectID]: next } };
        });
        break;
      }
      case "session.removed": {
        const p = env.payload as SessionRemovedEvent;
        set((s) => {
          const sessionsByProject: Record<string, Session[]> = {};
          for (const [pid, list] of Object.entries(s.sessionsByProject)) {
            sessionsByProject[pid] = list.filter((x) => x.id !== p.sessionID);
          }
          return {
            sessionsByProject,
            activeSessionId: s.activeSessionId === p.sessionID ? null : s.activeSessionId,
          };
        });
        break;
      }
      case "session.status": {
        const p = env.payload as SessionStatusEvent;
        // Surface turn-end by refreshing the session list order (title/updated).
        if (p.status === "idle") {
          const { activeProjectId } = get();
          if (activeProjectId) {
            get()
              .refreshSessions(activeProjectId)
              .catch(() => undefined);
          }
        }
        break;
      }
    }
    set((s) => ({ chat: applyEvent(s.chat, env) }));
  },

  resync: async () => {
    const { refreshProjects, activeProjectId, activeSessionId } = get();
    await refreshProjects().catch(() => undefined);
    if (activeProjectId) {
      await get().refreshSessions(activeProjectId).catch(() => undefined);
      if (activeSessionId) {
        const history = await api
          .messages(activeProjectId, activeSessionId, 0)
          .catch(() => null);
        if (history) {
          set((s) => ({ chat: mergeHydrated(s.chat, activeSessionId, history) }));
        }
      }
    }
  },
}));

function firstModelKey(models: { id: string; provider: string }[]): string {
  if (models.length === 0) return "";
  const preferred =
    models.find((m) => m.provider === "anthropic") ?? models.find((m) => /claude|gpt|glm/i.test(m.id)) ?? models[0];
  return `${preferred.provider}:${preferred.id}`;
}
