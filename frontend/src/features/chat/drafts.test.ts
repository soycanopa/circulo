import { describe, expect, it } from "vitest";
import {
  clearDraft,
  draftKey,
  loadDraft,
  pruneDrafts,
  saveDraft,
  type DraftStorage,
} from "./drafts";
import type { CommandScope } from "@/lib/agent/protocol";
import type { Segment } from "./richInput";

/** In-memory Storage, so these tests run without a DOM. */
function memStore(): DraftStorage & { map: Map<string, string> } {
  const map = new Map<string, string>();
  return {
    map,
    getItem: (k) => map.get(k) ?? null,
    setItem: (k, v) => void map.set(k, v),
    removeItem: (k) => void map.delete(k),
    get length() {
      return map.size;
    },
    key: (i) => [...map.keys()][i] ?? null,
  };
}

const seg = {
  text: (value: string): Segment => ({ kind: "text", value }),
  file: (path: string): Segment => ({ kind: "file", path }),
  command: (name: string, scope?: CommandScope): Segment => ({ kind: "command", name, scope }),
};

describe("draftKey", () => {
  it("separates sessions of the same project", () => {
    expect(draftKey("p1", "ses_a")).not.toBe(draftKey("p1", "ses_b"));
  });

  it("separates projects", () => {
    expect(draftKey("p1", "ses_a")).not.toBe(draftKey("p2", "ses_a"));
  });

  // A new session aimed at a different branch is a different target.
  it("includes the branch for a new session", () => {
    expect(draftKey("p1", undefined, "main")).not.toBe(draftKey("p1", undefined, "feature"));
  });

  it("treats no session as its own target", () => {
    expect(draftKey("p1")).toBe(draftKey("p1", undefined));
  });
});

describe("save/load", () => {
  it("round-trips plain text", () => {
    const s = memStore();
    saveDraft(draftKey("p1", "ses_1"), [seg.text("hello")], s);
    expect(loadDraft(draftKey("p1", "ses_1"), s)).toEqual([seg.text("hello")]);
  });

  // The whole point: a chip is a DOM element, not a character. Restoring the
  // projected string alone would paste "@src/app.ts" as text, which then reads
  // as a command the user never picked.
  it("round-trips chips, not just their text", () => {
    const s = memStore();
    const segments: Segment[] = [
      seg.file("src/app.ts"),
      seg.text(" explain "),
      seg.command("review", "builtin"),
    ];
    saveDraft(draftKey("p1", "ses_1"), segments, s);
    const back = loadDraft(draftKey("p1", "ses_1"), s);
    expect(back).toEqual(segments);
    expect(back?.some((x) => x.kind === "file")).toBe(true);
    expect(back?.some((x) => x.kind === "command")).toBe(true);
  });

  it("keeps drafts of different targets apart", () => {
    const s = memStore();
    saveDraft(draftKey("p1", "ses_1"), [seg.text("for session one")], s);
    saveDraft(draftKey("p1", "ses_2"), [seg.text("for session two")], s);
    expect(loadDraft(draftKey("p1", "ses_1"), s)).toEqual([seg.text("for session one")]);
    expect(loadDraft(draftKey("p1", "ses_2"), s)).toEqual([seg.text("for session two")]);
  });

  // Saving an empty draft must remove the key, not store an empty string:
  // a key left behind is a key that shows up in every later restore sweep.
  it("treats an empty save as a clear", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    saveDraft(key, [seg.text("x")], s);
    expect(s.map.size).toBe(1);
    saveDraft(key, [], s);
    expect(s.map.size).toBe(0);
    expect(loadDraft(key, s)).toBeNull();
  });

  it("returns null for a target that never had a draft", () => {
    expect(loadDraft(draftKey("p1", "nope"), memStore())).toBeNull();
  });

  it("drops a corrupt entry instead of failing forever", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    s.setItem(key, "{not json");
    expect(loadDraft(key, s)).toBeNull();
    expect(s.map.has(key)).toBe(false);
  });

  it("drops an entry whose shape is wrong", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    s.setItem(key, JSON.stringify({ segments: "nope", savedAt: Date.now() }));
    expect(loadDraft(key, s)).toBeNull();
  });

  // Anything that is not a segment is dropped, so a hand-edited or
  // half-migrated entry cannot inject junk into the editor.
  it("filters out entries that are not segments", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    s.setItem(
      key,
      JSON.stringify({
        segments: [{ kind: "text", value: "keep" }, { kind: "bogus" }, null, 42],
        savedAt: Date.now(),
      }),
    );
    expect(loadDraft(key, s)).toEqual([seg.text("keep")]);
  });

  it("drops a draft older than a week", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    s.setItem(key, JSON.stringify({ segments: [seg.text("stale")], savedAt: 1 }));
    // now=0 makes the entry look a week and a half in the future → expired
    expect(loadDraft(key, s, 8 * 24 * 60 * 60 * 1000)).toBeNull();
    expect(s.map.has(key)).toBe(false);
  });

  it("keeps a fresh draft", () => {
    const s = memStore();
    const key = draftKey("p1", "ses_1");
    saveDraft(key, [seg.text("fresh")], s);
    expect(loadDraft(key, s, Date.now() + 60_000)).toEqual([seg.text("fresh")]);
  });

  it("degrades to a no-op without storage", () => {
    expect(() => saveDraft("k", [seg.text("x")], null)).not.toThrow();
    expect(loadDraft("k", null)).toBeNull();
    expect(() => clearDraft("k", null)).not.toThrow();
  });

  it("survives a storage that throws on write", () => {
    const hostile: DraftStorage = {
      getItem: () => null,
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
      removeItem: () => undefined,
    };
    // A draft must never be worth failing a keystroke over.
    expect(() => saveDraft("k", [seg.text("x")], hostile)).not.toThrow();
  });

  it("survives a storage that throws on read", () => {
    const hostile: DraftStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => undefined,
      removeItem: () => undefined,
    };
    expect(loadDraft("k", hostile)).toBeNull();
  });
});

describe("pruneDrafts", () => {
  it("removes drafts whose target is gone", () => {
    const s = memStore();
    const live = draftKey("p1", "ses_1");
    saveDraft(live, [seg.text("keep")], s);
    saveDraft(draftKey("p1", "ses_gone"), [seg.text("drop")], s);
    saveDraft(draftKey("p2", "ses_2"), [seg.text("drop too")], s);

    pruneDrafts(new Set([live]), s);
    expect([...s.map.keys()]).toEqual([live]);
  });

  it("never touches keys that are not drafts", () => {
    const s = memStore();
    s.setItem("circulogo.sidebarWidth", "260");
    s.setItem("some.other.key", "x");
    saveDraft(draftKey("p1", "ses_gone"), [seg.text("drop")], s);

    pruneDrafts(new Set(), s);
    expect(s.map.has("circulogo.sidebarWidth")).toBe(true);
    expect(s.map.has("some.other.key")).toBe(true);
  });

  it("does nothing on a storage that cannot enumerate", () => {
    const minimal: DraftStorage = {
      getItem: () => null,
      setItem: () => undefined,
      removeItem: () => undefined,
    };
    expect(() => pruneDrafts(new Set(), minimal)).not.toThrow();
  });
});
