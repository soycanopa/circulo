import { describe, expect, it } from "vitest";

import { parseFlowBlock } from "./Flowchart";

const VALID = JSON.stringify({
  nodes: [
    { id: "a", row: 0, x: 0.5, w: 300, kind: "Trigger", hue: "purple", title: "New order", caption: "fires" },
    { id: "b", row: 1, x: 0.5, kind: "If / Else", condition: [["order.flavor", "is", "Pistachio"]] },
    { id: "c", row: 2, x: 0.2, hue: "green", title: "Ship it" },
  ],
  edges: [
    { from: "a", to: "b" },
    { from: "b", to: "c" },
  ],
});

describe("parseFlowBlock", () => {
  it("accepts a valid document with defaults filled", () => {
    const doc = parseFlowBlock(VALID);
    expect(doc).not.toBeNull();
    expect(doc!.nodes).toHaveLength(3);
    expect(doc!.nodes[0].w).toBe(300);
    expect(doc!.nodes[1].w).toBe(300); // default
    expect(doc!.nodes[1].condition).toEqual([["order.flavor", "is", "Pistachio"]]);
    expect(doc!.edges).toEqual([
      { from: "a", to: "b" },
      { from: "b", to: "c" },
    ]);
  });

  it("rejects malformed json, missing fields, bad x, unknown edges, duplicate ids", () => {
    expect(parseFlowBlock("not json")).toBeNull();
    expect(parseFlowBlock("{}")).toBeNull();
    expect(
      parseFlowBlock(JSON.stringify({ nodes: [{ id: "a", row: 0, x: 2 }], edges: [] })),
    ).toBeNull(); // x out of range
    expect(
      parseFlowBlock(JSON.stringify({ nodes: [{ id: "a", row: 0, x: 0.5 }], edges: [{ from: "a", to: "ghost" }] })),
    ).toBeNull(); // unknown edge target
    expect(
      parseFlowBlock(
        JSON.stringify({
          nodes: [
            { id: "a", row: 0, x: 0.5 },
            { id: "a", row: 1, x: 0.5 },
          ],
          edges: [],
        }),
      ),
    ).toBeNull(); // duplicate id
    expect(
      parseFlowBlock(JSON.stringify({ nodes: [{ id: "a", row: "0", x: 0.5 }], edges: [] })),
    ).toBeNull(); // row not a number
  });
});
