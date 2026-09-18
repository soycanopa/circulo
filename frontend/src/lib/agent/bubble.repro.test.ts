import { describe, expect, it } from "vitest";

import type { Envelope } from "./protocol";
import {
  addOptimisticUserMessage,
  applyEvent,
  emptyChatState,
  mergeHydrated,
} from "./reducer";
import { visibleMessages } from "@/features/chat/Message";

function env(type: string, payload: unknown): Envelope {
  return { type, payload } as Envelope;
}

// Payloads captured verbatim from the real adapter against a live v2
// 2.0.8 server (tmpprobe run, 2026-09-18).
const SID = "ses_f49ff35f0ffeQRYBWtcGuiKEBU";
const USER_MSG = "msg_0b600ccbe001j5kFh3nKbK5Y1c";

const sessionUp = env("session.updated", {
  projectID: "probe",
  session: { id: SID, title: "t", directory: "/tmp/x", timeCreated: 1, timeUpdated: 1 },
});

const userEcho: Envelope[] = [
  env("message.updated", {
    projectID: "probe",
    sessionID: SID,
    message: { id: USER_MSG, sessionID: SID, role: "user", created: 1789759900864, completed: 0 },
  }),
  env("part.updated", {
    projectID: "probe",
    sessionID: SID,
    messageID: USER_MSG,
    part: { id: USER_MSG + ":text", type: "text", text: "Hola" },
  }),
];

function userTexts(messages: ReturnType<typeof visibleMessages>): string[] {
  return visibleMessages(messages)
    .filter((m) => m.info.role === "user")
    .map((m) => m.parts.find((p) => p.type === "text")?.text ?? "<no-text-part>");
}

describe("user bubble reproduction", () => {
  it("happy path: optimistic + live echo keeps the text", () => {
    let chat = applyEvent(emptyChatState, sessionUp);
    chat = addOptimisticUserMessage(chat, SID, "client_1", "Hola");
    for (const e of userEcho) chat = applyEvent(chat, e);
    expect(userTexts(chat.sessions[SID].messages)).toEqual(["Hola"]);
  });

  it("hydrate-while-optimistic race keeps the text", () => {
    let chat = applyEvent(emptyChatState, sessionUp);
    chat = addOptimisticUserMessage(chat, SID, "client_1", "Hola");
    chat = mergeHydrated(chat, SID, [
      {
        info: { id: USER_MSG, sessionID: SID, role: "user", created: 1789759900864, completed: 0 },
        parts: [{ id: USER_MSG + ":text", type: "text", text: "Hola" }],
      },
    ]);
    expect(userTexts(chat.sessions[SID].messages)).toEqual(["Hola"]);
  });

  it("hydrate then echo (session reopened mid-turn)", () => {
    let chat = applyEvent(emptyChatState, sessionUp);
    chat = addOptimisticUserMessage(chat, SID, "client_1", "Hola");
    // resync lands BEFORE the live echo, with history that already contains
    // the user message without parts (server had not persisted text yet)
    chat = mergeHydrated(chat, SID, [
      { info: { id: USER_MSG, sessionID: SID, role: "user", created: 1789759900864, completed: 0 }, parts: [] },
    ]);
    for (const e of userEcho) chat = applyEvent(chat, e);
    expect(userTexts(chat.sessions[SID].messages)).toEqual(["Hola"]);
  });

  it("hydration replaces a stale running tool part (lost success frame)", () => {
    let chat = applyEvent(emptyChatState, sessionUp);
    chat = applyEvent(chat, env("part.updated", {
      projectID: "probe",
      sessionID: SID,
      messageID: "msg_a",
      part: {
        id: "call_1", type: "tool", callID: "call_1", tool: "webfetch",
        state: { status: "running", input: { url: "https://x.io" } },
      },
    }));
    // the idle history refetch carries the server's authoritative state
    chat = mergeHydrated(chat, SID, [{
      info: { id: "msg_a", sessionID: SID, role: "assistant", created: 1, completed: 2 },
      parts: [{
        id: "call_1", type: "tool", callID: "call_1", tool: "webfetch",
        state: { status: "completed", input: { url: "https://x.io" }, output: "done" },
      }],
    }]);
    const assistant = chat.sessions[SID].messages.find((m) => m.info.id === "msg_a");
    expect(assistant?.parts[0].state?.status).toBe("completed");
  });
});
