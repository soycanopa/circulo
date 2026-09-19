import { useEffect, useRef, useState } from "react";

/* Word-by-word reveal for streaming text (owner's StreamingText design):
 * the display string advances toward the target at ~55 ms per word. The
 * step grows with the backlog so a fast token burst catches up instead of
 * lagging seconds behind. Reduced motion and finished parts snap to the
 * full text instantly. */

const TICK_MS = 55;

export function useSmoothText(target: string, active: boolean, partKey: string): string {
  const [len, setLen] = useState(() => (active ? 0 : target.length));
  const reduceMotion = useRef(false);
  useEffect(() => {
    reduceMotion.current = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }, []);

  // New part (key change): restart the reveal; finished parts render whole.
  useEffect(() => {
    setLen(active ? 0 : target.length);
  }, [partKey]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!active) {
      setLen(target.length);
      return;
    }
    if (reduceMotion.current) {
      setLen(target.length);
      return;
    }
    const t = setInterval(() => {
      setLen((prev) => {
        if (prev >= target.length) return prev;
        const behind = target.length - prev;
        // ~6 chars per word: grow the step when the backlog builds up.
        const words = Math.min(10, Math.max(1, Math.ceil(behind / 6 / 10)));
        let next = prev;
        for (let i = 0; i < words; i++) {
          const space = target.indexOf(" ", next + 1);
          next = space === -1 ? target.length : space;
          if (next >= target.length) break;
        }
        return Math.min(next, target.length);
      });
    }, TICK_MS);
    return () => clearInterval(t);
  }, [target, active]);

  return target.slice(0, len);
}
