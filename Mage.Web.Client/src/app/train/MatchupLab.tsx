import { useQuery } from '@tanstack/react-query';
import { Eye, FlaskConical, X } from 'lucide-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { DeckCardLists } from '../../core/decks/types';
import { winRate, type WinRate } from '../../core/train/matchup';
import type { BatchJob, JobSpec } from '../../protocol/generated/views';
import { api } from '../connection';
import { formatNumber, registerMessages, useT, type Translate } from '../i18n';
import messages from '../i18n/en/career';
import { queryClient, useServerState } from '../queries';
import { offeredAiTypes } from '../screens/aiSetup';
import type { AiType } from '../stores/play';
import { useDecks } from '../stores/decks';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { DeckSelect } from './DeckSelect';
import styles from './Train.module.css';

const JOBS_KEY = ['train', 'matchups'] as const;
const GAME_COUNTS = [10, 20, 50, 100];
const SKILLS = [
  { value: 2, label: 'train.lab.skill.casual' },
  { value: 4, label: 'train.lab.skill.normal' },
  { value: 6, label: 'train.lab.skill.strong' },
] as const;
const KEPT_REPLAYS = 3;

registerMessages(messages);

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
  // each side's AI (S044), and keeping the lethal positions the AI finds as puzzles (S031)
  const [aiA, setAiA] = useState<AiType>('COMPUTER_MAD');
  const [aiB, setAiB] = useState<AiType>('COMPUTER_MAD');
  const [minePuzzles, setMinePuzzles] = useState(false);
  const t = useT();
  const aiTypes = offeredAiTypes(useServerState().data?.playerTypes);
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
        nameA: a.name || t('train.lab.deckA'),
        nameB: b.name === a.name ? `${b.name || t('train.lab.deckB')} (B)` : b.name || t('train.lab.deckB'),
        deckA: forServer(a),
        deckB: forServer(b),
        games,
        skillA: skill,
        skillB: skill,
        aiA,
        aiB,
        minePuzzles,
        keepReplays: KEPT_REPLAYS,
      };
      await api.soloBatchSubmit(spec, 'matchup');
      void queryClient.invalidateQueries({ queryKey: JOBS_KEY });
    } catch (failure) {
      notify(t('train.lab.title'), failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(false);
    }
  }

  const list = (jobs.data ?? []).filter((job) => job.kind === 'matchup' || job.kind === 'versus');
  return (
    <section className={styles.tool} aria-labelledby="matchup-title">
      <header className={styles.toolHead}>
        <h2 id="matchup-title">{t('train.lab.title')}</h2>
        <p>{t('train.lab.lead')}</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect label={t('train.lab.deckA')} value={deckA} onChange={setDeckA} />
        <DeckSelect label={t('train.lab.deckB')} value={deckB} onChange={setDeckB} />
        <label className={styles.field}>
          <span>{t('train.lab.games')}</span>
          <select value={games} onChange={(event) => setGames(Number(event.target.value))}>
            {GAME_COUNTS.map((count) => <option key={count} value={count}>{count}</option>)}
          </select>
        </label>
        <label className={styles.field}>
          <span>{t('train.lab.skill')}</span>
          <select value={skill} onChange={(event) => setSkill(Number(event.target.value))}>
            {SKILLS.map((option) => <option key={option.value} value={option.value}>{t(option.label)}</option>)}
          </select>
        </label>
        {aiTypes.length > 1 && (
          <>
            <label className={styles.field}>
              <span>{t('solo.batch.aiA')}</span>
              <select value={aiA} onChange={(event) => setAiA(event.target.value as AiType)}>
                {aiTypes.map((type) => <option key={type.value} value={type.value}>{t(type.label)}</option>)}
              </select>
            </label>
            <label className={styles.field}>
              <span>{t('solo.batch.aiB')}</span>
              <select value={aiB} onChange={(event) => setAiB(event.target.value as AiType)}>
                {aiTypes.map((type) => <option key={type.value} value={type.value}>{t(type.label)}</option>)}
              </select>
            </label>
          </>
        )}
        <label className={styles.check} title={t('solo.batch.mineHint')}>
          <input type="checkbox" checked={minePuzzles} onChange={(event) => setMinePuzzles(event.target.checked)} />
          {t('solo.batch.mine')}
        </label>
        <Button variant="decision" icon={<FlaskConical size={16} />} busy={busy} disabled={!deckA || !deckB} onClick={() => void submit()}>{t('train.lab.run')}</Button>
      </div>
      <p className={styles.quiet}>{t('train.lab.note')}</p>

      {jobs.isError && <p className={styles.error} role="alert">{t('train.lab.loadFailed')}</p>}
      {list.length === 0 && jobs.data && <p className={styles.quiet}>{t('train.lab.none')}</p>}
      {list.map((job) => <JobCard key={job.id} job={job} onWatch={(id) => navigate(`/replay/${id}`)} />)}
    </section>
  );
}

