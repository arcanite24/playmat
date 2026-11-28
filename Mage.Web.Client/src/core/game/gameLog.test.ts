import { describe, expect, it } from 'vitest';
import type { CardView, GameView } from '../../protocol/generated/views';
import {
  buildLogRows,
  presenceChange,
  cardInk,
  classifyLog,
  collectCards,
  findLogCard,
  isEmote,
  parseChatMessage,
  parseLogHtml,
  parseTurnInfo,
  type LogSegment,
} from './gameLog';

const MOUNTAIN = "<font color='#B0C4DE' object_id='3d2cae7c-9785-47b6-a636-84b07d939425'>Mountain</font> [3d2]";
const HUMAN = "<font color='#20B2AA'>Human</font>";

describe('parseLogHtml', () => {
  it('turns player and card fonts into references and drops the short ids', () => {
    expect(parseLogHtml(`${HUMAN} plays ${MOUNTAIN} from hand onto the Battlefield`)).toEqual([
      { kind: 'player', name: 'Human' },
      { kind: 'text', text: ' plays ' },
      { kind: 'card', name: 'Mountain', objectId: '3d2cae7c-9785-47b6-a636-84b07d939425', color: '#B0C4DE', alternativeName: null },
      { kind: 'text', text: ' from hand onto the Battlefield' },
    ]);
  });

  it('reads colored names without ids as cards, and other colors as text', () => {
    const segments = parseLogHtml("<font color='#90EE90'>Llanowar Elves</font> is <font color='#D2691E'>countered</font>");
    expect(segments).toEqual([
      { kind: 'card', name: 'Llanowar Elves', objectId: null, color: '#90EE90', alternativeName: null },
      { kind: 'text', text: ' is countered' },
    ]);
  });

  it('keeps the alternative name for cards shown under another face', () => {
    const [card] = parseLogHtml("<font color='#FF6347' object_id='abc-1' alternative_name='Brazen%20Borrower'>Petty Theft</font>");
    expect(card).toMatchObject({ kind: 'card', name: 'Petty Theft', alternativeName: 'Brazen Borrower' });
  });

  it('never keeps markup: scripts, unknown tags and entities become plain text', () => {
    const segments = parseLogHtml('<script>alert(1)</script><img src=x onerror=alert(1)>Fish &amp; chips &lt;b&gt; <b>bold</b><br>next');
    expect(segments).toEqual([{ kind: 'text', text: 'Fish & chips <b> bold next' }]);
  });

  it('survives unclosed and nested fonts', () => {
    expect(parseLogHtml("<font color='White'>" + HUMAN + ' draws a card')).toEqual([
      { kind: 'player', name: 'Human' },
      { kind: 'text', text: ' draws a card' },
    ]);
    expect(parseLogHtml("<font color='#FF6347' object_id='x'>Shock")).toEqual([
      { kind: 'card', name: 'Shock', objectId: 'x', color: '#FF6347', alternativeName: null },
    ]);
  });

  it('names a face-down object by what it is', () => {
    expect(parseLogHtml("<font color='#B0C4DE' object_id='y'>[fc7]</font>")[0]).toMatchObject({ kind: 'card', name: 'face-down card' });
  });

  it('returns nothing for empty input', () => {
    expect(parseLogHtml(undefined)).toEqual([]);
    expect(parseLogHtml('   ')).toEqual([]);
  });
});

describe('classifyLog', () => {
  it.each([
    ['Human casts Shock targeting Bob', 'cast'],
    ['Human plays Mountain from hand onto the Battlefield', 'land'],
    ['Human attacks Bob with 2 creatures', 'attack'],
    ['Grizzly Bears blocks Goblin Guide', 'block'],
    ['Shock deals 2 damage to Bob', 'damage'],
    ['Bob gains 3 life', 'lifeGain'],
    ['Bob loses 2 life', 'lifeLoss'],
    ['Bob draws a card', 'draw'],
    ['Grizzly Bears was destroyed', 'destroy'],
    ['Bob has conceded.', 'lose'],
    ['TURN 3 for Bob (20 - 20)', 'turn'],
    ['Bob decides to take mulligan', 'mulligan'],
    ['It has become day', 'info'],
  ])('%s → %s', (text, icon) => {
    expect(classifyLog(text)).toBe(icon);
  });
});

