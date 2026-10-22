/**
 * The deck simulator (S016): deal the deck many times and play it out alone, a land a turn, to answer what the
 * hypergeometric tables can't: lands in play by turn, how often there's a spell to cast on curve with the right
 * colors, and the turn each card first shows up. Pure: the random source is a parameter, so tests can seed it.
 */
import { BASIC_FOR, costPips, isLand, MANA_LETTERS, shuffle, type AnalysisCard, type AnalysisEntry, type ManaLetter } from './analysis';

export interface SimulationOptions {
  runs?: number;
  /** turns played out (1 to 10) */
  turns?: number;
  onThePlay?: boolean;
  random?: () => number;
}

export interface TurnFigures {
  turn: number;
  /** average lands in play at the end of the turn */
  averageLands: number;
  /** share of games that made every land drop so far */
  allDrops: number;
  /** share of games with a spell of exactly this mana value in hand and the lands and colors to cast it */
  onCurve: number;
  /** share of games with any spell castable this turn */
  anyPlay: number;
}

export interface CardFigures {
  name: string;
  copies: number;
  /** median turn the first copy is in hand; null when it doesn't show up in half the games within the turns played */
  medianTurn: number | null;
  /** share of games with a copy in hand by turn 3 and by the last turn played */
  byTurn3: number;
  byLastTurn: number;
}

export interface SimulationResult {
  runs: number;
  turns: number;
  onThePlay: boolean;
  byTurn: TurnFigures[];
  cards: CardFigures[];
  /** share of games stuck on two lands or fewer on turn 4 */
  screwed: number;
  /** share of games holding three lands or more in hand on turn 6 */
  flooded: number;
}

/** The colors a land makes: the basics by name, others from the "Add {X}" and "any color" in their rules. */
export function landColors(name: string, card: AnalysisCard | undefined): ManaLetter[] {
  for (const letter of MANA_LETTERS) {
    if (BASIC_FOR[letter] === name) return [letter];
  }
  const rules = (card?.rules ?? []).join(' ');
  if (/any color|any one color|mana of any type/i.test(rules)) return [...MANA_LETTERS];
  const found = new Set<ManaLetter>();
  for (const match of rules.matchAll(/add[^.]*?\./gi)) {
    for (const symbol of match[0].match(/\{[^}]+\}/g) ?? []) {
      for (const letter of MANA_LETTERS) {
        if (symbol.toUpperCase().includes(letter)) found.add(letter);
      }
    }
  }
  return [...found];
}

export interface SimCard {
  name: string;
  land: boolean;
  value: number;
  pips: Record<ManaLetter, number>;
  colors: ManaLetter[];
}

/** whether lands in play pay a cost: enough of them, and each colored pip by a land that makes it */
export function canPay(lands: readonly ManaLetter[][], value: number, pips: Record<ManaLetter, number>): boolean {
  if (lands.length < value) return false;
  const needs: ManaLetter[] = [];
  for (const letter of MANA_LETTERS) {
    for (let i = 0; i < Math.ceil(pips[letter] - 1e-9); i++) needs.push(letter);
  }
  if (needs.length === 0) return true;
  // match pips to lands, the scarcest colors first (a small greedy is exact enough for a deck view)
  const used = new Array<boolean>(lands.length).fill(false);
  const order = [...needs].sort((a, b) => sources(lands, a) - sources(lands, b));
  for (const letter of order) {
    let pick = -1;
    for (let i = 0; i < lands.length; i++) {
      if (used[i] || !lands[i].includes(letter)) continue;
      // prefer the land that makes the fewest other colors
      if (pick < 0 || lands[i].length < lands[pick].length) pick = i;
    }
    if (pick < 0) return false;
    used[pick] = true;
  }
  return true;
}

function sources(lands: readonly ManaLetter[][], letter: ManaLetter): number {
  return lands.filter((land) => land.includes(letter)).length;
}

/** a card as the simulator plays it */
export function simCard(name: string, card: AnalysisCard | undefined): SimCard {
  const land = isLand(card) || Object.values(BASIC_FOR).includes(name) || name === 'Wastes';
  return { name, land, value: card?.manaValue ?? 0, pips: costPips(card), colors: land ? landColors(name, card) : [] };
}

