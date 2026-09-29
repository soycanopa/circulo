import { describe, expect, it } from "vitest";
import { can, type Capabilities } from "./protocol";

/**
 * `can` is the single place the UI reads capabilities, so the failure mode
 * that matters is a missing or partial block being read as support — that is
 * how a control ends up on screen doing nothing.
 */

const full: Capabilities = {
  permissions: true,
  forms: true,
  steer: true,
  branch: true,
  agents: true,
  commands: true,
  accessMode: true,
};

describe("can", () => {
  it("is true for a declared capability", () => {
    expect(can({ capabilities: full }, "branch")).toBe(true);
  });

  it("is false for a capability the backend does not have", () => {
    expect(can({ capabilities: { ...full, branch: false } }, "branch")).toBe(false);
  });

  it("is false when meta has not loaded yet", () => {
    expect(can(undefined, "branch")).toBe(false);
    expect(can({}, "branch")).toBe(false);
  });

  // A partial block must not make the missing keys read as true.
  it("treats a partial capability block as unsupported, not supported", () => {
    const partial = { branch: true } as Capabilities;
    expect(can({ capabilities: partial }, "branch")).toBe(true);
    expect(can({ capabilities: partial }, "permissions")).toBe(false);
    expect(can({ capabilities: partial }, "steer")).toBe(false);
  });

  it("is false for every capability on an empty block", () => {
    const none = {} as Capabilities;
    for (const key of Object.keys(full) as (keyof Capabilities)[]) {
      expect(can({ capabilities: none }, key)).toBe(false);
    }
  });
});
