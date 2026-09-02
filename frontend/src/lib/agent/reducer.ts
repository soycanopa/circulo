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
    };
    state.sessions[session.id] = s;
  } else {
    s.session = session;
  }
  return s;
}

function ensureMessage(s: SessionState, info: MessageInfo): MessageRecord {
  let m = s.messages.find((x) => x.info.id === info.id);
  if (!m) {
    m = { info, parts: [] };
    s.messages.push(m);
  } else {
    // Metadata update; content lives in parts.
    m.info = info;
  }
  return m;
}

/**
 * stubMessage finds a message by id or creates a metadata-stub. Stubs use
 * created: 0 so mergeHydrated can detect and fill real metadata; they never
 * overwrite existing info (a part.updated for a user message must not flip
 * its role).
 */
function stubMessage(s: SessionState, messageID: string, sessionID: string): MessageRecord {
  let m = s.messages.find((x) => x.info.id === messageID);
  if (!m) {
    m = {
      info: { id: messageID, sessionID, role: "assistant", created: 0, completed: 0 },
      parts: [],
    };
    s.messages.push(m);
  }
  return m;
}

function upsertPart(m: MessageRecord, part: Part): void {
  const idx = m.parts.findIndex((p) => p.id === part.id);
  if (idx === -1) {
    m.parts.push(part);
  } else {
    m.parts[idx] = part;
  }
}

function applyDeltaToPart(part: Part, field: string, delta: string): void {
  if (field !== "text" && field !== "reasoning") return;
  if (part.type !== field) return;
  part[field] = (part[field] ?? "") + delta;
}

/** Reducer entry: returns a NEW ChatState (immutable update for React). */
export function applyEvent(state: ChatState, env: Envelope): ChatState {
  switch (env.type) {
    case "session.updated": {
      const p = env.payload as SessionUpdatedEvent;
      if (!state.sessions[p.session.id]) {
        const next = { sessions: { ...state.sessions } };
        ensureSession(next, p.session);
        return next;
      }
      // Existing session: update in place (new object for referential equality).
      const prev = state.sessions[p.session.id];
      const s: SessionState = { ...prev, session: p.session };
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
        const m = stubMessage(s, p.messageID, p.sessionID);
        m.parts = [...m.parts];
        upsertPart(m, p.part);
        return next;
      }
      const s: SessionState = { ...prev, messages: [...prev.messages] };
      const m = stubMessage(s, p.messageID, p.sessionID);
      m.parts = [...m.parts];
      upsertPart(m, p.part);
      return { sessions: { ...state.sessions, [p.sessionID]: s } };
    }

    case "part.delta": {
      const p = env.payload as import("./protocol").PartDeltaEvent;
      const prev = state.sessions[p.sessionID];
      if (!prev) return state;
      const s: SessionState = { ...prev, messages: [...prev.messages] };
      const m = s.messages.find((x) => x.info.id === p.messageID);
      if (!m) return state;
      m.parts = m.parts.map((part) => {
        if (part.id !== p.partID) return part;
        const copy = { ...part };
        applyDeltaToPart(copy, p.field, p.delta);
        return copy;
      });
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
  const prev = state.sessions[sessionID];
  if (!prev) return state;
  const s: SessionState = { ...prev, messages: [...prev.messages] };
  for (const h of history) {
    const m = s.messages.find((x) => x.info.id === h.info.id);
    if (!m) {
      s.messages.push({ info: h.info, parts: [...h.parts] });
      continue;
    }
    if (m.info.created === 0 && h.info.created) {
      m.info = h.info; // fill stub metadata
    }
    const parts = [...m.parts];
    for (const p of h.parts) {
      if (!parts.some((x) => x.id === p.id)) parts.push(p);
    }
    m.parts = parts;
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
  const prev = state.sessions[sessionID];
  if (!prev) return state;
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
