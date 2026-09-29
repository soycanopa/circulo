/**
 * Composer drafts, keyed by conversation target.
 *
 * Without this, switching sessions while a prompt is half-written throws that
 * prompt away — and the moment that hurts most is exactly the moment you are
 * looking at another session, not the one you were writing to.
 *
 * Two decisions worth stating:
 *
 *   - Storage is `localStorage` and injectable. The alternative was a
 *     server-side draft store, which is what waku does because it has a
 *     daemon. Circulo has no daemon and drafts have no reason to sync between
 *     anything, so a local key is the whole feature. Injecting the storage
 *     also keeps this module pure enough to test in node.
 *   - A draft saves *segments*, not text. The composer is a contenteditable
 *     whose @file and /command tags are DOM chips, not characters: storing
 *     only the projected string would come back as a chip's name pasted as
 *     plain text, which then reads as a command the user never picked.
 *
 * The key includes the branch, because for a new session the target is not
 * fully described without it — the same project and "new session" can be
 * aimed at different branches.
 */

import type { Segment } from "./richInput";

/** The subset of the Web Storage API this module needs. */
export interface DraftStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
  /** Only pruneDrafts needs these, so a minimal fake can omit them. */
  length?: number;
  key?(index: number): string | null;
}

const PREFIX = "circulo.draft.";

/** A draft older than this is not worth restoring. */
const MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;

export interface StoredDraft {
  segments: Segment[];
  savedAt: number;
}

/**
 * Identity of the conversation a draft belongs to. sessionID is undefined for
 * a not-yet-created session, and branch only matters in that case.
 */
export function draftKey(projectID: string, sessionID?: string, branch?: string): string {
  return `${PREFIX}${projectID}:${sessionID ?? "new"}${branch ? `:${branch}` : ""}`;
}

function defaultStorage(): DraftStorage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    // Private mode / blocked storage: drafts are a convenience, not a
    // requirement, so the feature degrades to doing nothing.
    return null;
  }
}

export function saveDraft(
  key: string,
  segments: Segment[],
  storage: DraftStorage | null = defaultStorage(),
): void {
  if (!storage) return;
  // An empty draft is a cleared draft. Storing "" would leave a key behind
  // for a target the user has nothing pending on.
  if (segments.length === 0) {
    storage.removeItem(key);
    return;
  }
  try {
    storage.setItem(key, JSON.stringify({ segments, savedAt: Date.now() } satisfies StoredDraft));
  } catch {
    // Quota exceeded: a draft is never worth failing a keystroke over.
  }
}

export function loadDraft(
  key: string,
  storage: DraftStorage | null = defaultStorage(),
  now = Date.now(),
): Segment[] | null {
  if (!storage) return null;
  let raw: string | null;
  try {
    raw = storage.getItem(key);
  } catch {
    return null;
  }
  if (!raw) return null;
  let parsed: StoredDraft;
  try {
    parsed = JSON.parse(raw) as StoredDraft;
  } catch {
    // Corrupt entry: drop it rather than leaving it to fail forever.
    clearDraft(key, storage);
    return null;
  }
  if (!Array.isArray(parsed?.segments) || parsed.segments.length === 0) {
    clearDraft(key, storage);
    return null;
  }
  if (typeof parsed.savedAt === "number" && now - parsed.savedAt > MAX_AGE_MS) {
    clearDraft(key, storage);
    return null;
  }
  return parsed.segments.filter(isSegment);
}

export function clearDraft(key: string, storage: DraftStorage | null = defaultStorage()): void {
  if (!storage) return;
  try {
    storage.removeItem(key);
  } catch {
    // Nothing to do: a stale key is inert.
  }
}

/** Drops every draft whose target no longer exists. */
export function pruneDrafts(
  liveKeys: Set<string>,
  storage: DraftStorage | null = defaultStorage(),
): void {
  // Enumeration needs the full Storage surface; a storage without it just
  // means drafts are not pruned, which is inert.
  if (!storage?.key || typeof storage.length !== "number") return;
  try {
    const stale: string[] = [];
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      if (key && key.startsWith(PREFIX) && !liveKeys.has(key)) stale.push(key);
    }
    for (const key of stale) storage.removeItem(key);
  } catch {
    // Best effort.
  }
}

function isSegment(value: unknown): value is Segment {
  if (!value || typeof value !== "object") return false;
  const s = value as Partial<Segment>;
  if (s.kind === "text") return typeof s.value === "string";
  if (s.kind === "file") return typeof s.path === "string";
  if (s.kind === "command") return typeof s.name === "string";
  return false;
}
