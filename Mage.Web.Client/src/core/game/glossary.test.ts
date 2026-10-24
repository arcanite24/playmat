import { describe, expect, it } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { explainCard, termSpans } from './glossary';

const creature = (rules: string[]): CardView => ({ name: 'Test', cardTypes: ['CREATURE'], rules });

describe('explainCard', () => {
  it('classifies keyword, triggered, activated and static lines', () => {
    const { lines } = explainCard(creature([
      'Flying, vigilance',
      'When Test enters, draw a card.',
      '{2}{G}, {T}: Put a +1/+1 counter on target creature.',
      'Other creatures you control get +1/+1.',
    ]));
    expect(lines.map((line) => line.kind)).toEqual(['keyword', 'triggered', 'activated', 'static']);
  });

  it('explains keywords and actions, once each, in order', () => {
    const { terms } = explainCard(creature(['Flying, lifelink', 'Whenever Test attacks, scry 1.', 'Creatures you control have flying.']));
    expect(terms.map((term) => term.name)).toEqual(['Flying', 'Lifelink', 'Scry']);
  });

  it('recognizes keywords with costs and ability words', () => {
    const { lines, terms } = explainCard(creature(['Ward {2}', 'Landfall — Whenever a land enters the battlefield under your control, Test gets +2/+2 until end of turn.']));
    expect(lines[1]).toMatchObject({ kind: 'triggered', abilityWord: 'Landfall' });
    expect(terms.map((term) => term.name)).toEqual(['Ward', 'Landfall']);
  });

  it("doesn't read keywords out of reminder text, card names or +1/+1 counters", () => {
    const { terms } = explainCard(creature([
      'Trample <i>(This creature can deal excess combat damage to the player or planeswalker it\'s attacking.)</i>',
      'Shadow of Doubt deals 2 damage.',
      'Put a +1/+1 counter on it.',
    ]));
    expect(terms.map((term) => term.name)).toEqual(['Trample']);
  });

  it('marks loyalty abilities and spell effects', () => {
    expect(explainCard({ cardTypes: ['PLANESWALKER'], rules: ['+1: Draw a card.', '−3: Destroy target creature.'] }).lines.map((line) => line.kind))
      .toEqual(['loyalty', 'loyalty']);
    expect(explainCard({ cardTypes: ['INSTANT'], rules: ['Counter target spell.'] })).toMatchObject({
      lines: [{ kind: 'spell' }],
      terms: [{ name: 'Counter' }],
    });
  });
});

describe('hints', () => {
  it('splits the live hints from the rules and reads their icon', () => {
    const card = creature([
      'Deathtouch',
      '<i>Revolt</i> &mdash; {this} enters with a +1/+1 counter on it if a permanent you controlled left the battlefield this turn.',
      '<br/><hintstart/>ICON_BAD<font color=#ff0000>A permanent left the battlefield under your control this turn</font>',
    ]);
    const { lines, hints, terms } = explainCard(card);
    expect(lines).toHaveLength(2);
    expect(lines[1]).toMatchObject({ kind: 'static', abilityWord: 'Revolt' });
    expect(lines[1].text).toContain('Test enters');
    expect(hints).toEqual([{ text: 'A permanent left the battlefield under your control this turn', tone: 'bad' }]);
    expect(terms.map((term) => term.name)).toEqual(['Deathtouch', 'Revolt']);
  });
});

