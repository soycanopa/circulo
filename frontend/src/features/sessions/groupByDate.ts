/** Pure date grouping for the flat session list (Circulo Paper: Today / Earlier). */

interface Dated {
  timeUpdated: number;
  timeCreated: number;
}

export interface SessionGroup<T> {
  label: string;
  key: "today" | "earlier";
  items: T[];
}


/** Two buckets per the Circulo design: Today, then everything earlier. */
export function groupByDate<T extends Dated>(items: T[], now = Date.now()): SessionGroup<T>[] {
  const startOfToday = new Date(now).setHours(0, 0, 0, 0);
  const groups: SessionGroup<T>[] = [
    { label: "Today", key: "today", items: [] },
    { label: "Earlier", key: "earlier", items: [] },
  ];
  for (const item of items) {
    const ts = item.timeUpdated || item.timeCreated;
    groups[ts >= startOfToday ? 0 : 1].items.push(item);
  }
  return groups.filter((g) => g.items.length > 0);
}
