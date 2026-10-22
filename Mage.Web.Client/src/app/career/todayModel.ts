import type { CareerDaily, CareerLevelReward, CareerLevelTrack, CareerProfile } from '../../protocol/generated/views';

/** The daily wins ladder as steps: what each win pays, which are paid, and which one is next. */
export function dailySteps(daily: CareerDaily | undefined): { coins: number; paid: boolean; next: boolean }[] {
  const ladder = daily?.winCoins ?? [];
  const wins = daily?.wins ?? 0;
  return ladder.map((coins, index) => ({ coins, paid: index < wins, next: index === wins }));
}

/** The next level's reward, or null at the top of the track. */
export function nextLevelReward(track: CareerLevelTrack | undefined): CareerLevelReward | null {
  const level = track?.level ?? 1;
  return track?.rewards?.find((reward) => (reward.level ?? 0) === level + 1) ?? null;
}

/** Where opening packs goes: the shop's pack reveal, or the starter pick before a Career collection exists. */
export function packsPath(profile: CareerProfile | undefined): string {
  return profile?.collection ? '/career/shop' : '/career';
}
