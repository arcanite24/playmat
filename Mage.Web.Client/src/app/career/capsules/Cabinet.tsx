import { useMemo, type CSSProperties, type PointerEvent as ReactPointerEvent, type Ref } from 'react';
import styles from './Capsules.module.css';

/** The most capsules the dome draws; a fuller machine just looks full. */
const MAX_SHOWN = 44;

/** A seeded shuffle, so a machine's capsules sit the same way on every visit. */
function seeded(seed: string) {
  let hash = 2166136261;
  for (let i = 0; i < seed.length; i++) hash = Math.imul(hash ^ seed.charCodeAt(i), 16777619);
  return () => {
    hash = Math.imul(hash ^ (hash >>> 15), 2246822507);
    hash = Math.imul(hash ^ (hash >>> 13), 3266489909);
    return ((hash ^= hash >>> 16) >>> 0) / 4294967296;
  };
}

/** Where the capsules lie in the dome: a heap from the bottom up, each a little off the grid. */
function heap(colors: readonly string[], seed: string) {
  const random = seeded(seed);
  const shown = colors.slice(0, MAX_SHOWN);
  const placed: { x: number; y: number; r: number; color: string; turn: number }[] = [];
  const radius = 12.5;
  let row = 0;
  let index = 0;
  while (index < shown.length && row < 9) {
    const y = 176 - row * 19.5;
    // the dome narrows toward the top
    const half = Math.sqrt(Math.max(0, 88 * 88 - (y - 104) ** 2)) - radius - 2;
    const count = Math.max(1, Math.floor((half * 2) / (radius * 2.05)));
    const offset = row % 2 ? radius : 0;
    for (let k = 0; k < count && index < shown.length; k++) {
      const x = 120 - half + offset + k * radius * 2.08 + (random() - 0.5) * 4;
      if (x > 120 + half) continue;
      placed.push({ x, y: y + (random() - 0.5) * 4, r: radius, color: shown[index++], turn: random() * 360 });
    }
    row++;
  }
  return placed;
}

export interface CabinetProps {
  /** the machine's id, which seeds where its capsules lie */
  seed: string;
  body: string;
  trim: string;
  /** one colour a capsule still inside */
  capsules: readonly string[];
  /** the crank's turn, in degrees */
  crank?: number;
  /** the token sliding into the slot */
  inserting?: boolean;
  /** the machine shakes as the capsule drops */
  rattling?: boolean;
  className?: string;
  style?: CSSProperties;
  /** a drag on the crank turns it (the pull sequence) */
  onCrankDown?(event: ReactPointerEvent<SVGGElement>): void;
  crankRef?: Ref<SVGGElement>;
  label: string;
}

/**
 * A capsule machine, drawn: a glass dome holding the capsules left (each in its set's colour, so the sets still to
 * finish show), the cabinet in the machine's colours, a coin slot, the crank and the chute.
 */
