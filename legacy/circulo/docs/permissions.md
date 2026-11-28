# Permission cards

- **Status:** implemented 2026-11-21 (permissions and choice questions).
- **Related:** [UX](ux.md) · [UI](ui.md) · [Providers](providers/opencode.md) · [TRD](trd.md)

An agent asks before it does something the user has not allowed: run a command,
edit a file, leave the workspace. Today Circulo answers every request with
`cancelled`, so those tools never run. This document records the reference
design, the wire contract and where each piece lives.

## 1. The reference (Waku 0.1.20)

Measured from the app and its own copy; structure and wording only, no code.

**Placement.** The card is a sibling of the transcript inside the main column:
transcript → **card** → composer. It pushes the composer down rather than
floating over the transcript, so a pending question is impossible to miss and
impossible to scroll past.

**Anatomy**, top to bottom, in a rounded 12 surface with a strong 1 px border:

1. **Header** — a warning glyph and a one-line title: *"The agent wants to run a
   command"*, *"Run `cargo test`?"*, *"The agent is asking for the `network`
   permission"*. Max two lines.
2. **Detail** — an inset block, monospace, secondary colour, vertically
   scrollable above ~92 px, padding 8, no truncation: the command in full, the
   path in full, or the diff. A blocked path reads **"Blocked path: /…"**.
3. **Options** — one button per option the agent offered, in the agent's order.
   The allow-kind options are primary (inverse background, on-inverse text),
   the reject-kind ones secondary (1 px strong border, secondary text). Height
   28, padding 13, rounded 7, semibold 12.5. Hover lightens the primary and
   lifts the secondary; press dims both.

**Copy the app ships:** Allow once · Allow for session · Always allow · Run a
tool · The agent wants to run %{tool} · Blocked path: %{path}.

Two lessons worth keeping: the card never invents options of its own, and the
detail block shows the command or path **in full** — the decision is about what
will actually run.

## 2. Questions

The agent sometimes needs an answer, not an approval. ACP models that as
`elicitation/create` with `mode: "form"`: a `message` and a `requestedSchema`
(flat object; each property is a primitive or an enum). The client must
advertise `clientCapabilities.elicitation.form` during `initialize` before an
agent may ask, and answers with `accept` (+ `content`), `decline` or `cancel`.

The card shows the agent's message, one block per field with its label, and the
enum values as full-width rows; the chosen one is tinted in the accent colour.
`Submit` stays disabled until every required field has an answer, choice or
typed, and the spec's two ways out are both present: `Decline` and `Cancel`.

A field without an enum is a free-text answer, using the same input the composer
uses (`crates/ui/src/input.rs`): its own buffer, focus and caret. The first text
field of a card takes the keyboard when it appears; clicking another one moves
focus. Enter submits the card from inside a field, Escape cancels it.

Field order is the agent's: the OpenCode provider enables `serde_json`'s
`preserve_order`, so a schema's properties come back in the order they were
written instead of alphabetically.

## 3. What ACP actually asks

`session/request_permission` (agent → client):

```json
{
  "sessionId": "sess_1",
  "toolCall": { "toolCallId": "call_1", "title": "Run cargo test", "kind": "execute",
                "content": [...], "locations": [...], "rawInput": {...} },
  "options": [
    { "optionId": "allow-once", "name": "Allow once", "kind": "allow_once" },
    { "optionId": "reject-once", "name": "Reject", "kind": "reject_once" }
  ]
}
```

The client answers `{"outcome":{"outcome":"selected","optionId":"allow-once"}}`
or `{"outcome":{"outcome":"cancelled"}}` — the latter **only** when the turn is
cancelled, never as a stand-in for "no". `kind` is a UI hint:
`allow_once`, `allow_always`, `reject_once`, `reject_always`.

The spec is explicit that the **agent owns remembered decisions**: it decides
what "always" covers and stops asking. Circulo therefore stores nothing and
sends nothing but the chosen `optionId`.

Security notes we follow: describe the operation from `kind`, `locations` and
the tool call's input; present `title` as agent-authored text; show commands and
paths in full; never auto-approve from a title or an option name.

## 4. The wire (protocol v1, additive)

