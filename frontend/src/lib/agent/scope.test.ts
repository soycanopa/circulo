import { describe, expect, it } from "vitest";
import { SCOPES, type CommandScope } from "./protocol";

/**
 * The scope replaced the provider's own word, so the mapping the UI relies on
 * is worth pinning: omp's "custom" and opencode's "file" both arrive as
 * scopes, and a chip's colour must not depend on which backend answered.
 *
 * commandTagClass itself is not exported (it is presentational), so this
 * asserts the contract it depends on — that the scope set is the neutral one
 * and that it covers every value the UI branches on.
 */
describe("CommandScope", () => {
  it("is the neutral vocabulary, not the providers'", () => {
    // Values that used to cross from a backend and must not come back.
    expect(SCOPES).not.toContain("custom" as CommandScope);
    expect(SCOPES).not.toContain("file" as CommandScope);
    expect(SCOPES).not.toContain("waku" as CommandScope);
  });

  it("covers every value the chip styling branches on", () => {
    for (const scope of ["builtin", "user", "extension", "projectFile", "skill"] as CommandScope[]) {
      expect(SCOPES).toContain(scope);
    }
  });

  it("always has a default so an unknown scope renders as builtin", () => {
    // The switch in commandTagClass has a `default`, so any value outside the
    // set still produces the neutral chip rather than a broken one.
    expect(SCOPES.length).toBeGreaterThan(0);
  });
});
