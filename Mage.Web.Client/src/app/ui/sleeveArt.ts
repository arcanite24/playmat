/**
 * Career sleeves: matte dyes like the free ones, so they read the same around card art, each with a painted card back
 * from the art pipeline (art/specs/sleeve.json). Decks store a sleeve as its colour, so the back is found by colour.
 */
export const CAREER_SLEEVES: Record<string, string> = {
  copper: '#6e3f24',
  silver: '#5b6168',
  gold: '#7a5f1c',
  obsidian: '#121014',
  foil: '#4a3d78',
  // a school's graduates
  'school-pauper': '#3d5a36',
  'school-commander': '#4b2a5e',
  'school-limited': '#66583c',
  'school-pioneer': '#2c4a66',
  'school-standard': '#22605b',
  'school-modern': '#6a2a2c',
  'school-legacy': '#25305e',
  'school-vintage': '#4f4a1e',
  'school-brawl': '#7b3550',
};

/** The Career sleeve a color belongs to, if it's one. */
export function careerSleeveOf(color: string | undefined): string | null {
  if (!color) return null;
  const lower = color.toLowerCase();
  return Object.keys(CAREER_SLEEVES).find((id) => CAREER_SLEEVES[id] === lower) ?? null;
}

/**
 * The supporter sleeve: a thank-you for supporting the server (support/supportModel.ts). A deep verdigris like the
 * other dyes, with a back drawn here rather than painted: a brass-foiled frame and a heart, in Playmat's own style.
 */
export const SUPPORTER_SLEEVE = '#1f5446';

const FOIL = 'url(#f)';
const SUPPORTER_BACK_SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88">'
  + '<defs><linearGradient id="f" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#b98d45"/>'
  + '<stop offset=".45" stop-color="#f1dfa4"/><stop offset=".55" stop-color="#e2c27a"/><stop offset="1" stop-color="#8f6a2c"/></linearGradient>'
  + '<pattern id="w" width="3" height="3" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0 0v3" stroke="#2a6a59" stroke-width=".8"/></pattern></defs>'
  + '<rect width="63" height="88" fill="#163d34"/><rect width="63" height="88" fill="url(#w)" opacity=".55"/>'
  + `<rect x="3.5" y="3.5" width="56" height="81" rx="2.5" fill="none" stroke="${FOIL}" stroke-width="1.3"/>`
  + `<rect x="6.5" y="6.5" width="50" height="75" rx="1.5" fill="none" stroke="${FOIL}" stroke-width=".5"/>`
  + `<circle cx="31.5" cy="44" r="14" fill="#163d34" stroke="${FOIL}" stroke-width="1.3"/><circle cx="31.5" cy="44" r="11.5" fill="none" stroke="${FOIL}" stroke-width=".4"/>`
  + `<path d="M31.5 51.5c-6.2-4.3-8.7-7.6-8.7-10.8a4.4 4.4 0 0 1 8.7-1.1 4.4 4.4 0 0 1 8.7 1.1c0 3.2-2.5 6.5-8.7 10.8z" fill="${FOIL}"/></svg>`;

/** The supporter sleeve's back, as a picture the card backs can show. */
export const SUPPORTER_BACK = `data:image/svg+xml,${encodeURIComponent(SUPPORTER_BACK_SVG)}`;

/** The back of a sleeve colour: a Career sleeve's painted one, or the supporter sleeve's drawn one. */
export function sleeveBackOf(color: string | undefined): string | null {
  if (color?.toLowerCase() === SUPPORTER_SLEEVE) return SUPPORTER_BACK;
  const id = careerSleeveOf(color);
  return id ? `/career-art/sleeve/${id}.webp` : null;
}
