import { describe, expect, it } from "vitest";

import { activeAtToken, activeSlashToken, findCommandToken, parseSlash } from "@/features/chat/Composer";

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
    // Whitespace after "@" ends the mention query.
    expect(activeAtToken("a @b c", 6)).toBeNull();
  });

  it("extracts the token right before the caret", () => {
    expect(activeAtToken("@src/ma", 7)).toBe("@src/ma");
    expect(activeAtToken("mira @src", 9)).toBe("@src");
  });

  it("keeps only the segment after the last @", () => {
    expect(activeAtToken("a @b @c", 7)).toBe("@c");
  });

  it("fires mid-word: @ works anywhere, no leading space required", () => {
    expect(activeAtToken("hola@", 5)).toBe("@");
    expect(activeAtToken("revisa c@d", 10)).toBe("@d");
  });

  it("matches @ alone (menu opens with empty query)", () => {
    expect(activeAtToken("@", 1)).toBe("@");
    expect(activeAtToken("hola @", 6)).toBe("@");
  });
});

describe("activeSlashToken", () => {
  it("extracts the /token before the caret", () => {
    expect(activeSlashToken("/con", 4)).toBe("/con");
    expect(activeSlashToken("hola /con", 9)).toBe("/con");
    expect(activeSlashToken("hola /", 6)).toBe("/");
  });

  it("requires the slash to start the text or follow whitespace", () => {
    // Paths and URLs must not pop the command menu mid-word.
    expect(activeSlashToken("src/comp", 8)).toBeNull();
    expect(activeSlashToken("https://x", 9)).toBeNull();
  });

  it("ends the query at whitespace after the slash", () => {
    expect(activeSlashToken("hola /con adios", 15)).toBeNull();
  });

  it("returns null outside a token", () => {
    expect(activeSlashToken("hola", 4)).toBeNull();
    expect(activeSlashToken("", 0)).toBeNull();
  });
});

describe("findCommandToken", () => {
  it("finds the token mid-text", () => {
    expect(findCommandToken("revisa /init eso", "/init")).toEqual({ start: 7, end: 12 });
    expect(findCommandToken("/init", "/init")).toEqual({ start: 0, end: 5 });
  });

  it("respects token boundaries", () => {
    // A longer token must not match a shorter command name, and a slash
    // glued to a preceding word is not a command.
    expect(findCommandToken("/initx", "/init")).toBeNull();
    expect(findCommandToken("x/init", "/init")).toBeNull();
    expect(findCommandToken("/initx y", "/init")).toBeNull();
  });

  it("can relax the leading boundary for mid-word @ picks", () => {
    expect(findCommandToken("revisa c@init", "@init", false)).toEqual({ start: 8, end: 13 });
    expect(findCommandToken("revisa c@init", "@init")).toBeNull();
    expect(findCommandToken("a @b@c d", "@b@c", false)).toEqual({ start: 2, end: 6 });
  });

  it("returns null when absent", () => {
    expect(findCommandToken("hola mundo", "/init")).toBeNull();
    expect(findCommandToken("hola /init", "/compact")).toBeNull();
  });
});
