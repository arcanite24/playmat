import { useQuery } from '@tanstack/react-query';
import { Clapperboard, Trophy } from 'lucide-react';
import { Fragment, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { MatchView, ReplayInfo } from '../../protocol/generated/views';
import { formatRules } from '../../core/decks/formats';
import { matchPlayers, matchWinner } from '../../core/history/matchRecord';
import { replayWinner } from '../../core/replay/timeline';
import { api } from '../connection';
import { formatDate, registerMessages, useT, type Translate } from '../i18n';
import messages from '../i18n/en/history';
import { NameLevel } from '../social/LevelBadge';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Zone } from '../ui/Zone';
import styles from './HistoryScreen.module.css';

registerMessages(messages);

/** "Ana Lv 12 vs Ben Lv 3": the players, each with their level when the server keeps one. */
function Players({ names }: { names: readonly string[] }) {
  const t = useT();
  return (
    <span className={styles.players}>
      {names.map((name, index) => (
        <Fragment key={`${index}-${name}`}>
          {index > 0 && ` ${t('history.vs')} `}
          {name}
          <NameLevel name={name} />
        </Fragment>
      ))}
    </span>
  );
}

/** in the app's language, not the browser's */
function when(time: number | undefined): string {
  return time ? formatDate(time, { dateStyle: 'medium', timeStyle: 'short' }) : '';
}

/** A replay's result (core/replay/timeline describeResult, in the player's language). */
function resultText(t: Translate, result: string | null | undefined): string {
  const winner = replayWinner(result);
  if (winner) return t('history.won', { name: winner });
  if (/draw/i.test(result ?? '')) return t('history.draw');
  return result ? result : t('history.unfinished');
}


/** Finished matches on this server, with their replays, and every saved replay. */
export function HistoryScreen() {
  const t = useT();
  const navigate = useNavigate();
  const roomId = useSession((state) => state.roomId);
  const me = useSession((state) => state.userName);
  const connected = useSession((state) => state.connection === 'open');
  const [mineOnly, setMineOnly] = useState(true);

  const matches = useQuery({
    queryKey: ['finishedMatches', roomId],
    queryFn: () => api.roomGetFinishedMatches(roomId!),
    enabled: !!roomId && connected,
    refetchInterval: 30_000,
  });
  const replays = useQuery({
    queryKey: ['replays'],
    queryFn: () => api.replayList(),
    enabled: connected,
    refetchInterval: 30_000,
  });

  const replayIds = useMemo(() => new Set((replays.data ?? []).map((replay) => replay.gameId)), [replays.data]);
  const shownMatches = useMemo(
    () => [...(matches.data ?? [])]
      .filter((match) => !mineOnly || matchPlayers(match.players).includes(me))
      .sort((a, b) => (b.endTime ?? 0) - (a.endTime ?? 0)),
    [matches.data, mineOnly, me],
  );
  const shownReplays = useMemo(
    () => (replays.data ?? []).filter((replay) => !mineOnly || replay.players?.some((player) => player.name === me)),
    [replays.data, mineOnly, me],
  );

  return (
    <div className={styles.page}>
      <header className={styles.head}>
        <h1 className={styles.title}>{t('history.title')}</h1>
        <div className={styles.filter} role="radiogroup" aria-label={t('history.show')}>
          <button type="button" role="radio" aria-checked={mineOnly} onClick={() => setMineOnly(true)}>{t('history.mine')}</button>
          <button type="button" role="radio" aria-checked={!mineOnly} onClick={() => setMineOnly(false)}>{t('history.everyone')}</button>
        </div>
      </header>
      <div className={styles.columns}>
        <Zone label={t('history.matches')} className={styles.zone}>
          {matches.isPending && <p className={styles.empty}>{t('history.loading')}</p>}
          {!matches.isPending && shownMatches.length === 0 && (
            <p className={styles.empty}>
              {t(mineOnly ? 'history.matches.noneMine' : 'history.matches.none')}
            </p>
          )}
          <ul className={styles.list}>
            {shownMatches.map((match) => (
              <MatchRow key={match.matchId} match={match} me={me} replayIds={replayIds} onReplay={(gameId) => navigate(`/replay/${gameId}`)} />
            ))}
          </ul>
        </Zone>
        <Zone label={t('history.replays')} className={styles.zone}>
          {replays.isPending && <p className={styles.empty}>{t('history.loading')}</p>}
          {replays.isError && <p className={styles.empty}>{t('history.replays.off')}</p>}
          {replays.isSuccess && shownReplays.length === 0 && (
            <p className={styles.empty}>
              {t(mineOnly ? 'history.replays.noneMine' : 'history.replays.none')}
            </p>
          )}
          <ul className={styles.list}>
            {shownReplays.map((replay) => (
              <ReplayRow key={replay.gameId} replay={replay} me={me} onWatch={() => navigate(`/replay/${replay.gameId}`)} onReview={() => navigate(`/review/${replay.gameId}`)} />
            ))}
          </ul>
        </Zone>
      </div>
    </div>
  );
}

