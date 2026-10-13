/**
 * The mulligan trainer (S015): keep or ship an opening hand, see what the table would have done, then goldfish the
 * kept hand for five turns. Pure: dealing takes a random source, and the verdict is a rule of thumb any player can
 * check — lands, early plays and colors, weighed against the kind of deck across the table.
 */
import { MANA_LETTERS, type ManaLetter } from '../decks/analysis';
import { canPay, pickLand, type SimCard } from '../decks/simulate';

export type Opposition = 'aggro' | 'midrange' | 'control';

export interface Verdict {
  keep: boolean;
  reason: string;
}

/** whether lands in a hand can make a spell's colors at all (not how many) */
function colorsCovered(lands: readonly SimCard[], spell: SimCard): boolean {
  return MANA_LETTERS.every((letter) => spell.pips[letter] <= 0 || lands.some((land) => land.colors.includes(letter)));
}

/**
 * The table's verdict on a hand: keep two to five lands of seven (fewer allowed after mulligans), at least one play
 * by turn three (two against aggro), and the colors for some of the spells.
 */
export function verdict(hand: readonly SimCard[], against: Opposition = 'midrange', mulligans = 0): Verdict {
  const size = hand.length;
  const lands = hand.filter((card) => card.land);
  const spells = hand.filter((card) => !card.land);
  const early = spells.filter((card) => card.value <= 3);
  const castable = spells.filter((card) => colorsCovered(lands, card));
  // a smaller hand keeps looser: shipping again costs another card
  const minLands = size <= 5 ? 1 : 2;
  const maxLands = size >= 7 ? 5 : size - 1;
  if (mulligans >= 2) {
    return { keep: true, reason: 'At five cards, keep almost anything: another mulligan costs more than a weak hand.' };
  }
  if (lands.length < minLands) {
    return { keep: false, reason: `${lands.length === 0 ? 'No lands' : 'One land'} in ${size} cards: most games, this hand doesn't cast its spells in time.` };
  }
  if (lands.length > maxLands) {
    return { keep: false, reason: `${lands.length} lands and ${spells.length} spells: too little to do with the mana.` };
  }
  const neededEarly = against === 'aggro' ? 2 : 1;
  if (early.length < neededEarly && against !== 'control') {
    return { keep: false, reason: against === 'aggro'
      ? 'Against aggro you need two plays by turn three, and this hand has fewer.'
      : 'Nothing to cast before turn four: this hand falls behind.' };
  }
  if (spells.length > 0 && castable.length === 0) {
    return { keep: false, reason: 'The lands make none of the colors your spells need.' };
  }
  if (lands.length === 2 && size === 7 && early.length === 0) {
    return { keep: false, reason: 'Two lands and only expensive spells: it needs to draw lands fast.' };
  }
  if (against === 'control' && early.length === 0 && lands.length >= 3) {
    return { keep: true, reason: 'Slow, but against control there\'s time, and the lands are there.' };
  }
  return { keep: true, reason: `${lands.length} lands, ${early.length} play${early.length === 1 ? '' : 's'} by turn three${castable.length < spells.length ? ', some colors still to find' : ''}: a fine keep.` };
}

export interface GoldfishTurn {
  turn: number;
  drew: string | null;
  land: string | null;
  cast: string[];
  lands: number;
}

export interface Goldfish {
  turns: GoldfishTurn[];
  /** a land drop missed in the first four turns */
  stalled: boolean;
  /** mana spent over the turns: how much of the hand got used */
  manaSpent: number;
}

/**
 * The kept hand played alone: a draw a turn (none on turn one on the play), a land, then the biggest spells the
 * mana pays for.
 */
export function goldfish(hand: readonly SimCard[], library: readonly SimCard[], onThePlay = true, turns = 5): Goldfish {
  const inHand = [...hand];
  let next = 0;
  const inPlay: ManaLetter[][] = [];
  const result: GoldfishTurn[] = [];
  let stalled = false;
  let manaSpent = 0;
  for (let turn = 1; turn <= turns; turn++) {
    let drew: string | null = null;
    if ((turn > 1 || !onThePlay) && next < library.length) {
      const card = library[next++];
      inHand.push(card);
      drew = card.name;
    }
    let land: string | null = null;
    const landAt = pickLand(inHand, inPlay);
    if (landAt >= 0) {
      land = inHand[landAt].name;
      inPlay.push(inHand[landAt].colors);
      inHand.splice(landAt, 1);
    } else if (turn <= 4) {
      stalled = true;
    }
    // spend the mana: the biggest castable spell first, while lands remain
    const cast: string[] = [];
    let available = [...inPlay];
    for (;;) {
      const options = inHand
        .map((card, index) => ({ card, index }))
        .filter(({ card }) => !card.land && card.value > 0 && canPay(available, card.value, card.pips))
        .sort((a, b) => b.card.value - a.card.value);
      if (options.length === 0) break;
      const { card, index } = options[0];
      cast.push(card.name);
      manaSpent += card.value;
      inHand.splice(index, 1);
      available = spend(available, card);
    }
    result.push({ turn, drew, land, cast, lands: inPlay.length });
  }
  return { turns: result, stalled, manaSpent };
}

/** the lands left after paying for a card: colored pips from lands that make them, the rest from any */
function spend(lands: ManaLetter[][], card: SimCard): ManaLetter[][] {
  const left = [...lands];
  for (const letter of MANA_LETTERS) {
    for (let i = 0; i < Math.ceil(card.pips[letter] - 1e-9); i++) {
      const at = left.findIndex((land) => land.includes(letter));
      if (at >= 0) left.splice(at, 1);
    }
  }
  const generic = Math.max(0, card.value - MANA_LETTERS.reduce((sum, letter) => sum + Math.ceil(card.pips[letter] - 1e-9), 0));
  // the generic part from the lands making the fewest colors
  left.sort((a, b) => a.length - b.length);
  return left.slice(generic);
}

export interface TrainerStats {
  hands: number;
  keeps: number;
  /** decisions that matched the table's verdict */
  agreed: number;
  /** kept hands that missed a land drop in the first four turns */
  stalls: number;
}

export const EMPTY_STATS: TrainerStats = { hands: 0, keeps: 0, agreed: 0, stalls: 0 };

export function record(stats: TrainerStats, kept: boolean, table: Verdict, stalled: boolean | null): TrainerStats {
  return {
    hands: stats.hands + 1,
    keeps: stats.keeps + (kept ? 1 : 0),
    agreed: stats.agreed + (kept === table.keep ? 1 : 0),
    stalls: stats.stalls + (kept && stalled ? 1 : 0),
  };
}
