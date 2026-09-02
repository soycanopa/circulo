/**
 * AppBar: the extensible top bar of the main column (docs/ui.md §2).
 * It owns only the chrome — height, border, slot layout. Features compose
 * their content into the left/center/right slots; new controls plug in
 * without touching the layout.
 *
 * It sits BELOW the 44px native titlebar strip (which must stay empty: the
 * invisible macOS titlebar owns that region for dragging + traffic lights).
 */

import type { ReactNode } from "react";

export function AppBar({
  left,
  center,
  right,
}: {
  left?: ReactNode;
  center?: ReactNode;
  right?: ReactNode;
}) {
  return (
    <div className="flex h-12 shrink-0 items-center gap-3 border-b border-border px-4">
      {left && <div className="flex min-w-0 items-center gap-2">{left}</div>}
      {center && <div className="flex min-w-0 flex-1 justify-center">{center}</div>}
      {!center && <div className="flex-1" />}
      {right && <div className="flex shrink-0 items-center gap-2">{right}</div>}
    </div>
  );
}
