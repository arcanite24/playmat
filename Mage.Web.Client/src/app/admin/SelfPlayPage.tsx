import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Pause, Play } from 'lucide-react';
import { useEffect, useState, type FormEvent } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { SelfPlayHour, SelfPlaySettings, SelfPlayView } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { describeAge, formatDuration, shortImage } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const STATE_LABEL: Record<string, string> = {
  idle: 'Idle',
  busy: 'Waiting: the server is busy',
  playing: 'Playing',
  paused: 'Paused',
  stopped: 'Stopped a chunk',
  pulling: 'Pulling the image',
  error: 'Error',
};

function hourLabel(start: number): string {
  return `${String(new Date(start * 1000).getUTCHours()).padStart(2, '0')}:00`;
}

function seconds(value?: number): string {
  return value == null ? '–' : formatDuration(value * 1000);
}

/**
 * Self-play (admins): the Rust AI's self-play worker on this server's host (ops/web/host/rl-worker.sh), which plays
 * training games while the server is idle, and the trainer's progress. The worker and the server share a volume:
 * the worker writes its state every 2 minutes, and the settings saved here reach it on its next run.
 */
export function SelfPlayPage({ api }: { api: AdminApi }) {
  const status = useQuery({
    queryKey: ['admin', 'selfplay'],
    queryFn: () => api.adminSelfPlayStatus(),
    refetchInterval: 15_000,
  });
  const data: SelfPlayView | undefined = status.data;
  // ages are told against the time of the answer, so a render stays the same
  const now = status.dataUpdatedAt;
  if (status.error && !data) return <p className={styles.error}>{String(status.error)}</p>;
  if (!data) return <p className={styles.quiet}>Reading the self-play worker…</p>;
  if (!data.installed) {
    return (
      <div className={styles.overview}>
        <section aria-labelledby="selfplay-state" className={styles.block}>
          <h2 id="selfplay-state">Self-play</h2>
          <p className={styles.lede}>
            The self-play worker isn't set up on this server. It plays training games for the Rust AI while the server
            is idle: <code>ops/web/host/rl-setup.sh</code> installs it (README, Host tools), and the server needs the
            <code> xmage-selfplay</code> volume from the current compose file.
          </p>
        </section>
      </div>
    );
  }
  return (
    <div className={styles.overview}>
      <WorkerState data={data} now={now} />
      <Numbers data={data} />
      <HourChart hours={data.hours ?? []} />
      <Trainer data={data} now={now} />
      <Controls api={api} data={data} now={now} />
      <RecentChunks data={data} now={now} />
    </div>
  );
}

function WorkerState({ data, now }: { data: SelfPlayView; now: number }) {
  const state = data.state ?? '';
  const playing = data.running && state === 'playing';
  const bad = !data.running || state === 'error';
  const chip = !data.running ? 'Not running' : (STATE_LABEL[state] ?? state);
  return (
    <section aria-labelledby="selfplay-state" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="selfplay-state">Self-play</h2>
        <span className={[styles.chip, playing ? styles.chipOn : bad ? styles.chipOff : ''].join(' ')}>{chip}</span>
      </div>
      <p className={styles.lede}>
        {!data.updatedAt
          ? 'The worker hasn\'t reported yet. Its timer runs every 2 minutes.'
          : !data.running
            ? `The worker last reported ${describeAge(data.updatedAt * 1000, now)}: its timer isn't running (systemctl status playmat-rl-worker.timer).`
            : `${data.detail || chip}. Updated ${describeAge(data.updatedAt * 1000, now)}.`}
      </p>
      {data.job ? (
        <p className={styles.quiet}>
          Job <span className={styles.mono}>{data.job.id}</span>
          {data.job.tag != null && <> · round {data.job.tag}</>}
          {data.job.image && <> · image <span className={styles.mono} title={data.job.image}>{shortImage(data.job.image)}</span></>}
        </p>
      ) : (
        <p className={styles.quiet}>No job: the trainer sends one at the start of each round.</p>
      )}
    </section>
  );
}

function Numbers({ data }: { data: SelfPlayView }) {
  return (
    <section aria-label="Self-play numbers" className={styles.block}>
      <dl className={[styles.numbers, styles.numbersTight].join(' ')}>
        <div className={styles.stat}><dt>Games, last 24 h</dt><dd>{(data.games24h ?? 0).toLocaleString()}</dd></div>
        <div className={styles.stat}><dt>Games, last 7 days</dt><dd>{(data.games7d ?? 0).toLocaleString()}</dd></div>
        <div className={styles.stat}><dt>Chunks thrown away, 24 h</dt><dd>{(data.chunksStopped24h ?? 0).toLocaleString()}</dd></div>
        <div className={styles.stat}><dt>Playing, 24 h</dt><dd>{seconds(data.seconds24h)}</dd></div>
        <div className={styles.stat}>
          <dt>Waiting to be pulled</dt>
          <dd>{data.chunksWaiting ?? 0} <span className={styles.quiet}>chunks, {data.spoolMb ?? 0} MB</span></dd>
        </div>
        <div className={styles.stat}><dt>Games, all time</dt><dd>{(data.totals?.games ?? 0).toLocaleString()}</dd></div>
      </dl>
    </section>
  );
}

