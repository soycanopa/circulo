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
  ModelInfo,
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
  metaModels: ModelInfo[];
  selectedAgent: string;
  selectedModel: string; // "provider:model"
  /** Reasoning-effort variant for the selected model ("" = default). */
  selectedVariant: string;
  setSelectedVariant: (v: string) => void;
  sessionSearch: string;
  setSessionSearch: (q: string) => void;

  setConnection: (s: ConnectionState) => void;
  /** Sidebar visibility (Cmd+B, ux.md §6). */
  sidebarOpen: boolean;
  toggleSidebar: () => void;
  refreshProjects: () => Promise<void>;
  refreshSessions: (projectID: string) => Promise<void>;
  addProject: (path: string, mode: "managed" | "attach", url?: string) => Promise<void>;
  removeProject: (projectID: string) => Promise<void>;
  setActiveProject: (projectID: string | null) => void;
  newSession: (projectID: string, title?: string, branch?: string) => Promise<string | null>;
  openSession: (projectID: string, sessionID: string) => Promise<void>;
  closeSession: () => void;
  deleteSession: (projectID: string, sessionID: string) => Promise<void>;
  loadMeta: (projectID: string) => Promise<void>;
  setSelectedAgent: (a: string) => void;
  setSelectedModel: (m: string) => void;
  sendPrompt: (text: string, target?: { projectID: string; branch?: string }) => Promise<void>;
  abort: () => Promise<void>;
  replyPermission: (permissionID: string, response: "once" | "always" | "reject") => Promise<void>;
  /** Answer a pending form (question tool): field key → value(s). */
  replyForm: (formID: string, answer: Record<string, string | string[]>) => Promise<void>;
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
  selectedVariant: "",
  setSelectedVariant: (selectedVariant) => set({ selectedVariant }),
  sessionSearch: "",
  setSessionSearch: (sessionSearch) => set({ sessionSearch }),

  setConnection: (connection) => set({ connection }),

  sidebarOpen: true,
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),

  refreshProjects: async () => {
    const projects = await api.listProjects();
    set((s) => ({
      projects,
      // First load with a stored project: activate it so the app lands in
      // the chat view instead of the welcome screen.
      activeProjectId:
        s.activeProjectId ?? (projects.length > 0 ? projects[0].id : null),
    }));
    await Promise.all(
      projects.map((p) =>
        get()
          .refreshSessions(p.id)
          .catch(() => undefined),
      ),
    );
    const { activeProjectId: id } = get();
    if (id) void get().loadMeta(id).catch(() => undefined);
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

  newSession: async (projectID, title, branch) => {
    try {
      const session = await api.createSession(projectID, title, branch);
      await get().refreshSessions(projectID);
      await get().openSession(projectID, session.id);
      return session.id;
    } catch (e) {
      console.error("newSession failed", e);
      // Surface it: a silent null made the composer swallow the draft with
      // no feedback when the server was still starting.
      set((s) => {
        const active = s.activeSessionId ? s.chat.sessions[s.activeSessionId] : undefined;
        if (!active) return s;
        return {
          chat: applyEvent(s.chat, {
            type: "session.error",
            payload: {
              projectID,
              sessionID: active.session.id,
              error: { name: "SessionCreateFailed", message: String(e) },
            },
          }),
        };
      });
      return null;
    }
  },

  openSession: async (projectID, sessionID) => {
    set({ activeSessionId: sessionID });
    // Seed the chat session from the sidebar entry: without it, a session
    // with empty history never enters chat.sessions and the app bar keeps
    // saying "New chat".
    set((s) => {
      if (s.chat.sessions[sessionID]) return s;
      const meta = (s.sessionsByProject[projectID] ?? []).find((x) => x.id === sessionID);
      // Seed even without list metadata (boot fetch may have failed): the
      // optimistic user bubble depends on the session entry existing.
      const session =
        meta ?? { id: sessionID, title: "", timeCreated: Date.now(), timeUpdated: Date.now() };
      return {
        chat: applyEvent(s.chat, {
          type: "session.updated",
          payload: { projectID, session },
        }),
      };
    });
    // Hydrate history into the chat reducer (flow.md §5).
    try {
      const history = await api.messages(projectID, sessionID, 0);
      set((s) => ({ chat: mergeHydrated(s.chat, sessionID, history) }));
    } catch (e) {
      console.error("hydrate failed", e);
    }
  },

  closeSession: () => set({ activeSessionId: null }),

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
      const defaultKey =
        meta.defaultProvider && meta.defaultModel
          ? `${meta.defaultProvider}:${meta.defaultModel}`
          : "";
      const fallbackKey = firstModelKey(meta.models);
      const chosen =
        defaultKey && meta.models.some((m) => `${m.provider}:${m.id}` === defaultKey)
          ? defaultKey
          : fallbackKey;
      set((s) => ({
        metaAgents: meta.agents,
        metaModels: meta.models,
        selectedAgent: s.selectedAgent || primary?.name || "build",
        selectedModel: s.selectedModel || chosen,
      }));
    } catch (e) {
      console.error("loadMeta failed", e);
    }
  },

  setSelectedAgent: (selectedAgent) => set({ selectedAgent }),
  setSelectedModel: (selectedModel) =>
    set((s) => {
      // Drop the effort variant unless the new model offers it.
      const model = s.metaModels.find((m) => `${m.provider}:${m.id}` === selectedModel);
      const keep = model?.variants?.includes(s.selectedVariant) ?? false;
      return { selectedModel, selectedVariant: keep ? s.selectedVariant : "" };
    }),

  sendPrompt: async (text, target) => {
    let { activeProjectId, activeSessionId } = get();
    const { selectedAgent, selectedModel, selectedVariant } = get();
    if (!activeProjectId || !text.trim()) return;
    // A new session may target a different project (composer picker): the
    // active project follows so the sidebar and context stay coherent.
    if (!activeSessionId && target?.projectID && target.projectID !== activeProjectId) {
      activeProjectId = target.projectID;
      set({ activeProjectId });
    }
    let sessionID = activeSessionId;
    if (!sessionID) {
      sessionID = await get().newSession(activeProjectId, undefined, target?.branch);
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
        variant: selectedVariant || undefined,
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

  replyForm: async (formID, answer) => {
    const { activeProjectId, activeSessionId } = get();
    if (!activeProjectId || !activeSessionId) return;
    await api
      .replyForm(activeProjectId, activeSessionId, formID, answer)
      .then(() => {
        // Close the card immediately; the form.replied echo is idempotent
        // (reducer drops unknown ids) if it arrives.
        set((s) => ({
          chat: applyEvent(s.chat, {
            type: "form.resolved",
            payload: { projectID: activeProjectId, sessionID: activeSessionId, formID },
          }),
        }));
      })
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
        // The boot requests (sessions/meta) can 503 while a managed server is
        // still starting; when it comes up, resync. Sessions resync for the
        // project regardless of selection (the sidebar lists them all); meta
        // only for the active one and only while still empty.
        if (p.state === "running") {
          const resync = () => {
            void get().refreshSessions(p.projectID).catch(() => undefined);
            if (p.projectID === get().activeProjectId && get().metaModels.length === 0) {
              void get().loadMeta(p.projectID).catch(() => undefined);
            }
          };
          resync();
          // Boot race: the running event can replay before refreshProjects
          // sets activeProjectId — one delayed pass covers it.
          setTimeout(resync, 800);
        }
        break;
      }
      case "session.updated": {
        const p = env.payload as SessionUpdatedEvent;
        set((s) => {
          const list = s.sessionsByProject[p.projectID] ?? [];
          const idx = list.findIndex((x) => x.id === p.session.id);
          if (idx === -1) {
            // v2 partial patches never create list entries: wait for the
            // authoritative session (created event or list refresh).
            if (!p.session.timeCreated) return s;
            return {
              sessionsByProject: {
                ...s.sessionsByProject,
                [p.projectID]: [p.session, ...list],
              },
            };
          }
          // Merge-patch: keep fields the patch does not carry.
          const prev = list[idx];
          const merged = {
            ...prev,
            title: p.session.title || prev.title,
            timeCreated: p.session.timeCreated || prev.timeCreated,
            timeUpdated: p.session.timeUpdated || prev.timeUpdated,
          };
          const next = list.map((x, i) => (i === idx ? merged : x));
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
          const { activeProjectId, activeSessionId } = get();
          if (activeProjectId) {
            get()
              .refreshSessions(activeProjectId)
              .catch(() => undefined);
            // Heal any live event drops at turn end: the user echo is a
            // single part.updated (no replace frames to repair it), so the
            // transcript refetches history once the turn settles.
            if (activeSessionId) {
              api
                .messages(activeProjectId, activeSessionId, 0)
                .then((history) =>
                  set((s) => ({ chat: mergeHydrated(s.chat, activeSessionId, history) })),
                )
                .catch(() => undefined);
            }
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
