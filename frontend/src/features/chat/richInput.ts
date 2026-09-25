/**
 * DOM bridge for the composer's rich input: a contenteditable whose content
 * is plain text runs plus atomic chip spans ("/cmd", "@file") rendered with
 * the agreed tag design. Chips are contenteditable=false so the browser
 * treats them as single units.
 *
 * The rest of the composer (menus, detectors, submit) works on a *projection*
 * of this content: plain text where each chip counts as one character (a
 * space). That keeps activeSlashToken / activeAtToken and their tests valid —
 * only the sync points here know about the DOM.
 */

export type Segment =
  | { kind: "text"; value: string }
  | { kind: "command"; name: string; source?: string }
  | { kind: "file"; path: string };

export const isChip = (n: Node): n is HTMLElement =>
  n.nodeType === Node.ELEMENT_NODE && (n as HTMLElement).dataset.chip !== undefined;

/** Walks the editor DOM into the segment model. <br> maps to "\n". */
export function readSegments(root: HTMLElement): Segment[] {
  const out: Segment[] = [];
  let text = "";
  const flush = () => {
    if (text) {
      out.push({ kind: "text", value: text });
      text = "";
    }
  };
  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      text += node.nodeValue ?? "";
      return;
    }
    if (node.nodeName === "BR") {
      text += "\n";
      return;
    }
    if (isChip(node)) {
      flush();
      const ds = node.dataset;
      if (ds.chip === "command") {
        out.push({ kind: "command", name: ds.name ?? "", source: ds.source || undefined });
      } else {
        out.push({ kind: "file", path: ds.path ?? "" });
      }
      return;
    }
    node.childNodes.forEach(walk);
  };
  root.childNodes.forEach(walk);
  flush();
  return out;
}

/** The prompt string the segments project to: text verbatim, chips as their
 * canonical tokens ("/name", "@path"). */
export function projectedPrompt(segs: Segment[]): string {
  return segs
    .map((s) => (s.kind === "text" ? s.value : s.kind === "command" ? `/${s.name}` : `@${s.path}`))
    .join("");
}

/** Projected plain text: verbatim text runs, one space per chip, "\n" per
 * <br>. The coordinate space the caret offset lives in. */
export function projectedText(root: HTMLElement): string {
  let out = "";
  const walk = (n: Node) => {
    if (n.nodeType === Node.TEXT_NODE) {
      out += n.nodeValue ?? "";
      return;
    }
    if (n.nodeName === "BR") {
      out += "\n";
      return;
    }
    if (isChip(n)) {
      out += " ";
      return;
    }
    n.childNodes.forEach(walk);
  };
  root.childNodes.forEach(walk);
  return out;
}

/** Projected text plus the caret offset into that projection, or null when
 * the selection is not inside the editor. */
export function caretProjection(root: HTMLElement): { text: string; caret: number | null } {
  const text = projectedText(root);
  const sel = window.getSelection();
  if (!sel || sel.anchorNode == null || !root.contains(sel.anchorNode)) {
    return { text, caret: null };
  }
  let acc = 0;
  let caret: number | null = null;
  const anchor = sel.anchorNode;
  const anchorOff = sel.anchorOffset;
  // sizeOf counts a node's projection length (chips = 1, br = 1).
  const sizeOf = (n: Node): number => {
    if (n.nodeType === Node.TEXT_NODE) return n.nodeValue?.length ?? 0;
    if (n.nodeName === "BR" || isChip(n)) return 1;
    let s = 0;
    n.childNodes.forEach((c) => (s += sizeOf(c)));
    return s;
  };
  const walk = (n: Node): boolean => {
    if (n === anchor) {
      let off = 0;
      if (n.nodeType === Node.TEXT_NODE) {
        off = anchorOff;
      } else {
        for (let i = 0; i < anchorOff; i++) off += sizeOf(n.childNodes[i]);
      }
      caret = acc + off;
      return true;
    }
    if (n.nodeType === Node.TEXT_NODE) {
      acc += n.nodeValue?.length ?? 0;
      return false;
    }
    if (n.nodeName === "BR" || isChip(n)) {
      acc += 1;
      return false;
    }
    for (const c of Array.from(n.childNodes)) if (walk(c)) return true;
    return false;
  };
  walk(root);
  // Anchor inside a chip subtree (click on a chip) has no projection slot —
  // clamp to the end.
  return { text, caret: caret ?? text.length };
}

/** The chip whose projection slot ends exactly at `caret` (i.e. Backspace at
 * `caret` deletes it), or null. */
