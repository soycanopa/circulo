import { describe, expect, it } from "vitest";

import { timeAgo } from "./timeAgo";

const now = new Date("2026-09-02T15:00:00").getTime();

describe("timeAgo", () => {
  it("formats minutes", () => {
    expect(timeAgo(now - 16 * 60_000, now)).toBe("16m");
  });
  it("formats hours and days", () => {
    expect(timeAgo(now - 3 * 3_600_000, now)).toBe("3h");
    expect(timeAgo(now - 2 * 86_400_000, now)).toBe("2d");
  });
  it("formats weeks and months", () => {
    expect(timeAgo(now - 10 * 86_400_000, now)).toBe("1w");
    expect(timeAgo(now - 45 * 86_400_000, now)).toBe("1mo");
  });
  it("handles just-now and empty", () => {
    expect(timeAgo(now - 5_000, now)).toBe("now");
    expect(timeAgo(0, now)).toBe("");
  });
});
