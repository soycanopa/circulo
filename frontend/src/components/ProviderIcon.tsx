/**
 * Provider mark for the model selector. OpenCode and omp ship their own SVGs
 * (inlined); unknown providers get a letter-mark until real icons are added.
 * `size` is the rendered HEIGHT for every provider — icons keep their own
 * aspect ratio, so marks with different viewBoxes align on the baseline
 * (owner call after the omp tab rendered shorter than OpenCode's).
 */

export function ProviderIcon({ provider, size = 16 }: { provider: string; size?: number }) {
  if (provider === "opencode") {
    return (
      <svg
        width={Math.round((size * 240) / 300)}
        height={size}
        viewBox="0 0 240 300"
        fill="none"
        aria-hidden
        className="shrink-0"
      >
        <path d="M180 240H60V120H180V240Z" fill="#4B4646" />
        <path d="M180 60H60V240H180V60ZM240 300H0V0H240V300Z" fill="#F1ECEC" />
      </svg>
    );
  }
  if (provider === "omp") {
    // omp's mark: π with plugin connector (can1357/oh-my-pi assets/icon.svg).
    return (
      <svg
        width={Math.round((size * 120) / 90)}
        height={size}
        viewBox="0 0 120 90"
        fill="none"
        aria-hidden
        className="shrink-0"
      >
        <rect x="10" y="8" width="100" height="12" rx="2" fill="#fafafa" />
        <rect x="25" y="20" width="12" height="62" rx="2" fill="#fafafa" />
        <rect x="75" y="20" width="12" height="45" rx="2" fill="#fafafa" />
        <rect x="71" y="55" width="20" height="16" rx="3" fill="#f97316" />
        <rect x="76" y="59" width="3" height="8" rx="1" fill="#0d0d0d" />
        <rect x="82" y="59" width="3" height="8" rx="1" fill="#0d0d0d" />
      </svg>
    );
  }
  return (
    <span
      aria-hidden
      className="flex shrink-0 items-center justify-center rounded-[4px] bg-bg-hover text-[9px] font-semibold uppercase text-text-secondary"
      style={{ width: size, height: size }}
    >
      {provider.charAt(0)}
    </span>
  );
}
