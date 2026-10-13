import { useQuery } from '@tanstack/react-query';
import { Eye, FlaskConical, X } from 'lucide-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { DeckCardLists } from '../../core/decks/types';
import { verdictLine, winRate } from '../../core/train/matchup';
import type { BatchJob, JobSpec } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useDecks } from '../stores/decks';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { DeckSelect } from './DeckSelect';
import styles from './Train.module.css';

const JOBS_KEY = ['train', 'matchups'] as const;
const GAME_COUNTS = [10, 20, 50, 100];
const SKILLS = [{ value: 2, label: 'Casual' }, { value: 4, label: 'Normal' }, { value: 6, label: 'Strong' }];
const KEPT_REPLAYS = 3;

function forServer(deck: DeckCardLists): JobSpec['deckA'] {
  return { name: deck.name, cards: deck.cards, sideboard: deck.sideboard ?? [] } as unknown as JobSpec['deckA'];
}

/**
 * The matchup lab (S017): two decks, N games between them played by the AI on the server, and a win rate with its
 * range. The closest games keep their replays.
 */
export function MatchupLab() {
  const navigate = useNavigate();
  const loadList = useDecks((state) => state.loadList);
  const [deckA, setDeckA] = useState<string | null>(null);
  const [deckB, setDeckB] = useState<string | null>(null);
  const [games, setGames] = useState(20);
  const [skill, setSkill] = useState(4);
  const [busy, setBusy] = useState(false);
  const jobs = useQuery({
    queryKey: JOBS_KEY,
    queryFn: () => api.soloBatchJobs(),
    refetchInterval: (query) => ((query.state.data ?? []).some((job) => job.status === 'queued' || job.status === 'running') ? 3000 : false),
  });

  async function submit() {
    if (!deckA || !deckB) return;
    setBusy(true);
    try {
      const [a, b] = await Promise.all([loadList(deckA), loadList(deckB)]);
      const spec: JobSpec = {
        nameA: a.name || 'Deck A',
        nameB: b.name === a.name ? `${b.name || 'Deck B'} (B)` : b.name || 'Deck B',
        deckA: forServer(a),
        deckB: forServer(b),
        games,
        skillA: skill,
        skillB: skill,
        keepReplays: KEPT_REPLAYS,
      };
      await api.soloBatchSubmit(spec, 'matchup');
      void queryClient.invalidateQueries({ queryKey: JOBS_KEY });
    } catch (failure) {
      notify('Matchup lab', failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(false);
    }
  }

  const list = (jobs.data ?? []).filter((job) => job.kind === 'matchup' || job.kind === 'versus');
  return (
    <section className={styles.tool} aria-labelledby="matchup-title">
      <header className={styles.toolHead}>
        <h2 id="matchup-title">Matchup lab</h2>
        <p>Two decks, played against each other by the AI on the server. Compare two versions of your deck, or test it against a rival, without playing a hundred games.</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect label="Deck A" value={deckA} onChange={setDeckA} />
        <DeckSelect label="Deck B" value={deckB} onChange={setDeckB} />
        <label className={styles.field}>
          <span>Games</span>
          <select value={games} onChange={(event) => setGames(Number(event.target.value))}>
            {GAME_COUNTS.map((count) => <option key={count} value={count}>{count}</option>)}
          </select>
        </label>
        <label className={styles.field}>
          <span>How well the AI plays</span>
          <select value={skill} onChange={(event) => setSkill(Number(event.target.value))}>
            {SKILLS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select>
        </label>
        <Button variant="decision" icon={<FlaskConical size={16} />} busy={busy} disabled={!deckA || !deckB} onClick={() => void submit()}>Run the games</Button>
      </div>
      <p className={styles.quiet}>Each game is seeded and the decks take turns going first. The server plays a few games at a time, so a big batch takes a while; you can leave this page.</p>

      {jobs.isError && <p className={styles.error} role="alert">Your batches couldn't be loaded.</p>}
      {list.length === 0 && jobs.data && <p className={styles.quiet}>No matchups yet.</p>}
      {list.map((job) => <JobCard key={job.id} job={job} onWatch={(id) => navigate(`/replay/${id}`)} />)}
    </section>
  );
}

function JobCard({ job, onWatch }: { job: BatchJob; onWatch(replayId: string): void }) {
  const result = job.result ?? {};
  const rate = winRate(result.winsA ?? 0, result.winsB ?? 0, result.draws ?? 0);
  const open = job.status === 'queued' || job.status === 'running';
  const played = (result.winsA ?? 0) + (result.winsB ?? 0) + (result.draws ?? 0);
  const firstRate = result.aFirst ? Math.round(((result.aFirstWins ?? 0) / result.aFirst) * 100) : null;
  const closest = (result.closest ?? []).filter((game) => !!game.replayId);
  async function cancel() {
    try {
      await api.soloBatchCancel(job.id!);
      void queryClient.invalidateQueries({ queryKey: JOBS_KEY });
    } catch (failure) {
      notify('Matchup lab', failure instanceof Error ? failure.message : String(failure));
    }
  }
  return (
    <article className={styles.result} aria-label={`${job.nameA} against ${job.nameB}`}>
      <div className={styles.controls}>
        <b>{job.nameA} vs {job.nameB}</b>
        <span className={styles.quiet}>{statusLine(job, played)}</span>
        {open && <Button size="sm" variant="quiet" icon={<X size={14} />} onClick={() => void cancel()}>Stop</Button>}
      </div>
      {played > 0 && (
        <>
          <p className={rate.low > 0.5 || rate.high < 0.5 ? styles.agree : styles.disagree}>
            <b>{job.nameA} wins {percent(rate.rate)}</b> (95% range {percent(rate.low)}–{percent(rate.high)}). {verdictLine(job.nameA ?? 'Deck A', job.nameB ?? 'Deck B', rate)}
          </p>
          <div className={styles.stats}>
            <span><b>{result.winsA ?? 0}</b>–<b>{result.winsB ?? 0}</b>{result.draws ? <> with <b>{result.draws}</b> drawn</> : null}</span>
            <span>Average game <b>{played ? ((result.turns ?? 0) / played).toFixed(1) : 0}</b> turns</span>
            {firstRate !== null && <span>{job.nameA} going first wins <b>{firstRate}%</b></span>}
            {result.errors ? <span>{result.errors} games failed</span> : null}
          </div>
        </>
      )}
      {closest.length > 0 && (
        <div className={styles.controls}>
          <span className={styles.quiet}>Closest games:</span>
          {closest.map((game) => (
            <Button key={game.replayId} size="sm" variant="quiet" icon={<Eye size={14} />} onClick={() => onWatch(game.replayId!)}>
              #{(game.n ?? 0) + 1} · {game.winner === 'draw' ? 'draw' : `${game.winner === 'A' ? job.nameA : job.nameB} won`} · {game.turns} turns
            </Button>
          ))}
        </div>
      )}
      {job.error && <p className={styles.error}>{job.error}</p>}
    </article>
  );
}

function statusLine(job: BatchJob, played: number): string {
  switch (job.status) {
    case 'queued': return `Waiting for the server · ${job.games} games`;
    case 'running': return `Playing · ${played} of ${job.games}`;
    case 'cancelled': return `Stopped after ${played} of ${job.games}`;
    case 'failed': return 'Failed';
    default: return `${played} games played`;
  }
}

function percent(value: number): string {
  return `${Math.round(value * 100)}%`;
}
