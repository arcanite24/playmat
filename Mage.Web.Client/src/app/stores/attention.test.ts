// @vitest-environment happy-dom
import { afterEach, describe, expect, test, vi } from 'vitest';
import { attention, notificationAllowed, pendingAttention } from './attention';
import { DEFAULT_NOTIFY_ON, useSettings } from './settings';

function setAway(away: boolean) {
  vi.spyOn(document, 'hasFocus').mockReturnValue(!away);
}

afterEach(() => {
  vi.restoreAllMocks();
  window.dispatchEvent(new Event('focus'));
});

describe('attention', () => {
  test('counts in the tab title while the player is away, and clears when they come back', () => {
    document.title = 'Playmat';
    setAway(true);
    attention('Your move', 'Bob attacks', { kind: 'turn', tag: 'game:1' });
    attention('Draft pick', 'Pack 1, pick 2', { kind: 'draft', tag: 'draft' });
    expect(document.title).toBe('(2) Playmat');
    expect(pendingAttention()).toBe(2);

    setAway(false);
    window.dispatchEvent(new Event('focus'));
    expect(document.title).toBe('Playmat');
  });

  test('stays quiet while the player is looking', () => {
    document.title = 'Playmat';
    setAway(false);
    attention('Your move', '', { kind: 'turn', tag: 'game:1' });
    expect(document.title).toBe('Playmat');
  });
});

describe('notification preferences (#208)', () => {
  test('nothing notifies with notifications off; with them on, each kind has its own switch', () => {
    expect(notificationAllowed({ notifications: false, notifyOn: DEFAULT_NOTIFY_ON }, 'whispers')).toBe(false);
    expect(notificationAllowed({ notifications: true, notifyOn: DEFAULT_NOTIFY_ON }, 'whispers')).toBe(true);
    const noWhispers = { ...DEFAULT_NOTIFY_ON, whispers: false };
    expect(notificationAllowed({ notifications: true, notifyOn: noWhispers }, 'whispers')).toBe(false);
    expect(notificationAllowed({ notifications: true, notifyOn: noWhispers }, 'tableJoins')).toBe(true);
  });

  test('a kind added after the settings were saved starts on', () => {
    const saved = { whispers: false } as unknown as typeof DEFAULT_NOTIFY_ON;
    expect(notificationAllowed({ notifications: true, notifyOn: saved }, 'tableJoins')).toBe(true);
    expect(notificationAllowed({ notifications: true, notifyOn: saved }, 'whispers')).toBe(false);
    expect(notificationAllowed({ notifications: true, notifyOn: undefined as unknown as typeof DEFAULT_NOTIFY_ON }, 'turn')).toBe(true);
  });

  test('only the kinds the player keeps on reach the browser', () => {
    const shown: string[] = [];
    class FakeNotification {
      static permission = 'granted';
      onclick: (() => void) | null = null;
      constructor(title: string) {
        shown.push(title);
      }
      close() {}
    }
    vi.stubGlobal('Notification', FakeNotification);
    const before = useSettings.getState().settings;
    useSettings.setState({ settings: { ...before, notifications: true, notifyOn: { ...DEFAULT_NOTIFY_ON, whispers: false } } });
    try {
      setAway(true);
      attention('Whisper from Bob', 'hi', { kind: 'whispers', tag: 'whisper:Bob' });
      attention('Bob joined your table', 'Friday', { kind: 'tableJoins', tag: 'table:1' });
      expect(shown).toEqual(['Bob joined your table']);
      expect(pendingAttention()).toBe(2);
    } finally {
      useSettings.setState({ settings: before });
      vi.unstubAllGlobals();
    }
  });
});
