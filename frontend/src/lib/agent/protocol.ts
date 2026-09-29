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
export const EventFormUpdated = "form.updated";
export const EventFormResolved = "form.resolved";
export const EventSessionError = "session.error";
export const EventTodoUpdated = "todo.updated";

// Adapter states.
export type AdapterState = "starting" | "running" | "stopped" | "error";

// Session statuses.
export type SessionStatusKind = "idle" | "busy" | "retry";

/** Context-window fill of the live session (context.updated). Window == 0
 * means the backend didn't expose one (render Used as a bare count). */
export interface ContextUsage {
  projectID: string;
  sessionID: string;
  used: number;
  window?: number;
}

export interface ContextUpdatedEvent {
  usage: ContextUsage;
}

/** Envelope types not tied to a payload schema live beside their interface. */
export const EventContextUpdated = "context.updated";

/** One slash command (built-ins, skills, user commands). */
export interface CommandInfo {
  name: string;
  description?: string;
  argsHint?: string;
  /** builtin|skill|custom|extension|file (provider vocabulary). */
  source?: string;
}

export interface CommandRequest {
  name: string;
  text: string;
}

export interface CommandsUpdatedEvent {
  projectID: string;
  commands: CommandInfo[];
}

export const EventCommandsUpdated = "commands.updated";

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

/**
 * Envelope is one SSE frame. `seq` and `epoch` are the resume contract and
 * mirror Go's protocol.Envelope exactly — changing one means changing the
 * other in the same commit.
 *
 * The epoch changes on every app start while seq restarts at 1, so a bare
 * seq cannot be trusted across a restart; that pair is what makes a resumed
 * stream safe. Both are optional because a frame produced outside the
 * orchestrator (tests) has neither.
 */
export interface Envelope<T = unknown> {
  type: string;
  seq?: number;
  epoch?: string;
  payload: T;
}

/**
 * ReplayStatus is the server's answer to a resume request, sent as the first
 * SSE control frame (event: replay). It is not an Envelope: it describes the
 * stream, not a domain event, and the reducer must never see it.
 */
export type ReplayStatus = "full" | "resumed" | "stale" | "gap";

export interface ReplayFrame {
  status: ReplayStatus;
  epoch: string;
  /** Highest sequence replayed on this connection; 0 when nothing was. */
  seq: number;
}

/**
 * Whether a ReplayStatus means the client must refetch. Only `resumed` means
 * the client is already current — everything else leaves a hole it cannot see.
 */
export function needsResync(status: ReplayStatus): boolean {
  return status !== "resumed";
}

/**
 * One stored workspace state. Checkpoints are git refs under refs/circulo/,
 * written at the edges of a turn: `start` is the tree the turn began from and
 * is both what a rewind restores and what its diff is measured against.
 */
export interface Checkpoint {
  sessionID: string;
  turn: number;
  kind: "start" | "end" | "guard";
  ref: string;
  commit: string;
  createdAt: number;
}

export interface CheckpointView {
  projectID: string;
  /** False for a project that is not a git repository — the feature hides. */
  available: boolean;
  /** The worktree has uncommitted changes right now. */
  dirty: boolean;
  checkpoints: Checkpoint[];
}

