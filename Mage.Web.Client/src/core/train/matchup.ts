/**
 * The matchup lab's numbers (S017): a win rate from N games is a guess, so it comes with a 95% range (the Wilson
 * score interval, which behaves at small N and near 0% or 100%). Draws count as half a win.
 */
export interface WinRate {
  games: number;
  rate: number;
  low: number;
  high: number;
}

export function winRate(wins: number, losses: number, draws = 0, z = 1.96): WinRate {
  const games = wins + losses + draws;
  if (games <= 0) return { games: 0, rate: 0, low: 0, high: 1 };
  const p = (wins + draws / 2) / games;
  const z2 = z * z;
  const centre = (p + z2 / (2 * games)) / (1 + z2 / games);
  const half = (z * Math.sqrt((p * (1 - p)) / games + z2 / (4 * games * games))) / (1 + z2 / games);
  return { games, rate: p, low: Math.max(0, centre - half), high: Math.min(1, centre + half) };
}

/** a plain reading of a result: who's favoured, and whether the games settle it */
export function verdictLine(nameA: string, nameB: string, rate: WinRate): string {
  if (rate.games === 0) return 'No games yet.';
  if (rate.low > 0.5) return `${nameA} is favoured: even the low end of the range is above 50%.`;
  if (rate.high < 0.5) return `${nameB} is favoured: even the high end of the range is below 50%.`;
  return `Too close to call from ${rate.games} game${rate.games === 1 ? '' : 's'}: the range spans 50%. More games narrow it.`;
}
