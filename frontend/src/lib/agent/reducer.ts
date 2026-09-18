/**
 * Pure event → state reducer for chat sessions (docs/trd.md §6).
 *
 * Identity is (sessionID, messageID, partID). `part.updated` is a full
 * replacement and the source of truth; `part.delta` is a best-effort
 * fast path. One reducer serves both live streaming and history
 * hydration, which is what keeps rendered history identical to live.
 */

import type {
  Envelope,
  MessageInfo,
  Part,
  PermissionRequest,
  Session,
  SessionErroredEvent,
  SessionStatusEvent,
  SessionUpdatedEvent,
  Task,
  TodoUpdatedEvent,
} from "./protocol";

export interface MessageRecord {
  info: MessageInfo;
  parts: Part[];
}

export interface SessionState {
  session: Session;
  status: "idle" | "busy" | "retry";
  retry?: { attempt: number; message: string; nextAt: number };
  messages: MessageRecord[];
  /** Unresolved permission requests, oldest first. */
  permissions: PermissionRequest[];
  /** Agent task list (todo.updated replaces it wholesale). */
  tasks: Task[];
  /** Last terminal turn error, cleared on the next user prompt. */
  lastError?: { name: string; message: string };
}

export interface ChatState {
  sessions: Record<string, SessionState>;
}

export const emptyChatState: ChatState = { sessions: {} };

function ensureSession(state: ChatState, session: Session): SessionState {
  let s = state.sessions[session.id];
  if (!s) {
    s = {
      session,
      status: "idle",
      messages: [],
      permissions: [],
      tasks: [],
    };
    state.sessions[session.id] = s;
  } else {
    s.session = session;
  }
  return s;
}

/**
 * Message updates MUST replace the MessageRecord object (never mutate it):
 * UserMessage/AssistantTurn are React.memo'd on the record, so an in-place
 * mutation leaves the reference equal and the bubble never re-renders —
 * that was the empty-user-bubble bug.
 */

function ensureMessage(s: SessionState, info: MessageInfo): void {
  const idx = s.messages.findIndex((x) => x.info.id === info.id);
  if (idx === -1) {
    s.messages.push({ info, parts: [] });
  } else {
    // Metadata update; content lives in parts.
    s.messages[idx] = { ...s.messages[idx], info };
  }
}

/**
 * Locates a message by id or creates a metadata-stub to update. Stubs use
 * created: 0 so mergeHydrated can detect and fill real metadata; they never
 * overwrite existing info (a part.updated for a user message must not flip
 * its role).
 */
function upsertMessagePart(
  s: SessionState,
  messageID: string,
  sessionID: string,
  part: Part,
): void {
  const idx = s.messages.findIndex((x) => x.info.id === messageID);
  if (idx === -1) {
    s.messages.push({
      info: { id: messageID, sessionID, role: "assistant", created: 0, completed: 0 },
      parts: [part],
    });
    return;
  }
  const m = s.messages[idx];
  const parts = [...m.parts];
  const pIdx = parts.findIndex((p) => p.id === part.id);
  if (pIdx === -1) parts.push(part);
  else parts[pIdx] = part;
  s.messages[idx] = { ...m, parts };
}

function applyDeltaToPart(
  s: SessionState,
  messageID: string,
  partID: string,
  field: string,
  delta: string,
): void {
  if (field !== "text" && field !== "reasoning") return;
  const idx = s.messages.findIndex((x) => x.info.id === messageID);
  if (idx === -1) return;
  const m = s.messages[idx];
  let touched = false;
  const parts = m.parts.map((part) => {
    if (part.id !== partID || part.type !== field) return part;
    touched = true;
    return { ...part, [field]: (part[field] ?? "") + delta };
  });
  if (touched) s.messages[idx] = { ...m, parts };
}

