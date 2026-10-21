/**
 * A release's notes for players, as /release-notes.json holds them: the GitHub release body (generated notes, sorted
 * into the categories of .github/release.yml) reduced to headed lists of one-line changes. Written at build time by
 * scripts/release-notes-plugin.ts, read by the "What's new" dialog (src/app/whatsNew.ts).
 */

/** The categories .github/release.yml sorts pull requests into; the dialog translates their headings. */
export type ReleaseSectionId = 'features' | 'fixes' | 'ops' | 'other';

export interface ReleaseSection {
  /** a known category, or absent for a heading the release author wrote */
  id?: ReleaseSectionId;
  title: string;
  items: string[];
}

export interface ReleaseNotes {
  version: string;
  sections: ReleaseSection[];
  /** the release page, for everything left out here (authors, links, the full changelog) */
  url?: string;
}

const SECTION_IDS: Record<string, ReleaseSectionId> = {
  features: 'features',
  'new features': 'features',
  fixes: 'fixes',
  'bug fixes': 'fixes',
  ops: 'ops',
  'server and hosting': 'ops',
  other: 'other',
  'other changes': 'other',
};

/** Where the desktop installers' download notes start (.github/workflows/desktop.yml adds them below the changes). */
export const DESKTOP_NOTES_MARKER = '<!-- desktop-downloads -->';
/** Sections that speak to contributors, not players. */
const SKIPPED = /^(new contributors|full changelog)/i;
const MAX_ITEMS = 12;
const MAX_ITEM_LENGTH = 200;

/** One change, as players should read it: GitHub's "by @someone in https://…/pull/12" and Markdown dropped. */
export function cleanItem(line: string): string {
  return line
    .replace(/\s+by\s+@[\w-]+(\[bot\])?\s+in\s+https?:\/\/\S+$/i, '')
    .replace(/\s+in\s+https?:\/\/\S+\/pull\/\d+$/i, '')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/[*_`]+/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, MAX_ITEM_LENGTH);
}

/**
 * Headings (## or ###) open sections and list items (*, -) fill them; anything else is left out. Items before any
 * heading go in an untitled "other" section. Empty sections and contributor sections are dropped, and so is everything
 * from the desktop download notes on.
 */
export function parseReleaseNotes(markdown: string, version: string, url?: string): ReleaseNotes {
  const sections: ReleaseSection[] = [];
  let current: ReleaseSection | null = null;
  let skipping = false;
  for (const raw of markdown.replace(/\r/g, '').split('\n')) {
    const line = raw.trim();
    if (line.includes(DESKTOP_NOTES_MARKER)) break;
    const heading = /^#{2,3}\s+(.+)$/.exec(line);
    if (heading) {
      const title = cleanItem(heading[1]).replace(/^[^\p{L}\p{N}]+/u, '').trim();
      skipping = SKIPPED.test(title) || /^what'?s changed$/i.test(title);
      current = skipping ? null : { id: SECTION_IDS[title.toLowerCase()], title, items: [] };
      if (current) sections.push(current);
      continue;
    }
    const item = /^[*-]\s+(.+)$/.exec(line);
    if (!item || skipping) continue;
    if (!current) {
      current = { id: 'other', title: 'Other', items: [] };
      sections.push(current);
    }
    const text = cleanItem(item[1]);
    if (text && current.items.length < MAX_ITEMS) current.items.push(text);
  }
  for (const section of sections) if (!section.id) delete section.id;
  return { version, sections: sections.filter((section) => section.items.length > 0), ...(url ? { url } : {}) };
}

/** What /release-notes.json holds, checked: a dialog never shows something it can't read. */
export function readReleaseNotes(value: unknown): ReleaseNotes | null {
  if (!value || typeof value !== 'object') return null;
  const notes = value as Partial<ReleaseNotes>;
  if (typeof notes.version !== 'string' || !Array.isArray(notes.sections)) return null;
  const sections = notes.sections.filter((section): section is ReleaseSection =>
    !!section && typeof section.title === 'string' && Array.isArray(section.items) && section.items.every((item) => typeof item === 'string'));
  if (sections.length === 0) return null;
  return { version: notes.version, sections, ...(typeof notes.url === 'string' && /^https:\/\//.test(notes.url) ? { url: notes.url } : {}) };
}
