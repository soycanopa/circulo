/**
 * EventSource wrapper for /agent/sse with reconnect + resync (flow.md §7).
 * The browser's EventSource reconnects natively; we use that and layer a
 * state hook (connecting/open/reconnecting) plus a resync callback that runs
 * whenever the stream (re)opens, because missed events are possible.
 */

import type { Envelope } from "./protocol";

export interface SseHandle {
  close: () => void;
}

export function connectSse(
  onEvent: (env: Envelope) => void,
  onState: (state: "open" | "reconnecting") => void,
  onResync: () => void,
): SseHandle {
  const source = new EventSource("/agent/sse");
  let everOpened = false;

  source.onopen = () => {
    const first = !everOpened;
    everOpened = true;
    onState("open");
    // Every open (including native reconnects) is a resync point: parts may
    // have been missed while the connection was down. Full-part semantics
    // make the refetch idempotent.
    onResync();
    if (first) onState("open");
  };
  source.onerror = () => {
    if (everOpened) onState("reconnecting");
  };
  source.onmessage = (msg) => {
    try {
      const env = JSON.parse(msg.data) as Envelope;
      onEvent(env);
    } catch (err) {
      // One malformed frame must not kill the stream.
      console.error("sse: bad payload", err);
    }
  };

  return {
    close: () => source.close(),
  };
}
