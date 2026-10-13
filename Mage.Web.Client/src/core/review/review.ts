/**
 * Post-game review (S012): read a recorded game turn by turn and point at the moments that decided it — the turning
 * point, lethal left on the table, mana left unspent with a play in hand, and blocks that threw a creature away. Pure
 * over replay frames, so it runs in the browser on the replay the server already keeps; each finding names the frame
 * to watch it from.
 */
import type { CardView, GameView, PermanentView, PlayerView } from '../../protocol/generated/views';

export interface ReviewFrame {
  view: GameView;
  myHand: Record<string, CardView> | null;
}

export type FindingKind = 'turningPoint' | 'missedLethal' | 'unspentMana' | 'badBlock';

export interface Finding {
  kind: FindingKind;
  turn: number;
  /** the frame to watch it from */
  frame: number;
  title: string;
  detail: string;
  /** bigger is worse: how much the moment cost */
  weight: number;
}

export interface Review {
  me: string;
  turns: number;
  findings: Finding[];
  /** the position after each turn, from your side: positive is ahead */
  scores: { turn: number; score: number }[];
}

interface TurnSpan {
  turn: number;
  active: string | null;
  first: number;
  last: number;
}

/** frames grouped by turn, in order */
export function turnSpans(frames: readonly ReviewFrame[]): TurnSpan[] {
  const spans: TurnSpan[] = [];
  frames.forEach((frame, index) => {
    const turn = frame.view.turn ?? 0;
    const current = spans[spans.length - 1];
    if (current && current.turn === turn) {
      current.last = index;
    } else {
      spans.push({ turn, active: frame.view.activePlayerName ?? null, first: index, last: index });
    }
  });
  return spans.filter((span) => span.turn > 0);
}

function player(view: GameView, name: string): PlayerView | undefined {
  return view.players?.find((candidate) => candidate.name === name);
}

function opponents(view: GameView, me: string): PlayerView[] {
  return (view.players ?? []).filter((candidate) => candidate.name !== me && !candidate.hasLeft);
}

function permanents(seat: PlayerView | undefined): PermanentView[] {
  return Object.values(seat?.battlefield ?? {});
}

const has = (card: CardView, type: string) => (card.cardTypes ?? []).includes(type as never);
const isCreature = (card: CardView) => has(card, 'CREATURE');
const isLand = (card: CardView) => has(card, 'LAND');
const number = (value: string | undefined) => {
  const parsed = Number.parseInt(value ?? '', 10);
  return Number.isFinite(parsed) ? parsed : 0;
};

function boardValue(seat: PlayerView | undefined): number {
  return permanents(seat).filter(isCreature).reduce((sum, card) => sum + Math.max(0, number(card.power)) + Math.max(0, number(card.toughness)) / 2, 0);
}

/** how the position stands for `me`: life, board and cards against the strongest opponent */
export function positionScore(view: GameView, me: string): number {
  const mine = player(view, me);
  if (!mine) return 0;
  const theirs = opponents(view, me);
  if (theirs.length === 0) return 0;
  const strongest = theirs.reduce((best, seat) => (score(seat) > score(best) ? seat : best), theirs[0]);
  return score(mine) - score(strongest);
}

function score(seat: PlayerView): number {
  const lands = permanents(seat).filter(isLand).length;
  return (seat.life ?? 0) + 2 * boardValue(seat) + 1.5 * (seat.handCount ?? 0) + lands;
}

/** what my creatures could swing for, and how many of them a blocker per untapped creature would stop */
function lethalOnBoard(view: GameView, me: string): { damage: number; target: PlayerView } | null {
  const mine = permanents(player(view, me)).filter((card) => isCreature(card) && !card.tapped && !card.summoningSickness);
  if (mine.length === 0) return null;
  for (const target of opponents(view, me)) {
    const life = target.life ?? 0;
    if (life <= 0) continue;
    const blockers = permanents(target).filter((card) => isCreature(card) && !card.tapped).length;
    const powers = mine.map((card) => Math.max(0, number(card.power))).sort((a, b) => b - a);
    // each blocker stops one attacker, the biggest first
    const through = powers.slice(blockers).reduce((sum, power) => sum + power, 0);
    if (through >= life) return { damage: through, target };
  }
  return null;
}

/** the frame of a turn where attackers are about to be chosen; else its last frame before combat damage */
function combatStart(frames: readonly ReviewFrame[], span: TurnSpan): number {
  for (let index = span.first; index <= span.last; index++) {
    const step = frames[index].view.step;
    if (step === 'BEGIN_COMBAT' || step === 'DECLARE_ATTACKERS') return index;
  }
  for (let index = span.first; index <= span.last; index++) {
    if (frames[index].view.step === 'PRECOMBAT_MAIN') {
      let last = index;
      while (last + 1 <= span.last && frames[last + 1].view.step === 'PRECOMBAT_MAIN') last++;
      return last;
    }
  }
  return span.first;
}

function cardName(card: CardView | undefined): string {
  return card?.displayName ?? card?.name ?? 'a creature';
}