```jsonc
// daemon → client
{ "type": "permission_requested", "session": "s1", "turn": 3, "id": "p-1",
  "request": { "title": "Run cargo test", "detail": "cargo test --workspace",
               "options": [ { "id": "allow-once", "label": "Allow once", "kind": "allow_once" },
                            { "id": "reject-once", "label": "Reject", "kind": "reject_once" } ] } }

// client → daemon
{ "type": "answer_permission", "id": "p-1", "option": "allow-once" }   // null = cancelled

// daemon → client
{ "type": "question_asked", "session": "s1", "turn": 3, "id": "q-1",
  "question": { "message": "How should I approach this?",
                "fields": [ { "id": "strategy", "label": "Strategy", "required": true,
                              "kind": { "kind": "choice",
                                        "options": ["conservative", "balanced"] } } ] } }

// client → daemon
{ "type": "answer_question", "id": "q-1", "action": "accept",
  "answers": { "strategy": "balanced" } }
```

Question ids are prefixed `q-`, permission ids `p-`, so a stale answer can never
hit the wrong kind of request.

`id` is the daemon's own, not the provider's: one turn may ask twice, and a
provider that restarts must not collide. Unknown ids are answered with an error
message and change nothing.

## 4. Where each piece sits

| Piece | Change |
|---|---|
| `crates/protocol` | `PermissionRequested`, `AnswerPermission`, `QuestionAsked`, `AnswerQuestion`, `PermissionOption*`, `Question*`; all additive, older peers skip them |
| `crates/providers/driver` | `Asker` trait: `permission(...) -> Decision` and `question(...) -> Answer`; `start_turn` takes it as an argument |
| `crates/core` | `Broker`: id, owner turn, pending senders, `answer(id, decision)`. `run_turn` builds an asker that emits the message and **blocks** on the reply channel; a closed channel reads as cancelled. Each entry remembers its turn, so `cancel_turn(session)` drops exactly that turn's requests and leaves every other session alone |
| `crates/providers/opencode` | turns the ACP request into a `PermissionRequest` (title from `toolCall.title`, detail from the command/path in `rawInput`/`locations`), and answers ACP with the chosen `optionId` |
| `crates/daemon` | while a turn runs the connection must keep reading, so writes move to a writer thread over a cloned socket (`WebSocket::from_raw_socket`); the reader loop stays on the connection thread and routes `AnswerPermission` to the broker |
| `crates/client` | `answer_permission(id, option)` |
| `crates/ui` | `permission.rs` and `question.rs`; `live::Transcript` holds one queue of cards so a permission and a question keep their order; rendered between transcript and composer; Enter allows / submits, Escape rejects / cancels |

## 5. Failure modes

- **Client disconnects mid-turn.** The writer channel closes, the turn thread's
  emit fails, the broker drops the sender, the driver answers `cancelled` and
  the turn ends. Nothing hangs.
- **Two requests at once** (parallel tool calls). The UI queues them and shows
  one card at a time, oldest first; answering pops it.
- **Answer for an unknown id.** Ignored with an `Error` message; the pending
  card stays.
- **Turn aborted.** `CancelTurn` cancels that turn's open requests (the asker
  reads `cancelled`, so the provider can answer the agent honestly) and asks
  the provider to stop. The turn ends as `Aborted`, not as a failure; the UI
  closes its cards when the turn finishes. An abort with no turn in flight is
  an error, so a stale client learns the truth. Queued prompts are not part of
  the turn: they stay queued and the UI can remove them.

## 6. How it was verified

OpenCode 2.3.2 **does** ask: a `write` or `bash` call in a project raises
`session/request_permission`, and the turn stays blocked until the card is
answered. Checked against a scratch project with a real turn: the card showed
the path, Enter allowed it, the file appeared and the agent finished.

The ACP round trip also has a deterministic test, `tests/permission.rs` in the
opencode crate: a scripted ACP agent asks (permission or question, by
`FAKE_ACP_MODE`), the driver turns that into the card's data, the answer goes
back verbatim, and a cancelled permission reads as `cancelled` on the agent's
side. That test exists because a provider's policy can change under us; the wire
should not depend on it, and OpenCode 2.3.2 does not elicit on its own.

The question card was checked the same way, in the app: choose an option,
`Submit` enables, the card goes away and the scripted agent reports
`accept:conservative`.

## 7. Not in this slice

Remembered decisions (the agent owns them), keyed shortcuts beyond Enter and
Escape, the diff/terminal views a detail block may want, and the workspace-path
policy that produces "Blocked path".
