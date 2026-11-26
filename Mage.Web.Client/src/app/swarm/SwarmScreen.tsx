import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Check, Copy } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { APP_NAME } from '../brand';
import {
  DEFAULT_SWARM_URL,
  formatCredit,
  formatUnixAge,
  roundProgress,
  swarmPublic,
  type GateStatus,
  type LeaderboardWindow,
  type NetworkInfo,
  type PublicRound,
  type SwarmStatus,
  type TrustLevel,
  type WatchGame,
} from './swarmApi';
import styles from './SwarmScreen.module.css';

type Section = 'overview' | 'projects' | 'networks' | 'watch' | 'leaderboard' | 'transparency' | 'contribute';

const NAV_ITEMS: { id: Section; label: string; to: string }[] = [
  { id: 'overview', label: 'Overview', to: '/swarm' },
  { id: 'projects', label: 'Projects', to: '/swarm/projects' },
  { id: 'networks', label: 'Networks & Gates', to: '/swarm/networks' },
  { id: 'watch', label: 'Watch', to: '/swarm/watch' },
  { id: 'leaderboard', label: 'Leaderboards', to: '/swarm/leaderboard' },
  { id: 'transparency', label: 'Transparency', to: '/swarm/transparency' },
  { id: 'contribute', label: 'Contribute', to: '/swarm/contribute' },
];

const WINDOWS: { id: LeaderboardWindow; label: string }[] = [
  { id: '24h', label: '24h' },
  { id: '7d', label: '7 days' },
  { id: '30d', label: '30 days' },
  { id: 'all', label: 'All time' },
];

function trustChipClass(trust: TrustLevel): string {
  if (trust === 'trusted' || trust === 'reputable') return `${styles.chip} ${styles.chipTrusted}`;
  if (trust === 'banned') return `${styles.chip} ${styles.chipBad}`;
  if (trust === 'probation') return `${styles.chip} ${styles.chipWarn}`;
  return styles.chip;
}

/**
 * Public Mana@Home tracker (`/swarm`, `/swarm/:section`, `/swarm/contributors/:name`).
 * Readable without signing in; subscribes to the coordinator's SSE stream for live numbers.
 */
export function SwarmScreen() {
  const { section, name } = useParams<{ section?: string; name?: string }>();
  const queryClient = useQueryClient();
  const [sseConnected, setSseConnected] = useState(false);

  const activeSection: Section =
    section === 'projects' ||
    section === 'networks' ||
    section === 'watch' ||
    section === 'leaderboard' ||
    section === 'transparency' ||
    section === 'contribute'
      ? section
      : 'overview';

  // Subscribe to SSE live status snapshots when available in browser environments
  useEffect(() => {
    if (typeof EventSource === 'undefined') return;
    let source: EventSource | null = null;
    try {
      source = new EventSource(`${DEFAULT_SWARM_URL}/api/v1/events`);
      source.addEventListener('status', (ev) => {
        try {
          const parsed = JSON.parse((ev as MessageEvent<string>).data) as SwarmStatus;
          queryClient.setQueryData(['swarm', 'status'], parsed);
          setSseConnected(true);
        } catch {
          // ignore malformed event frames
        }
      });
      source.onerror = () => setSseConnected(false);
    } catch {
      // EventSource construction failed; fall back to React Query polling
    }
    return () => {
      source?.close();
    };
  }, [queryClient]);

  return (
    <main className={styles.page}>
      <div className={styles.shell}>
        <header className={styles.header}>
          <Link to="/" className={styles.back}>
            <ArrowLeft size={16} aria-hidden="true" /> {APP_NAME}
          </Link>
          <div className={styles.brand}>
            <h1 className={styles.title}>Mana@Home</h1>
            <p className={styles.subtitle}>Public self-play swarm tracker for Playmat AI</p>
          </div>
          <span className={styles.liveBadge} aria-live="polite">
            <span className={sseConnected ? styles.liveDot : styles.liveDotOffline} aria-hidden="true" />
            {sseConnected ? 'Live stream' : 'Polling'}
          </span>
        </header>

        <nav className={styles.nav} aria-label="Swarm sections">
          {NAV_ITEMS.map((item) => {
            const active = !name && activeSection === item.id;
            return (
              <Link
                key={item.id}
                to={item.to}
                className={active ? `${styles.navLink} ${styles.navLinkActive}` : styles.navLink}
                aria-current={active ? 'page' : undefined}
              >
                {item.label}
              </Link>
            );
          })}
        </nav>

        {name ? (
          <ContributorView account={name} />
        ) : activeSection === 'overview' ? (
          <OverviewView />
        ) : activeSection === 'projects' ? (
          <ProjectsView />
        ) : activeSection === 'networks' ? (
          <NetworksView />
        ) : activeSection === 'watch' ? (
          <WatchView />
        ) : activeSection === 'leaderboard' ? (
          <LeaderboardView />
        ) : activeSection === 'transparency' ? (
          <TransparencyView />
        ) : (
          <ContributeView />
        )}
      </div>
    </main>
  );
}