function JobCard({ job, onWatch }: { job: BatchJob; onWatch(replayId: string): void }) {
  const t = useT();
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
      notify(t('train.lab.title'), failure instanceof Error ? failure.message : String(failure));
    }
  }
  return (
    <article className={styles.result} aria-label={t('train.lab.against', { a: job.nameA ?? '', b: job.nameB ?? '' })}>
      <div className={styles.controls}>
        <b>{t('train.lab.vs', { a: job.nameA ?? '', b: job.nameB ?? '' })}</b>
        <span className={styles.quiet}>{statusLine(t, job, played)}</span>
        {open && <Button size="sm" variant="quiet" icon={<X size={14} />} onClick={() => void cancel()}>{t('train.lab.stop')}</Button>}
      </div>
      {played > 0 && (
        <>
          <p className={rate.low > 0.5 || rate.high < 0.5 ? styles.agree : styles.disagree}>
            <b>{t('train.lab.wins', { name: job.nameA ?? '', percent: percent(rate.rate) })}</b> {t('train.lab.range', { low: percent(rate.low), high: percent(rate.high) })} {verdictText(t, job.nameA ?? t('train.lab.deckA'), job.nameB ?? t('train.lab.deckB'), rate)}
          </p>
          <div className={styles.stats}>
            <span><b>{result.winsA ?? 0}</b>–<b>{result.winsB ?? 0}</b>{result.draws ? <> {t('train.lab.drawn', { count: result.draws })}</> : null}</span>
            <span>{t('train.lab.average', { turns: formatNumber(played ? (result.turns ?? 0) / played : 0, { maximumFractionDigits: 1, minimumFractionDigits: played ? 1 : 0 }) })}</span>
            {firstRate !== null && <span>{t('train.lab.first', { name: job.nameA ?? '', percent: percent(firstRate / 100) })}</span>}
            {result.errors ? <span>{t('train.lab.errors', { count: result.errors })}</span> : null}
            {result.mined ? <span>{t('solo.batch.mined', { count: result.mined })}</span> : null}
          </div>
        </>
      )}
      {closest.length > 0 && (
        <div className={styles.controls}>
          <span className={styles.quiet}>{t('train.lab.closest')}</span>
          {closest.map((game) => (
            <Button key={game.replayId} size="sm" variant="quiet" icon={<Eye size={14} />} onClick={() => onWatch(game.replayId!)}>
              #{(game.n ?? 0) + 1} · {game.winner === 'draw' ? t('train.lab.draw') : t('train.lab.won', { name: (game.winner === 'A' ? job.nameA : job.nameB) ?? '' })} · {t('train.lab.turns', { count: game.turns ?? 0 })}
            </Button>
          ))}
        </div>
      )}
      {job.error && <p className={styles.error}>{job.error}</p>}
    </article>
  );
}

function statusLine(t: Translate, job: BatchJob, played: number): string {
  const count = job.games ?? 0;
  switch (job.status) {
    case 'queued': return t('train.lab.status.queued', { count });
    case 'running': return t('train.lab.status.running', { played, count });
    case 'cancelled': return t('train.lab.status.cancelled', { played, count });
    case 'failed': return t('train.lab.status.failed');
    default: return t('train.lab.status.done', { count: played });
  }
}

/** The matchup's conclusion (core/train/matchup verdictLine, in the player's language). */
function verdictText(t: Translate, nameA: string, nameB: string, rate: WinRate): string {
  if (rate.games === 0) return t('train.lab.verdict.none');
  if (rate.low > 0.5) return t('train.lab.verdict.a', { name: nameA });
  if (rate.high < 0.5) return t('train.lab.verdict.b', { name: nameB });
  return t('train.lab.verdict.close', { count: rate.games });
}

function percent(value: number): string {
  return formatNumber(value, { style: 'percent', maximumFractionDigits: 0 });
}
