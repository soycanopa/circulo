/**
 * ContextGauge — circular context-window meter on the composer's right side
 * (Waku pattern): an SVG ring fills counter-clockwise as the session's
 * context window fills; the percentage sits centered.
 *
 * Colors shift with pressure (tertiary → warning → danger), matching the
 * app's semantic palette. Without a known window (backend didn't expose one)
 * the ring renders empty and the label falls back to a bare token count.
 */

import { cn } from "@/lib/utils";

const RADIUS = 9;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

function pressure(percent: number): string {
  if (percent >= 95) return "text-danger";
  if (percent >= 80) return "text-warning";
  return "text-text-tertiary";
}

export function ContextGauge({
  used,
  window: win,
}: {
  used: number;
  window?: number;
}) {
  const percent = win && win > 0 ? Math.min(100, (used / win) * 100) : 0;
  const color = pressure(percent);
  const label =
    win && win > 0
      ? `${Math.round(percent)}%`
      : used > 0
        ? `${formatTokens(used)}`
        : "";

  return (
    <div
      className="flex shrink-0 items-center gap-1.5"
      title={
        win && win > 0
          ? `Context ${Math.round(percent)}% — ${formatTokens(used)} of ${formatTokens(win)} tokens`
          : `Context ${formatTokens(used)} tokens`
      }
      role="status"
      aria-label={`Context ${Math.round(percent)} percent full`}
    >
      <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden className="-rotate-90">
        {/* track */}
        <circle
          cx="12"
          cy="12"
          r={RADIUS}
          fill="none"
          strokeWidth="2"
          className="stroke-border-strong"
        />
        {/* fill */}
        <circle
          cx="12"
          cy="12"
          r={RADIUS}
          fill="none"
          strokeWidth="2"
          strokeLinecap="round"
          className={cn(color, "transition-[stroke-dashoffset] duration-500")}
          strokeDasharray={CIRCUMFERENCE}
          strokeDashoffset={CIRCUMFERENCE * (1 - percent / 100)}
        />
      </svg>
      {label && (
        <span className={cn("text-[11px] font-medium tabular-nums", color)}>{label}</span>
      )}
    </div>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 100) / 10}k`;
  return `${n}`;
}
