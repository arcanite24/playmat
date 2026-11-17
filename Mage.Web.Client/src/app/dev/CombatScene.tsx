import { useEffect, useMemo, useRef, useState } from 'react';
import { useStore } from 'zustand';
import { GameSession } from '../../core/game/gameSession';
import { EventBus } from '../../core/rpc/EventBus';
import type { CardView, CombatGroupView, GameEventView, GameView, PermanentView } from '../../protocol/generated/views';
import { api } from '../connection';
import { MatchStage } from '../match/MatchStage';
import { useSettings } from '../stores/settings';

/**
 * Dev builds only (/dev/combat): combat and board effects (#413) on a real match stage, fed scripted views the way a
 * replay is. Each scene plays a few views a second or so apart: the attack, then the damage with its game events.
 * Fancy or plain combat effects switch from the bar, so the two can be compared on the same scene.
 * `node e2e/combat-shots.mjs http://localhost:5173` saves frames of the combat scenes, frozen moments after the first
 * swing, into .impeccable/combat (PLAYWRIGHT_CHANNEL=chrome where Playwright's own browsers aren't installed).
 */

const NO_EVENTS = {
  onEvent: () => () => undefined,
  onSessionStart: () => () => undefined,
  onStatus: () => () => undefined,
};

type Fixture = Partial<PermanentView> & { id: string; name: string; set: string; number: string };

let seq = 1;

function permanent({ set, number, ...card }: Fixture, controllerId: string): PermanentView {
  return {
    cardId: card.id, expansionSetCode: set, cardNumber: number, controllerId, ownerId: controllerId,
    cardTypes: ['CREATURE'], rules: [], ...card,
  } as PermanentView;
}

const BEARS = (id: string): Fixture => ({ id, name: 'Grizzly Bears', set: '10E', number: '268', power: '2', toughness: '2', manaValue: 2 });
const ELVES = (id: string): Fixture => ({ id, name: 'Llanowar Elves', set: 'FDN', number: '227', power: '1', toughness: '1', manaValue: 1 });
const GIANT = (id: string): Fixture => ({ id, name: 'Hill Giant', set: '10E', number: '212', power: '3', toughness: '3', manaValue: 4 });
const WURM = (id: string): Fixture => ({ id, name: 'Craw Wurm', set: 'M10', number: '173', power: '6', toughness: '4', manaValue: 6 });
const BRONTO = (id: string): Fixture => ({ id, name: 'Ancient Brontodon', set: 'XLN', number: '175', power: '9', toughness: '9', manaValue: 8 });
const ACE = (id: string): Fixture => ({ id, name: 'Fencing Ace', set: 'J25', number: '199', power: '4', toughness: '4', manaValue: 2, rules: ['Double strike'] });
const NIGHTHAWK = (id: string): Fixture => ({ id, name: 'Vampire Nighthawk', set: 'FDN', number: '757', power: '2', toughness: '3', manaValue: 3, rules: ['Flying', 'Deathtouch', 'Lifelink'] });
const MYR = (id: string): Fixture => ({ id, name: 'Darksteel Myr', set: 'SOM', number: '151', power: '0', toughness: '1', manaValue: 3, cardTypes: ['ARTIFACT', 'CREATURE'], rules: ['Indestructible'] });
const STINGER = (id: string): Fixture => ({ id, name: 'Plague Stinger', set: 'SOM', number: '75', power: '1', toughness: '1', manaValue: 2, rules: ['Flying', 'Infect'] });
const JACE = (id: string, loyalty: number): Fixture => ({ id, name: 'Jace Beleren', set: 'M10', number: '58', cardTypes: ['PLANESWALKER'], loyalty: String(loyalty), manaValue: 3 });
const GARRUK = (id: string, loyalty: number): Fixture => ({ id, name: 'Garruk Wildspeaker', set: 'CMD', number: '157', cardTypes: ['PLANESWALKER'], loyalty: String(loyalty), manaValue: 4 });
const AJANI = (id: string, loyalty: number): Fixture => ({ id, name: 'Ajani Goldmane', set: 'M11', number: '1', cardTypes: ['PLANESWALKER'], loyalty: String(loyalty), manaValue: 4 });
const DREADMAW = (id: string): Fixture => ({ id, name: 'Colossal Dreadmaw', set: 'J25', number: '646', power: '6', toughness: '6', manaValue: 6, rules: ['Trample'] });
const SOLDIER = (id: string): Fixture => ({ id, name: 'Soldier', set: 'M21', number: '', power: '1', toughness: '1', isToken: true });

