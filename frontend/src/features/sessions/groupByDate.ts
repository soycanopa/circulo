/** Pure date grouping for the session list (docs/ui.md, Sidebar spec). */

interface Dated {
  timeUpdated: number;
  timeCreated: number;
}

export interface SessionGroup<T extends Dated> {
  label: string;
  items: T[];
}

const DAY = 86_400_000;

/** Ordered buckets: Today, Yesterday, This week, Older. */
export function groupByDate<T extends Dated>(items: T[], now = Date.now()): SessionGroup<T>[] {
  const startOfToday = new Date(now).setHours(0, 0, 0, 0);
  const bounds: [string, number][] = [
    ["Today", startOfToday],
    ["Yesterday", startOfToday - DAY],
    ["This week", startOfToday - 6 * DAY],
    ["Older", -Infinity],
  ];
  const out: SessionGroup<T>[] = bounds.map(([label]) => ({ label, items: [] }));
  for (const item of items) {
    const ts = item.timeUpdated || item.timeCreated;
    const bucket =
      out.find((_, i) => i < bounds.length - 1 && ts >= bounds[i][1]) ?? out[out.length - 1];
    bucket.items.push(item);
  }
  return out.filter((g) => g.items.length > 0);
}