function MatchRow({ match, me, replayIds, onReplay }: { match: MatchView; me: string; replayIds: Set<string | undefined>; onReplay(gameId: string): void }) {
  const t = useT();
  const games = match.games ?? [];
  const won = matchWinner(match.result) === me;
  const players = matchPlayers(match.players);
  return (
    <li className={styles.row}>
      <div className={styles.main}>
        <p className={styles.name}>
          {won && <Trophy size={14} className={styles.trophy} aria-label={t('history.youWon')} />}
          {players.length > 0 ? <Players names={players} /> : match.matchName}
        </p>
        <p className={styles.meta}>
          {[formatRules(match.deckType ?? '').label, match.gameType, match.rated ? t('history.rated') : '', when(match.endTime)].filter(Boolean).join(' · ')}
        </p>
        {match.result && <p className={styles.result}>{match.result}</p>}
      </div>
      <div className={styles.actions}>
        {games.map((gameId, index) => (replayIds.has(gameId)
          ? (
            <Button key={gameId} size="sm" variant="quiet" icon={<Clapperboard size={15} />} onClick={() => onReplay(gameId)}>
              {games.length > 1 ? t('history.game', { number: index + 1 }) : t('history.replay')}
            </Button>
          )
          : null))}
      </div>
    </li>
  );
}

function ReplayRow({ replay, me, onWatch, onReview }: { replay: ReplayInfo; me: string; onWatch(): void; onReview(): void }) {
  const t = useT();
  const played = (replay.players ?? []).some((player) => player.name === me);
  const names = (replay.players ?? []).map((player) => player.name ?? '').filter(Boolean);
  const minutes = replay.startedAt && replay.endedAt ? Math.max(1, Math.round((replay.endedAt - replay.startedAt) / 60_000)) : null;
  return (
    <li className={styles.row}>
      <div className={styles.main}>
        <p className={styles.name}>
          {replayWinner(replay.result) === me && <Trophy size={14} className={styles.trophy} aria-label={t('history.youWon')} />}
          <Players names={names} />
        </p>
        <p className={styles.meta}>
          {[
            formatRules(replay.deckType ?? '').label,
            replay.turns ? t('history.turns', { count: replay.turns }) : '',
            minutes ? t('history.minutes', { count: minutes }) : '',
            when(replay.endedAt ?? replay.startedAt),
          ].filter(Boolean).join(' · ')}
        </p>
        <p className={styles.result}>{resultText(t, replay.result)}</p>
      </div>
      <div className={styles.actions}>
        {played && <Button size="sm" variant="quiet" onClick={onReview}>{t('history.review')}</Button>}
        <Button size="sm" icon={<Clapperboard size={15} />} onClick={onWatch}>{t('history.watch')}</Button>
      </div>
    </li>
  );
}