interface Board {
  mine: Fixture[];
  theirs: Fixture[];
  myLife?: number;
  theirLife?: number;
  theirPoison?: number;
  /** cards in the graveyards (creatures that died fly there) */
  myGrave?: Fixture[];
  theirGrave?: Fixture[];
}

interface Frame {
  board: Board;
  step: string;
  combat?: { attacker: string; defender: string; blockers?: string[] }[];
  events?: Omit<GameEventView, 'seq'>[];
  stack?: Record<string, CardView>;
  /** ms before the next frame */
  wait?: number;
}

function view(frame: Frame): GameView {
  const mine = new Map(frame.board.mine.map((card) => [card.id, permanent(card, 'me')]));
  const theirs = new Map(frame.board.theirs.map((card) => [card.id, permanent(card, 'them')]));
  const all = new Map([...mine, ...theirs]);
  const combat: CombatGroupView[] = (frame.combat ?? []).map((group) => ({
    attackers: { [group.attacker]: all.get(group.attacker)! },
    blockers: Object.fromEntries((group.blockers ?? []).filter((id) => all.has(id)).map((id) => [id, all.get(id)!])),
    defenderId: group.defender,
    isBlocked: (group.blockers ?? []).length > 0,
  }));
  const attacking = new Set((frame.combat ?? []).map((group) => group.attacker));
  for (const id of attacking) {
    const card = all.get(id);
    if (card) card.tapped = true;
  }
  return {
    players: [
      {
        playerId: 'me', name: 'Alice', life: frame.board.myLife ?? 20, handCount: 3, libraryCount: 40, battlefield: Object.fromEntries(mine),
        graveyard: Object.fromEntries((frame.board.myGrave ?? []).map((card) => [card.id, permanent(card, 'me')])),
      },
      {
        playerId: 'them', name: 'Bob', life: frame.board.theirLife ?? 20, handCount: 4, libraryCount: 41, battlefield: Object.fromEntries(theirs),
        graveyard: Object.fromEntries((frame.board.theirGrave ?? []).map((card) => [card.id, permanent(card, 'them')])),
        counters: frame.board.theirPoison ? [{ name: 'poison', count: frame.board.theirPoison }] : [],
      },
    ],
    step: frame.step as GameView['step'],
    phase: frame.step.includes('COMBAT') || frame.step.startsWith('DECLARE') ? 'COMBAT' : 'PRECOMBAT_MAIN',
    turn: 5,
    activePlayerId: 'me',
    activePlayerName: 'Alice',
    priorityPlayerName: 'Alice',
    combat,
    stack: frame.stack ?? {},
    events: frame.events?.map((event) => ({ ...event, seq: seq++ })),
  } as GameView;
}

const hit = (sourceId: string, targetId: string, amount: number): Omit<GameEventView, 'seq'> => ({ kind: 'DAMAGE', combat: true, sourceId, targetId, amount });
const died = (targetId: string, toZone = 'GRAVEYARD'): Omit<GameEventView, 'seq'> => ({ kind: 'ZONE', targetId, fromZone: 'BATTLEFIELD', toZone });
const lost = (playerId: string, amount: number): Omit<GameEventView, 'seq'> => ({ kind: 'LIFE_LOSS', playerId, targetId: playerId, amount });
const attack = (sourceId: string, targetId: string): Omit<GameEventView, 'seq'> => ({ kind: 'ATTACK', sourceId, targetId });
const block = (sourceId: string, targetId: string): Omit<GameEventView, 'seq'> => ({ kind: 'BLOCK', sourceId, targetId });