describe('parseChatMessage', () => {
  it('reads game lines with their turn and icon', () => {
    const entry = parseChatMessage({ messageType: 'GAME', message: `${HUMAN} draws a card`, turnInfo: 'T5.DRAW', time: 10 }, 'k1');
    expect(entry).toMatchObject({ tone: 'game', who: null, text: 'Human draws a card', icon: 'draw', turn: 5, at: 10, turnStart: null });
  });

  it('recognizes the start of a turn', () => {
    const entry = parseChatMessage({ messageType: 'GAME', message: `TURN 4 (extra) for ${HUMAN} (20 - 17)`, turnInfo: 'T4' }, 'k2', 5);
    expect(entry?.turnStart).toEqual({ turn: 4, player: 'Human', extra: true });
    expect(entry?.icon).toBe('turn');
  });

  it('keeps what players type as text, markup included', () => {
    const entry = parseChatMessage({ messageType: 'TALK', username: 'Bob', message: "gg <font color='#20B2AA'>wp</font> &lt;3" }, 'k3', 1);
    expect(entry).toMatchObject({ tone: 'chat', who: 'Bob', text: 'gg wp <3', icon: 'chat', segments: [{ kind: 'text', text: 'gg wp <3' }] });
  });

  it('skips empty messages', () => {
    expect(parseChatMessage({ messageType: 'GAME', message: '<br>' }, 'k4')).toBeNull();
  });
});

describe('parseTurnInfo', () => {
  it('reads the turn number', () => {
    expect(parseTurnInfo('T12.M2')).toBe(12);
    expect(parseTurnInfo('T0')).toBe(0);
    expect(parseTurnInfo(undefined)).toBeNull();
  });
});

describe('buildLogRows', () => {
  const line = (key: string, html: string, turnInfo?: string) => parseChatMessage({ messageType: 'GAME', message: html, turnInfo }, key, 0)!;

  it('turns the server turn line into a separator and marks the player own turns', () => {
    const rows = buildLogRows([line('a', `TURN 1 for ${HUMAN} (20 - 20)`, 'T1'), line('b', `${HUMAN} draws a card`, 'T1.DRAW')], 'Human');
    expect(rows.map((row) => row.kind)).toEqual(['separator', 'entry']);
    expect(rows[0]).toMatchObject({ turn: 1, player: 'Human', mine: true });
  });

  it('starts a turn from the turn number when the server line is missing', () => {
    const rows = buildLogRows([line('a', 'x', 'T2.M1'), line('b', 'y', 'T2.M1'), line('c', 'z', 'T3.UP'), { ...line('d', 'w', 'T3.UP'), turnOwner: 'Bob' }], 'Me');
    expect(rows.map((row) => (row.kind === 'separator' ? `T${row.turn}:${row.player}` : row.key))).toEqual(['T2:null', 'a', 'b', 'T3:Bob', 'c', 'd']);
  });

  it('adds no separators before the game starts', () => {
    expect(buildLogRows([line('a', 'Bob has joined the game', 'T0')], 'Me').map((row) => row.kind)).toEqual(['entry']);
  });
});

describe('card lookup', () => {
  const bolt: CardView = { id: 'b1', name: 'Lightning Bolt' };
  const bear: CardView = { id: 'g1', name: 'Grizzly Bears' };
  const view: GameView = { stack: { b1: bolt }, players: [{ battlefield: { g1: bear } as never }] };
  const ref = (name: string, objectId: string | null): Extract<LogSegment, { kind: 'card' }> => ({ kind: 'card', name, objectId, color: null, alternativeName: null });

  it('finds cards by id, then by name', () => {
    const cards = collectCards(view);
    expect(findLogCard(cards, ref('Lightning Bolt', 'b1'))).toBe(bolt);
    expect(findLogCard(cards, ref('Grizzly Bears', 'gone'))).toBe(bear);
    expect(findLogCard(cards, ref('Island', null))).toBeNull();
  });
});

describe('isEmote', () => {
  it('matches only the quick phrases', () => {
    expect(isEmote('Good game')).toBe(true);
    expect(isEmote('Good game, really')).toBe(false);
  });
});

describe('cardInk', () => {
  it('maps the server name colors to card colors', () => {
    expect(cardInk('#90EE90')).toBe('g');
    expect(cardInk('#DAA520')).toBe('multi');
    expect(cardInk('White')).toBeNull();
    expect(cardInk(null)).toBeNull();
  });
});

describe('presenceChange', () => {
  it('reads players dropping off and coming back from the status lines', () => {
    expect(presenceChange('bashbob has lost connection')).toEqual({ name: 'bashbob', online: false });
    expect(presenceChange('bashbob catch connection problems for 35 secs (left before expire: 145 secs)')).toEqual({ name: 'bashbob', online: false });
    expect(presenceChange('bashbob has left XMage')).toEqual({ name: 'bashbob', online: false });
    expect(presenceChange('bashbob session expired')).toEqual({ name: 'bashbob', online: false });
    expect(presenceChange('bashbob has joined')).toEqual({ name: 'bashbob', online: true });
    // back from a sleep without leaving the chat: the server says so instead of "has joined" (#171)
    expect(presenceChange('bashbob has reconnected')).toEqual({ name: 'bashbob', online: true });
  });

  it('ignores other lines', () => {
    expect(presenceChange('bashbob casts Lightning Bolt')).toBeNull();
    expect(presenceChange('Bob has joined the game as a watcher')).toBeNull();
  });
});
