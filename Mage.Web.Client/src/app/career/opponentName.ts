/**
 * The name to put in "Victory against …": the face we found, or the result's opponent when it's a name. A mode's game
 * names its opponent by the mode and run ("limited:<run id>"), which is never something to print (#145).
 */
export function opponentName(found: string | undefined, opponent: string | undefined): string | null {
  if (found) return found;
  return opponent && !opponent.includes(':') ? opponent : null;
}