/** Reducer entry: returns a NEW ChatState (immutable update for React). */
export function applyEvent(state: ChatState, env: Envelope): ChatState {
  switch (env.type) {
    case "session.updated": {
      const p = env.payload as SessionUpdatedEvent;
      const prev = state.sessions[p.session.id];
      if (!prev) {
        // v2 partial patches (session.renamed carries only the title) must
        // not create sessions with empty metadata: ignore them until the
        // full session arrives (created event or hydration).
        if (!p.session.timeCreated) return state;
        const next = { sessions: { ...state.sessions } };
        ensureSession(next, p.session);
        return next;
      }
      // Existing session: merge-patch — v2 patches (renamed) carry only the
      // changed fields; empty strings and zero times keep existing values.
      const session = { ...prev.session };
      if (p.session.title) session.title = p.session.title;
      if (p.session.timeCreated) session.timeCreated = p.session.timeCreated;
      if (p.session.timeUpdated) session.timeUpdated = p.session.timeUpdated;
      if (p.session.directory) session.directory = p.session.directory;
      const s: SessionState = { ...prev, session };
      return { sessions: { ...state.sessions, [p.session.id]: s } };
    }

    case "session.status": {
      const p = env.payload as SessionStatusEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = {
        ...prev,
        status: p.status,
        retry: p.retry,
        // A new busy turn clears the previous terminal error.
        lastError: p.status === "busy" ? undefined : prev.lastError,
      };
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "session.error": {
      const p = env.payload as SessionErroredEvent;
      if (!p.sessionID) return state;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = {
        ...prev,
        lastError: p.error,
      };
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "session.removed": {
      if (!(env.payload as { sessionID: string }).sessionID) return state;
      const { sessionID } = env.payload as { sessionID: string };
      if (!state.sessions[sessionID]) return state;
      const next = { ...state.sessions };
      delete next[sessionID];
      return { sessions: next };
    }

    case "message.updated": {
      const p = env.payload as import("./protocol").MessageUpdatedEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = { ...prev, messages: [...prev.messages] };
      ensureMessage(s, p.message);
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "part.updated": {
      const p = env.payload as import("./protocol").PartUpdatedEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) {
        // Parts can arrive before we saw the session (e.g. after reconnect
        // while another window created it). Create a stub keyed by id.
        const next = { sessions: { ...state.sessions } };
        const s = ensureSession(next, {
          id: p.sessionID,
          title: p.sessionID,
          timeCreated: 0,
          timeUpdated: 0,
        });
        upsertMessagePart(s, p.messageID, p.sessionID, p.part);
        return next;
      }
      const s: SessionState = { ...prev, messages: [...prev.messages] };
      upsertMessagePart(s, p.messageID, p.sessionID, p.part);
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "part.delta": {
      const p = env.payload as import("./protocol").PartDeltaEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = { ...prev, messages: [...prev.messages] };
      applyDeltaToPart(s, p.messageID, p.partID, p.field, p.delta);
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "todo.updated": {
      const p = env.payload as TodoUpdatedEvent;
      const prevState = state.sessions[p.sessionID];
      if (!prevState) return state;
      const s: SessionState = { ...prevState, tasks: p.tasks };
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "permission.request": {
      const p = env.payload as import("./protocol").PermissionRequestedEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      if (prev.permissions.some((x) => x.id === p.permission.id)) return state;
      const s: SessionState = {
        ...prev,
        permissions: [...prev.permissions, p.permission],
      };
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "permission.resolved": {
      const p = env.payload as import("./protocol").PermissionResolvedEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = {
        ...prev,
        permissions: prev.permissions.filter((x) => x.id !== p.permissionID),
      };
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    default:
      // Unknown event types are ignored by design (NFR-3).
      return state;
  }
}

/**
 * mergeHydrated folds history (GET …/messages) into a session that may
 * already hold live parts. Hydration only CREATES missing messages/parts —
 * live state wins on conflicts (flow.md §5 race rule).
 */
export function mergeHydrated(
  state: ChatState,
  sessionID: string,
  history: { info: MessageInfo; parts: Part[] }[],
): ChatState {
  let prev = state.sessions[sessionID];
  if (!prev) {
    // The session entry can be missing after a failed boot fetch — the user's
    // message must always show, so seed a minimal session for it.
    prev = {
      session: { id: sessionID, title: "", timeCreated: Date.now(), timeUpdated: Date.now() },
      status: "idle",
      messages: [],
      permissions: [],
      tasks: [],
    };
  }
  const s: SessionState = { ...prev, messages: [...prev.messages] };
  for (const h of history) {
    const idx = s.messages.findIndex((x) => x.info.id === h.info.id);
    if (idx === -1) {
      s.messages.push({ info: h.info, parts: [...h.parts] });
      continue;
    }
    const m = s.messages[idx];
    const info = m.info.created === 0 && h.info.created ? h.info : m.info;
    // Hydration is server truth: REPLACE parts we already hold (heals a
    // live tool row stuck on running after a lost success frame) and keep
    // live-only parts the server has not persisted yet.
    const byId = new Map(m.parts.map((p) => [p.id, p]));
    for (const p of h.parts) byId.set(p.id, p);
    s.messages[idx] = { ...m, info, parts: [...byId.values()] };
  }
  return { sessions: { ...state.sessions, [sessionID]: s } };
}

/** Optimistic user message, replaced by the server echo (flow.md §3). */
export function addOptimisticUserMessage(
  state: ChatState,
  sessionID: string,
  clientID: string,
  text: string,
): ChatState {
  let prev = state.sessions[sessionID];
  if (!prev) {
    // The session entry can be missing after a failed boot fetch — the user's
    // message must always show, so seed a minimal session for it.
    prev = {
      session: { id: sessionID, title: "", timeCreated: Date.now(), timeUpdated: Date.now() },
      status: "idle",
      messages: [],
      permissions: [],
      tasks: [],
    };
  }
  const s: SessionState = { ...prev, messages: [...prev.messages] };
  s.messages.push({
    info: {
      id: clientID,
      sessionID,
      role: "user",
      created: Date.now(),
      completed: 0,
    },
    parts: [{ id: `${clientID}:text`, type: "text", text }],
  });
  return { sessions: { ...state.sessions, [sessionID]: s } };
}
