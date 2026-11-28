import { describe, expect, test } from 'vitest';
import { currentReleaseNotes, pendingReleaseNotes, type WhatsNewDeps } from './whatsNew';

const NOTES = { version: 'v1.3.0', sections: [{ id: 'features', title: 'Features', items: ['Sealed events'] }] };

function deps(version: string, seen: string | null, notes: unknown = NOTES) {
  const state = { seen, loads: 0 };
  const value: WhatsNewDeps = {
    version,
    async load() {
      state.loads++;
      if (notes instanceof Error) throw notes;
      return notes;
    },
    read: () => state.seen,
    write: (next) => {
      state.seen = next;
    },
  };
  return { deps: value, state };
}

describe("what's new", () => {
  test('an update shows its notes once', async () => {
    const { deps: d, state } = deps('v1.3.0', 'v1.2.0');
    expect(await pendingReleaseNotes(d)).toEqual(NOTES);
    expect(state.seen).toBe('v1.3.0');
    expect(await pendingReleaseNotes(d)).toBeNull();
    expect(state.loads).toBe(1);
  });

  test('a first visit only remembers the version', async () => {
    const { deps: d, state } = deps('v1.3.0', null);
    expect(await pendingReleaseNotes(d)).toBeNull();
    expect(state).toEqual({ seen: 'v1.3.0', loads: 0 });
  });

  test('development builds never ask', async () => {
    const { deps: d, state } = deps('dev', 'v1.2.0');
    expect(await pendingReleaseNotes(d)).toBeNull();
    expect(state).toEqual({ seen: 'v1.2.0', loads: 0 });
  });

  test('missing, broken or other versions\' notes show nothing, and the update still counts as seen', async () => {
    for (const notes of [null, new Error('offline'), { nope: true }, { ...NOTES, version: 'v1.2.9' }]) {
      const { deps: d, state } = deps('v1.3.0', 'v1.2.0', notes);
      expect(await pendingReleaseNotes(d)).toBeNull();
      expect(state.seen).toBe('v1.3.0');
    }
  });

  test('the link shows this build\'s notes again, seen or not (#139)', async () => {
    const { deps: d } = deps('v1.3.0', 'v1.3.0');
    expect(await currentReleaseNotes(d)).toEqual(NOTES);
  });

  test('the link has nothing for notes of another version, or none at all', async () => {
    expect(await currentReleaseNotes(deps('v1.4.0', 'v1.4.0').deps)).toBeNull();
    expect(await currentReleaseNotes(deps('v1.3.0', 'v1.3.0', new Error('offline')).deps)).toBeNull();
  });
});