export function chipBeforeCaret(root: HTMLElement, caret: number): HTMLElement | null {
  let acc = 0;
  const walk = (n: Node): HTMLElement | null => {
    if (isChip(n)) {
      acc += 1;
      return acc === caret ? n : null;
    }
    if (n.nodeType === Node.TEXT_NODE) {
      acc += n.nodeValue?.length ?? 0;
      return null;
    }
    if (n.nodeName === "BR") {
      acc += 1;
      return null;
    }
    for (const c of Array.from(n.childNodes)) {
      const hit = walk(c);
      if (hit) return hit;
    }
    return null;
  };
  return walk(root);
}

/** Maps a projected offset to a DOM position (text node + offset, or an
 * element + child index at chip boundaries). */
export function resolveOffset(
  root: HTMLElement,
  target: number,
): { node: Node; offset: number } | null {
  let acc = 0;
  const childIndex = (parent: Node, n: Node) =>
    Array.prototype.indexOf.call(parent.childNodes, n);
  const visit = (n: Node): { node: Node; offset: number } | null => {
    if (n.nodeType === Node.TEXT_NODE) {
      const len = n.nodeValue?.length ?? 0;
      if (target < acc + len) return { node: n, offset: target - acc };
      acc += len;
      if (target === acc) return { node: n, offset: len };
      return null;
    }
    if (isChip(n)) {
      const parent = n.parentElement ?? root;
      if (target === acc) return { node: parent, offset: childIndex(parent, n) };
      acc += 1;
      if (target === acc)
        return { node: parent, offset: childIndex(parent, n) + 1 };
      return null;
    }
    if (n.nodeName === "BR") {
      acc += 1;
      return null;
    }
    for (const c of Array.from(n.childNodes)) {
      const r = visit(c);
      if (r) return r;
    }
    return null;
  };
  return visit(root);
}

/**
 * Replaces the projected range [start, end) with `chip` and `trailing` text,
 * then parks the caret after it. Used by both command and file accepts —
 * chips land exactly where the user was typing.
 */
export function insertChipAt(
  root: HTMLElement,
  chip: HTMLElement,
  start: number,
  end: number,
  trailing = " ",
): void {
  const from = resolveOffset(root, start);
  const to = resolveOffset(root, end);
  if (!from || !to) return;
  const range = document.createRange();
  range.setStart(from.node, from.offset);
  range.setEnd(to.node, to.offset);
  range.deleteContents();
  range.insertNode(chip);
  const space = document.createTextNode(trailing);
  chip.after(space);
  const sel = window.getSelection();
  const after = document.createRange();
  after.setStart(space, trailing.length);
  after.collapse(true);
  sel?.removeAllRanges();
  sel?.addRange(after);
}

const X_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>';

export interface ChipSpec {
  kind: "command" | "file";
  label: string;
  title: string;
  className: string;
  name?: string;
  path?: string;
  source?: string;
  onRemove: () => void;
}

/** Builds one tag chip — the same visual contract as the old top-row tag:
 * rounded-full bordered pill, label, X button. Imperative DOM because it is
 * inserted directly into the contenteditable. */
export function buildChip(spec: ChipSpec): HTMLElement {
  const el = document.createElement("span");
  el.contentEditable = "false";
  el.dataset.chip = spec.kind;
  if (spec.name !== undefined) el.dataset.name = spec.name;
  if (spec.path !== undefined) el.dataset.path = spec.path;
  if (spec.source) el.dataset.source = spec.source;
  el.className = `mr-0.5 inline-flex shrink-0 items-center gap-1 self-center rounded-full border px-1.5 py-0.5 text-xs leading-[14px] font-medium align-baseline ${spec.className}`;
  el.title = spec.title;
  const label = document.createElement("span");
  label.textContent = spec.label;
  el.appendChild(label);
  const btn = document.createElement("button");
  btn.type = "button";
  btn.setAttribute("aria-label", `Remove ${spec.label.trim()}`);
  btn.className = "-mr-0.5 rounded-full p-px transition-colors hover:bg-black/20";
  btn.innerHTML = X_SVG;
  btn.addEventListener("mousedown", (e) => e.preventDefault());
  btn.addEventListener("click", () => {
    el.remove();
    spec.onRemove();
  });
  // The chip body swallows mousedowns so clicking it never steals focus or
  // moves the caret into an uneditable region.
  el.addEventListener("mousedown", (e) => e.preventDefault());
  el.appendChild(btn);
  return el;
}
