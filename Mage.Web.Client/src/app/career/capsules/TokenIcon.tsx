/** A capsule token: a brass coin stamped with a capsule. */
export function TokenIcon({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" style={{ flex: 'none', verticalAlign: 'middle' }}>
      <circle cx="12" cy="12" r="11" fill="#e7c25e" stroke="#8a6a22" strokeWidth="1.5" />
      <circle cx="12" cy="12" r="8" fill="none" stroke="#8a6a22" strokeOpacity="0.5" strokeWidth="1" />
      <path d="M 7 12 A 5 5 0 0 1 17 12 Z" fill="#b8402f" />
      <path d="M 7 12 A 5 5 0 0 0 17 12 Z" fill="#f6f0dc" />
      <line x1="7" x2="17" y1="12" y2="12" stroke="#5c4413" strokeWidth="1" />
    </svg>
  );
}
