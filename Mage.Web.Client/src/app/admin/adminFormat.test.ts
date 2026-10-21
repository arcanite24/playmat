import { describe, expect, it, test } from 'vitest';
import { atLeast, describeAge, formatBytes, formatDuration, inviteLink, inviteState, money } from './adminFormat';

describe('admin formatting', () => {
  test('bytes', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(4096)).toBe('4.0 KB');
    expect(formatBytes(300 * 1024)).toBe('300 KB');
    expect(formatBytes(3 * 1024 * 1024)).toBe('3.0 MB');
  });
  test('durations and ages', () => {
    expect(formatDuration(5 * 60_000)).toBe('5 min');
    expect(formatDuration(125 * 60_000)).toBe('2 h 5 min');
    expect(formatDuration(50 * 3_600_000)).toBe('2 days 2 h');
    expect(describeAge(1000, 20_000)).toBe('just now');
    expect(describeAge(0, 20_000)).toBe('');
  });
});

describe('invites', () => {
  const now = 1_000_000;
  it('says why a link stopped working', () => {
    expect(inviteState({ uses: 0, maxUses: 1, expiresAt: now + 1 }, now)).toBe('active');
    expect(inviteState({ uses: 1, maxUses: 1, expiresAt: now + 1 }, now)).toBe('used up');
    expect(inviteState({ uses: 0, maxUses: 1, expiresAt: now }, now)).toBe('expired');
    expect(inviteState({ revoked: true, uses: 0, maxUses: 1, expiresAt: now + 1 }, now)).toBe('revoked');
  });

  it('builds the join link', () => {
    expect(inviteLink('a-b_c', 'https://play.example.com/')).toBe('https://play.example.com/join?invite=a-b_c');
  });
  test('staff roles rank', () => {
    expect(atLeast('owner', 'admin')).toBe(true);
    expect(atLeast('admin', 'admin')).toBe(true);
    expect(atLeast('moderator', 'admin')).toBe(false);
    expect(atLeast(null, 'moderator')).toBe(false);
  });
});

describe('support amounts', () => {
  test('in their own currency, and readable when the code is not one', () => {
    expect(money(500)).toBe('$5.00');
    expect(money(300, 'EUR')).toBe('€3.00');
    expect(money(1234, 'NOPE')).toBe('12.34 NOPE');
  });
});
