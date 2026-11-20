import { useState } from 'react';
import type { LadderState, LeaderboardEntry } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { ScoreBoard } from './ScoreBoard';
import modes from './CareerModes.module.css';

const KINDS: { mode: string; title: MessageKey }[] = [
  { mode: 'ladder-peak', title: 'career.boards.peak' },
  { mode: 'ladder-mythic', title: 'career.boards.mythic' },
  { mode: 'ladder-climb', title: 'career.boards.climb' },
];

/**
 * The ladder's season boards (#434) for one format and queue: peak rank, Mythic rating, fastest to Mythic, for this
 * season or a past one, on the server or among friends.
 */
export function SeasonBoards({ state }: { state: LadderState }) {
  const t = useT();
  const current = state.season ?? 0;
  const [kind, setKind] = useState(KINDS[0]);
  const [picked, setPicked] = useState<number | null>(null);
  const season = picked ?? current;
  const board = `${state.queue ?? 'open'}.${state.format ?? 'standard'}.${season}`;
  const line = (entry: LeaderboardEntry) => {
    if (kind.mode === 'ladder-peak') return [entry.detail, t('career.boards.games', { count: entry.v2 ?? 0 })].filter(Boolean).join(' · ');
    if (kind.mode === 'ladder-mythic') return String(entry.v1 ?? 0);
    return t('career.boards.games', { count: entry.v1 ?? 0 });
  };
  const seasons = Array.from({ length: current + 1 }, (_, index) => current - index);

  return (
    <div className={modes.seasonBoards}>
      <div className={modes.boardHead}>
        <div className={modes.boardScope} role="radiogroup" aria-label={t('career.boards.kind')}>
          {KINDS.map((item) => (
            <button key={item.mode} type="button" role="radio" aria-checked={item === kind} onClick={() => setKind(item)}>
              {t(item.title)}
            </button>
          ))}
        </div>
        <select
          className={modes.seasonPick}
          aria-label={t('career.boards.season')}
          value={season}
          onChange={(event) => setPicked(Number(event.target.value))}
        >
          {seasons.map((item) => <option key={item} value={item}>{t('career.season.name', { n: item + 1 })}</option>)}
        </select>
      </div>
      <ScoreBoard key={`${kind.mode}-${board}`} mode={kind.mode} board={board} title={t(kind.title)} format={line} />
    </div>
  );
}
