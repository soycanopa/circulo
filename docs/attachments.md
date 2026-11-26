# Circulo — Attachments

- **Status:** Implemented (2026-11-26), first slice: images and text files.
- **Related:** [TRD](trd.md) · [UI](ui.md) · [OpenCode](providers/opencode.md) · [Codex](providers/codex.md).

## Goal

A prompt can carry files: screenshots, logs, specs. The daemon copies them into
a content-addressed blob store before the prompt is queued, so a queued prompt
cannot lose its files, history survives a move of the source, and a remote
client later has bytes on the daemon side.

Scope of this slice: **raster images and text files**. Binaries are refused.
Drag/drop, paste, previews and remote upload are out of scope.

## Blob store (persistence, schema 5)

- Root: `<database dir>/blobs` (`~/.circulo/blobs` by default).
- Layout: `blobs/<blake3 hex>/<original file name>`. The hash is the content
  address; the file name keeps the extension providers sniff. The same bytes
  under the same name are written once.
- Table `attachments`: `id` (UUID), `hash`, `name`, `mime`, `size`,
  `created_at`. Every pick gets its own row, even when it deduplicates bytes.
- Table `turn_attachments`: `(session_id, turn, position, attachment_id)`.
  `Store::trim_turns` deletes the links of the turns it removes. Blobs stay:
  there is no GC yet, and a deleted turn's blob is harmless.
- MIME comes from the extension (a small map in `persistence`); unknown files
  are `application/octet-stream` and the core refuses them.
- Limits: **10 MiB per file** (store), **20 MiB per prompt** and **8 files**
  (core). Text inlined for Codex is capped at **256 KiB** per file.
- The store never touches a provider: the core asks for the blob path and hands
  it to the driver.

## Wire (additive, version stays 3)

```jsonc
// client → daemon; paths on the daemon host
{ "type": "prompt", "session": "s1", "text": "look",
  "attachments": ["/Users/me/shot.png", "/Users/me/log.txt"] }
{ "type": "steer", "session": "s1", "text": "also this",
  "attachments": ["/Users/me/note.md"] }

// daemon → client
AttachmentSummary = { "id": "…", "name": "shot.png", "mime": "image/png", "size": 1234 }
// turn_started and history turns carry them
{ "type": "turn_started", "session": "s1", "turn": 3, "prompt": "look",
  "attachments": [AttachmentSummary] }
{ "type": "history", "session": "s1", "turns": [
  { "turn": 3, "prompt": "look", "parts": [], "attachments": [AttachmentSummary] } ] }
// queued prompts carry them too
{ "type": "queue", "session": "s1", "items": [
  { "id": "q-1", "text": "next", "attachments": [AttachmentSummary] } ] }
```

Old peers decode without the fields (serde defaults); they simply cannot send
files. A remote client will need an upload message before it can use paths that
do not exist on the daemon host; that is post-v1.0.

## Core

- `enqueue_with` / `steer_with` ingest the paths **before** the queue lock, so a
  queued prompt already owns its blobs.
- The queue item carries `AttachmentSummary`; the blob path is resolved from
  the store when the turn starts.
- `run_turn_prompt` links the summaries to the turn, builds
  `provider::Attachment { name, mime, size, path }` and puts them in
  `SessionRef.attachments` (the per-turn ref the driver already receives).
- `TurnStarted` and `HistoryTurn` carry the summaries; the UI draws chips.
- An unsupported file (binary) is refused with an error and the prompt is not
  queued. A file that disappears between pick and send fails the same way.

## Providers

| Provider | Image | Text |
|---|---|---|
| OpenCode (ACP) | `image` block with base64 data when `agentCapabilities.promptCapabilities.image` is true; otherwise a `resource_link` | `resource_link` to the blob (`file://` URI, name, mime, size) |
| Codex (app-server) | `localImage { path }` | inlined as a second `text` input: `Attached file: <name>` plus a fenced block, capped at 256 KiB |

The demo ignores attachments; the data path is covered by the core and
provider tests. A provider that gets a file it cannot use never fails the
prompt silently: the core only sends supported kinds, and each driver maps what
its protocol has.

## UI

- A paperclip in the composer and **Cmd/Ctrl+Shift+A** open the native
  multi-file picker.
- Picked files sit as chips above the field, each removable, at most 8.
- Sending (or queueing/steering) hands the paths to the daemon and clears the
  chips; a queued prompt shows an attachment count in its row.
- A user turn shows its file names under the prompt inside the bubble, from the
  wire, so a reopened session draws them too.
- The chip is a name, not a preview; clicking it does nothing yet.

## Verification

- Persistence: v4→v5 migration, dedup by bytes, per-file cap, link and
  transcript round trip, trim removes links and keeps the blob.
- Core: a text file reaches the driver as `SessionRef.attachments`, the turn is
  linked, history returns it; a binary is refused before a turn.
- OpenCode: a scripted ACP agent advertises `image` and reports
  `blocks=text,image,resource_link`.
- Codex: a scripted app-server reports `input=text,localImage,text`.
- UI: a `TurnStarted` with attachments fills the turn and draws chips; the
  composer's pending list and picker are manual smoke (native panel).

## Limits, recorded on purpose

- No binary attachments, no PDF/office previews, no image thumbnails.
- No drag/drop or paste; no remote upload yet.
- Blobs are never garbage-collected; attachment rows accumulate one per pick.
- Text files are inlined for Codex, so a 256 KiB file costs tokens; the cap is
  visible in the prompt.