describe('Duskmourn', () => {
  const names = (card: CardView) => explainCard(card).terms.map((term) => term.name);

  it('explains impending on an Overlord as a keyword line', () => {
    const overlord: CardView = {
      name: 'Overlord of the Mistmoors',
      cardTypes: ['ENCHANTMENT', 'CREATURE'],
      rules: [
        'Impending 4&mdash;{2}{W}{W} <i>(If you cast this spell for its impending cost, it enters with four time counters and isn\'t a creature until the last is removed. At the beginning of your end step, remove a time counter from it.)</i>',
        'Whenever {this} enters or attacks, create two 2/1 white Insect creature tokens with flying.',
      ],
    };
    const { lines, terms } = explainCard(overlord);
    expect(lines[0].kind).toBe('keyword');
    expect(terms.map((term) => term.name)).toEqual(['Impending', 'Flying']);
    expect(terms[0].text).toMatch(/time counters/);
  });

  it('tells manifest dread from manifest', () => {
    expect(names(creature(['When {this} dies, manifest dread.']))).toEqual(['Manifest dread']);
    expect(names(creature(['Whenever you manifest dread, put a card you put into your graveyard this way into your hand.']))).toEqual(['Manifest dread']);
    expect(names(creature(['Its controller manifests dread.']))).toEqual(['Manifest dread']);
    expect(names(creature(['Manifest the top card of your library.']))).toEqual(['Manifest']);
  });

  it('explains Rooms by their type, and doors and unlocking where they are mentioned', () => {
    const room = { name: 'Bottomless Pool // Locker Room', cardTypes: ['ENCHANTMENT'], subTypes: ['ROOM'], rules: ['(You may cast either half. That door unlocks on the battlefield.)'] } as unknown as CardView;
    expect(names(room)).toEqual(['Room']);
    expect(names(creature(['Whenever {this} deals combat damage to a player, unlock a locked door of up to one target Room you control.']))).toEqual(['Room']);
    const room2 = explainCard(room).terms[0].text;
    expect(room2).toMatch(/sorcery/);
    expect(room2).toMatch(/eerie/);
  });

  it('reads the ability words, landcycling and counters with rules of their own', () => {
    expect(names(creature(['<i>Eerie</i> &mdash; Whenever an enchantment you control enters and whenever you fully unlock a Room, draw a card.'])))
      .toEqual(['Eerie', 'Room']);
    expect(names(creature(['<i>Survival</i> &mdash; At the beginning of your second main phase, if {this} is tapped, manifest dread.'])))
      .toEqual(['Survival', 'Manifest dread']);
    expect(names(creature(['Mountaincycling {2}']))).toEqual(['Cycling']);
    expect(names(creature(['When {this} enters, tap target creature an opponent controls and put a stun counter on it.']))).toEqual(['Stun counter']);
    expect(names(creature(['Return target creature card from your graveyard to the battlefield with a finality counter on it.']))).toEqual(['Finality counter']);
    expect(names(creature(["<i>Council's dilemma</i> &mdash; Starting with you, each player votes for time or money."]))).toEqual(["Council's dilemma"]);
  });

  it("doesn't read keywords out of the card's own name, and finds an ability word after a sentence", () => {
    const bargain: CardView = { name: "Betrayer's Bargain", cardTypes: ['INSTANT'], rules: ['{this} deals 5 damage to target creature.'] };
    expect(names(bargain)).toEqual([]);
    expect(termSpans("Betrayer's Bargain deals 5 damage.", bargain).some((span) => span.term)).toBe(false);
    expect(names({ name: 'Unholy Heat', cardTypes: ['INSTANT'], rules: ['{this} deals 2 damage to target creature.<i>Delirium</i> &mdash; It deals 6 damage instead.'] }))
      .toEqual(['Delirium']);
  });

  it('explains a face-down permanent, whatever it is', () => {
    expect(names({ name: 'Manifest', faceDown: true, cardTypes: ['CREATURE'], rules: [] })).toEqual(['Face-down']);
  });

  it('explains a counter the permanent has even when its text never mentions it', () => {
    const stunned = { ...creature(['Flying']), counters: [{ name: 'stun', count: 1 }] } as unknown as CardView;
    expect(names(stunned)).toEqual(['Flying', 'Stun counter']);
  });

  it('has an explanation for every keyword and ability word printed on a DSK card', () => {
    const printed = ['Flash', 'Ward {2}', 'Convoke', 'Flashback {3}{W}{W}{B}', 'Ninjutsu {1}{U}{B}', 'Crew 1', 'Equip {2}', 'Enchant creature', 'Delirium — Draw a card.', 'Surveil 2.'];
    for (const line of printed) expect(names(creature([line])).length, line).toBeGreaterThan(0);
  });
});

describe('termSpans', () => {
  const marked = (text: string) => termSpans(text).filter((span) => span.term).map((span) => [span.text, span.term!.name]);

  it('cuts text into terms and the text between, losing nothing', () => {
    const text = 'Flying, lifelink. Whenever this attacks, scry 1 and manifest dread.';
    const spans = termSpans(text);
    expect(spans.map((span) => span.text).join('')).toBe(text);
    expect(marked(text)).toEqual([['Flying', 'Flying'], ['lifelink', 'Lifelink'], ['scry', 'Scry'], ['manifest dread', 'Manifest dread']]);
  });

  it("doesn't mark reminder text, card names or strict keywords inside sentences", () => {
    expect(marked('Trample (This creature can deal excess combat damage to the player or planeswalker it\'s attacking.)')).toEqual([['Trample', 'Trample']]);
    expect(marked('Shadow of Doubt deals 2 damage.')).toEqual([]);
    expect(marked('Shadow')).toEqual([['Shadow', 'Shadow']]);
    expect(marked('Put a +1/+1 counter on it.')).toEqual([]);
  });

  it('marks ability words only before their dash, and the DSK mechanics', () => {
    expect(marked('Eerie — Whenever you fully unlock a Room, draw a card.')).toEqual([['Eerie', 'Eerie'], ['unlock', 'Room'], ['Room', 'Room']]);
    expect(marked('The eerie silence.')).toEqual([]);
    expect(marked('Impending 4—{1}{U}{U}')).toEqual([['Impending', 'Impending']]);
    expect(marked('Put a stun counter on it.')).toEqual([['stun counter', 'Stun counter']]);
  });
});
