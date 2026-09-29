/**
 * Resume behavior of the SSE client: cursor ownership, dedup, and — the part
 * that motivated the change — not refetching on a clean resume.
 *
 * EventSource is stubbed so the tests drive frames directly instead of
 * depending on a live server.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { connectSse, type SseCallbacks } from "./sse";
import type { Envelope } from "./protocol";

class FakeEventSource {
  static last: FakeEventSource | null = null;
  static urls: string[] = [];
  static instances: FakeEventSource[] = [];

  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  closed = false;
  private listeners = new Map<string, ((e: MessageEvent) => void)[]>();

  constructor(readonly url: string) {
    FakeEventSource.last = this;
    FakeEventSource.urls.push(url);
    FakeEventSource.instances.push(this);
  }

  addEventListener(name: string, fn: (e: MessageEvent) => void) {
    const list = this.listeners.get(name) ?? [];
    list.push(fn);
    this.listeners.set(name, list);
  }

  close() {
    this.closed = true;
  }

  /* --- test drivers --- */
  open() {
    this.onopen?.();
  }
  fail() {
    this.onerror?.();
  }
  replay(data: unknown) {
    for (const fn of this.listeners.get("replay") ?? []) {
      fn({ data: JSON.stringify(data) } as MessageEvent);
    }
  }
  event(env: Envelope) {
    this.onmessage?.({ data: JSON.stringify(env) } as MessageEvent);
  }
}

function setup() {
  FakeEventSource.urls = [];
  FakeEventSource.instances = [];
  FakeEventSource.last = null;
  vi.stubGlobal("EventSource", FakeEventSource as unknown as typeof EventSource);
  const onEvent = vi.fn<(env: Envelope) => void>();
  const onState = vi.fn<(s: "open" | "reconnecting") => void>();
  const onResync = vi.fn<(reason: string) => void>();
  const cb: SseCallbacks = { onEvent, onState, onResync };
  return { onEvent, onState, onResync, handle: connectSse(cb) };
}

