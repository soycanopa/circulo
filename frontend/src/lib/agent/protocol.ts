/**
 * TypeScript mirror of internal/agent/protocol (the neutral wire contract).
 * AGENTS.md: any struct change in Go lands in the same commit as this file.
 */

// Event types (Envelope.type).
export const EventAdapterStatus = "adapter.status";
export const EventSessionUpdated = "session.updated";
export const EventSessionStatus = "session.status";
export const EventSessionRemoved = "session.removed";
export const EventMessageUpdated = "message.updated";
export const EventPartUpdated = "part.updated";
export const EventPartDelta = "part.delta";
export const EventPermissionRequest = "permission.request";
export const EventPermissionResolved = "permission.resolved";
export const EventSessionError = "session.error";

// Adapter states.
export type AdapterState = "starting" | "running" | "stopped" | "error";

// Session statuses.
export type SessionStatusKind = "idle" | "busy" | "retry";

// Part types.
export type PartType =
  | "text"
  | "reasoning"
  | "tool"
  | "step-start"
  | "step-finish"
  | "patch"
  | "agent"
  | "subtask"
  | "file";

// Tool states.
export type ToolStatus = "pending" | "running" | "completed" | "error";

export interface Envelope<T = unknown> {
  type: string;
  payload: T;
}

export interface AdapterStatus {
  projectID: string;
  state: AdapterState;
  detail?: string;
}

export interface Session {
  id: string;
  title: string;
  directory?: string;
  timeCreated: number; // unix millis
  timeUpdated: number;
}

export interface RetryInfo {
  attempt: number;
  message: string;
  nextAt: number;
}

export interface SessionStatusEvent {
  projectID: string;
  sessionID: string;
  status: SessionStatusKind;
  retry?: RetryInfo;
}

export interface TokenUsage {
  input: number;
  output: number;
  reasoning: number;
  cacheRead: number;
  cacheWrite: number;
}

export interface MessageInfo {
  id: string;
  sessionID: string;
  role: "user" | "assistant";
  created: number;
  completed: number;
  provider?: string;
  model?: string;
  agent?: string;
  cost?: number;
  tokens?: TokenUsage;
  finish?: string;
  errorName?: string;
  errorMsg?: string;
}

export interface TimeRange {
  start: number;
  end: number;
}

export interface ToolState {
  status: ToolStatus;
  input?: unknown;
  title?: string;
  output?: string;
  error?: string;
  metadata?: unknown;
  time?: TimeRange;
}

/**
 * Part is a flat union: which fields are meaningful depends on `type`.
 * Mirrors protocol.Part in Go.
 */
export interface Part {
  id: string;
  type: PartType;
  // text, reasoning
  text?: string;
  time?: TimeRange;
  // tool
  callID?: string;
  tool?: string;
  state?: ToolState;
  // step-finish
  reason?: string;
  cost?: number;
  tokens?: TokenUsage;
  // patch
  hash?: string;
  files?: string[];
  // agent
  name?: string;
  // subtask
  prompt?: string;
  description?: string;
  agent?: string;
  // file
  mime?: string;
  path?: string;
}

export interface PartUpdatedEvent {
  projectID: string;
  sessionID: string;
  messageID: string;
  part: Part;
}

export interface PartDeltaEvent {
  projectID: string;
  sessionID: string;
  messageID: string;
  partID: string;
  field: string;
  delta: string;
}

export interface SessionUpdatedEvent {
  projectID: string;
  session: Session;
}

export interface SessionRemovedEvent {
  projectID: string;
  sessionID: string;
}

export interface MessageUpdatedEvent {
  projectID: string;
  sessionID: string;
  message: MessageInfo;
}

export interface PermissionRequest {
  id: string;
  kind?: string;
  title?: string;
  pattern?: unknown;
  metadata?: unknown;
  callID?: string;
  createdAt?: number;
}

export interface PermissionRequestedEvent {
  projectID: string;
  sessionID: string;
  permission: PermissionRequest;
}

export interface PermissionResolvedEvent {
  projectID: string;
  sessionID: string;
  permissionID: string;
  response: string;
}

export interface SessionErroredEvent {
  projectID: string;
  sessionID?: string;
  error: { name: string; message: string };
}

export interface PromptRequest {
  text: string;
  agent?: string;
  provider?: string;
  model?: string;
}

export interface ModelInfo {
  id: string;
  name?: string;
  provider: string;
}

export interface AgentInfo {
  name: string;
  description?: string;
  mode?: string;
}

export interface Meta {
  agents: AgentInfo[];
  models: ModelInfo[];
}

/** REST view from GET /agent/projects. */
export interface ProjectView {
  id: string;
  path: string;
  mode: "managed" | "attach";
  url?: string;
  status: AdapterState;
  detail?: string;
}

/** Hydrated history entry from GET …/messages. */
export interface HydratedMessage {
  info: MessageInfo;
  parts: Part[];
}

export function isKnownPartType(t: string): t is PartType {
  return [
    "text",
    "reasoning",
    "tool",
    "step-start",
    "step-finish",
    "patch",
    "agent",
    "subtask",
    "file",
  ].includes(t);
}
