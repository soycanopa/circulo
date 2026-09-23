import { describe, expect, it } from "vitest";

import { activeAtToken, parseSlash } from "@/features/chat/Composer";

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

describe("activeAtToken", () => {
  it("returns null when the caret is outside a mention", () => {
    expect(activeAtToken("hola mundo", 10)).toBeNull();
    expect(activeAtToken("", 0)).toBeNull();
  });

  it("extracts the token right before the caret", () => {
    expect(activeAtToken("@src/ma", 7)).toBe("@src/ma");
    // Query starts after whitespace only.
    expect(activeAtToken("mira @src", 9)).toBe("@src");
  });

  it("keeps only the token segment after the last whitespace", () => {
    expect(activeAtToken("a @b @c", 7)).toBe("@c");
    // Non-mention word before the caret is not a query.
    expect(activeAtToken("a @b c@d", 8)).toBeNull();
  });

  it("matches @ alone (menu opens with empty query)", () => {
    expect(activeAtToken("@", 1)).toBe("@");
    expect(activeAtToken("hola @", 6)).toBe("@");
  });
});
