import { describe, expect, it } from "vitest";

import type { Envelope, PartUpdatedEvent, Session } from "./protocol";
import {
  addOptimisticUserMessage,
  applyEvent,
  emptyChatState,
  mergeHydrated,
  type ChatState,
} from "./reducer";

function session(id: string): Session {
  return { id, title: `s-${id}`, timeCreated: 1, timeUpdated: 1 };
}

function env(type: string, payload: unknown): Envelope {
  return { type, payload } as Envelope;
}

function sessionCreated(id: string): Envelope {
  return env("session.updated", { projectID: "p", session: session(id) });
}

/** Golden stream captured from docs: user echo → busy → step-start →
 *  text(empty)+deltas → final text → step-finish → idle. */
const turnStream: Envelope[] = [
  sessionCreated("ses_1"),
  env("session.status", {
    projectID: "p",
    sessionID: "ses_1",
    status: "busy",
  }),
  env("message.updated", {
    projectID: "p",
    sessionID: "ses_1",
    message: {
      id: "msg_u",
      sessionID: "ses_1",
      role: "user",
      created: 1,
      completed: 0,
    },
  }),
  env("part.updated", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_u",
    part: { id: "prt_u", type: "text", text: "hello" },
  }),
  env("message.updated", {
    projectID: "p",
    sessionID: "ses_1",
    message: {
      id: "msg_a",
      sessionID: "ses_1",
      role: "assistant",
      created: 2,
      completed: 0,
    },
  }),
  env("part.updated", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    part: { id: "prt_ss", type: "step-start" },
  }),
  env("part.updated", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    part: { id: "prt_t", type: "text", text: "" },
  }),
  env("part.delta", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    partID: "prt_t",
    field: "text",
    delta: "O",
  }),
  env("part.delta", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    partID: "prt_t",
    field: "text",
    delta: "K",
  }),
  env("part.updated", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    part: { id: "prt_t", type: "text", text: "OK" },
  }),
  env("part.updated", {
    projectID: "p",
    sessionID: "ses_1",
    messageID: "msg_a",
    part: {
      id: "prt_sf",
      type: "step-finish",
      reason: "stop",
      cost: 0.01,
      tokens: { input: 10, output: 3, reasoning: 0, cacheRead: 1, cacheWrite: 0 },
    },
  }),
  env("session.status", {
    projectID: "p",
    sessionID: "ses_1",
    status: "idle",
  }),
];

function playAll(state: ChatState, events: Envelope[]): ChatState {
  return events.reduce((acc, e) => applyEvent(acc, e), state);
}

describe("reducer: golden turn stream", () => {
  it("produces ordered messages and parts", () => {
    const s = playAll(emptyChatState, turnStream);
    const ses = s.sessions["ses_1"];
    expect(ses.status).toBe("idle");
    expect(ses.messages.map((m) => m.info.role)).toEqual(["user", "assistant"]);
    expect(ses.messages[0].parts.map((p) => p.text)).toEqual(["hello"]);
    expect(ses.messages[1].parts.map((p) => p.id)).toEqual([
      "prt_ss",
      "prt_t",
      "prt_sf",
    ]);
    const textPart = ses.messages[1].parts[1];
    expect(textPart.text).toBe("OK");
  });

  it("is idempotent under full-part replays", () => {
    const once = playAll(emptyChatState, turnStream);
    const twice = playAll(once, turnStream);
    expect(twice.sessions["ses_1"].messages[1].parts).toHaveLength(3);
    expect(twice.sessions["ses_1"].messages[1].parts[1].text).toBe("OK");
  });
});

describe("reducer: unknown events", () => {
  it("ignores them (forward compat)", () => {
    const base = playAll(emptyChatState, [sessionCreated("ses_1")]);
    const next = applyEvent(base, env("future.thing.awesome", { x: 1 }));
    expect(next).toBe(base);
  });
});

describe("reducer: part before message/session", () => {
  it("creates stubs so late-joined streams still render", () => {
    const next = applyEvent(emptyChatState, env("part.updated", {
      projectID: "p",
      sessionID: "ses_late",
      messageID: "msg_x",
      part: { id: "prt_x", type: "text", text: "hi" },
    }) as Envelope<PartUpdatedEvent>);
    const ses = next.sessions["ses_late"];
    expect(ses).toBeDefined();
    expect(ses.messages[0].parts[0].text).toBe("hi");
  });
});

describe("reducer: hydration + live overlap", () => {
  it("hydration fills gaps, live parts win", () => {
    const live = playAll(emptyChatState, [
      sessionCreated("ses_1"),
      env("part.updated", {
        projectID: "p",
        sessionID: "ses_1",
        messageID: "msg_a",
        part: { id: "prt_t", type: "text", text: "OK" },
      }),
    ]);
    const merged = mergeHydrated(live, "ses_1", [
      {
        info: {
          id: "msg_u",
          sessionID: "ses_1",
          role: "user",
          created: 1,
          completed: 0,
        },
        parts: [{ id: "prt_u", type: "text", text: "hello" }],
      },
      {
        info: {
          id: "msg_a",
          sessionID: "ses_1",
          role: "assistant",
          created: 2,
          completed: 3,
        },
        parts: [
          { id: "prt_ss", type: "step-start" },
          { id: "prt_t", type: "text", text: "OK" }, // duplicate: dropped
        ],
      },
    ]);
    const msgs = merged.sessions["ses_1"].messages;
    expect(msgs.map((m) => m.info.id)).toEqual(["msg_a", "msg_u"]);
    const msgA = msgs.find((m) => m.info.id === "msg_a")!;
    expect(msgA.parts).toHaveLength(2); // step-start added, text not duplicated
    expect(msgA.parts.find((p) => p.id === "prt_t")!.text).toBe("OK");
    expect(msgA.info.completed).toBe(3); // stub metadata filled
  });
});