/** Games played each hour of the last day, with the chunks thrown away in it. */
export function HourChart({ hours }: { hours: SelfPlayHour[] }) {
  const most = Math.max(1, ...hours.map((h) => h.games ?? 0));
  const total = hours.reduce((sum, h) => sum + (h.games ?? 0), 0);
  return (
    <section aria-labelledby="selfplay-hours" className={styles.block}>
      <h2 id="selfplay-hours">The last 24 hours</h2>
      <ol className={styles.hourChart} aria-label={`Games played each hour (UTC), ${total.toLocaleString()} in all`}>
        {hours.map((h) => {
          const games = h.games ?? 0;
          const stopped = h.chunksStopped ?? 0;
          const label = `${hourLabel(h.start ?? 0)} UTC: ${games.toLocaleString()} games${stopped ? `, ${stopped} chunks thrown away` : ''}`;
          return (
            <li key={h.start} title={label} aria-label={label}>
              <span className={styles.hourBar} style={{ blockSize: `${(games / most) * 100}%` }} />
              {stopped > 0 && <span className={styles.hourStopped} aria-hidden="true" />}
            </li>
          );
        })}
      </ol>
      <p className={styles.quiet}>
        UTC hours, oldest first. A red mark is an hour where a chunk was thrown away because the server got busy.
      </p>
    </section>
  );
}

