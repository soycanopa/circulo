/**
 * Provider mark for the model selector. OpenCode ships the owner's SVG
 * (frontend/src/assets/opencode.svg, inlined); unknown providers get a
 * letter-mark until real icons are added.
 */

export function ProviderIcon({ provider, size = 16 }: { provider: string; size?: number }) {
  if (provider === "opencode") {
    return (
      <svg
        width={size}
        height={Math.round((size * 300) / 240)}
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
