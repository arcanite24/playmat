/**
 * How a player shows up: a sigil (the server keeps it as an avatar id) and a flag (the server keeps the desktop
 * client's flag file name, "de.png"). Avatar ids are bounded by the server (10 to 32); 10 is the default, shown as
 * the player's initial, and 11 is reserved for the server's own staff avatars.
 */
export const DEFAULT_AVATAR_ID = 10;

export const SIGILS: { id: number; key: string; label: string }[] = [
  { id: 12, key: 'dragon', label: 'Dragon' },
  { id: 13, key: 'owl', label: 'Owl' },
  { id: 14, key: 'wolf', label: 'Wolf' },
  { id: 15, key: 'fox', label: 'Fox' },
  { id: 16, key: 'serpent', label: 'Serpent' },
  { id: 17, key: 'lion', label: 'Lion' },
  { id: 18, key: 'bear', label: 'Bear' },
  { id: 19, key: 'eagle', label: 'Eagle' },
  { id: 20, key: 'kraken', label: 'Kraken' },
  { id: 21, key: 'scorpion', label: 'Scorpion' },
  { id: 22, key: 'spider', label: 'Spider' },
  { id: 23, key: 'turtle', label: 'Turtle' },
  { id: 24, key: 'unicorn', label: 'Unicorn' },
  { id: 25, key: 'flame', label: 'Flame' },
  { id: 26, key: 'frost', label: 'Frost' },
  { id: 27, key: 'lightning', label: 'Lightning' },
  { id: 28, key: 'moon', label: 'Moon' },
  { id: 29, key: 'sun', label: 'Sun' },
  { id: 30, key: 'leaf', label: 'Leaf' },
  { id: 31, key: 'skull', label: 'Skull' },
  { id: 32, key: 'blade', label: 'Blade' },
];

const SIGIL_BY_ID = new Map(SIGILS.map((sigil) => [sigil.id, sigil]));

/** The sigil key for an avatar id, or null for the default (and desktop avatars, which have no web artwork). */
export function sigilOf(avatarId: number | null | undefined): string | null {
  return avatarId ? SIGIL_BY_ID.get(avatarId)?.key ?? null : null;
}

const COUNTRIES = 'ad ae af ag ai al am ao aq ar as at au aw ax az ba bb bd be bf bg bh bi bj bm bn bo bq br bs bt bw by bz ca cc cd cf cg ch ci ck cl cm cn co cr cu cv cx cy cz de dj dk dm do dz ec ee eg eh er es et fi fj fk fm fo fr ga gb gd ge gf gh gi gl gm gn gp gq gr gs gt gu gw gy hk hn hr ht hu id ie il in io iq ir is it jm jo jp ke kg kh ki km kn kp kr kw ky kz la lb lc li lk lr ls lt lu lv ly ma mc md me mg mh mk ml mm mn mo mp mq mr ms mt mu mv mw mx my mz na nc ne nf ng ni nl no np nr nu nz om pa pe pf pg ph pk pl pm pn pr ps pt pw py qa re ro rs ru rw sa sb sc sd se sg sh si sk sl sm sn so sr st sv sy sz tc td tf tg th tj tk tl tm tn to tr tt tv tw tz ua ug us uy uz va vc ve vg vi vn vu wf ws ye yt za zm zw'.split(' ');

const SPECIAL: Record<string, { text: string; label: string }> = {
  world: { text: '', label: 'Anywhere' },
  europeanunion: { text: 'EU', label: 'European Union' },
  england: { text: 'ENG', label: 'England' },
  scotland: { text: 'SCO', label: 'Scotland' },
  wales: { text: 'WAL', label: 'Wales' },
  lgbt: { text: 'LGBT', label: 'Pride' },
  trans: { text: 'TRANS', label: 'Trans pride' },
};

/** The flag code from the server's file name ("de.png" -> "de"). */
export function flagCode(flagName: string | null | undefined): string {
  return (flagName ?? '').replace(/\.png$/i, '').toLowerCase() || 'world';
}

/** A compact text badge for special flags and two-letter codes; empty asks the UI for the globe fallback. */
export function flagText(code: string): string {
  return SPECIAL[code]?.text ?? (/^[a-z]{2}$/.test(code) ? code.toUpperCase() : '');
}

let regionNames: Intl.DisplayNames | null | undefined;

export function flagLabel(code: string): string {
  const special = SPECIAL[code];
  if (special) return special.label;
  if (regionNames === undefined) {
    try {
      regionNames = new Intl.DisplayNames(undefined, { type: 'region' });
    } catch {
      regionNames = null;
    }
  }
  try {
    return regionNames?.of(code.toUpperCase()) ?? code.toUpperCase();
  } catch {
    return code.toUpperCase();
  }
}

/** Every flag a player can pick, the specials first, then countries by name. */
export function flagChoices(): { code: string; label: string }[] {
  const specials = Object.keys(SPECIAL).map((code) => ({ code, label: flagLabel(code) }));
  const countries = COUNTRIES.map((code) => ({ code, label: flagLabel(code) }))
    .sort((a, b) => a.label.localeCompare(b.label));
  return [...specials, ...countries];
}
