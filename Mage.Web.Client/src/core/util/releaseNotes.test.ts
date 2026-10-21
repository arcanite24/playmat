import { describe, expect, test } from 'vitest';
import { cleanItem, parseReleaseNotes, readReleaseNotes } from './releaseNotes';

// what GitHub generates with .github/release.yml
const GENERATED = `<!-- Release notes generated using configuration in .github/release.yml at v1.3.0 -->

## What's Changed
### Features
* Players see the version and what changed by @arcanite24 in https://github.com/arcanite24/playmat/pull/60
* **Career**: a third tier of opponents by @someone-else in https://github.com/arcanite24/playmat/pull/61
### Fixes
* The draft timer no longer stops at zero by @arcanite24 in https://github.com/arcanite24/playmat/pull/62
### Ops
* Deploys from GitHub releases by @arcanite24 in https://github.com/arcanite24/playmat/pull/63

## New Contributors
* @someone-else made their first contribution in https://github.com/arcanite24/playmat/pull/61

**Full Changelog**: https://github.com/arcanite24/playmat/compare/v1.2.0...v1.3.0
`;

describe('release notes', () => {
  test('generated notes become headed lists of changes', () => {
    expect(parseReleaseNotes(GENERATED, 'v1.3.0', 'https://github.com/arcanite24/playmat/releases/tag/v1.3.0')).toEqual({
      version: 'v1.3.0',
      url: 'https://github.com/arcanite24/playmat/releases/tag/v1.3.0',
      sections: [
        { id: 'features', title: 'Features', items: ['Players see the version and what changed', 'Career: a third tier of opponents'] },
        { id: 'fixes', title: 'Fixes', items: ['The draft timer no longer stops at zero'] },
        { id: 'ops', title: 'Ops', items: ['Deploys from GitHub releases'] },
      ],
    });
  });

  test('hand-written notes keep their own headings, and loose items go under Other', () => {
    const notes = parseReleaseNotes('A quiet release.\n\n- Faster lobby\n\n## 🎉 Highlights\n- [Sealed](https://x.y) events\n- \n', 'v2.0.0');
    expect(notes.sections).toEqual([
      { id: 'other', title: 'Other', items: ['Faster lobby'] },
      { title: 'Highlights', items: ['Sealed events'] },
    ]);
    expect(notes.url).toBeUndefined();
  });

  test('the desktop download notes are not changes', () => {
    const body = `${GENERATED}\n<!-- desktop-downloads -->\nPlaymat 1.3.0 for your own computer.\n\n### The first time\n- **Windows** may say...\n`;
    expect(parseReleaseNotes(body, 'v1.3.0').sections.map((section) => section.title)).toEqual(['Features', 'Fixes', 'Ops']);
  });

  test('nothing for players means no sections', () => {
    expect(parseReleaseNotes('**Full Changelog**: https://github.com/a/b/compare/v1...v2', 'v2').sections).toEqual([]);
  });

  test('items lose authors, links and markup', () => {
    expect(cleanItem('Fix `x` by @dependabot[bot] in https://github.com/a/b/pull/9')).toBe('Fix x');
    expect(cleanItem('Fix y in https://github.com/a/b/pull/10')).toBe('Fix y');
    expect(cleanItem('x'.repeat(500))).toHaveLength(200);
  });

  test('only well-formed notes are read', () => {
    expect(readReleaseNotes(null)).toBeNull();
    expect(readReleaseNotes({ version: 'v1', sections: [] })).toBeNull();
    expect(readReleaseNotes({ version: 'v1', sections: [{ title: 'A', items: [1] }] })).toBeNull();
    expect(readReleaseNotes({ version: 'v1', url: 'javascript:alert(1)', sections: [{ title: 'A', items: ['b'] }] }))
      .toEqual({ version: 'v1', sections: [{ title: 'A', items: ['b'] }] });
  });
});
