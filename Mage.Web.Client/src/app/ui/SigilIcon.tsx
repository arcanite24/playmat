import { Flame, Leaf, Moon, Skull, Snowflake, Sun, Sword, Zap } from 'lucide-react';

/** The legacy avatar ids keep their subjects, drawn with the same strokes as the UI icons. */
const CREATURES: Record<string, string> = {
  dragon: 'M4 20l4-5-4-4 6 1 2-8 3 4 5 1-3 4-4 1 1 6z M15 9h.01 M8 15l5-1',
  owl: 'M5 4l3 3h8l3-3v11a7 7 0 0 1-14 0z M10 11a2 2 0 1 1-4 0 2 2 0 0 1 4 0 M18 11a2 2 0 1 1-4 0 2 2 0 0 1 4 0 M10 16l2 2 2-2',
  wolf: 'M5 3l5 6h4l5-6-1 13-6 5-6-5z M8 12l2 1 M16 12l-2 1 M10 17l2 1 2-1',
  fox: 'M3 3l9 5 9-5-3 12-6 6-6-6z M4 7l5 7 3 7 3-7 5-7 M8 12h.01 M16 12h.01',
  serpent: 'M16 5c0-3-8-3-8 2 0 4 10 2 10 8 0 7-14 7-14 1 M16 5l4 1-4 3h-3 M13 5h.01 M20 6l2-1 M20 6l2 1',
  lion: 'M8 3h8l5 5v8l-5 5H8l-5-5V8z M8 8h8l1 6-5 4-5-4z M9 11h.01 M15 11h.01 M10 14h4l-2 2z',
  bear: 'M6 8a3 3 0 1 1 3-4 M15 4a3 3 0 1 1 3 4 M5 10c0-9 14-9 14 0v5c0 7-14 7-14 0z M9 10h.01 M15 10h.01 M9 16a3 3 0 1 1 6 0 M11 14h2',
  eagle: 'M3 8l6 2 3-6 3 6 6-2-3 8-4-2v6h-4v-6l-4 2z M12 7l3 1',
  kraken: 'M7 12V8a5 5 0 0 1 10 0v4 M7 12c-5 0-5 7-3 7s3-5 5-5 M17 12c5 0 5 7 3 7s-3-5-5-5 M10 14c-3 8 2 9 2 4 0 5 5 4 2-4 M10 9h.01 M14 9h.01',
  scorpion: 'M10 12c-6 0-7-7-4-9l2 3-2 2 M10 12l5-2 3 3-3 4-5-1z M18 13l3-3v-3 M21 10l-3-2 M9 13l-4 2 M10 16l-3 4 M14 17l1 4 M16 16l4 3',
  spider: 'M10 6a2 2 0 1 1 4 0 M12 8c-6 0-6 10 0 10s6-10 0-10 M8 10L4 7V3 M16 10l4-3V3 M8 12l-6-1 M16 12l6-1 M8 14l-5 3v4 M16 14l5 3v4 M9 17l-2 4 M15 17l2 4',
  turtle: 'M6 10c0-7 12-7 12 0v6c0 5-12 5-12 0z M10 5V3h4v2 M6 9L3 7 M18 9l3-2 M6 17l-3 3 M18 17l3 3 M12 8l4 4-4 5-4-5z',
  unicorn: 'M10 8l3-6 1 6 4 3 1 5-4 1-1 4H5l2-7 3-6z M14 8l3-3v5 M14 12h.01 M10 9l-5 4-2 6',
};

export function SigilIcon({ sigil, size = 24 }: { sigil: string; size?: number | string }) {
  switch (sigil) {
    case 'flame': return <Flame size={size} aria-hidden="true" />;
    case 'frost': return <Snowflake size={size} aria-hidden="true" />;
    case 'lightning': return <Zap size={size} aria-hidden="true" />;
    case 'moon': return <Moon size={size} aria-hidden="true" />;
    case 'sun': return <Sun size={size} aria-hidden="true" />;
    case 'leaf': return <Leaf size={size} aria-hidden="true" />;
    case 'skull': return <Skull size={size} aria-hidden="true" />;
    case 'blade': return <Sword size={size} aria-hidden="true" />;
  }
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={CREATURES[sigil]} />
    </svg>
  );
}