export interface RewindPreview {
  ref: string;
  turn: number;
  /** Unified diff of what the rewind would undo. */
  diff: string;
  /** Uncommitted work exists, so a rewind discards something. */
  willLose: boolean;
  /** Where the current state is preserved, so the rewind is undoable. */
  guardRef: string;
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

/** v2 note: updates may be partial patches (session.renamed carries only
 * id+title); reducers merge them and never create from an empty patch. */
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

/** Git state of a project's folder (gates the branch picker). */
export interface ProjectVcs {
  isRepo: boolean;
  provider?: string;
  branch?: string;
  defaultBranch?: string;
}

/** One choice of a select-style form field (question tool). */
export interface FormOption {
  value: string;
  label: string;
  description?: string;
}

/** A form field; v0 covers select-style and free-text (custom) strings. */
export interface FormField {
  key: string;
  /** string|multiselect|… — multiselect renders as multi-answer */
  type?: string;
  title?: string;
  description?: string;
  options?: FormOption[];
  custom?: boolean;
}

/** A pending form to answer (opencode v2 question tool). */
export interface FormInfo {
  id: string;
  title?: string;
  kind?: string;
  callID?: string;
  fields: FormField[];
}

export interface FormUpdatedEvent {
  projectID: string;
  sessionID: string;
  form: FormInfo;
}

export interface FormResolvedEvent {
  projectID: string;
  sessionID: string;
  formID: string;
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
  /** Reasoning-effort variant (OpenCode prompt "variant"); empty = default. */
  variant?: string;
  /** @-mentioned workspace paths (relative to the project root). */
  files?: string[];
}

/** One workspace file match for the @-mention autocomplete. */
export interface FileHit {
  /** Path relative to the project root. */
  path: string;
}

export interface ModelInfo {
  id: string;
  name?: string;
  provider: string;
  /** capabilities.reasoning from the agent server. */
  reasoning?: boolean;
  /** Reasoning-effort options the model accepts (e.g. low/high/max). */
  variants?: string[];
}

export interface AgentInfo {
  name: string;
  description?: string;
  mode?: string;
}

/** Access modes (omp provider). Mirrors protocol.Access* constants in Go. */
export type AccessMode = "supervised" | "edits" | "full";

export interface AccessModeInfo {
  id: AccessMode;
  title: string;
  description: string;
}

export interface AccessState {
  mode?: AccessMode;
  supported: boolean;
}

export interface Meta {
  agents: AgentInfo[];
  models: ModelInfo[];
  /** Server-configured default (GET /config/providers -> default). */
  defaultProvider?: string;
  defaultModel?: string;
  /** Access modes the provider exposes; empty for opencode (no surface). */
  accessModes?: AccessModeInfo[];
  /** Current access mode. */
  access: AccessState;
  /** Slash commands; absent = provider exposes no command surface. */
  commands?: CommandInfo[];
  /**
   * What this backend can actually do, mirrored from Go's
   * protocol.Capabilities. Branch on these instead of on the provider name:
   * identity (logo, product name, which "add project" entry) legitimately uses
   * the name, but behavior must not.
   */
  capabilities?: Capabilities;
}

export interface Capabilities {
  /** A real approve/deny round-trip (omp: no). */
  permissions: boolean;
  /** A real answer channel for select/confirm/input requests. */
  forms: boolean;
  /** A prompt while the agent is working steers instead of starting a turn. */
  steer: boolean;
  /** The working branch can actually be pinned. */
  branch: boolean;
  /** A named-agent registry exists to pick from. */
  agents: boolean;
  /** A slash-command surface exists. */
  commands: boolean;
  /** An access-mode selector exists. */
  accessMode: boolean;
}

/**
 * Capabilities are opt-in: a missing or partial block means the feature is not
 * available. Reading a missing key as "true" would be the failure mode that
 * makes a capability flag worse than none — the UI would offer a control that
 * silently does nothing.
 */
export function can(
  meta: { capabilities?: Capabilities } | undefined,
  capability: keyof Capabilities,
): boolean {
  return meta?.capabilities?.[capability] === true;
}

/** REST view from GET /agent/projects. */
export interface ProjectView {
  id: string;
  path: string;
  mode: "managed" | "attach";
  /** Agent backend: "opencode" | "omp". */
  provider: "opencode" | "omp";
  url?: string;
  status: AdapterState;
  detail?: string;
}

/** Hydrated history entry from GET …/messages. */
export interface HydratedMessage {
  info: MessageInfo;
  parts: Part[];
}

export interface Task {
  id?: string;
  content: string;
  /** Agent vocabulary passthrough: pending | in_progress | completed. */
  status: string;
  priority?: string;
}

export interface TodoUpdatedEvent {
  projectID: string;
  sessionID: string;
  tasks: Task[];
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
