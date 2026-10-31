import { describe, expect, it } from 'vitest';
import type { PermanentView } from '../../protocol/generated/views';
import { counterBadges, isImpending } from './counters';

describe('counterBadges', () => {
  const permanent = (counters: { name: string; count: number }[], extra: Partial<PermanentView> = {}) => ({ counters, cardTypes: ['CREATURE'], rules: [], ...extra }) as PermanentView;

  it('prints +1/+1 counters as they are and gives the others a symbol', () => {
    const badges = counterBadges(permanent([{ name: '+1/+1', count: 2 }, { name: '-1/-1', count: 1 }, { name: 'stun', count: 1 }, { name: 'finality', count: 1 }, { name: 'empty', count: 0 }]));
    expect(badges.map((badge) => [badge.kind, badge.label, badge.showCount])).toEqual([
      ['pt', '+1/+1', true],
      ['pt', '−1/−1', false],
      ['stun', '', false],
      ['finality', '', false],
    ]);
    expect(badges[2].word).toBe('stun');
  });

  it('always shows how many time counters, and keeps unknown names whole when short', () => {
    expect(counterBadges(permanent([{ name: 'time', count: 1 }]))[0]).toMatchObject({ kind: 'time', label: '', showCount: true });
    expect(counterBadges(permanent([{ name: 'bounty', count: 1 }]))[0]).toMatchObject({ kind: 'other', label: 'bounty' });
    expect(counterBadges(permanent([{ name: 'incarnation', count: 3 }]))[0]).toMatchObject({ label: 'incarnati…', showCount: true });
  });

  it('says how long an impending Overlord has left before it is a creature', () => {
    const overlord = permanent([{ name: 'time', count: 4 }], {
      cardTypes: ['ENCHANTMENT'],
      rules: ['Impending 4&mdash;{2}{W}{W} <i>(If you cast this spell for its impending cost...)</i>'],
    });
    expect(isImpending(overlord)).toBe(true);
    expect(counterBadges(overlord)[0]).toMatchObject({ kind: 'time', count: 4, impending: true });
    // once it is a creature (the counters gone, or cast for its full cost) it is not impending
    expect(isImpending({ ...overlord, cardTypes: ['ENCHANTMENT', 'CREATURE'] })).toBe(false);
    expect(isImpending({ ...overlord, counters: [] })).toBe(false);
  });
});
