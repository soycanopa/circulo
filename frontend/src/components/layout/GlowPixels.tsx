import { useEffect, useRef } from "react";

/** Animated pixel texture for the bottom glow (owner call): pixels pop in
 *  and out in random order, dissolving toward the top of the strip so the
 *  texture only reads inside the glow. Reduced motion draws the static
 *  texture. Canvas keeps per-pixel randomness cheap (~2k dots at ~20fps). */
export function GlowPixels() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const PITCH = 9;
    const SIZE = 2;
    const BASE = 0.35;
    let dots: { x: number; y: number; phase: number; cycle: number }[] = [];
    let w = 0;
    let h = 0;
    let raf = 0;
    let last = 0;

    const build = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = canvas.clientWidth;
      h = canvas.clientHeight;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      dots = [];
      for (let y = PITCH / 2; y < h; y += PITCH)
        for (let x = PITCH / 2; x < w; x += PITCH)
          dots.push({ x, y, phase: Math.random(), cycle: 3800 + Math.random() * 5200 });
    };

    const paint = (twinkle: boolean, now: number) => {
      ctx.clearRect(0, 0, w, h);
      for (const d of dots) {
        // canvas y grows downward: full presence at the glow (bottom),
        // dissolving to nothing at the strip's top edge
        const fade = d.y / h;
        const a = BASE * fade * (twinkle ? Math.sin(((now / d.cycle + d.phase) % 1) * Math.PI) ** 2 : 1);
        if (a < 0.02) continue;
        ctx.fillStyle = `rgba(115, 120, 242, ${a})`;
        ctx.fillRect(d.x - SIZE / 2, d.y - SIZE / 2, SIZE, SIZE);
      }
    };

    const loop = (now: number) => {
      raf = requestAnimationFrame(loop);
      if (now - last < 50) return; // ~20fps is plenty for a slow twinkle
      last = now;
      paint(true, now);
    };

    build();
    if (reduce) paint(false, 0);
    else raf = requestAnimationFrame(loop);

    const ro = new ResizeObserver(build);
    ro.observe(canvas);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, []);

  return (
    <canvas
      ref={ref}
      aria-hidden
      className="pointer-events-none absolute inset-x-0 bottom-0 h-[150px] w-full"
    />
  );
}