const SCENES: Record<string, () => Frame[]> = {
  'Planeswalker (dies)': () => {
    const board = { mine: [BEARS('bear')], theirs: [JACE('jace', 2)] };
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat: [{ attacker: 'bear', defender: 'jace' }], events: [attack('bear', 'jace')] },
      // the Rust host names the player once the planeswalker is gone
      { board: { mine: board.mine, theirs: [], theirGrave: board.theirs }, step: 'COMBAT_DAMAGE', combat: [{ attacker: 'bear', defender: 'them' }], events: [hit('bear', 'jace', 2), { kind: 'COUNTERS_REMOVED', targetId: 'jace', counter: 'loyalty', amount: 2 }, died('jace')] },
    ];
  },
  'Planeswalker (survives)': () => {
    const board = { mine: [GIANT('giant')], theirs: [GARRUK('garruk', 4)] };
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat: [{ attacker: 'giant', defender: 'garruk' }], events: [attack('giant', 'garruk')] },
      { board: { mine: board.mine, theirs: [GARRUK('garruk', 1)] }, step: 'COMBAT_DAMAGE', combat: [{ attacker: 'giant', defender: 'garruk' }], events: [hit('giant', 'garruk', 3), { kind: 'COUNTERS_REMOVED', targetId: 'garruk', counter: 'loyalty', amount: 3 }] },
    ];
  },
  'Double strike (two steps)': () => {
    const board = { mine: [ACE('ace')], theirs: [] };
    const combat = [{ attacker: 'ace', defender: 'them' }];
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat, events: [attack('ace', 'them')] },
      { board: { ...board, theirLife: 16 }, step: 'FIRST_COMBAT_DAMAGE', combat, events: [hit('ace', 'them', 4), lost('them', 4)], wait: 1100 },
      { board: { ...board, theirLife: 12 }, step: 'COMBAT_DAMAGE', combat, events: [hit('ace', 'them', 4), lost('them', 4)] },
    ];
  },
  'Double strike (one update)': () => {
    const board = { mine: [ACE('ace')], theirs: [] };
    const combat = [{ attacker: 'ace', defender: 'them' }];
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat, events: [attack('ace', 'them')] },
      { board: { ...board, theirLife: 12 }, step: 'COMBAT_DAMAGE', combat, events: [hit('ace', 'them', 4), lost('them', 4), hit('ace', 'them', 4), lost('them', 4)] },
    ];
  },
  'Two blockers + trample': () => {
    const board = { mine: [BRONTO('bronto')], theirs: [BEARS('bear'), ELVES('elf')] };
    const combat = [{ attacker: 'bronto', defender: 'them', blockers: ['bear', 'elf'] }];
    const theirs = board.theirs.map((card) => ({ ...card }));
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat: [{ attacker: 'bronto', defender: 'them' }], events: [attack('bronto', 'them')] },
      { board: { mine: board.mine, theirs }, step: 'DECLARE_BLOCKERS', combat, events: [block('bear', 'bronto'), block('elf', 'bronto')] },
      {
        board: { mine: board.mine, theirs: [], theirLife: 14, theirGrave: [ELVES('elf'), BEARS('bear')] }, step: 'COMBAT_DAMAGE', combat,
        events: [hit('bronto', 'bear', 2), hit('bronto', 'elf', 1), hit('bronto', 'them', 6), lost('them', 6), hit('bear', 'bronto', 2), hit('elf', 'bronto', 1), died('bear'), died('elf')],
      },
    ];
  },
  'Lifelink + deathtouch': () => {
    const board = { mine: [NIGHTHAWK('hawk')], theirs: [WURM('wurm')] };
    const combat = [{ attacker: 'hawk', defender: 'them', blockers: ['wurm'] }];
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat: [{ attacker: 'hawk', defender: 'them' }], events: [attack('hawk', 'them')] },
      { board, step: 'DECLARE_BLOCKERS', combat, events: [block('wurm', 'hawk')] },
      {
        board: { mine: [], theirs: [], myLife: 22, myGrave: board.mine, theirGrave: board.theirs }, step: 'COMBAT_DAMAGE', combat,
        events: [hit('hawk', 'wurm', 2), hit('wurm', 'hawk', 6), { kind: 'LIFE_GAIN', playerId: 'me', targetId: 'me', amount: 2 }, died('wurm'), died('hawk')],
      },
    ];
  },
  'Hit weights': () => {
    const board = { mine: [ELVES('elf'), GIANT('giant'), WURM('wurm'), BRONTO('bronto')], theirs: [] };
    const combat = board.mine.map((card) => ({ attacker: card.id, defender: 'them' }));
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat, events: combat.map((group) => attack(group.attacker, 'them')) },
      { board: { ...board, theirLife: 1 }, step: 'COMBAT_DAMAGE', combat, events: [hit('elf', 'them', 1), hit('giant', 'them', 3), hit('wurm', 'them', 6), hit('bronto', 'them', 9), lost('them', 19)] },
    ];
  },
  'Lethal': () => {
    const board = { mine: [BRONTO('bronto')], theirs: [], theirLife: 5 };
    const combat = [{ attacker: 'bronto', defender: 'them' }];
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat, events: [attack('bronto', 'them')] },
      { board: { ...board, theirLife: -4 }, step: 'COMBAT_DAMAGE', combat, events: [hit('bronto', 'them', 9), lost('them', 9)] },
    ];
  },
  'Indestructible + infect': () => {
    const board = { mine: [GIANT('giant'), STINGER('stinger')], theirs: [MYR('myr')] };
    const combat = [{ attacker: 'giant', defender: 'them', blockers: ['myr'] }, { attacker: 'stinger', defender: 'them' }];
    return [
      { board, step: 'PRECOMBAT_MAIN' },
      { board, step: 'DECLARE_ATTACKERS', combat: combat.map(({ attacker, defender }) => ({ attacker, defender })), events: [attack('giant', 'them'), attack('stinger', 'them')] },
      { board, step: 'DECLARE_BLOCKERS', combat, events: [block('myr', 'giant')] },
      {
        board: { mine: board.mine, theirs: [{ ...MYR('myr'), damage: 3 }], theirPoison: 1 }, step: 'COMBAT_DAMAGE', combat,
        events: [hit('giant', 'myr', 3), hit('stinger', 'them', 1)],
      },
    ];
  },
  'Alpha strike (8)': () => {
    const mine = Array.from({ length: 8 }, (_, i) => BEARS(`bear${i}`));
    const combat = mine.map((card) => ({ attacker: card.id, defender: 'them' }));
    return [
      { board: { mine, theirs: [] }, step: 'PRECOMBAT_MAIN' },
      { board: { mine, theirs: [] }, step: 'DECLARE_ATTACKERS', combat, events: combat.map((group) => attack(group.attacker, 'them')) },
      { board: { mine, theirs: [], theirLife: 4 }, step: 'COMBAT_DAMAGE', combat, events: [...mine.map((card) => hit(card.id, 'them', 2)), lost('them', 16)] },
    ];
  },
  'Board: land, token, counters, walker': () => {
    const bear = BEARS('bear');
    const start = { mine: [bear, AJANI('ajani', 4)], theirs: [GIANT('giant')] };
    const ability: CardView = { id: 'ability', name: 'Ajani Goldmane', mageObjectType: 'ABILITY_STACK_FROM_CARD', controllerId: 'me', targets: [], rules: ['Put a +1/+1 counter on each creature you control.'] } as CardView;
    return [
      { board: start, step: 'PRECOMBAT_MAIN' },
      // a big creature lands, a token appears
      { board: { ...start, mine: [...start.mine, DREADMAW('maw'), SOLDIER('token')] }, step: 'PRECOMBAT_MAIN', events: [{ kind: 'TOKEN', playerId: 'me', targetId: 'token' }], wait: 1600 },
      // Ajani's +1 on the stack, then resolving: loyalty up, counters onto the creatures
      { board: { ...start, mine: [bear, AJANI('ajani', 5), DREADMAW('maw'), SOLDIER('token')] }, step: 'PRECOMBAT_MAIN', stack: { ability }, events: [{ kind: 'COUNTERS_ADDED', targetId: 'ajani', counter: 'loyalty', amount: 1 }] },
      {
        board: { ...start, mine: [{ ...bear, power: '3', toughness: '3' }, AJANI('ajani', 5), { ...DREADMAW('maw'), power: '7', toughness: '7' }, { ...SOLDIER('token'), power: '2', toughness: '2' }] }, step: 'PRECOMBAT_MAIN',
        events: [{ kind: 'COUNTERS_ADDED', targetId: 'bear', counter: '+1/+1', amount: 1 }, { kind: 'COUNTERS_ADDED', targetId: 'maw', counter: '+1/+1', amount: 1 }, { kind: 'COUNTERS_ADDED', targetId: 'token', counter: '+1/+1', amount: 1 }],
        wait: 1400,
      },
      // a −1/−1 counter on theirs, then it taps and untaps
      { board: { ...start, mine: [{ ...bear, power: '3', toughness: '3' }, AJANI('ajani', 5)], theirs: [{ ...GIANT('giant'), power: '2', toughness: '2' }] }, step: 'PRECOMBAT_MAIN', events: [{ kind: 'COUNTERS_ADDED', targetId: 'giant', counter: '-1/-1', amount: 1 }], wait: 1000 },
      { board: { ...start, mine: [{ ...bear, power: '3', toughness: '3', tapped: true }, AJANI('ajani', 5)], theirs: [{ ...GIANT('giant'), power: '2', toughness: '2' }] }, step: 'PRECOMBAT_MAIN', wait: 900 },
      { board: { ...start, mine: [{ ...bear, power: '3', toughness: '3' }, AJANI('ajani', 5)], theirs: [{ ...GIANT('giant'), power: '2', toughness: '2' }] }, step: 'PRECOMBAT_MAIN' },
      // exiled
      { board: { ...start, mine: [{ ...bear, power: '3', toughness: '3' }, AJANI('ajani', 5)], theirs: [] }, step: 'PRECOMBAT_MAIN', events: [died('giant', 'EXILED')] },
    ];
  },
};