describe("connectSse resume", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it("opens with no cursor on first connect", () => {
    setup();
    expect(FakeEventSource.urls).toEqual(["/agent/sse"]);
  });

  it("does not resync when the server resumes us where we left off", () => {
    const { onResync, onEvent } = setup();
    const src = FakeEventSource.last!;
    src.open();
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "session.status", seq: 1, epoch: "e1", payload: {} });
    src.event({ type: "session.status", seq: 2, epoch: "e1", payload: {} });
    onResync.mockClear(); // the first connect legitimately refetched

    // A reconnect while the stream was healthy: the server resumes us.
    src.fail();
    vi.advanceTimersByTime(1000);
    const resumed = FakeEventSource.last!;
    expect(resumed.url).toBe("/agent/sse?epoch=e1&seq=2");
    resumed.open();
    resumed.replay({ status: "resumed", epoch: "e1", seq: 2 });

    expect(onResync).not.toHaveBeenCalled();
    expect(onEvent).toHaveBeenCalledTimes(2); // only the original two
  });

  it("refetches when the server reports a stale epoch", () => {
    const { onResync } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "session.status", seq: 7, epoch: "e1", payload: {} });

    src.fail();
    vi.advanceTimersByTime(1000);
    const next = FakeEventSource.last!;
    next.replay({ status: "stale", epoch: "e2", seq: 0 });

    expect(onResync).toHaveBeenCalledWith("stale");
  });

  it("refetches when the server reports a gap", () => {
    const { onResync } = setup();
    FakeEventSource.last!.replay({ status: "gap", epoch: "e1", seq: 900 });
    expect(onResync).toHaveBeenCalledWith("gap");
  });

  it("drops the cursor after a gap so the next open starts clean", () => {
    const { onResync } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "session.status", seq: 5, epoch: "e1", payload: {} });
    src.fail();
    vi.advanceTimersByTime(1000);
    FakeEventSource.last!.replay({ status: "gap", epoch: "e1", seq: 900 });
    expect(onResync).toHaveBeenCalledWith("gap");

    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(1000);
    expect(FakeEventSource.last!.url).toBe("/agent/sse?epoch=e1&seq=0");
  });

  it("drops events it has already applied", () => {
    const { onEvent } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "a", seq: 1, epoch: "e1", payload: {} });
    src.event({ type: "b", seq: 2, epoch: "e1", payload: {} });
    src.event({ type: "b-again", seq: 2, epoch: "e1", payload: {} });
    src.event({ type: "stale-seq", seq: 1, epoch: "e1", payload: {} });

    const types = onEvent.mock.calls.map((c) => (c[0] as Envelope).type);
    expect(types).toEqual(["a", "b"]);
  });

  it("ignores events from a different epoch even when the sequence looks valid", () => {
    const { onEvent } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "current", seq: 1, epoch: "e1", payload: {} });
    // A frame from a previous run, arriving with a sequence we would happily
    // accept if we only compared numbers.
    src.event({ type: "from-old-run", seq: 2, epoch: "e0", payload: {} });

    const types = onEvent.mock.calls.map((c) => (c[0] as Envelope).type);
    expect(types).toEqual(["current"]);
  });

  it("still applies unsequenced frames (tests, non-orchestrator sources)", () => {
    const { onEvent } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.event({ type: "legacy", payload: {} });
    expect(onEvent).toHaveBeenCalledTimes(1);
  });

  it("survives a malformed frame", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const { onEvent } = setup();
    const src = FakeEventSource.last!;
    src.replay({ status: "full", epoch: "e1", seq: 0 });
    src.onmessage?.({ data: "{not json" } as MessageEvent);
    src.event({ type: "after", seq: 1, epoch: "e1", payload: {} });
    expect(onEvent).toHaveBeenCalledTimes(1);
    spy.mockRestore();
  });

  it("survives a malformed replay frame", () => {
    const { onResync, onEvent } = setup();
    const src = FakeEventSource.last!;
    src.addEventListener("replay", () => {}); // noop, we drive it directly
    // Drive a bad payload through the same listener path.
    for (const fn of (src as unknown as { listeners: Map<string, ((e: MessageEvent) => void)[]> }).listeners.get("replay")!) {
      fn({ data: "<<broken>>" } as MessageEvent);
    }
    src.event({ type: "still-working", seq: 1, epoch: "", payload: {} });
    expect(onResync).not.toHaveBeenCalled();
    expect(onEvent).toHaveBeenCalledTimes(1);
  });

  it("backs off instead of spinning when the backend is down", () => {
    setup();
    const first = FakeEventSource.last!;
    first.fail();
    // No immediate reopen: the ladder starts at 250ms.
    expect(FakeEventSource.urls).toHaveLength(1);
    vi.advanceTimersByTime(249);
    expect(FakeEventSource.urls).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(FakeEventSource.urls).toHaveLength(2);

    // A source that never opened must not reset the ladder, so the next
    // attempt waits longer. Resetting on construction instead is what turns
    // this into a 250ms-forever loop.
    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(499);
    expect(FakeEventSource.urls).toHaveLength(2);
    vi.advanceTimersByTime(1);
    expect(FakeEventSource.urls).toHaveLength(3);

    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(999);
    expect(FakeEventSource.urls).toHaveLength(3);
    vi.advanceTimersByTime(1);
    expect(FakeEventSource.urls).toHaveLength(4);
  });

  it("resets the ladder after a real open", () => {
    setup();
    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(250);
    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(500);
    expect(FakeEventSource.urls).toHaveLength(3);
    // This one connects, so the delay starts over at the first rung.
    FakeEventSource.last!.open();
    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(250);
    expect(FakeEventSource.urls).toHaveLength(4);
  });

  it("stops retrying once closed", () => {
    const { handle, onState } = setup();
    handle.close();
    FakeEventSource.last!.fail();
    vi.advanceTimersByTime(10000);
    expect(FakeEventSource.urls).toHaveLength(1);
    expect(onState).not.toHaveBeenCalledWith("reconnecting");
  });
});
