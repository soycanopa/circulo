/**
 * EventSource wrapper for /agent/sse with resume (flow.md §7).
 *
 * The client owns the cursor rather than letting the browser reconnect on its
 * own, because the resume position has two parts — a sequence *and* the epoch
 * it belongs to — and EventSource can only carry a bare Last-Event-ID header on
 * a native reconnect. Owning it also lets the connection be rebuilt after the
 * server deliberately ends it, which is what happens when a subscriber falls
 * behind (the relay closes the stream so the ring can hand back the exact tail).
 *
 * The replay frame arrives before any event and says whether the client is
 * current (resumed — nothing to do) or owes a refetch (full / stale / gap).
 * Before this existed the client refetched projects, sessions, history and the
 * whole model catalog on *every* reconnect, including a one-second blip.
 */

import type { Envelope, ReplayFrame, ReplayStatus } from "./protocol";

export interface SseHandle {
  close: () => void;
}

export interface SseCallbacks {
  onEvent: (env: Envelope) => void;
  onState: (state: "open" | "reconnecting") => void;
  /**
   * Called only when the server says the client's position is not continuous
   * (full / stale / gap). A clean resume does not call it.
   */
  onResync: (reason: ReplayStatus) => void;
}

export function connectSse(cb: SseCallbacks): SseHandle {
  let source: EventSource | null = null;
  let closed = false;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;
  let backoff = 250;
  // Position held in this connection's epoch. Zeroed whenever the server's
  // epoch is not the one we have been tracking, so a restart is never mistaken
  // for a continuation.
  let epoch = "";
  let seq = 0;

  function scheduleReopen() {
    if (closed || retryTimer !== null) return;
    retryTimer = setTimeout(() => {
      retryTimer = null;
      open();
    }, backoff);
    // Back off so a down backend is not hammered in a tight loop, and cap it
    // so recovery is not slow after a long outage.
    backoff = Math.min(backoff * 2, 5000);
  }

  function open() {
    if (closed) return;
    const url = epoch ? `/agent/sse?epoch=${encodeURIComponent(epoch)}&seq=${seq}` : "/agent/sse";
    source = new EventSource(url);

    source.onopen = () => {
      // Reset the ladder only on a *successful* open. Resetting it when the
      // EventSource is merely constructed would keep the delay pinned at the
      // first rung, so a backend that never comes up would be retried in a hot
      // loop forever.
      backoff = 250;
      if (!closed) cb.onState("open");
    };
    // The server's answer to our cursor. Handling this as a named event keeps
    // it out of onmessage, so a control frame can never reach the reducer.
    source.addEventListener("replay", (ev) => {
      let frame: ReplayFrame;
      try {
        frame = JSON.parse((ev as MessageEvent).data) as ReplayFrame;
      } catch {
        return;
      }
      epoch = frame.epoch;
      if (frame.status === "resumed") {
        // We are current: the tail that follows was delivered as ordinary
        // events. No refetch.
        seq = Math.max(seq, frame.seq);
        return;
      }
      // full / stale / gap: our position is not usable. Drop it so the next
      // open is a clean first connect, and tell the caller to refetch.
      seq = 0;
      cb.onResync(frame.status);
    });
    source.onerror = () => {
      if (closed) return;
      cb.onState("reconnecting");
      // EventSource retries the same URL on its own, which would resend a
      // stale cursor. Rebuild it ourselves — after a backoff — so the next
      // attempt carries the position we actually hold.
      if (source) {
        source.close();
        source = null;
      }
      scheduleReopen();
    };
    source.onmessage = (msg) => {
      let env: Envelope;
      try {
        env = JSON.parse(msg.data) as Envelope;
      } catch (err) {
        // One malformed frame must not kill the stream.
        console.error("sse: bad payload", err);
        return;
      }
      // Dedupe on the pair. A replayed tail can overlap what we already
      // applied, and an event from a previous epoch must never be applied to
      // the current one even though its sequence looks plausible.
      if (env.epoch && epoch && env.epoch !== epoch) return;
      if (typeof env.seq === "number") {
        if (env.seq <= seq) return;
        seq = env.seq;
      }
      cb.onEvent(env);
    };
  }

  open();

  return {
    close: () => {
      closed = true;
      if (retryTimer !== null) {
        clearTimeout(retryTimer);
        retryTimer = null;
      }
      source?.close();
      source = null;
    },
  };
}
