import { useEffect, useRef, useState } from "react";

/* ─────────────────────────────────────────────────────────
 * FLOWCHART — an agent workflow on a dotted canvas (owner's
 * design). The agent emits a ```circulogo-flow fenced block
 * with JSON (taught via the session format instruction);
 * MarkdownView renders it here instead of as code.
 *
 * JSON shape (validated by parseFlowBlock):
 *   { nodes: [{ id, row, x, w?, kind?, hue?, title?, caption?,
 *               condition?: [[left, op, right], …] }],
 *     edges: [{ from, to }] }
 * row = depth (0..n, top to bottom); x = horizontal center 0–1;
 * hue = purple|amber|blue|green|red. Condition nodes render
 * read-only chip rows (If / and).
 * ───────────────────────────────────────────────────────── */

export type FlowNode = {
  id: string;
  row: number;
  x: number; // 0–1 center of the node
  w?: number;
  kind?: string;
  hue?: string;
  title?: string;
  caption?: string;
  condition?: [string, string, string][];
};

export type FlowEdge = { from: string; to: string };
export type FlowDoc = { nodes: FlowNode[]; edges: FlowEdge[] };

const HUES: Record<string, string> = {
  purple: "#9a5cff",
  amber: "#f09a2f",
  blue: "#4f8df7",
  green: "#35b47c",
  red: "#e66b6b",
};

const hueColor = (hue?: string): string =>
  (hue && HUES[hue]) || (hue?.startsWith("#") ? hue : HUES.purple);

const mix = (hue: string, pct: number, base = "var(--bg-popover)") =>
  `color-mix(in srgb, ${hue} ${pct}%, ${base})`;

/** Validates + normalizes one circulogo-flow block; null → render as code. */
export function parseFlowBlock(text: string): FlowDoc | null {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof raw !== "object" || raw === null) return null;
  const { nodes, edges } = raw as Record<string, unknown>;
  if (!Array.isArray(nodes) || nodes.length === 0 || !Array.isArray(edges)) return null;

  const outNodes: FlowNode[] = [];
  const ids = new Set<string>();
  for (const n of nodes) {
    if (typeof n !== "object" || n === null) return null;
    const node = n as Record<string, unknown>;
    if (typeof node.id !== "string" || typeof node.row !== "number" || typeof node.x !== "number") {
      return null;
    }
    if (node.x < 0 || node.x > 1) return null;
    if (ids.has(node.id)) return null;
    ids.add(node.id);
    let condition: [string, string, string][] | undefined;
    if (Array.isArray(node.condition)) {
      condition = [];
      for (const row of node.condition) {
        if (!Array.isArray(row) || row.length < 2) return null;
        const [left, op, right] = row;
        if (typeof left !== "string" || typeof op !== "string") return null;
        condition.push([left, op, typeof right === "string" ? right : ""]);
      }
    }
    outNodes.push({
      id: node.id,
      row: node.row,
      x: node.x,
      w: typeof node.w === "number" ? node.w : 300,
      kind: typeof node.kind === "string" ? node.kind : undefined,
      hue: typeof node.hue === "string" ? node.hue : undefined,
      title: typeof node.title === "string" ? node.title : undefined,
      caption: typeof node.caption === "string" ? node.caption : undefined,
      condition,
    });
  }
  const outEdges: FlowEdge[] = [];
  for (const e of edges) {
    if (typeof e !== "object" || e === null) return null;
    const edge = e as Record<string, unknown>;
    if (typeof edge.from !== "string" || typeof edge.to !== "string") return null;
    if (!ids.has(edge.from) || !ids.has(edge.to)) return null;
    outEdges.push({ from: edge.from, to: edge.to });
  }
  return { nodes: outNodes, edges: outEdges };
}

/* ── pieces ── */

function Chevron() {
  return (
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" className="shrink-0 text-text-tertiary">
      <path d="m6 9 6 6 6-6" />
    </svg>
  );
}

function Handle() {
  return (
    <svg width="10" height="16" viewBox="0 0 10 16" className="shrink-0 cursor-grab text-text-tertiary/70">
      {[3, 8, 13].flatMap((y) => [
        <circle key={`l${y}`} cx="3" cy={y} r="1.1" fill="currentColor" />,
        <circle key={`r${y}`} cx="7.5" cy={y} r="1.1" fill="currentColor" />,
      ])}
    </svg>
  );
}

