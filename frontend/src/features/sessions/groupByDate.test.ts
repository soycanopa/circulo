import { describe, expect, it } from "vitest";

import { groupByDate } from "./groupByDate";

const now = new Date("2026-09-02T15:00:00").getTime();
const at = (iso: string) => new Date(iso).getTime();
const s = (id: string, ts: number) => ({ id, timeUpdated: ts, timeCreated: ts });

describe("groupByDate", () => {
  it("buckets into Today / Yesterday / This week / Older, in order", () => {
    const groups = groupByDate(
      [
        s("old", at("2026-08-01T10:00:00")),
        s("week", at("2026-08-29T10:00:00")),
        s("yesterday", at("2026-09-01T23:59:00")),
        s("today-late", at("2026-09-02T14:00:00")),
        s("today-early", at("2026-09-02T00:05:00")),
      ],
      now,
    );
    expect(groups.map((g) => g.label)).toEqual([
      "Today",
      "Yesterday",
      "This week",
      "Older",
    ]);
    expect(groups[0].items.map((i) => i.id)).toEqual(["today-late", "today-early"]);
  });

  it("drops empty buckets and keeps newest first within a bucket (input order)", () => {
    const groups = groupByDate([s("only", at("2026-09-02T09:00:00"))], now);
    expect(groups).toHaveLength(1);
    expect(groups[0].label).toBe("Today");
  });

  it("falls back to timeCreated when timeUpdated is 0", () => {
    const groups = groupByDate(
      [{ id: "c", timeUpdated: 0, timeCreated: at("2026-09-02T08:00:00") }],
      now,
    );
    expect(groups[0].label).toBe("Today");
  });
});