export function Cabinet({ seed, body, trim, capsules, crank = 0, inserting, rattling, className, style, onCrankDown, crankRef, label }: CabinetProps) {
  const placed = useMemo(() => heap(capsules, seed), [capsules, seed]);
  const id = `cab-${seed}`;
  return (
    <svg
      viewBox="0 0 240 360"
      className={[styles.cabinet, rattling ? styles.rattle : '', className ?? ''].join(' ')}
      style={{ ...style, '--cab-body': body, '--cab-trim': trim } as CSSProperties}
      role="img"
      aria-label={label}
    >
      <defs>
        <radialGradient id={`${id}-glass`} cx="38%" cy="30%" r="75%">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.32" />
          <stop offset="0.45" stopColor="#ffffff" stopOpacity="0.06" />
          <stop offset="1" stopColor="#0a1410" stopOpacity="0.35" />
        </radialGradient>
        <linearGradient id={`${id}-body`} x1="0" x2="1">
          <stop offset="0" stopColor="#000" stopOpacity="0.28" />
          <stop offset="0.35" stopColor="#fff" stopOpacity="0.1" />
          <stop offset="1" stopColor="#000" stopOpacity="0.32" />
        </linearGradient>
        <clipPath id={`${id}-dome`}><circle cx="120" cy="104" r="88" /></clipPath>
      </defs>

      {/* the cap on top of the dome */}
      <rect x="96" y="6" width="48" height="16" rx="6" fill="var(--cab-trim)" />
      <rect x="104" y="1" width="32" height="8" rx="4" fill="var(--cab-body)" />

      {/* the dome and its capsules */}
      <circle cx="120" cy="104" r="90" fill="#0f1a16" />
      <g clipPath={`url(#${id}-dome)`}>
        {placed.map((capsule, index) => (
          <g key={index} transform={`translate(${capsule.x} ${capsule.y}) rotate(${capsule.turn})`}>
            <circle r={capsule.r} fill="#f1ede2" />
            <path d={`M ${-capsule.r} 0 A ${capsule.r} ${capsule.r} 0 0 1 ${capsule.r} 0 Z`} fill={capsule.color} />
            <line x1={-capsule.r} x2={capsule.r} y1="0" y2="0" stroke="#000" strokeOpacity="0.25" strokeWidth="1.2" />
            <circle cx={-capsule.r * 0.35} cy={-capsule.r * 0.45} r={capsule.r * 0.22} fill="#fff" fillOpacity="0.5" />
          </g>
        ))}
      </g>
      <circle cx="120" cy="104" r="88" fill={`url(#${id}-glass)`} />
      <path d="M 62 52 A 72 72 0 0 1 112 28" stroke="#fff" strokeOpacity="0.45" strokeWidth="5" strokeLinecap="round" fill="none" />
      <circle cx="120" cy="104" r="89" fill="none" stroke="var(--cab-trim)" strokeWidth="3" />

      {/* the collar and the cabinet */}
      <rect x="44" y="184" width="152" height="22" rx="8" fill="var(--cab-trim)" />
      <path d="M 36 206 H 204 L 214 344 Q 214 354 204 354 H 36 Q 26 354 26 344 Z" fill="var(--cab-body)" />
      <path d="M 36 206 H 204 L 214 344 Q 214 354 204 354 H 36 Q 26 354 26 344 Z" fill={`url(#${id}-body)`} />
      <rect x="34" y="214" width="172" height="5" fill="var(--cab-trim)" opacity="0.8" />

      {/* the coin slot and its price plate */}
      <rect x="56" y="238" width="34" height="46" rx="6" fill="#000" fillOpacity="0.25" stroke="var(--cab-trim)" strokeWidth="2" />
      <rect x="70" y="246" width="6" height="22" rx="3" fill="#0b0f0d" />
      <circle cx="73" cy="276" r="3.5" fill="var(--cab-trim)" />
      <g className={inserting ? styles.tokenIn : styles.tokenOut} aria-hidden="true">
        <circle cx="73" cy="226" r="9" fill="#e7c25e" stroke="#8a6a22" strokeWidth="2" />
        <circle cx="73" cy="226" r="4" fill="none" stroke="#8a6a22" strokeWidth="1.5" />
      </g>

      {/* the crank */}
      <g ref={crankRef} className={onCrankDown ? styles.crankLive : undefined} onPointerDown={onCrankDown}>
        <circle cx="140" cy="262" r="30" fill="var(--cab-trim)" />
        <circle cx="140" cy="262" r="24" fill="#000" fillOpacity="0.18" />
        <g transform={`rotate(${crank} 140 262)`}>
          <rect x="134" y="234" width="12" height="56" rx="6" fill="#f3efe4" />
          <circle cx="140" cy="238" r="9" fill="var(--cab-body)" stroke="#f3efe4" strokeWidth="3" />
        </g>
        <circle cx="140" cy="262" r="6" fill="#f3efe4" />
        {/* a bigger target for a finger */}
        <circle cx="140" cy="262" r="40" fill="transparent" />
      </g>

      {/* the chute */}
      <rect x="84" y="304" width="72" height="38" rx="10" fill="#070b09" stroke="var(--cab-trim)" strokeWidth="3" />
      <path d="M 90 314 H 150" stroke="#fff" strokeOpacity="0.12" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}