function ConditionBody({ rows }: { rows: [string, string, string][] }) {
  return (
    <div className="flex flex-col gap-1.5 px-3 py-2.5">
      {rows.map((row, i) => (
        <div key={i} className="flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-1">
          <Handle />
          <span className="w-7 text-[12.5px] text-text-secondary">{i === 0 ? "If" : "and"}</span>
          <span className="inline-flex h-6 max-w-full shrink items-center truncate rounded-[6px] bg-bg-code px-1.5 text-[12px] font-medium text-text-primary">
            {row[0]}
          </span>
          <span className="text-[12.5px] text-text-secondary">{row[1] || "is"}</span>
          <span className="inline-flex h-6 max-w-full items-center gap-1 truncate rounded-[6px] bg-bg-code px-1.5 text-[12px] font-medium text-text-primary">
            <span className="size-1.5 shrink-0 rounded-full bg-warning" />
            <span className="truncate">{row[2]}</span>
          </span>
        </div>
      ))}
    </div>
  );
}

function StepBody({ node }: { node: FlowNode }) {
  const hue = hueColor(node.hue);
  return (
    <div className="flex items-center gap-2.5 p-2.5">
      <span
        className="flex size-9 shrink-0 items-center justify-center rounded-[8px] font-mono text-[13px] font-semibold"
        style={{
          background: mix(hue, 12),
          color: hue,
          boxShadow: `0 0 0 1px ${mix(hue, 20)}`,
        }}
      >
        {(node.title ?? node.id).slice(0, 2).toUpperCase()}
      </span>
      <span className="min-w-0 text-left">
        <span className="block truncate text-[13px] font-semibold leading-tight text-text-primary">
          {node.title ?? node.id}
        </span>
        {node.caption && (
          <span className="mt-0.5 block text-[12px] leading-snug text-text-secondary">
            {node.caption}
          </span>
        )}
      </span>
    </div>
  );
}

/* ── the canvas ── */
const PAD_Y = 24;
const ROW_GAP = 64;
const PILL_OFFSET = 30; // kind pill + gap above a card

