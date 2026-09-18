import { describe, expect, it } from "vitest";

import { groupByDate } from "./groupByDate";

const now = new Date("2026-09-02T15:00:00").getTime();
const at = (iso: string) => new Date(iso).getTime();
const s = (id: string, ts: number) => ({ id, timeUpdated: ts, timeCreated: ts });

describe("groupByDate", () => {
  it("buckets into Today / Earlier, in order", () => {
    const groups = groupByDate(
      [
        s("old", at("2026-08-01T10:00:00")),
        s("yesterday", at("2026-09-01T23:59:00")),
        s("today-late", at("2026-09-02T14:00:00")),
        s("today-early", at("2026-09-02T00:05:00")),
      ],
      now,
    );
    expect(groups.map((g) => g.label)).toEqual(["Today", "Earlier"]);
    expect(groups[0].items.map((i) => i.id)).toEqual(["today-late", "today-early"]);
    expect(groups[1].items.map((i) => i.id)).toEqual(["old", "yesterday"]);
  });

  it("drops empty buckets (single today-only input)", () => {
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

  it("boundary: a session exactly at the start of today is Today", () => {
    const groups = groupByDate([s("edge", at("2026-09-02T00:00:00"))], now);
    expect(groups[0].key).toBe("today");
  });
});
