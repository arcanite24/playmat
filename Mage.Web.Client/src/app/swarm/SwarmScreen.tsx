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
  type LeaderboardWindow,
  type PublicRound,
  type SwarmStatus,
  type TrustLevel,
} from './swarmApi';
import styles from './SwarmScreen.module.css';

type Section = 'overview' | 'projects' | 'leaderboard' | 'transparency' | 'contribute';

const NAV_ITEMS: { id: Section; label: string; to: string }[] = [
  { id: 'overview', label: 'Overview', to: '/swarm' },
  { id: 'projects', label: 'Projects', to: '/swarm/projects' },
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
    section === 'projects' || section === 'leaderboard' || section === 'transparency' || section === 'contribute'
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
  const dockerCmd = `docker run -d --name mana --restart unless-stopped --cpus 4 \\
  -e MANA_COORDINATOR=${DEFAULT_SWARM_URL} \\
  -e MANA_NAME=my-pc \\
  -e MANA_TOKEN=YOUR_WORKER_TOKEN \\
  -e MANA_JOBS=4 \\
  -v mana-work:/work \\
  ghcr.io/arcanite24/mana-worker:latest`;

  const cliCmd = `cargo build --release -p playmat-worker --features strict
./rust/target/release/playmat-worker run \\
  --coordinator ${DEFAULT_SWARM_URL} \\
  --name my-pc \\
  --token-file ~/.config/mana/worker.token \\
  --jobs 4 \\
  --when idle`;

  return (
    <div className={styles.split}>
      <section className={styles.card}>
        <h2 className={styles.cardTitle}>Docker worker (recommended)</h2>
        <p className={styles.quiet}>
          Multi-arch (amd64 and arm64), built with strict deterministic math and pinned card suites.
        </p>
        <CopyableCode label="Docker worker command" code={dockerCmd} />
      </section>

      <section className={styles.card}>
        <h2 className={styles.cardTitle}>Native CLI worker</h2>
        <p className={styles.quiet}>
          Runs at lowest OS priority and pauses automatically when your computer is in use (--when idle).
        </p>
        <CopyableCode label="Native CLI worker command" code={cliCmd} />
      </section>

      <section className={styles.card}>
        <h2 className={styles.cardTitle}>How trust and credit work</h2>
        <p className={styles.quiet}>
          New accounts and new hosts start in probation (100% re-run audit on the first 20 accepted units), then move
          to normal (~3% random re-run sample) and reputable (30 days and 2,000 accepted units with zero mismatches).
          Credits follow Folding@home style reference core-seconds with a 1.5x quick-return bonus cap and mature 7 days
          after acceptance.
        </p>
      </section>
    </div>
  );
}