export function Flowchart({ doc }: { doc: FlowDoc }) {
  const { nodes, edges } = doc;
  const canvasRef = useRef<HTMLDivElement>(null);
  const nodeRefs = useRef(new Map<string, HTMLElement>());
  const [width, setWidth] = useState(0);
  const [heights, setHeights] = useState<Record<string, number>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [offsets, setOffsets] = useState<Record<string, { dx: number; dy: number }>>({});
  const drag = useRef<{ id: string; startX: number; startY: number; baseDx: number; baseDy: number; moved: boolean } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const measure = () => {
      setWidth(canvas.clientWidth);
      setHeights((prev) => {
        const next = { ...prev };
        let changed = false;
        nodeRefs.current.forEach((el, id) => {
          const h = el.offsetHeight;
          if (h && Math.abs(h - (next[id] ?? 0)) > 0.5) {
            next[id] = h;
            changed = true;
          }
        });
        return changed ? next : prev;
      });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(canvas);
    nodeRefs.current.forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  const rows = [...new Set(nodes.map((n) => n.row))].sort((a, b) => a - b);
  const rowH = rows.map((r) => Math.max(...nodes.filter((n) => n.row === r).map((n) => heights[n.id] ?? 90)));
  const rowY: number[] = [];
  rows.forEach((_, i) => {
    rowY[i] = i === 0 ? PAD_Y : rowY[i - 1] + rowH[i - 1] + ROW_GAP;
  });
  const canvasH = rowY[rows.length - 1] + rowH[rows.length - 1] + PAD_Y;

  const cw = width || 480;
  const place = (n: FlowNode) => {
    const w = Math.min(n.w ?? 300, cw * 0.92);
    const off = offsets[n.id];
    return { w, cx: n.x * cw + (off?.dx ?? 0), top: rowY[rows.indexOf(n.row)] + (off?.dy ?? 0) };
  };

  const anchors = (n: FlowNode) => {
    const { cx, top } = place(n);
    return {
      top: { x: cx, y: top + (n.kind ? PILL_OFFSET : 0) },
      bottom: { x: cx, y: top + (heights[n.id] ?? 90) },
    };
  };

  const bezier = (edge: FlowEdge) => {
    const fromNode = nodes.find((n) => n.id === edge.from);
    const toNode = nodes.find((n) => n.id === edge.to);
    if (!fromNode || !toNode) return "";
    const from = anchors(fromNode).bottom;
    const to = anchors(toNode).top;
    const k = Math.min(Math.max(Math.abs(to.y - from.y) * 0.55, 24), 84);
    return `M ${from.x} ${from.y} C ${from.x} ${from.y + k}, ${to.x} ${to.y - k}, ${to.x} ${to.y}`;
  };

  const onPointerDown = (node: FlowNode) => (event: React.PointerEvent<HTMLDivElement>) => {
    drag.current = {
      id: node.id,
      startX: event.clientX,
      startY: event.clientY,
      baseDx: offsets[node.id]?.dx ?? 0,
      baseDy: offsets[node.id]?.dy ?? 0,
      moved: false,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onPointerMove = (node: FlowNode) => (event: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.id !== node.id) return;
    const dx = d.baseDx + event.clientX - d.startX;
    const dy = d.baseDy + event.clientY - d.startY;
    if (!d.moved && Math.hypot(dx - d.baseDx, dy - d.baseDy) < 3) return;
    d.moved = true;
    const { w } = place(node);
    const h = heights[node.id] ?? 90;
    const baseCx = node.x * cw;
    const baseTop = rowY[rows.indexOf(node.row)];
    const cx = Math.min(Math.max(baseCx + dx, w / 2 + 8), cw - w / 2 - 8);
    const top = Math.min(Math.max(baseTop + dy, 8), canvasH - h - 8);
    setOffsets((current) => ({ ...current, [node.id]: { dx: cx - baseCx, dy: top - baseTop } }));
  };

  const onPointerUp = (node: FlowNode) => () => {
    const d = drag.current;
    if (d?.id === node.id) {
      // a real drag shouldn't also toggle selection
      if (d.moved) setTimeout(() => (drag.current = null), 0);
      else drag.current = null;
    }
  };

  const isLit = (edge: FlowEdge) => selected === edge.from || selected === edge.to;

  return (
    <div
      ref={canvasRef}
      className="relative my-2 w-full select-none overflow-hidden rounded-xl border border-border bg-bg-code"
      style={{
        height: canvasH,
        backgroundImage: "radial-gradient(var(--border-strong) 1px, transparent 1.25px)",
        backgroundSize: "22px 22px",
        backgroundPosition: "center",
      }}
    >
      <svg width={cw} height={canvasH} className="pointer-events-none absolute inset-0">
        {edges.map((edge) => (
          <path
            key={`${edge.from}-${edge.to}`}
            d={bezier(edge)}
            fill="none"
            stroke={isLit(edge) ? "var(--accent-cir)" : "var(--border-strong)"}
            strokeWidth="1.25"
            className="transition-[stroke] duration-150"
          />
        ))}
      </svg>

      {nodes.map((node) => {
        const { w, cx, top } = place(node);
        const active = selected === node.id;
        const hue = hueColor(node.hue);
        return (
          <div
            key={node.id}
            ref={(el) => {
              if (el) nodeRefs.current.set(node.id, el);
              else nodeRefs.current.delete(node.id);
            }}
            onPointerDown={onPointerDown(node)}
            onPointerMove={onPointerMove(node)}
            onPointerUp={onPointerUp(node)}
            className="absolute flex -translate-x-1/2 touch-none flex-col items-start gap-1.5"
            style={{ left: cx, top, width: w, zIndex: drag.current?.id === node.id ? 2 : 1 }}
          >
            {node.kind && (
              <span
                className="inline-flex h-6 items-center rounded-[6px] px-2 text-[11.5px] font-medium"
                style={{
                  background: mix(hue, 14, "var(--bg-code)"),
                  color: mix(hue, 80, "var(--text-primary)"),
                }}
              >
                {node.kind}
              </span>
            )}
            {node.condition ? (
              <div className="w-full rounded-[18px] border border-border bg-bg-popover">
                <ConditionBody rows={node.condition} />
              </div>
            ) : (
              <button
                type="button"
                onClick={() => setSelected(active ? null : node.id)}
                aria-pressed={active}
                className={`w-full cursor-pointer rounded-[18px] border bg-bg-popover text-left outline-none transition-shadow duration-150 focus-visible:shadow-[0_0_0_1.5px_var(--accent-cir)] ${
                  active
                    ? "border-accent-cir/60 shadow-[0_0_0_1.5px_var(--accent-cir),0_2px_10px_rgba(0,0,0,0.25)]"
                    : "border-border hover:border-border-strong"
                }`}
              >
                <StepBody node={node} />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