function OverviewView() {
  const statusQ = useQuery({
    queryKey: ['swarm', 'status'],
    queryFn: () => swarmPublic.status(),
    refetchInterval: 15_000,
  });
  const feedQ = useQuery({
    queryKey: ['swarm', 'feed'],
    queryFn: () => swarmPublic.feed(25),
    refetchInterval: 15_000,
  });

  if (statusQ.isError) {
    return <p className={styles.error} role="alert">Could not reach coordinator: {String(statusQ.error)}</p>;
  }
  const st = statusQ.data;
  if (!st) return <p className={styles.quiet}>Loading live swarm status...</p>;

  const platforms = Object.entries(st.platforms ?? {});
  const gamesPerSec = (st.games_last_hour / 3600).toFixed(2);

  return (
    <>
      <dl className={styles.kpiGrid}>
        <div className={styles.kpi}>
          <dt>Games accepted</dt>
          <dd>{st.games_accepted.toLocaleString()}</dd>
          <span className={styles.kpiNote}>{st.units.accepted ?? 0} verified units</span>
        </div>
        <div className={styles.kpi}>
          <dt>Throughput</dt>
          <dd>{st.games_last_hour.toLocaleString()}/hr</dd>
          <span className={styles.kpiNote}>{gamesPerSec} games/sec in last hour</span>
        </div>
        <div className={styles.kpi}>
          <dt>Active workers</dt>
          <dd>{st.hosts}</dd>
          <span className={styles.kpiNote}>{st.threads} CPU threads online</span>
        </div>
        <div className={styles.kpi}>
          <dt>Decisions recorded</dt>
          <dd>{st.total_decisions.toLocaleString()}</dd>
          <span className={styles.kpiNote}>{st.units.queued ?? 0} queued, {st.units.leased ?? 0} leased</span>
        </div>
      </dl>

      {st.active_round && <RoundCard round={st.active_round} heading="Active round" />}

      {platforms.length > 0 && (
        <section className={styles.card} aria-labelledby="swarm-platforms">
          <h2 id="swarm-platforms" className={styles.cardTitle}>Platform mix (last 10 min)</h2>
          <div className={styles.chips}>
            {platforms.map(([os, count]) => (
              <span key={os} className={styles.chip}>{os}: {count}</span>
            ))}
          </div>
        </section>
      )}

      <section className={styles.card} aria-labelledby="swarm-feed">
        <div className={styles.cardHead}>
          <h2 id="swarm-feed" className={styles.cardTitle}>Recently accepted units</h2>
          <span className={styles.quiet}>Replay and re-run verified</span>
        </div>
        {feedQ.isError ? (
          <p className={styles.error}>{String(feedQ.error)}</p>
        ) : !feedQ.data ? (
          <p className={styles.quiet}>Loading feed...</p>
        ) : feedQ.data.length === 0 ? (
          <p className={styles.quiet}>No accepted units yet.</p>
        ) : (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Unit</th>
                  <th>Project / round</th>
                  <th>Contributor</th>
                  <th>Host</th>
                  <th className={styles.num}>Games</th>
                  <th className={styles.num}>Credit</th>
                  <th className={styles.num}>Accepted</th>
                </tr>
              </thead>
              <tbody>
                {feedQ.data.map((item) => (
                  <tr key={item.unit}>
                    <td className={styles.mono}>{item.unit}</td>
                    <td>{item.project}/{item.run} r{item.round}</td>
                    <td><Link to={`/swarm/contributors/${encodeURIComponent(item.account)}`}>{item.account}</Link></td>
                    <td className={styles.quiet}>{item.host}</td>
                    <td className={styles.num}>{item.games}</td>
                    <td className={styles.num}>{formatCredit(item.credit)}</td>
                    <td className={styles.num}>{formatUnixAge(item.done_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  );
}

function RoundCard({ round, heading }: { round: PublicRound; heading?: string }) {
  const { done, total, pct } = roundProgress(round.units);
  return (
    <section className={styles.card}>
      <div className={styles.cardHead}>
        <h2 className={styles.cardTitle}>
          {heading ? `${heading}: ` : ''}{round.project}/{round.run} round {round.round}
        </h2>
        <div className={styles.chips}>
          <span className={styles.chip}>Priority {round.priority}</span>
          {round.trained && <span className={`${styles.chip} ${styles.chipTrusted}`}>Trained</span>}
          {round.verify_pending > 0 && (
            <span className={`${styles.chip} ${styles.chipWarn}`}>{round.verify_pending} verifying</span>
          )}
        </div>
      </div>
      <div className={styles.progressTrack} role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100} aria-label={`${round.project}/${round.run} round ${round.round} progress`}>
        <div className={styles.progressFill} style={{ width: `${pct}%` }} />
      </div>
      <div className={styles.toolbar}>
        <span className={styles.quiet}>{done} / {total} units ({pct}%) · {round.games.toLocaleString()} games</span>
        <div className={styles.chips}>
          {Object.entries(round.units).map(([state, count]) => (
            <span key={state} className={styles.chip}>{state}: {count}</span>
          ))}
        </div>
      </div>
    </section>
  );
}

function ProjectsView() {
  const projectsQ = useQuery({
    queryKey: ['swarm', 'projects'],
    queryFn: () => swarmPublic.projects(),
    refetchInterval: 20_000,
  });
  if (projectsQ.isError) return <p className={styles.error} role="alert">{String(projectsQ.error)}</p>;
  if (!projectsQ.data) return <p className={styles.quiet}>Loading projects and rounds...</p>;
  if (projectsQ.data.length === 0) return <p className={styles.quiet}>No rounds published yet.</p>;

  return (
    <>
      {projectsQ.data.map((round) => (
        <RoundCard key={round.id} round={round} />
      ))}
    </>
  );
}

function statusChipClass(status: string): string {
  if (status === 'promoted' || status === 'passed') return `${styles.chip} ${styles.chipTrusted}`;
  if (status === 'failed' || status === 'rejected' || status === 'rolled-back') return `${styles.chip} ${styles.chipBad}`;
  if (status === 'held' || status === 'concentration-hold' || status === 'running') {
    return `${styles.chip} ${styles.chipWarn}`;
  }
  return styles.chip;
}

function EloChart({ networks }: { networks: NetworkInfo[] }) {
  const rated = [...networks]
    .filter((n) => n.bench_elo !== null && n.bench_games > 0)
    .reverse();

  if (rated.length === 0) {
    return <p className={styles.quiet}>No Archbench ratings recorded yet.</p>;
  }

  const w = 680;
  const h = 190;
  const padL = 52;
  const padR = 24;
  const padT = 18;
  const padB = 34;
  const elos = rated.map((n) => n.bench_elo as number);
  const minElo = Math.floor((Math.min(...elos, 1000) - 25) / 25) * 25;
  const maxElo = Math.ceil((Math.max(...elos, 1050) + 25) / 25) * 25;
  const span = Math.max(50, maxElo - minElo);
  const xFor = (i: number) =>
    rated.length === 1 ? padL + (w - padL - padR) / 2 : padL + (i / (rated.length - 1)) * (w - padL - padR);
  const yFor = (elo: number) => padT + ((maxElo - elo) / span) * (h - padT - padB);
  const points = rated.map((n, i) => `${xFor(i).toFixed(1)},${yFor(n.bench_elo as number).toFixed(1)}`).join(' ');
  const ticks = [minElo, Math.round((minElo + maxElo) / 2), maxElo];

  return (
    <svg viewBox={`0 0 ${w} ${h}`} className={styles.chartSvg} role="img" aria-label="Archbench Elo over network generations">
      {ticks.map((t) => {
        const y = yFor(t);
        return (
          <g key={t}>
            <line x1={padL} x2={w - padR} y1={y} y2={y} stroke="var(--ink-faint)" strokeDasharray="3 3" />
            <text x={padL - 8} y={y + 4} textAnchor="end" fill="var(--ink-dim)" fontSize="11">
              {t}
            </text>
          </g>
        );
      })}
      {rated.length > 1 && (
        <polyline fill="none" stroke="var(--sage)" strokeWidth="2.5" points={points} />
      )}
      {rated.map((n, i) => {
        const cx = xFor(i);
        const cy = yFor(n.bench_elo as number);
        return (
          <g key={n.sha256}>
            <circle cx={cx} cy={cy} r="4.5" fill="var(--decision)" />
            <text x={cx} y={h - 10} textAnchor="middle" fill="var(--ink-dim)" fontSize="11">
              {n.label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function GateCard({ gate }: { gate: GateStatus }) {
  const games = gate.wins + gate.draws + gate.losses;
  const bound = Math.max(Math.abs(gate.lower_bound), Math.abs(gate.upper_bound), 1);
  const clamped = Math.max(-bound, Math.min(bound, gate.llr));
  const halfPct = (Math.abs(clamped) / bound) * 50;
  const barStyle = clamped >= 0
    ? { left: '50%', width: `${halfPct}%` }
    : { left: `${50 - halfPct}%`, width: `${halfPct}%` };

  return (
    <section className={styles.card}>
      <div className={styles.cardHead}>
        <h3 className={styles.cardTitle}>
          Gate #{gate.id}: {gate.candidate_label} vs {gate.baseline_label}
        </h3>
        <div className={styles.chips}>
          <span className={statusChipClass(gate.status)}>{gate.status}</span>
          {gate.held && <span className={`${styles.chip} ${styles.chipWarn}`}>held</span>}
          <span className={styles.chip}>
            SPRT [{gate.elo0.toFixed(0)}, {gate.elo1.toFixed(0)}]
          </span>
        </div>
      </div>

      <div
        className={styles.llrTrack}
        role="progressbar"
        aria-valuenow={Number(gate.llr.toFixed(2))}
        aria-valuemin={Number(gate.lower_bound.toFixed(2))}
        aria-valuemax={Number(gate.upper_bound.toFixed(2))}
        aria-label={`Gate ${gate.id} SPRT log-likelihood ratio`}
      >
        <div className={styles.llrMid} />
        <div
          className={clamped < 0 ? `${styles.llrBar} ${styles.llrBarNeg}` : styles.llrBar}
          style={barStyle}
        />
      </div>

      <div className={styles.toolbar}>
        <span className={styles.quiet}>
          {gate.wins}W-{gate.draws}D-{gate.losses}L ({games} games) · {gate.elo >= 0 ? '+' : ''}
          {gate.elo.toFixed(1)} +/- {gate.elo_ci95.toFixed(1)} Elo · LLR {gate.llr >= 0 ? '+' : ''}
          {gate.llr.toFixed(2)} ({gate.lower_bound.toFixed(2)} .. {gate.upper_bound.toFixed(2)})
        </span>
        <span className={styles.quiet}>
          Top account share: {(gate.top_account_share * 100).toFixed(0)}% (limit {(gate.max_share * 100).toFixed(0)}%)
        </span>
      </div>
      {gate.decision_reason && <p className={styles.quiet}>{gate.decision_reason}</p>}
    </section>
  );
}

function NetworksView() {
  const netsQ = useQuery({
    queryKey: ['swarm', 'networks'],
    queryFn: () => swarmPublic.networks(),
    refetchInterval: 20_000,
  });
  const gatesQ = useQuery({
    queryKey: ['swarm', 'gates'],
    queryFn: () => swarmPublic.gates(),
    refetchInterval: 15_000,
  });

  if (netsQ.isError) return <p className={styles.error} role="alert">{String(netsQ.error)}</p>;
  if (gatesQ.isError) return <p className={styles.error} role="alert">{String(gatesQ.error)}</p>;

  const networks = netsQ.data ?? [];
  const gates = gatesQ.data ?? [];

  return (
    <>
      <section className={styles.card} aria-labelledby="swarm-elo-chart">
        <div className={styles.cardHead}>
          <h2 id="swarm-elo-chart" className={styles.cardTitle}>Archbench strength over generations</h2>
          <span className={styles.quiet}>Anchored to host@s6 = 1000 Elo</span>
        </div>
        <p className={styles.quiet}>
          Archbench ratings against fixed non-evolving anchor opponents are the headline strength measure. Self-play
          gate Elo only compares adjacent generations and can drift.
        </p>
        {!netsQ.data ? <p className={styles.quiet}>Loading network ratings...</p> : <EloChart networks={networks} />}
      </section>

      <section className={styles.card} aria-labelledby="swarm-gates-heading">
        <div className={styles.cardHead}>
          <h2 id="swarm-gates-heading" className={styles.cardTitle}>SPRT promotion gates</h2>
          <span className={styles.quiet}>Reputable and trusted workers · cross-account concentration guard</span>
        </div>
        {!gatesQ.data ? (
          <p className={styles.quiet}>Loading gates...</p>
        ) : gates.length === 0 ? (
          <p className={styles.quiet}>No SPRT gates published yet.</p>
        ) : (
          gates.map((gate) => <GateCard key={gate.id} gate={gate} />)
        )}
      </section>

      <section className={styles.card} aria-labelledby="swarm-lineage">
        <h2 id="swarm-lineage" className={styles.cardTitle}>Network lineage</h2>
        {!netsQ.data ? (
          <p className={styles.quiet}>Loading networks...</p>
        ) : networks.length === 0 ? (
          <p className={styles.quiet}>No networks registered yet.</p>
        ) : (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Network</th>
                  <th>Status</th>
                  <th>Gate</th>
                  <th className={styles.num}>Archbench Elo</th>
                  <th>Anchors</th>
                  <th>Weights</th>
                </tr>
              </thead>
              <tbody>
                {networks.map((n) => (
                  <tr key={n.sha256}>
                    <td>
                      <div><strong>{n.label}</strong></div>
                      <div className={`${styles.mono} ${styles.quiet}`}>{n.sha256.slice(0, 12)}</div>
                    </td>
                    <td><span className={statusChipClass(n.status)}>{n.status}</span></td>
                    <td className={styles.quiet}>
                      {n.gate_id !== null
                        ? `#${n.gate_id} ${n.gate_status ?? ''} (${n.gate_elo !== null ? `${n.gate_elo >= 0 ? '+' : ''}${n.gate_elo.toFixed(1)} Elo` : '-'})`
                        : '-'}
                    </td>
                    <td className={styles.num}>
                      {n.bench_elo !== null && n.bench_games > 0
                        ? `${Math.round(n.bench_elo)} +/- ${Math.round(n.bench_ci95 ?? 0)} (${n.bench_games}g)`
                        : 'Unrated'}
                    </td>
                    <td className={styles.quiet}>
                      {n.anchors.length === 0
                        ? '-'
                        : n.anchors.map((a) => `${a.opponent}: ${a.wins}-${a.draws}-${a.losses}`).join(' · ')}
                    </td>
                    <td>
                      <a href={n.download_url} target="_blank" rel="noreferrer">.pmnet</a>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  );
}

function describeAnswerToken(tok: string): string {
  if (tok.length < 2) return tok;
  const seat = `P${tok[0]}`;
  const kind = tok[1];
  const rest = tok.slice(2);
  if (kind === 'm') return `${seat} mulligan (${rest === '1' ? 'keep' : 'mull'})`;
  if (kind === 'b') return `${seat} bottom card #${rest}`;
  if (kind === 'A') return `${seat} action ${rest}`;
  if (kind === 'O') return `${seat} target/object ${rest}`;
  if (kind === 'B') return `${seat} blockers ${rest || 'none'}`;
  if (kind === 'K') return `${seat} attackers ${rest || 'none'}`;
  if (kind === 'Y') return `${seat} choice (${rest === '1' ? 'yes' : 'no'})`;
  return `${seat} ${kind}${rest}`;
}

function WatchView() {
  const [selected, setSelected] = useState<WatchGame | null>(null);
  const [step, setStep] = useState(0);

  const watchQ = useQuery({
    queryKey: ['swarm', 'watch'],
    queryFn: () => swarmPublic.watch(24),
    refetchInterval: 15_000,
  });
  const replayQ = useQuery({
    queryKey: ['swarm', 'watchReplay', selected?.unit, selected?.deal],
    queryFn: () => swarmPublic.watchReplay(selected!.unit, selected!.deal),
    enabled: selected !== null,
  });

  if (watchQ.isError) return <p className={styles.error} role="alert">{String(watchQ.error)}</p>;
  const games = watchQ.data ?? [];

  function openGame(g: WatchGame) {
    setSelected(g);
    setStep(0);
  }

  const tokens = replayQ.data?.tokens ?? [];
  const activeStep = tokens.length > 0 ? Math.min(step, tokens.length - 1) : 0;

  return (
    <>
      {selected && (
        <section className={styles.card} aria-labelledby="swarm-watch-replay">
          <div className={styles.cardHead}>
            <h2 id="swarm-watch-replay" className={styles.cardTitle}>
              Replay: {selected.unit} · Deal #{selected.deal}
            </h2>
            <button type="button" className={styles.copyBtn} onClick={() => setSelected(null)}>
              Close
            </button>
          </div>
          <p className={styles.quiet}>
            {selected.project}/{selected.run} r{selected.round} ({selected.kind}) · Learner seat P
            {selected.learner_seat} vs {selected.opponent} · {selected.turns} turns · {selected.answers} answers ·{' '}
            {selected.winner === null ? 'Draw' : `Winner: P${selected.winner}`}
          </p>
          {replayQ.isPending ? (
            <p className={styles.quiet}>Loading answer log...</p>
          ) : replayQ.isError ? (
            <p className={styles.error}>{String(replayQ.error)}</p>
          ) : tokens.length === 0 ? (
            <p className={styles.quiet}>{replayQ.data?.note || 'No answer-log preview available.'}</p>
          ) : (
            <>
              <div className={styles.toolbar}>
                <div className={styles.chips}>
                  <button
                    type="button"
                    className={styles.copyBtn}
                    disabled={activeStep <= 0}
                    onClick={() => setStep((s) => Math.max(0, s - 1))}
                  >
                    Previous
                  </button>
                  <button
                    type="button"
                    className={styles.copyBtn}
                    disabled={activeStep >= tokens.length - 1}
                    onClick={() => setStep((s) => Math.min(tokens.length - 1, s + 1))}
                  >
                    Next
                  </button>
                  <span className={styles.chip}>
                    Step {activeStep + 1} / {tokens.length}
                  </span>
                </div>
                <strong className={styles.mono}>{describeAnswerToken(tokens[activeStep])}</strong>
              </div>
              <input
                type="range"
                aria-label="Replay step"
                min={0}
                max={Math.max(0, tokens.length - 1)}
                value={activeStep}
                onChange={(e) => setStep(Number(e.target.value))}
              />
              <div className={styles.codeBlock}>
                <pre>{tokens.slice(Math.max(0, activeStep - 8), activeStep + 12).join(' ')}</pre>
              </div>
            </>
          )}
        </section>
      )}

      <section className={styles.card} aria-labelledby="swarm-watch-list">
        <div className={styles.cardHead}>
          <h2 id="swarm-watch-list" className={styles.cardTitle}>Live swarm games</h2>
          <span className={styles.quiet}>Recent verified games across self-play, SPRT gates, and Archbench</span>
        </div>
        {!watchQ.data ? (
          <p className={styles.quiet}>Loading recent games...</p>
        ) : games.length === 0 ? (
          <p className={styles.quiet}>No games available to watch yet.</p>
        ) : (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Unit / Deal</th>
                  <th>Kind</th>
                  <th>Opponent</th>
                  <th>Result</th>
                  <th className={styles.num}>Turns</th>
                  <th className={styles.num}>Answers</th>
                  <th>Contributor</th>
                  <th>Replay</th>
                </tr>
              </thead>
              <tbody>
                {games.map((g) => (
                  <tr key={`${g.unit}-${g.deal}`}>
                    <td className={styles.mono}>{g.unit} #{g.deal}</td>
                    <td><span className={styles.chip}>{g.kind}</span></td>
                    <td className={styles.quiet}>{g.opponent}</td>
                    <td>
                      {g.winner === null
                        ? 'Draw'
                        : g.winner === g.learner_seat
                          ? 'Learner win'
                          : 'Opponent win'}
                    </td>
                    <td className={styles.num}>{g.turns}</td>
                    <td className={styles.num}>{g.answers}</td>
                    <td><Link to={`/swarm/contributors/${encodeURIComponent(g.account)}`}>{g.account}</Link></td>
                    <td>
                      <button
                        type="button"
                        className={styles.copyBtn}
                        onClick={() => openGame(g)}
                        aria-label={`Inspect deal ${g.deal} from ${g.unit}`}
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  );
}

function LeaderboardView() {
  const [window_, setWindow] = useState<LeaderboardWindow>('all');
  const [mode, setMode] = useState<'accounts' | 'teams'>('accounts');

  const accountsQ = useQuery({
    queryKey: ['swarm', 'leaderboard', window_],
    queryFn: () => swarmPublic.leaderboard(window_),
    enabled: mode === 'accounts',
    refetchInterval: 30_000,
  });
  const teamsQ = useQuery({
    queryKey: ['swarm', 'teams', window_],
    queryFn: () => swarmPublic.teams(window_),
    enabled: mode === 'teams',
    refetchInterval: 30_000,
  });

  return (
    <section className={styles.card} aria-labelledby="swarm-lb-title">
      <div className={styles.toolbar}>
        <h2 id="swarm-lb-title" className={styles.cardTitle}>
          {mode === 'accounts' ? 'Contributor leaderboard' : 'Team leaderboard'}
        </h2>
        <div className={styles.chips}>
          <div className={styles.segGroup} role="group" aria-label="Leaderboard view">
            <button
              type="button"
              className={mode === 'accounts' ? `${styles.segBtn} ${styles.segBtnActive}` : styles.segBtn}
              onClick={() => setMode('accounts')}
            >
              Contributors
            </button>
            <button
              type="button"
              className={mode === 'teams' ? `${styles.segBtn} ${styles.segBtnActive}` : styles.segBtn}
              onClick={() => setMode('teams')}
            >
              Teams
            </button>
          </div>
          <div className={styles.segGroup} role="group" aria-label="Time window">
            {WINDOWS.map((w) => (
              <button
                key={w.id}
                type="button"
                className={window_ === w.id ? `${styles.segBtn} ${styles.segBtnActive}` : styles.segBtn}
                onClick={() => setWindow(w.id)}
              >
                {w.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {mode === 'accounts' ? (
        accountsQ.isError ? (
          <p className={styles.error}>{String(accountsQ.error)}</p>
        ) : !accountsQ.data ? (
          <p className={styles.quiet}>Loading contributors...</p>
        ) : accountsQ.data.length === 0 ? (
          <p className={styles.quiet}>No contributors in this window yet.</p>
        ) : (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>#</th>
                  <th>Contributor</th>
                  <th>Trust</th>
                  <th>Team</th>
                  <th className={styles.num}>Units</th>
                  <th className={styles.num}>Verified</th>
                  <th className={styles.num}>Games</th>
                  <th className={styles.num}>Credit</th>
                </tr>
              </thead>
              <tbody>
                {accountsQ.data.map((e, i) => (
                  <tr key={e.account}>
                    <td>{i + 1}</td>
                    <td><Link to={`/swarm/contributors/${encodeURIComponent(e.account)}`}>{e.account}</Link></td>
                    <td><span className={trustChipClass(e.trust)}>{e.trust}</span></td>
                    <td className={styles.quiet}>{e.team ?? '-'}</td>
                    <td className={styles.num}>{e.accepted_units.toLocaleString()}</td>
                    <td className={styles.num}>{e.verified_units.toLocaleString()}</td>
                    <td className={styles.num}>{e.games.toLocaleString()}</td>
                    <td className={styles.num}>{formatCredit(e.credit)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      ) : teamsQ.isError ? (
        <p className={styles.error}>{String(teamsQ.error)}</p>
      ) : !teamsQ.data ? (
        <p className={styles.quiet}>Loading teams...</p>
      ) : teamsQ.data.length === 0 ? (
        <p className={styles.quiet}>No teams created yet.</p>
      ) : (
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>#</th>
                <th>Team</th>
                <th>Description</th>
                <th className={styles.num}>Members</th>
                <th className={styles.num}>Units</th>
                <th className={styles.num}>Games</th>
                <th className={styles.num}>Credit</th>
              </tr>
            </thead>
            <tbody>
              {teamsQ.data.map((team, i) => (
                <tr key={team.id}>
                  <td>{i + 1}</td>
                  <td>{team.name}</td>
                  <td className={styles.quiet}>{team.description || '-'}</td>
                  <td className={styles.num}>{team.members}</td>
                  <td className={styles.num}>{team.accepted_units.toLocaleString()}</td>
                  <td className={styles.num}>{team.games.toLocaleString()}</td>
                  <td className={styles.num}>{formatCredit(team.credit)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function ContributorView({ account }: { account: string }) {
  const profileQ = useQuery({
    queryKey: ['swarm', 'contributor', account],
    queryFn: () => swarmPublic.contributor(account),
  });
  if (profileQ.isError) return <p className={styles.error} role="alert">{String(profileQ.error)}</p>;
  const p = profileQ.data;
  if (!p) return <p className={styles.quiet}>Loading contributor profile...</p>;

  return (
    <>
      <section className={styles.card}>
        <div className={styles.cardHead}>
          <h2 className={styles.cardTitle}>{p.account}</h2>
          <div className={styles.chips}>
            <span className={trustChipClass(p.trust)}>{p.trust}</span>
            {p.team && <span className={styles.chip}>Team: {p.team}</span>}
            {p.badges.map((badge) => (
              <span key={badge} className={styles.chip}>{badge}</span>
            ))}
          </div>
        </div>
        <dl className={styles.kpiGrid}>
          <div className={styles.kpi}>
            <dt>Total credit</dt>
            <dd>{formatCredit(p.credit)}</dd>
            <span className={styles.kpiNote}>
              {formatCredit(p.credit_matured)} matured · {formatCredit(p.credit_pending)} pending
            </span>
          </div>
          <div className={styles.kpi}>
            <dt>Accepted units</dt>
            <dd>{p.accepted_units.toLocaleString()}</dd>
            <span className={styles.kpiNote}>{p.verified_units.toLocaleString()} verifications completed</span>
          </div>
          <div className={styles.kpi}>
            <dt>Games played</dt>
            <dd>{p.games.toLocaleString()}</dd>
            <span className={styles.kpiNote}>Joined {formatUnixAge(p.created)}</span>
          </div>
        </dl>
      </section>

      <div className={styles.split}>
        <section className={styles.card}>
          <h3 className={styles.cardTitle}>Hosts ({p.hosts.length})</h3>
          {p.hosts.length === 0 ? (
            <p className={styles.quiet}>No hosts reported yet.</p>
          ) : (
            <div className={styles.tableWrap}>
              <table className={styles.table}>
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>OS</th>
                    <th className={styles.num}>Threads</th>
                    <th className={styles.num}>Units</th>
                    <th className={styles.num}>Seen</th>
                  </tr>
                </thead>
                <tbody>
                  {p.hosts.map((h) => (
                    <tr key={h.name}>
                      <td>{h.name}</td>
                      <td className={styles.quiet}>{h.os}</td>
                      <td className={styles.num}>{h.threads}</td>
                      <td className={styles.num}>{h.accepted_units}</td>
                      <td className={styles.num}>{formatUnixAge(h.seen)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <section className={styles.card}>
          <h3 className={styles.cardTitle}>Recent accepted units</h3>
          {p.recent.length === 0 ? (
            <p className={styles.quiet}>No accepted units yet.</p>
          ) : (
            <div className={styles.tableWrap}>
              <table className={styles.table}>
                <thead>
                  <tr>
                    <th>Unit</th>
                    <th className={styles.num}>Games</th>
                    <th className={styles.num}>Credit</th>
                    <th className={styles.num}>Done</th>
                  </tr>
                </thead>
                <tbody>
                  {p.recent.map((u) => (
                    <tr key={u.unit}>
                      <td className={styles.mono}>{u.unit}</td>
                      <td className={styles.num}>{u.games}</td>
                      <td className={styles.num}>{formatCredit(u.credit)}</td>
                      <td className={styles.num}>{formatUnixAge(u.done_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>
    </>
  );
}

function TransparencyView() {
  const transQ = useQuery({
    queryKey: ['swarm', 'transparency'],
    queryFn: () => swarmPublic.transparency(),
    refetchInterval: 30_000,
  });
  if (transQ.isError) return <p className={styles.error} role="alert">{String(transQ.error)}</p>;
  const data = transQ.data;
  if (!data) return <p className={styles.quiet}>Loading public audit log...</p>;

  return (
    <>
      <dl className={styles.kpiGrid}>
        <div className={styles.kpi}>
          <dt>Accepted units</dt>
          <dd>{data.accepted_units.toLocaleString()}</dd>
        </div>
        <div className={styles.kpi}>
          <dt>Verifications passed</dt>
          <dd>{data.verified_units.toLocaleString()}</dd>
        </div>
        <div className={styles.kpi}>
          <dt>Rejected units</dt>
          <dd>{data.rejected_units.toLocaleString()}</dd>
        </div>
        <div className={styles.kpi}>
          <dt>Quarantined units</dt>
          <dd>{data.quarantined_units.toLocaleString()}</dd>
        </div>
      </dl>

      <section className={styles.card} aria-labelledby="swarm-transparency-links">
        <h2 id="swarm-transparency-links" className={styles.cardTitle}>Open data and protocol</h2>
        <p className={styles.quiet}>
          Every accepted game is replayed from its full answer log, and sampled units are re-run bit-for-bit across
          accounts under the strict math build. Public networks and datasets are published on Hugging Face under{' '}
          <a href="https://huggingface.co/nerijs" target="_blank" rel="noreferrer">huggingface.co/nerijs</a>.
        </p>
      </section>

      <section className={styles.card} aria-labelledby="swarm-public-audit">
        <h2 id="swarm-public-audit" className={styles.cardTitle}>Public run and verification log</h2>
        {data.events.length === 0 ? (
          <p className={styles.quiet}>No public run decisions recorded yet.</p>
        ) : (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Actor</th>
                  <th>Action</th>
                  <th>Detail</th>
                </tr>
              </thead>
              <tbody>
                {data.events.map((ev) => (
                  <tr key={ev.id}>
                    <td className={styles.quiet}>{formatUnixAge(ev.at)}</td>
                    <td>{ev.actor}</td>
                    <td><span className={styles.chip}>{ev.action}</span></td>
                    <td className={styles.mono}>{ev.detail}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  );
}

function CopyableCode({ label, code }: { label: string; code: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      // clipboard unavailable
    }
  }
  return (
    <div className={styles.codeBlock}>
      <pre>{code}</pre>
      <button type="button" className={styles.copyBtn} onClick={() => void copy()} aria-label={`Copy ${label}`}>
        {copied ? <><Check size={12} aria-hidden="true" /> Copied</> : <><Copy size={12} aria-hidden="true" /> Copy</>}
      </button>
    </div>
  );
}

function ContributeView() {
  const [handle, setHandle] = useState('');
  const [minted, setMinted] = useState<{ account: string; token: string } | null>(null);
  const [regError, setRegError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function onRegister(e: React.FormEvent) {
    e.preventDefault();
    const clean = handle.trim();
    if (!clean) return;
    setSubmitting(true);
    setRegError(null);
    try {
      const reply = await swarmPublic.register(clean);
      setMinted({ account: clean, token: reply.token });
    } catch (err) {
      setRegError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  }

  const tokenDisplay = minted ? minted.token : 'YOUR_WORKER_TOKEN';
  const hostDisplay = minted ? `${minted.account}-pc` : 'my-pc';

  const dockerCmd = `docker run -d --name mana --restart unless-stopped --cpus 4 \\
  -e MANA_COORDINATOR=${DEFAULT_SWARM_URL} \\
  -e MANA_NAME=${hostDisplay} \\
  -e MANA_TOKEN=${tokenDisplay} \\
  -e MANA_JOBS=4 \\
  -v mana-work:/work \\
  ghcr.io/arcanite24/mana-worker:latest`;

  const cliCmd = `cargo build --release -p playmat-worker --features strict
./rust/target/release/playmat-worker ui \\
  --coordinator ${DEFAULT_SWARM_URL} \\
  --name ${hostDisplay} \\
  --token-file ~/.config/mana/worker.token \\
  --jobs 4 \\
  --when idle`;

  return (
    <>
      <section className={styles.card} aria-labelledby="swarm-signup-title">
        <h2 id="swarm-signup-title" className={styles.cardTitle}>Get a volunteer worker token</h2>
        <p className={styles.quiet}>
          Choose a contributor handle (3-32 characters of letters, digits, dot, hyphen, underscore) to mint a one-time
          worker token. New accounts start in probation and never require a Playmat login.
        </p>
        <form onSubmit={(e) => void onRegister(e)} className={styles.toolbar}>
          <label className={styles.quiet}>
            Contributor handle{' '}
            <input
              type="text"
              value={handle}
              onChange={(e) => setHandle(e.target.value)}
              placeholder="your-handle"
              minLength={3}
              maxLength={32}
              required
              style={{
                padding: '6px 10px',
                borderRadius: 6,
                border: '1px solid var(--ink-faint)',
                background: 'var(--mat-950)',
                color: 'var(--ink-strong)',
              }}
            />
          </label>
          <button
            type="submit"
            className={styles.copyBtn}
            aria-label="Create worker token"
            disabled={submitting || handle.trim().length < 3}
          >
            {submitting ? 'Creating...' : 'Create worker token'}
          </button>
        </form>
        {regError && <p className={styles.error} role="alert">{regError}</p>}
        {minted && (
          <>
            <p className={styles.quiet} role="status">
              Token created for <strong>{minted.account}</strong>. Copy and save it now; it is shown only once and has
              been filled into the commands below.
            </p>
            <CopyableCode label="worker token" code={minted.token} />
          </>
        )}
      </section>

      <div className={styles.split}>
        <section className={styles.card}>
          <h2 className={styles.cardTitle}>Docker worker (recommended)</h2>
          <p className={styles.quiet}>
            Multi-arch (amd64 and arm64), built with strict deterministic math and pinned card suites.
          </p>
          <CopyableCode label="Docker worker command" code={dockerCmd} />
        </section>

        <section className={styles.card}>
          <h2 className={styles.cardTitle}>Desktop control panel and CLI</h2>
          <p className={styles.quiet}>
            Runs at lowest OS priority, pauses when your computer is busy (--when idle), and serves a local control
            panel at http://127.0.0.1:17195. Replace ui with run for headless boxes.
          </p>
          <CopyableCode label="Native worker command" code={cliCmd} />
        </section>

        <section className={styles.card}>
          <h2 className={styles.cardTitle}>Browser and Android worker (WASM)</h2>
          <p className={styles.quiet}>
            The strict Rust worker also compiles to WebAssembly (playmat-worker-wasm, strict-1 class) and aarch64 Android
            so single-threaded Web Workers and Capacitor Android builds can contribute strict games and replay checks.
          </p>
        </section>

        <section className={styles.card}>
          <h2 className={styles.cardTitle}>How trust and credit work</h2>
          <p className={styles.quiet}>
            New accounts and new hosts start in probation (100% re-run audit on the first 20 accepted units), then move
            to normal (~3% random re-run sample) and reputable (30 days and 2,000 accepted units with zero mismatches).
            Credits follow Folding@home style reference core-seconds with a 1.5x quick-return bonus cap, mature 7 days
            after acceptance, and can be redeemed for Playmat cosmetic capsule tokens in the Capsule Shop.
          </p>
        </section>
      </div>
    </>
  );
}
