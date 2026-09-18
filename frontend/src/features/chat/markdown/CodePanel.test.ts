import { describe, expect, it } from "vitest";

import { highlight, parseDiffLines } from "./CodePanel";

const DIFF = `--- a/churn.ts
+++ b/churn.ts
@@ -1,6 +1,7 @@
 export async function churnBatch() {
-  await freezer.store(base, { temp: "-14C" });
+  await freezer.store(base, { temp: "-16C" });
+  if (!base.approved) return null;
   return base.gallons;
 }`;

describe("parseDiffLines", () => {
  it("skips headers, types rows, and synthesizes gutter numbers", () => {
    const rows = parseDiffLines(DIFF);
    expect(rows.map((r) => r.type)).toEqual([
      "ctx", "del", "add", "add", "ctx", "ctx",
    ]);
    // gutter numbers: del keeps the old number, add shows the new one
    expect(rows[1]).toMatchObject({ old: 2, cur: null });
    expect(rows[2]).toMatchObject({ old: null, cur: 3 });
    expect(rows[3]).toMatchObject({ old: null, cur: 4 });
    expect(rows[4]).toMatchObject({ old: 3, cur: 5 });
    expect(rows[5]).toMatchObject({ old: 4, cur: 6 });
  });

  it("pairs a del+add run into word-level pieces", () => {
    const rows = parseDiffLines(DIFF);
    const del = rows[1];
    const add = rows[2];
    expect(del.pieces).toEqual([
      { text: '  await freezer.store(base, { temp: ' },
      { text: '"-14C"', change: "del" },
      { text: ' });' },
    ]);
    expect(add.pieces).toEqual([
      { text: '  await freezer.store(base, { temp: ' },
      { text: '"-16C"', change: "add" },
      { text: ' });' },
    ]);
    // standalone adds are single-piece
    expect(rows[3].pieces).toEqual([{ text: '  if (!base.approved) return null;' }]);
  });
});

describe("highlight", () => {
  it("colors keywords, strings, numbers and calls without losing text", () => {
    const nodes = highlight('const n = getFlavor(2) + "hi"');
    const text = nodes.map((n) => (typeof n === "object" && n !== null && "props" in n ? String(n.props.children) : String(n))).join("");
    expect(text).toBe('const n = getFlavor(2) + "hi"');
  });
});