export function simulateDeck(
  entries: readonly AnalysisEntry[],
  infoOf: (entry: AnalysisEntry) => AnalysisCard | undefined,
  options: SimulationOptions = {},
): SimulationResult {
  const runs = Math.max(1, Math.min(10_000, options.runs ?? 1000));
  const turns = Math.max(1, Math.min(10, options.turns ?? 7));
  const onThePlay = options.onThePlay ?? true;
  const random = options.random ?? Math.random;

  const library: SimCard[] = [];
  const copies = new Map<string, number>();
  for (const entry of entries) {
    const sim = simCard(entry.cardName, infoOf(entry));
    for (let i = 0; i < entry.amount; i++) library.push(sim);
    if (!sim.land) copies.set(entry.cardName, (copies.get(entry.cardName) ?? 0) + entry.amount);
  }

  const landSum = new Array<number>(turns).fill(0);
  const allDrops = new Array<number>(turns).fill(0);
  const onCurve = new Array<number>(turns).fill(0);
  const anyPlay = new Array<number>(turns).fill(0);
  const firstSeen = new Map<string, number[]>();
  for (const name of copies.keys()) firstSeen.set(name, []);
  let screwed = 0;
  let flooded = 0;

  for (let run = 0; run < runs && library.length > 0; run++) {
    const order = shuffle(library, random);
    const hand = order.slice(0, 7);
    let next = 7;
    const inPlay: ManaLetter[][] = [];
    const seen = new Map<string, number>();
    const note = (card: SimCard, turn: number) => {
      if (!card.land && !seen.has(card.name)) seen.set(card.name, turn);
    };
    for (const card of hand) note(card, 1);
    let missed = false;
    for (let turn = 1; turn <= turns; turn++) {
      if ((turn > 1 || !onThePlay) && next < order.length) {
        const drawn = order[next++];
        hand.push(drawn);
        note(drawn, turn);
      }
      // play a land: the one adding a color the hand needs most, else any
      const landAt = pickLand(hand, inPlay);
      if (landAt >= 0) {
        inPlay.push(hand[landAt].colors);
        hand.splice(landAt, 1);
      } else {
        missed = true;
      }
      const t = turn - 1;
      landSum[t] += inPlay.length;
      if (!missed) allDrops[t]++;
      const spells = hand.filter((card) => !card.land);
      if (spells.some((card) => card.value === turn && canPay(inPlay, card.value, card.pips))) onCurve[t]++;
      if (spells.some((card) => canPay(inPlay, card.value, card.pips))) anyPlay[t]++;
      if (turn === 4 && inPlay.length <= 2) screwed++;
      if (turn === 6 && hand.filter((card) => card.land).length >= 3) flooded++;
    }
    for (const [name, turn] of seen) firstSeen.get(name)?.push(turn);
  }

  const share = (count: number) => count / runs;
  const byTurn: TurnFigures[] = landSum.map((sum, index) => ({
    turn: index + 1,
    averageLands: sum / runs,
    allDrops: share(allDrops[index]),
    onCurve: share(onCurve[index]),
    anyPlay: share(anyPlay[index]),
  }));
  const cards: CardFigures[] = [...copies.entries()].map(([name, count]) => {
    const turnsSeen = (firstSeen.get(name) ?? []).sort((a, b) => a - b);
    const half = Math.ceil(runs / 2);
    return {
      name,
      copies: count,
      medianTurn: turnsSeen.length >= half ? turnsSeen[half - 1] : null,
      byTurn3: share(turnsSeen.filter((turn) => turn <= 3).length),
      byLastTurn: share(turnsSeen.length),
    };
  }).sort((a, b) => b.copies - a.copies || a.name.localeCompare(b.name));

  return { runs, turns, onThePlay, byTurn, cards, screwed: share(screwed), flooded: share(flooded) };
}

/** which land in hand to play: one making a color the spells in hand need and the lands in play lack, else the first */
export function pickLand(hand: readonly SimCard[], inPlay: readonly ManaLetter[][]): number {
  let best = -1;
  let bestScore = -1;
  const wanted = new Set<ManaLetter>();
  for (const card of hand) {
    if (card.land) continue;
    for (const letter of MANA_LETTERS) {
      if (card.pips[letter] > sources(inPlay, letter)) wanted.add(letter);
    }
  }
  hand.forEach((card, index) => {
    if (!card.land) return;
    const score = card.colors.filter((letter) => wanted.has(letter)).length;
    if (score > bestScore) {
      best = index;
      bestScore = score;
    }
  });
  return best;
}
