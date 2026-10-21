import type { PermanentView } from '../../protocol/generated/views';

/**
 * Automatic crewing, decided on the client: the server asks which creatures tap to crew a Vehicle one at a time
 * ("Select another untapped creature you control (selected power 0 of 3)"). When exactly one way to crew is worth
 * considering, the client picks it for the player; any real choice stays theirs.
 */

/** The power a crew prompt asks for and what is chosen so far, or null when the prompt is not a crew choice. */
export function crewNeed(text: string): { selected: number; need: number } | null {
  const match = /selected power (-?\d+) of (\d+)/i.exec(text);
  return match ? { selected: Number(match[1]), need: Number(match[2]) } : null;
}

export interface CrewCandidate {
  id: string;
  power: number;
  /** creatures that differ in nothing a player would weigh (two Soldier tokens) share a kind */
  kind: string;
}

/** A candidate from a permanent, or null when it crews by other rules than its power (toughness, +2). */
export function crewCandidate(permanent: PermanentView): CrewCandidate | null {
  const rules = (permanent.rules ?? []).join(' ').replace(/<[^>]*>/g, '');
  if (/\bcrews?\b[^.]*\bas though\b/i.test(rules)) return null;
  const power = Number(permanent.power ?? 0);
  const kind = JSON.stringify([
    permanent.name,
    permanent.power,
    permanent.toughness,
    !!permanent.summoningSickness,
    permanent.damage ?? 0,
    (permanent.counters ?? []).map((counter) => [counter.name, counter.count]),
    (permanent.attachments ?? []).length,
    permanent.rules ?? [],
  ]);
  return { id: permanent.id!, power: Number.isFinite(power) ? power : 0, kind };
}

const MAX_CANDIDATES = 14;

/**
 * The creatures to tap, when there is a single way to crew worth considering: the only inclusion-minimal group whose
 * power reaches `need` (no creature in it is spare), counting interchangeable creatures as one choice. Null when
 * there are several, none, or too many creatures to tell.
 */
export function autoCrewPick(need: number, candidates: CrewCandidate[]): string[] | null {
  const useful = candidates.filter((candidate) => candidate.power > 0);
  if (need <= 0 || useful.length === 0 || useful.length > MAX_CANDIDATES) return null;
  let pick: CrewCandidate[] | null = null;
  let pickKinds: string | null = null;
  for (let mask = 1; mask < 1 << useful.length; mask++) {
    const group = useful.filter((_, index) => mask & (1 << index));
    const total = group.reduce((sum, candidate) => sum + candidate.power, 0);
    if (total < need || group.some((candidate) => total - candidate.power >= need)) continue;
    const kinds = group.map((candidate) => candidate.kind).sort().join('\n');
    if (pickKinds === null) {
      pick = group;
      pickKinds = kinds;
    } else if (kinds !== pickKinds) {
      return null;
    }
  }
  return pick?.map((candidate) => candidate.id) ?? null;
}
