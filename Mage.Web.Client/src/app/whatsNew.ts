import { readReleaseNotes, type ReleaseNotes } from '../core/util/releaseNotes';
import { APP_VERSION } from './brand';
import { readJson, writeJson } from './stores/persist';

const SEEN_KEY = 'playmat.seenVersion';

export interface WhatsNewDeps {
  version: string;
  load(): Promise<unknown>;
  read(): string | null;
  write(version: string): void;
}

/**
 * The notes to show once after an update: when this build's version differs from the one this browser last ran and
 * /release-notes.json describes this version. A first visit only remembers the version (a new player needs no
 * changelog), development builds never ask, and missing or unreadable notes show nothing. The version counts as seen
 * as soon as the notes are handed out, so a reload doesn't show them again.
 */
export async function pendingReleaseNotes(deps: WhatsNewDeps = browserDeps()): Promise<ReleaseNotes | null> {
  if (deps.version === 'dev') return null;
  const seen = deps.read();
  if (seen === deps.version) return null;
  deps.write(deps.version);
  if (seen === null) return null;
  let notes: ReleaseNotes | null = null;
  try {
    notes = readReleaseNotes(await deps.load());
  } catch {
    // offline, or a build without notes: nothing to say
  }
  return notes && notes.version === deps.version ? notes : null;
}

function browserDeps(): WhatsNewDeps {
  return {
    version: APP_VERSION,
    async load() {
      // never the HTTP cache's copy: the notes belong to the build that just arrived
      const response = await fetch('/release-notes.json', { cache: 'no-cache' });
      return response.ok ? response.json() : null;
    },
    read: () => readJson<string | null>(SEEN_KEY, null),
    write: (version) => writeJson(SEEN_KEY, version),
  };
}
