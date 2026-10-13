import { useQuery } from '@tanstack/react-query';
import { Clapperboard, Trophy } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { MatchView, ReplayInfo } from '../../protocol/generated/views';
import { formatRules } from '../../core/decks/formats';
import { matchPlayers, matchWinner } from '../../core/history/matchRecord';
import { describeResult, replayWinner } from '../../core/replay/timeline';
import { api } from '../connection';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Zone } from '../ui/Zone';
import styles from './HistoryScreen.module.css';

function when(time: number | undefined): string {
  return time ? new Date(time).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' }) : '';
}


/** Finished matches on this server, with their replays, and every saved replay. */
export function HistoryScreen() {
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
        <h1 className={styles.title}>History</h1>
        <div className={styles.filter} role="radiogroup" aria-label="Show">
          <button type="button" role="radio" aria-checked={mineOnly} onClick={() => setMineOnly(true)}>Mine</button>
          <button type="button" role="radio" aria-checked={!mineOnly} onClick={() => setMineOnly(false)}>Everyone</button>
        </div>
      </header>
      <div className={styles.columns}>
        <Zone label="Finished matches" className={styles.zone}>
          {matches.isPending && <p className={styles.empty}>Loading…</p>}
          {!matches.isPending && shownMatches.length === 0 && (
            <p className={styles.empty}>
              {mineOnly ? 'No finished matches of yours since the server last started.' : 'No matches have finished since the server last started.'}
            </p>
          )}
          <ul className={styles.list}>
            {shownMatches.map((match) => (
              <MatchRow key={match.matchId} match={match} me={me} replayIds={replayIds} onReplay={(gameId) => navigate(`/replay/${gameId}`)} />
            ))}
          </ul>
        </Zone>
        <Zone label="Replays" className={styles.zone}>
          {replays.isPending && <p className={styles.empty}>Loading…</p>}
          {replays.isError && <p className={styles.empty}>This server doesn't keep replays.</p>}
          {replays.isSuccess && shownReplays.length === 0 && (
            <p className={styles.empty}>
              {mineOnly ? 'Your games are recorded when they finish; they show up here.' : 'No replays saved on this server yet.'}
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
  const games = match.games ?? [];
  const won = matchWinner(match.result) === me;
  return (
    <li className={styles.row}>
      <div className={styles.main}>
        <p className={styles.name}>
          {won && <Trophy size={14} className={styles.trophy} aria-label="You won" />}
          {matchPlayers(match.players).join(' vs ') || match.matchName}
        </p>
        <p className={styles.meta}>
          {[formatRules(match.deckType ?? '').label, match.gameType, match.rated ? 'Rated' : '', when(match.endTime)].filter(Boolean).join(' · ')}
        </p>
        {match.result && <p className={styles.result}>{match.result}</p>}
      </div>
      <div className={styles.actions}>
        {games.map((gameId, index) => (replayIds.has(gameId)
          ? (
            <Button key={gameId} size="sm" variant="quiet" icon={<Clapperboard size={15} />} onClick={() => onReplay(gameId)}>
              {games.length > 1 ? `Game ${index + 1}` : 'Replay'}
            </Button>
          )
          : null))}
      </div>
    </li>
  );
}

function ReplayRow({ replay, me, onWatch, onReview }: { replay: ReplayInfo; me: string; onWatch(): void; onReview(): void }) {
  const played = (replay.players ?? []).some((player) => player.name === me);
  const names = (replay.players ?? []).map((player) => player.name ?? '').filter(Boolean);
  const minutes = replay.startedAt && replay.endedAt ? Math.max(1, Math.round((replay.endedAt - replay.startedAt) / 60_000)) : null;
  return (
    <li className={styles.row}>
      <div className={styles.main}>
        <p className={styles.name}>
          {replayWinner(replay.result) === me && <Trophy size={14} className={styles.trophy} aria-label="You won" />}
          {names.join(' vs ')}
        </p>
        <p className={styles.meta}>
          {[
            formatRules(replay.deckType ?? '').label,
            replay.turns ? `${replay.turns} ${replay.turns === 1 ? 'turn' : 'turns'}` : '',
            minutes ? `${minutes} min` : '',
            when(replay.endedAt ?? replay.startedAt),
          ].filter(Boolean).join(' · ')}
        </p>
        <p className={styles.result}>{describeResult(replay.result)}</p>
      </div>
      <div className={styles.actions}>
        {played && <Button size="sm" variant="quiet" onClick={onReview}>Review</Button>}
        <Button size="sm" icon={<Clapperboard size={15} />} onClick={onWatch}>Watch</Button>
      </div>
    </li>
  );
}
