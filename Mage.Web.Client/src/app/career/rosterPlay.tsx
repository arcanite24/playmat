import type { CareerOpponent } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { playCareer } from './careerData';
import { DECK_MIN } from './careerModel';
import { useAdaptive } from './careerModesData';
import { phaseLines } from './ladderModel';
import type { useCareerDeck } from './useCareerDeck';
import { Versus } from './Versus';

/**
 * The versus screen before a roster duel; Fight starts the table with the Career deck. A rival says their chapter's
 * line, a nemesis its taunt; a boss's phases and a nemesis's bounty are among the stakes.
 */
export function RosterVersus({ opponent, deck, onClose }: {
  opponent: CareerOpponent;
  deck: ReturnType<typeof useCareerDeck>;
  onClose(): void;
}) {
  const t = useT();
  const adaptive = useAdaptive((state) => state.on);
  const tier = t('career.tier', { tier: opponent.tier ?? 1, name: opponent.tierName ?? '' });
  const line = opponent.rivalBefore || (opponent.nemesis ? opponent.lines?.nemesis : undefined) || opponent.lines?.intro;
  const stakes = [
    t('career.versus.pays', { coins: opponent.winCoins ?? 0 }),
    ...(opponent.bounty ? [t('career.nemesis.bounty', { coins: opponent.bounty })] : []),
    t('career.versus.record', { count: opponent.wins ?? 0 }),
    t('career.ladder.pips', { pips: opponent.pips ?? 1 }),
    ...(opponent.raid ? [t('career.ladder.raid')] : []),
    ...phaseLines(opponent.twists).map((phase) => t(phase.key, phase.vars)),
    ...(adaptive ? [t('career.adaptive.on')] : []),
  ];
  return (
    <Versus
      kicker={opponent.rivalChapter ? `${t('career.rival.chapter', { chapter: opponent.rivalChapter })} · ${tier}` : tier}
      opponent={{ name: opponent.name ?? '', tagline: opponent.tagline, colors: opponent.colors, cover: opponent.cover, line, boss: !!opponent.raid }}
      deck={{ name: deck.deck?.name ?? t('career.nav.deck'), art: deck.cover, colors: deck.colors }}
      stakes={stakes}
      onFight={async () => {
        if (!deck.deck) throw new Error(t('career.deck.notReady', { min: DECK_MIN, count: 0 }));
        await playCareer(opponent.id!, deck.deck, opponent.lines ?? null, adaptive);
      }}
      onClose={onClose}
    />
  );
}