export function reviewGame(frames: readonly ReviewFrame[], me: string): Review {
  const spans = turnSpans(frames);
  const findings: Finding[] = [];
  const scores: { turn: number; score: number }[] = [];

  for (const span of spans) {
    const end = frames[span.last].view;
    scores.push({ turn: span.turn, score: positionScore(end, me) });
    const mine = span.active === me;

    if (mine) {
      // lethal on the board at the start of combat, and the opponent lived through the turn
      const at = combatStart(frames, span);
      const lethal = lethalOnBoard(frames[at].view, me);
      const survivor = lethal ? player(end, lethal.target.name ?? '') : undefined;
      if (lethal && survivor && (survivor.life ?? 0) > 0 && !survivor.hasLeft) {
        findings.push({
          kind: 'missedLethal', turn: span.turn, frame: at, weight: 100,
          title: `Lethal on turn ${span.turn}`,
          detail: `Your untapped creatures had ${lethal.damage} damage past their blockers and ${lethal.target.name} was at ${lethal.target.life}. Check the attack you made.`,
        });
      }

      // mana left over at the end of the turn with a sorcery-speed play in hand
      const lastFrame = frames[span.last];
      const untapped = permanents(player(lastFrame.view, me)).filter((card) => isLand(card) && !card.tapped).length;
      const hand = Object.values(lastFrame.myHand ?? {});
      const playable = hand
        .filter((card) => !isLand(card) && !has(card, 'INSTANT') && (card.manaValue ?? 0) > 0 && (card.manaValue ?? 0) <= untapped)
        .sort((a, b) => (b.manaValue ?? 0) - (a.manaValue ?? 0));
      if (untapped >= 2 && playable.length > 0) {
        const card = playable[0];
        findings.push({
          kind: 'unspentMana', turn: span.turn, frame: span.last, weight: 10 + 5 * (card.manaValue ?? 0),
          title: `${untapped} mana unspent on turn ${span.turn}`,
          detail: `You ended the turn with ${untapped} lands untapped and ${cardName(card)} (${card.manaValue}) in hand.`,
        });
      }
    } else {
      // blocks on their turn: a blocker that died while its attacker lived, when the damage could have been taken
      const blocks = new Map<string, { blocker: CardView; attacker: CardView; frame: number }>();
      for (let index = span.first; index <= span.last; index++) {
        for (const group of frames[index].view.combat ?? []) {
          for (const [id, blocker] of Object.entries(group.blockers ?? {})) {
            const attacker = Object.values(group.attackers ?? {})[0];
            if (attacker && ownedBy(frames[index].view, me, id)) blocks.set(id, { blocker, attacker, frame: index });
          }
        }
      }
      const lifeBefore = player(frames[span.first].view, me)?.life ?? 0;
      for (const [id, block] of blocks) {
        const blockerGone = !hasPermanent(end, me, id);
        const attackerAlive = (end.players ?? []).some((seat) => !!seat.battlefield?.[block.attacker.id ?? '']);
        const hit = Math.max(0, number(block.attacker.power));
        if (blockerGone && attackerAlive && lifeBefore - hit > 8) {
          findings.push({
            kind: 'badBlock', turn: span.turn, frame: block.frame, weight: 15 + 3 * number(block.blocker.power),
            title: `A lost blocker on turn ${span.turn}`,
            detail: `${cardName(block.blocker)} blocked ${cardName(block.attacker)} and died without killing it, while you were at ${lifeBefore} life and could take ${hit}.`,
          });
        }
      }
    }
  }

  // the turning point: the biggest swing against you, in a game that ended badly for you or swung hard
  let worst: { turn: number; drop: number; index: number } | null = null;
  for (let i = 1; i < scores.length; i++) {
    const drop = scores[i - 1].score - scores[i].score;
    if (!worst || drop > worst.drop) worst = { turn: scores[i].turn, drop, index: i };
  }
  if (worst && worst.drop >= 8) {
    const span = spans[worst.index];
    const before = scores[worst.index - 1].score;
    const after = scores[worst.index].score;
    findings.push({
      kind: 'turningPoint', turn: worst.turn, frame: span.first, weight: 50 + worst.drop,
      title: `Turn ${worst.turn} turned the game`,
      detail: `${standing(before)} going in, ${standing(after)} after it${span.active === me ? ', on your own turn' : `, on ${span.active}'s turn`}. ${swingDetail(frames, spans[worst.index - 1], span, me)}`,
    });
  }

  // the worst moments first, a few of each kind
  const perKind = new Map<FindingKind, number>();
  const kept = findings
    .sort((a, b) => b.weight - a.weight || a.turn - b.turn)
    .filter((finding) => {
      const count = perKind.get(finding.kind) ?? 0;
      perKind.set(finding.kind, count + 1);
      return count < 3;
    });
  return { me, turns: spans.length, findings: kept, scores };
}

function standing(score: number): string {
  if (score >= 4) return 'You were ahead';
  if (score <= -4) return 'you were behind';
  return 'it was even';
}

function swingDetail(frames: readonly ReviewFrame[], before: TurnSpan, after: TurnSpan, me: string): string {
  const start = frames[before.last].view;
  const end = frames[after.last].view;
  const parts: string[] = [];
  const myLife = (player(end, me)?.life ?? 0) - (player(start, me)?.life ?? 0);
  if (myLife < 0) parts.push(`you lost ${-myLife} life`);
  const lost = permanents(player(start, me)).filter((card) => isCreature(card) && !hasPermanent(end, me, card.id ?? ''));
  if (lost.length > 0) parts.push(`you lost ${lost.map(cardName).join(', ')}`);
  const theirsBefore = opponents(start, me).flatMap((seat) => permanents(seat).filter(isCreature)).length;
  const theirsAfter = opponents(end, me).flatMap((seat) => permanents(seat).filter(isCreature)).length;
  if (theirsAfter > theirsBefore) parts.push(`they added ${theirsAfter - theirsBefore} creature${theirsAfter - theirsBefore === 1 ? '' : 's'}`);
  if (parts.length === 0) return '';
  const text = parts.join(', and ');
  return `In that turn ${text}.`;
}

function hasPermanent(view: GameView, me: string, id: string): boolean {
  return !!player(view, me)?.battlefield?.[id];
}

function ownedBy(view: GameView, me: string, id: string): boolean {
  return hasPermanent(view, me, id);
}

