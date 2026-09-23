import { describe, expect, it } from "vitest";

import { parseSlash } from "@/features/chat/Composer";

describe("parseSlash", () => {
  it("rejects non-slash text", () => {
    expect(parseSlash("hello world")).toBeNull();
    expect(parseSlash("")).toBeNull();
    // A slash mid-text is not an invocation.
    expect(parseSlash("run /context now")).toBeNull();
  });

  it("parses a bare name", () => {
    expect(parseSlash("/context")).toEqual({ name: "context", args: "" });
  });

  it("parses name while typing (partial filter)", () => {
    expect(parseSlash("/cont")).toEqual({ name: "cont", args: "" });
  });

  it("parses name with arguments", () => {
    expect(parseSlash("/switch opus")).toEqual({ name: "switch", args: "opus" });
    // Leading whitespace of args is trimmed; inner spacing is preserved
    // verbatim for the provider to parse.
    expect(parseSlash("/switch  multi  space ")).toEqual({
      name: "switch",
      args: "multi  space ",
    });
  });

  it("keeps empty args when the slash prefix stands alone", () => {
    expect(parseSlash("/")).toEqual({ name: "", args: "" });
  });
});