function Trainer({ data, now }: { data: SelfPlayView; now: number }) {
  const trainer = data.trainer;
  if (!trainer) {
    return (
      <section aria-labelledby="selfplay-trainer" className={styles.block}>
        <h2 id="selfplay-trainer">Training</h2>
        <p className={styles.quiet}>The trainer hasn't reported. rounds.py --remote reports each phase of a round.</p>
      </section>
    );
  }
  const rounds = trainer.history ?? [];
  return (
    <section aria-labelledby="selfplay-trainer" className={styles.block}>
      <h2 id="selfplay-trainer">Training</h2>
      <p className={styles.lede}>
        Run <span className={styles.mono}>{trainer.run}</span>, round {trainer.round}
        {trainer.rounds ? ` of ${trainer.rounds}` : ''}: {trainer.phase ?? 'unknown'}
        {trainer.at ? `, reported ${describeAge(trainer.at * 1000, now)}` : ''}.
      </p>
      <p className={styles.quiet}>
        {(trainer.localGames ?? 0).toLocaleString()} games on the trainer
        {trainer.remoteGames != null ? `, ${trainer.remoteGames.toLocaleString()} from this server` : ''} this round.
      </p>
      {rounds.length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr><th>Round</th><th>Result</th><th>Elo</th><th>From this server</th><th>Self-play</th><th>Training</th><th>Gate</th></tr>
          </thead>
          <tbody>
            {rounds.map((r) => (
              <tr key={r.round}>
                <td>{r.round}</td>
                <td>
                  <span className={[styles.chip, r.accepted ? styles.chipOn : styles.chipOff].join(' ')}>
                    {r.accepted ? 'kept' : 'not kept'}
                  </span>
                </td>
                <td className={styles.mono}>{r.elo ?? '–'}</td>
                <td>{r.remoteGames != null ? `${r.remoteGames.toLocaleString()} games` : '–'}</td>
                <td>{seconds(r.selfplaySeconds)}</td>
                <td>{seconds(r.trainSeconds)}</td>
                <td>{seconds(r.sprtSeconds)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function numberOrNull(text: string): number | null {
  const trimmed = text.trim();
  return trimmed === '' ? null : Number(trimmed);
}

function initial(data: SelfPlayView): SelfPlaySettings {
  return { ...(data.effective ?? {}), ...(data.console ?? {}) };
}

function Controls({ api, data, now }: { api: AdminApi; data: SelfPlayView; now: number }) {
  const client = useQueryClient();
  const start = initial(data);
  const paused = Boolean(data.console?.paused ?? data.effective?.paused);
  const [quiet, setQuiet] = useState(start.quietUtc ?? '');
  const [cpu, setCpu] = useState(start.cpuMax?.toString() ?? '');
  const [games, setGames] = useState(start.gamesMax?.toString() ?? '');
  const [jobs, setJobs] = useState(start.jobs?.toString() ?? '');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [saved, setSaved] = useState(false);
  const savedAt = data.console?.at;
  // a save elsewhere (another admin) shows up on the next refresh
  useEffect(() => {
    const next = initial(data);
    setQuiet(next.quietUtc ?? '');
    setCpu(next.cpuMax?.toString() ?? '');
    setGames(next.gamesMax?.toString() ?? '');
    setJobs(next.jobs?.toString() ?? '');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [savedAt]);

  async function save(pause: boolean) {
    setSaving(true);
    setError('');
    setSaved(false);
    try {
      const view = await api.adminSelfPlaySet(pause, quiet.trim(), numberOrNull(cpu), numberOrNull(games), numberOrNull(jobs));
      client.setQueryData(['admin', 'selfplay'], view);
      setSaved(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    void save(paused);
  }

  const who = data.console?.by ? ` Last saved by ${data.console.by}${savedAt ? ` ${describeAge(savedAt * 1000, now)}` : ''}.` : '';
  return (
    <section aria-labelledby="selfplay-controls" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="selfplay-controls">Settings</h2>
        <Button size="sm" variant={paused ? 'decision' : 'print'} icon={paused ? <Play size={14} /> : <Pause size={14} />}
          busy={saving} disabled={saving} onClick={() => void save(!paused)}>
          {paused ? 'Resume' : 'Pause'}
        </Button>
      </div>
      <p className={styles.quiet}>
        The worker reads these on its next run, within 2 minutes; a pause also stops a chunk under way. An empty field
        keeps the host's own setting (/etc/playmat/rl.env).{who}
      </p>
      <form onSubmit={submit}>
        <ul className={styles.settings}>
          <li className={styles.setting}>
            <label className={styles.settingText} htmlFor="selfplay-quiet">
              <span className={styles.settingLabel}>Quiet hours (UTC)</span>
              <span className={styles.quiet}>Play only in these hours, like 02-12 or 02:30-11:00. Empty: any idle hour.</span>
            </label>
            <input id="selfplay-quiet" className={[styles.settingInput, styles.settingWide].join(' ')} value={quiet}
              onChange={(e) => setQuiet(e.target.value)} placeholder="any hour" />
          </li>
          <li className={styles.setting}>
            <label className={styles.settingText} htmlFor="selfplay-cpu">
              <span className={styles.settingLabel}>Server CPU limit</span>
              <span className={styles.quiet}>Percent of one core (5 to 200) the server may use before the worker stops.</span>
            </label>
            <input id="selfplay-cpu" className={styles.settingInput} inputMode="numeric" value={cpu}
              onChange={(e) => setCpu(e.target.value)} />
          </li>
          <li className={styles.setting}>
            <label className={styles.settingText} htmlFor="selfplay-games">
              <span className={styles.settingLabel}>Open games limit</span>
              <span className={styles.quiet}>Open games (0 to 50) before the worker stops. Most open games wait on people.</span>
            </label>
            <input id="selfplay-games" className={styles.settingInput} inputMode="numeric" value={games}
              onChange={(e) => setGames(e.target.value)} />
          </li>
          <li className={styles.setting}>
            <label className={styles.settingText} htmlFor="selfplay-jobs">
              <span className={styles.settingLabel}>Cores</span>
              <span className={styles.quiet}>Cores the games run on (1 to 3), always at the lowest priority.</span>
            </label>
            <input id="selfplay-jobs" className={styles.settingInput} inputMode="numeric" value={jobs}
              onChange={(e) => setJobs(e.target.value)} />
          </li>
        </ul>
        <div className={styles.toolbar}>
          <Button type="submit" size="sm" busy={saving} disabled={saving}>Save</Button>
          {saved && !error && <span className={styles.quiet} role="status">Saved.</span>}
          {error && <span className={styles.error} role="alert">{error}</span>}
        </div>
      </form>
    </section>
  );
}

function RecentChunks({ data, now }: { data: SelfPlayView; now: number }) {
  const recent = data.recent ?? [];
  return (
    <section aria-labelledby="selfplay-recent" className={styles.block}>
      <h2 id="selfplay-recent">Last chunks</h2>
      {recent.length === 0 ? (
        <p className={styles.quiet}>None yet.</p>
      ) : (
        <table className={styles.table}>
          <thead><tr><th>When</th><th>Job</th><th>Chunk</th><th>Outcome</th><th>Games</th><th>Time</th></tr></thead>
          <tbody>
            {recent.map((c) => (
              <tr key={`${c.job}-${c.chunk}-${c.at}`}>
                <td>{describeAge((c.at ?? 0) * 1000, now)}</td>
                <td className={[styles.mono, styles.clip].join(' ')}>{c.job}</td>
                <td className={styles.mono}>{c.chunk}</td>
                <td>
                  <span className={[styles.chip, c.outcome === 'done' ? styles.chipOn : styles.chipOff].join(' ')}
                    title={c.reason ?? undefined}>
                    {c.outcome === 'done' ? 'done' : c.outcome === 'stopped' ? 'thrown away' : (c.outcome ?? '')}
                  </span>
                  {c.reason && <span className={styles.quiet}> {c.reason}</span>}
                </td>
                <td>{c.games ?? 0}</td>
                <td>{c.seconds ?? 0} s</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
