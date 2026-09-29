import { describe, expect, it } from "vitest";
import { rewindableTurns } from "./RewindControl";
import type { Checkpoint } from "@/lib/agent/protocol";

function cp(turn: number, kind: Checkpoint["kind"]): Checkpoint {
  return { sessionID: "ses_1", turn, kind, ref: `refs/circulo/x-${turn}-${kind}`, commit: "abc", createdAt: 0 };
}

describe("rewindableTurns", () => {
  it("offers nothing when there are no checkpoints", () => {
    expect(rewindableTurns([])).toEqual([]);
  });

  it("offers a turn that started and finished", () => {
    expect(rewindableTurns([cp(1, "start"), cp(1, "end")])).toEqual([1]);
  });

  it("orders newest first", () => {
    const all = [
      cp(1, "start"), cp(1, "end"),
      cp(2, "start"), cp(2, "end"),
      cp(3, "start"), cp(3, "end"),
    ];
    expect(rewindableTurns(all)).toEqual([3, 2, 1]);
  });

  // A turn that is still running (or died mid-flight) has a start but no
  // end. Offering it would discard work the user is watching right now.
  it("excludes a turn that started but never finished", () => {
    expect(rewindableTurns([cp(1, "start"), cp(1, "end"), cp(2, "start")])).toEqual([1]);
  });

  it("excludes an end without a start, which is a corrupt pair", () => {
    expect(rewindableTurns([cp(1, "end")])).toEqual([]);
  });

  // Guard refs exist so a rewind can be undone. They are not rewind targets.
  it("ignores guard checkpoints", () => {
    const after = [cp(1, "start"), cp(1, "end"), cp(1, "guard")];
    expect(rewindableTurns(after)).toEqual([1]);
    expect(rewindableTurns([cp(1, "guard")])).toEqual([]);
  });

  it("does not offer the same turn twice when both kinds repeat", () => {
    const duplicated = [cp(1, "start"), cp(1, "start"), cp(1, "end"), cp(1, "end")];
    expect(rewindableTurns(duplicated)).toEqual([1]);
  });

  it("caps the list so the control stays compact", () => {
    const many: Checkpoint[] = [];
    for (let turn = 1; turn <= 25; turn++) {
      many.push(cp(turn, "start"), cp(turn, "end"));
    }
    const offered = rewindableTurns(many);
    expect(offered).toHaveLength(10);
    // The newest ten, not the first ten.
    expect(offered[0]).toBe(25);
    expect(offered[9]).toBe(16);
  });
});