export function CombatScene() {
  const session = useMemo(() => new GameSession(api, new EventBus(NO_EVENTS), { gameId: 'dev-combat', playerId: null, mode: 'replay' }), []);
  useEffect(() => () => session.dispose(), [session]);
  const state = useStore(session.store);
  const combatFx = useSettings((store) => store.settings.combatFx);
  const update = useSettings((store) => store.update);
  const [scene, setScene] = useState(Object.keys(SCENES)[0]);
  const timers = useRef<ReturnType<typeof setTimeout>[]>([]);

  const play = (name: string) => {
    setScene(name);
    for (const timer of timers.current) clearTimeout(timer);
    timers.current = [];
    const frames = SCENES[name]();
    // a blank table first, so the scene's first board lands without diffing against the last scene
    session.showFrame(view({ board: { mine: [], theirs: [] }, step: 'PRECOMBAT_MAIN' }));
    let at = 250;
    for (const frame of frames) {
      timers.current.push(setTimeout(() => session.showFrame(view(frame)), at));
      at += frame.wait ?? 1300;
    }
  };

  useEffect(() => {
    session.setPerspective('me');
    session.showFrame(view({ board: { mine: [], theirs: [] }, step: 'PRECOMBAT_MAIN' }));
    return () => timers.current.forEach(clearTimeout);
  }, [session]);

  return (
    <div style={{ position: 'fixed', inset: 0, display: 'flex', flexDirection: 'column', background: '#0d1716' }}>
      <div data-testid="combat-scenes" style={{ position: 'absolute', top: 0, left: 340, right: 340, zIndex: 10000, background: 'rgb(13 23 22 / 0.8)', display: 'flex', flexWrap: 'wrap', gap: 6, padding: 8, color: '#eee', fontFamily: 'system-ui', fontSize: 13 }}>
        <label>
          Combat effects{' '}
          <select value={combatFx} onChange={(event) => update({ combatFx: event.target.value as 'fancy' | 'simple' })}>
            <option value="fancy">fancy</option>
            <option value="simple">plain</option>
          </select>
        </label>
        {Object.keys(SCENES).map((name) => (
          <button key={name} type="button" onClick={() => play(name)} style={{ fontWeight: name === scene ? 700 : 400 }}>{name}</button>
        ))}
      </div>
      <div style={{ position: 'relative', flex: 1, minHeight: 0 }}>
        {state.view && <MatchStage session={session} state={state} onLeave={() => undefined} />}
      </div>
    </div>
  );
}