describe("reducer: permissions", () => {
  it("stacks and resolves by id", () => {
    let s = playAll(emptyChatState, [sessionCreated("ses_1")]);
    s = applyEvent(s, env("permission.request", {
      projectID: "p",
      sessionID: "ses_1",
      permission: { id: "per_1", kind: "bash", title: "run it" },
    }));
    s = applyEvent(s, env("permission.request", {
      projectID: "p",
      sessionID: "ses_1",
      permission: { id: "per_2", kind: "edit" },
    }));
    expect(s.sessions["ses_1"].permissions.map((p) => p.id)).toEqual([
      "per_1",
      "per_2",
    ]);
    // Duplicate request is ignored.
    s = applyEvent(s, env("permission.request", {
      projectID: "p",
      sessionID: "ses_1",
      permission: { id: "per_1", kind: "bash", title: "run it" },
    }));
    expect(s.sessions["ses_1"].permissions).toHaveLength(2);
    s = applyEvent(s, env("permission.resolved", {
      projectID: "p",
      sessionID: "ses_1",
      permissionID: "per_1",
      response: "once",
    }));
    expect(s.sessions["ses_1"].permissions.map((p) => p.id)).toEqual(["per_2"]);
  });
});

describe("reducer: session status and errors", () => {
  it("tracks retry and clears error on next busy", () => {
    let s = playAll(emptyChatState, [sessionCreated("ses_1")]);
    s = applyEvent(s, env("session.status", {
      projectID: "p",
      sessionID: "ses_1",
      status: "retry",
      retry: { attempt: 2, message: "429", nextAt: 99 },
    }));
    expect(s.sessions["ses_1"].status).toBe("retry");
    expect(s.sessions["ses_1"].retry?.attempt).toBe(2);
    s = applyEvent(s, env("session.error", {
      projectID: "p",
      sessionID: "ses_1",
      error: { name: "APIError", message: "quota" },
    }));
    expect(s.sessions["ses_1"].lastError?.name).toBe("APIError");
    s = applyEvent(s, env("session.status", {
      projectID: "p",
      sessionID: "ses_1",
      status: "busy",
    }));
    expect(s.sessions["ses_1"].lastError).toBeUndefined();
  });
});

describe("reducer: optimistic user message", () => {
  it("server echo replaces content by id mapping", () => {
    let s = playAll(emptyChatState, [sessionCreated("ses_1")]);
    s = addOptimisticUserMessage(s, "ses_1", "client_1", "fix tests");
    expect(s.sessions["ses_1"].messages).toHaveLength(1);
    // The server echoes with its own message id; reducer adds it. The
    // optimistic row is removed by the UI once any server user message with
    // identical text exists (kept simple: optimistic rows render muted until
    // a server user message follows them).
    s = applyEvent(s, env("part.updated", {
      projectID: "p",
      sessionID: "ses_1",
      messageID: "msg_srv",
      part: { id: "prt_srv", type: "text", text: "fix tests" },
    }));
    expect(s.sessions["ses_1"].messages.map((m) => m.info.id)).toEqual([
      "client_1",
      "msg_srv",
    ]);
  });
});

describe("reducer: todo/tasks", () => {
  it("replaces the task list wholesale and requires a known session", () => {
    let s = playAll(emptyChatState, [sessionCreated("ses_1")]);
    s = applyEvent(s, env("todo.updated", {
      projectID: "p",
      sessionID: "ses_1",
      tasks: [
        { id: "1", content: "Draft", status: "completed" },
        { id: "2", content: "Save", status: "in_progress" },
        { id: "3", content: "Suggest", status: "pending" },
      ],
    }));
    expect(s.sessions["ses_1"].tasks).toHaveLength(3);
    expect(s.sessions["ses_1"].tasks[1].status).toBe("in_progress");

    // Unknown session: ignored.
    const next = applyEvent(s, env("todo.updated", {
      projectID: "p",
      sessionID: "ses_ghost",
      tasks: [{ content: "x", status: "pending" }],
    }));
    expect(next).toBe(s);

    // Empty list clears.
    s = applyEvent(s, env("todo.updated", { projectID: "p", sessionID: "ses_1", tasks: [] }));
    expect(s.sessions["ses_1"].tasks).toHaveLength(0);
  });
});

describe("session.updated merge-patch (opencode v2)", () => {
  it("merges a partial patch (renamed) keeping existing times", () => {
    let s = emptyChatState;
    s = applyEvent(s, env("session.updated", { projectID: "p", session: session("ses_1") }));
    expect(s.sessions["ses_1"].session.title).toBe("s-ses_1");
    expect(s.sessions["ses_1"].session.timeCreated).toBe(1);

    s = applyEvent(s, env("session.updated", {
      projectID: "p",
      session: { id: "ses_1", title: "new title", timeCreated: 0, timeUpdated: 0 },
    }));
    expect(s.sessions["ses_1"].session.title).toBe("new title");
    expect(s.sessions["ses_1"].session.timeCreated).toBe(1);
  });

  it("never creates a session from an empty partial patch", () => {
    const s = emptyChatState;
    const next = applyEvent(s, env("session.updated", {
      projectID: "p",
      session: { id: "ses_ghost", title: "renamed", timeCreated: 0, timeUpdated: 0 },
    }));
    expect(next.sessions["ses_ghost"]).toBeUndefined();
  });

  it("creates a session from a full update (created event)", () => {
    const next = applyEvent(emptyChatState, env("session.updated", {
      projectID: "p",
      session: { id: "ses_new", title: "fresh", timeCreated: 9, timeUpdated: 9 },
    }));
    expect(next.sessions["ses_new"].session.title).toBe("fresh");
  });
});
