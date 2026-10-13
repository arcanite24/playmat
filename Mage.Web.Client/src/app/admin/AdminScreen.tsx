import * as Tabs from '@radix-ui/react-tabs';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Ban, Eye, KeyRound, LogOut, Mic, MicOff, Power, Trash2 } from 'lucide-react';
import { useEffect, useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { createApi } from '../../protocol/generated/api';
import type { ClientErrorGroup, PlayerReport, ServerStats, TableView, UserView } from '../../protocol/generated/views';
import { RpcClient } from '../../core/rpc/RpcClient';
import { APP_NAME } from '../brand';
import { api as appApi } from '../connection';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { AccessPage } from './AccessPage';
import { AccountsPage } from './AccountsPage';
import { AuditPage } from './AuditPage';
import { ConfirmAction } from './ConfirmAction';
import { FeedbackPage } from './FeedbackPage';
import { SettingsPage } from './SettingsPage';
import { SoloPage } from './SoloPage';
import { StaffPage } from './StaffPage';
import { atLeast, describeAge, formatBytes, formatDate, formatDuration, MUTE_CHOICES, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

export type AdminApi = ReturnType<typeof createApi>;

interface StaffSession {
  api: AdminApi;
  role: StaffRole;
  /** the name the audit log records */
  actor: string;
}

/**
 * The server's admin console. Staff open it on their own signed-in session (Admin in the account menu) and see the
 * sections their role allows. The shared admin password is a break-glass key: Console sign-in opens a connection of
 * its own as an owner, for when nobody with a role can sign in.
 */
export function AdminScreen() {
  const phase = useSession((state) => state.phase);
  const userName = useSession((state) => state.userName);
  const client = useQueryClient();
  const [console_, setConsole] = useState<{ api: AdminApi; rpc: RpcClient } | null>(null);
  const [dropped, setDropped] = useState(false);
  useEffect(() => () => console_?.rpc.disconnect(), [console_]);
  // opened from a bookmark: come back on this device's remembered sign-in
  useEffect(() => {
    if (useSession.getState().phase === 'signedOut') void useSession.getState().resume();
  }, []);
  const me = useQuery({
    queryKey: ['staffMe', userName],
    queryFn: () => appApi.staffMe(),
    enabled: phase === 'signedIn',
    staleTime: 60_000,
  });

  function endConsole() {
    setConsole(null);
    client.removeQueries({ queryKey: ['admin'] });
    // the console may have changed our own role
    void client.invalidateQueries({ queryKey: ['staffMe'] });
  }

  const staff: StaffSession | null = console_
    ? { api: console_.api, role: 'owner', actor: 'console' }
    : phase === 'signedIn' && me.data?.role ? { api: appApi, role: me.data.role as StaffRole, actor: me.data.actor ?? userName } : null;

  return (
    <main className={styles.page}>
      <header className={styles.head}>
        <Link to="/" className={styles.back}><ArrowLeft size={16} aria-hidden="true" /> {APP_NAME}</Link>
        <h1 className={styles.title}>Server admin</h1>
        {staff && <span className={styles.who}>{staff.actor} · {staff.role}</span>}
        {console_ && (
          <Button size="sm" variant="quiet" icon={<LogOut size={15} />} onClick={endConsole}>Leave console</Button>
        )}
      </header>
      {staff
        ? <Console key={staff.actor} staff={staff} />
        : (
          <div className={styles.signIn}>
            {phase === 'signingIn' || me.isLoading ? <p className={styles.quiet}>Checking your role…</p>
              : phase !== 'signedIn' ? <p className={styles.lede}>Sign in with your staff account, then open Admin from the account menu. <Link to="/login" state={{ from: '/admin' }}>Sign in</Link></p>
                : <p className={styles.lede}>{userName} has no staff role on this server. The owner gives roles from Admin, Staff.</p>}
            {me.error && <p className={styles.error}>{String(me.error)}</p>}
            {dropped && <p className={styles.error} role="alert">The console connection dropped. Sign in again.</p>}
            <ConsoleSignIn onSignedIn={(next) => {
              setDropped(false);
              next.rpc.onSessionStart(() => {
                // a new connection is a new session without the console: the password isn't kept to sign in again
                setDropped(true);
                endConsole();
              });
              setConsole(next);
            }} />
          </div>
        )}
    </main>
  );
}

function ConsoleSignIn({ onSignedIn }: { onSignedIn(admin: { api: AdminApi; rpc: RpcClient }): void }) {
  const serverUrl = useSession((state) => state.serverUrl);
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const rpc = new RpcClient();
    const api = createApi(rpc);
    try {
      await rpc.connect(serverUrl);
      const ok = await api.connectAdmin(password);
      if (!ok) throw new Error('The server refused the admin password.');
      setPassword('');
      onSignedIn({ api, rpc });
    } catch (caught) {
      rpc.disconnect();
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <details className={styles.breakGlass}>
      <summary><KeyRound size={15} aria-hidden="true" /> Console sign-in</summary>
      <form className={styles.signIn} onSubmit={submit}>
        <p className={styles.quiet}>For when no owner can sign in: the server's admin password (XMAGE_ADMIN_PASSWORD) opens the console as an owner on its own connection. Wrong passwords slow further tries down.</p>
        <Field label="Admin password" type="password" value={password} autoComplete="current-password"
          onChange={(event) => setPassword(event.target.value)} required />
        {error && <p className={styles.error} role="alert">{error}</p>}
        <Button type="submit" variant="decision" busy={busy} disabled={!password}>Open the console</Button>
      </form>
    </details>
  );
}

const SECTIONS: { id: string; label: string; role: StaffRole }[] = [
  { id: 'overview', label: 'Overview', role: 'moderator' },
  { id: 'players', label: 'Players', role: 'moderator' },
  { id: 'accounts', label: 'Accounts', role: 'moderator' },
  { id: 'reports', label: 'Reports', role: 'moderator' },
  { id: 'feedback', label: 'Feedback', role: 'moderator' },
  { id: 'tables', label: 'Tables', role: 'moderator' },
  { id: 'broadcast', label: 'Broadcast', role: 'admin' },
  { id: 'access', label: 'Access', role: 'admin' },
  { id: 'settings', label: 'Settings', role: 'admin' },
  { id: 'solo', label: 'Single player', role: 'admin' },
  { id: 'staff', label: 'Staff', role: 'owner' },
  { id: 'errors', label: 'Client errors', role: 'moderator' },
  { id: 'audit', label: 'Audit', role: 'moderator' },
];

function Console({ staff }: { staff: StaffSession }) {
  const { api, role } = staff;
  const sections = SECTIONS.filter((section) => atLeast(role, section.role));
  const fromHash = window.location.hash.slice(1);
  const [section, setSection] = useState(sections.some((s) => s.id === fromHash) ? fromHash : 'overview');
  function choose(next: string) {
    setSection(next);
    window.history.replaceState(null, '', `#${next}`);
  }
  return (
    <Tabs.Root value={section} onValueChange={choose} orientation="vertical" className={styles.console}>
      <Tabs.List className={styles.sideNav} aria-label="Admin sections">
        {sections.map((s) => <Tabs.Trigger key={s.id} value={s.id} className={styles.navItem}>{s.label}</Tabs.Trigger>)}
      </Tabs.List>
      <Tabs.Content value="overview" className={styles.panel}><Overview api={api} /></Tabs.Content>
      <Tabs.Content value="players" className={styles.panel}><Players api={api} role={role} /></Tabs.Content>
      <Tabs.Content value="accounts" className={styles.panel}><AccountsPage api={api} role={role} /></Tabs.Content>
      <Tabs.Content value="reports" className={styles.panel}><Reports api={api} role={role} /></Tabs.Content>
      <Tabs.Content value="feedback" className={styles.panel}><FeedbackPage api={api} /></Tabs.Content>
      <Tabs.Content value="tables" className={styles.panel}><TablesPanel api={api} role={role} /></Tabs.Content>
      {atLeast(role, 'admin') && (
        <>
          <Tabs.Content value="broadcast" className={styles.panel}><Broadcast api={api} /></Tabs.Content>
          <Tabs.Content value="access" className={styles.panel}><AccessPage api={api} /></Tabs.Content>
          <Tabs.Content value="settings" className={styles.panel}><SettingsPage api={api} role={role} /></Tabs.Content>
          <Tabs.Content value="solo" className={styles.panel}><SoloPage api={api} role={role} /></Tabs.Content>
        </>
      )}
      {atLeast(role, 'owner') && <Tabs.Content value="staff" className={styles.panel}><StaffPage api={api} actor={staff.actor} /></Tabs.Content>}
      <Tabs.Content value="errors" className={styles.panel}><ClientErrors api={api} /></Tabs.Content>
      <Tabs.Content value="audit" className={styles.panel}><AuditPage api={api} /></Tabs.Content>
    </Tabs.Root>
  );
}

function Overview({ api }: { api: AdminApi }) {
  const stats = useQuery({ queryKey: ['admin', 'stats'], queryFn: () => api.adminServerStats(), refetchInterval: 5_000 });
  const data: ServerStats | undefined = stats.data;
  if (stats.error) return <p className={styles.error}>{String(stats.error)}</p>;
  if (!data) return <p className={styles.quiet}>Loading…</p>;
  const bridge = data.bridge;
  const buckets = bridge?.buckets ?? [];
  const limits = bridge?.bucketLimits ?? [];
  const most = Math.max(1, ...buckets);
  const backup = data.backup;
  const maintenance = data.maintenance;
  return (
    <div className={styles.overview}>
      <p className={styles.quiet}>
        Version {data.version ?? 'unknown'}
        {data.commit
          ? <> · commit <a className={styles.code} href={`https://github.com/arcanite24/playmat/commit/${data.commit}`} target="_blank" rel="noreferrer">{data.commit.slice(0, 7)}</a></>
          : ' · a development build'}
        {backup && (
          <span className={backup.problem ? styles.backupBad : undefined}>
            {' · '}{backup.problem ?? `last backup ${describeAge((backup.lastOkAt ?? 0) * 1000, data.now ?? 0)}${backup.size ? ` (${backup.size})` : ''}`}
          </span>
        )}
      </p>
      {maintenance && (
        <p className={styles.maintenance}>
          Maintenance {(maintenance.startsAt ?? 0) > (data.now ?? 0) ? `at ${formatDate(maintenance.startsAt ?? 0)}` : 'under way'}, announced by {maintenance.by}: no new tables can start.
        </p>
      )}
      <dl className={styles.numbers}>
        <Stat label="Players online" value={data.usersOnline ?? 0} />
        <Stat label="Tables" value={data.tables ?? 0} />
        <Stat label="Games running" value={data.activeGames ?? 0} />
        <Stat label="Connections" value={bridge?.connections ?? 0} />
        <Stat label="CPU (this server)" value={data.processCpu !== undefined && data.processCpu >= 0 ? `${Math.round(data.processCpu * 100)}%` : '—'} />
        <Stat label="Load (1 min)" value={data.systemLoad !== undefined && data.systemLoad >= 0 ? `${data.systemLoad.toFixed(2)} / ${data.processors} cores` : '—'} />
        <Stat label="Memory" value={`${formatBytes(data.heapUsed ?? 0)} of ${formatBytes(data.heapMax ?? 0)}`} />
        <Stat label="Up for" value={formatDuration(data.uptimeMillis ?? 0)} />
      </dl>

      <section aria-labelledby="sizes" className={styles.block}>
        <h2 id="sizes">Messages to web clients</h2>
        <p className={styles.quiet}>{(bridge?.messages ?? 0).toLocaleString()} sent, {formatBytes(bridge?.chars ?? 0)} of JSON; {(bridge?.requests ?? 0).toLocaleString()} requests received.</p>
        {(bridge?.stateFullChars ?? 0) > 0 && (
          <p className={styles.quiet}>
            Game states: {(bridge?.statePatches ?? 0).toLocaleString()} sent as patches, {(bridge?.stateComplete ?? 0).toLocaleString()} complete
            ({(bridge?.stateResyncs ?? 0).toLocaleString()} resyncs); {formatBytes(bridge?.stateChars ?? 0)} instead
            of {formatBytes(bridge?.stateFullChars ?? 0)}, {Math.round((1 - (bridge?.stateChars ?? 0) / (bridge?.stateFullChars ?? 1)) * 100)}% saved.
          </p>
        )}
        <ol className={styles.histogram}>
          {buckets.map((count, index) => (
            <li key={index}>
              <span className={styles.bucketLabel}>{index < limits.length ? `≤ ${formatBytes(limits[index])}` : `> ${formatBytes(limits[limits.length - 1] ?? 0)}`}</span>
              <span className={styles.bar} style={{ inlineSize: `${(count / most) * 100}%` }} />
              <span className={styles.bucketCount}>{count.toLocaleString()}</span>
            </li>
          ))}
        </ol>
      </section>

      <section aria-labelledby="heavy" className={styles.block}>
        <h2 id="heavy">Heaviest callbacks</h2>
        <table className={styles.table}>
          <thead><tr><th>Callback</th><th>Sent</th><th>Total</th><th>Largest</th></tr></thead>
          <tbody>
            {(bridge?.heaviest ?? []).map((entry) => (
              <tr key={entry.method}>
                <td>{entry.method}</td>
                <td>{(entry.count ?? 0).toLocaleString()}</td>
                <td>{formatBytes(entry.chars ?? 0)}</td>
                <td className={(entry.maxChars ?? 0) > 256 * 1024 ? styles.alarm : undefined}>{formatBytes(entry.maxChars ?? 0)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {(bridge?.large?.length ?? 0) > 0 && (
        <section aria-labelledby="large" className={styles.block}>
          <h2 id="large" className={styles.alarm}>Messages over 256 KB</h2>
          <ul className={styles.list}>
            {bridge!.large!.map((entry, index) => (
              <li key={index}>{entry.method} · {formatBytes(entry.chars ?? 0)} · {describeAge(entry.at ?? 0, data.now ?? 0)}{entry.objectId ? ` · ${entry.objectId}` : ''}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className={styles.stat}>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

type PlayerAction = { kind: 'lock' | 'deactivate' | 'disconnect'; name: string; sessionId?: string };

function Players({ api, role }: { api: AdminApi; role: StaffRole }) {
  const client = useQueryClient();
  const users = useQuery({ queryKey: ['admin', 'users'], queryFn: () => api.adminGetUsers(), refetchInterval: 5_000 });
  const [filter, setFilter] = useState('');
  const [minutes, setMinutes] = useState(MUTE_CHOICES[1].minutes);
  const [pending, setPending] = useState<PlayerAction | null>(null);
  const admin = atLeast(role, 'admin');
  const act = useMutation({
    mutationFn: (work: () => Promise<unknown>) => work(),
    onSettled: () => client.invalidateQueries({ queryKey: ['admin', 'users'] }),
  });
  const list = (users.data ?? []).filter((user: UserView) => !filter || user.userName?.toLowerCase().includes(filter.toLowerCase()));
  const now = users.dataUpdatedAt;
  const choice = MUTE_CHOICES.find((c) => c.minutes === minutes)?.label ?? `${minutes} minutes`;

  async function confirm(reason: string) {
    if (!pending) return;
    if (pending.kind === 'lock') await api.adminLockUser(pending.name, minutes, reason);
    else if (pending.kind === 'deactivate') await api.adminActivateUser(pending.name, false, reason);
    else await api.adminDisconnectUser(pending.sessionId!, reason);
    setPending(null);
    await client.invalidateQueries({ queryKey: ['admin', 'users'] });
  }

  return (
    <div className={styles.block}>
      <div className={styles.toolbar}>
        <Field label="Find a player" value={filter} onChange={(event) => setFilter(event.target.value)} />
        <label className={styles.select}>
          <span>{admin ? 'Mute or lock for' : 'Mute for'}</span>
          <select value={minutes} onChange={(event) => setMinutes(Number(event.target.value))}>
            {MUTE_CHOICES.map((c) => <option key={c.minutes} value={c.minutes}>{c.label}</option>)}
          </select>
        </label>
      </div>
      {pending && (
        <ConfirmAction
          title={pending.kind === 'lock' ? `Lock ${pending.name} out` : pending.kind === 'deactivate' ? `Deactivate ${pending.name}` : `Sign ${pending.name} out`}
          what={pending.kind === 'lock' ? `${pending.name} is signed out and can't sign in for ${choice}.`
            : pending.kind === 'deactivate' ? `${pending.name} is signed out and can't sign in until the account is activated again.`
              : `${pending.name} is signed out now and can sign in again straight away.`}
          confirmLabel={pending.kind === 'lock' ? 'Lock out' : pending.kind === 'deactivate' ? 'Deactivate' : 'Sign out'}
          onConfirm={confirm}
          onCancel={() => setPending(null)}
        />
      )}
      {act.error && <p className={styles.error}>{String(act.error)}</p>}
      <table className={styles.table}>
        <thead><tr><th>Player</th><th>Address</th><th>Status</th><th>Playing</th><th>Online for</th><th><span className="visually-hidden">Actions</span></th></tr></thead>
        <tbody>
          {list.map((user, index) => {
            const name = user.userName ?? '';
            const muted = !!user.muteChatUntil && user.muteChatUntil > now;
            return (
              // a player who just left keeps a row without a session for a while, next to their new one
              <tr key={user.sessionId || `${name}-${index}`}>
                <td>{name}{muted && <span className={styles.tag}>muted</span>}</td>
                <td>{user.host}</td>
                <td>{user.userState}</td>
                <td className={styles.clip}>{user.gameInfo}</td>
                <td>{user.timeConnected ? formatDuration(now - user.timeConnected) : ''}</td>
                <td className={styles.actions}>
                  {muted
                    ? <Button size="sm" variant="quiet" icon={<Mic size={14} />} title={`Let ${name} chat again`}
                      onClick={() => act.mutate(() => api.adminMuteUser(name, 0))}>Unmute</Button>
                    : <Button size="sm" variant="quiet" icon={<MicOff size={14} />} title={`Mute ${name} for ${choice}`}
                      onClick={() => act.mutate(() => api.adminMuteUser(name, minutes))}>Mute</Button>}
                  {admin && (
                    <>
                      <Button size="sm" variant="quiet" icon={<Ban size={14} />} title={`Lock ${name} out for ${choice}`}
                        onClick={() => setPending({ kind: 'lock', name })}>Lock</Button>
                      <Button size="sm" variant="quiet" icon={<Power size={14} />} title={`Deactivate ${name}'s account`}
                        onClick={() => setPending({ kind: 'deactivate', name })}>Deactivate</Button>
                    </>
                  )}
                  <Button size="sm" variant="danger" icon={<LogOut size={14} />} title={`Sign ${name} out`} disabled={!user.sessionId}
                    onClick={() => setPending({ kind: 'disconnect', name, sessionId: user.sessionId })}>Sign out</Button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <p className={styles.quiet}>Players who are offline are under Accounts.</p>
    </div>
  );
}

function TablesPanel({ api, role }: { api: AdminApi; role: StaffRole }) {
  const client = useQueryClient();
  const room = useQuery({ queryKey: ['admin', 'room'], queryFn: () => api.serverGetMainRoomId(), staleTime: Infinity });
  const tables = useQuery({
    queryKey: ['admin', 'tables', room.data],
    queryFn: () => api.roomGetAllTables(room.data!),
    enabled: !!room.data,
    refetchInterval: 5_000,
  });
  const [pending, setPending] = useState<TableView | null>(null);
  const [watchError, setWatchError] = useState<string | null>(null);
  const navigate = useNavigate();
  // a game opens on the player's own connection; the console sign-in has no game screen to open it in
  const ownSession = api === appApi;
  async function watch(table: TableView) {
    setWatchError(null);
    try {
      const gameId = await api.adminTableWatch(table.tableId!);
      if (gameId) navigate(`/game/${gameId}`);
      else setWatchError(`“${table.tableName}” has no game running that you can watch.`);
    } catch (caught) {
      setWatchError(caught instanceof Error ? caught.message : String(caught));
    }
  }
  async function remove(reason: string) {
    await api.adminTableRemove(pending!.tableId!, reason);
    setPending(null);
    await client.invalidateQueries({ queryKey: ['admin', 'tables'] });
  }
  return (
    <div className={styles.block}>
      {pending && (
        <ConfirmAction title={`Remove “${pending.tableName}”`} what={`The table closes and its players lose the game: ${pending.seatsInfo || 'no one seated'}.`}
          confirmLabel="Remove" onConfirm={remove} onCancel={() => setPending(null)} />
      )}
      {watchError && <p className={styles.error} role="alert">{watchError}</p>}
      {tables.data?.length === 0 && <p className={styles.quiet}>No tables are open.</p>}
      <table className={styles.table}>
        <thead><tr><th>Table</th><th>Host</th><th>Game</th><th>State</th><th>Seats</th><th><span className="visually-hidden">Actions</span></th></tr></thead>
        <tbody>
          {(tables.data ?? []).map((table: TableView) => (
            <tr key={table.tableId}>
              <td>{table.tableName}</td>
              <td>{table.controllerName}</td>
              <td>{table.gameType} · {table.deckType}</td>
              <td>{table.tableStateText ?? table.tableState}</td>
              <td className={styles.clip}>{table.seatsInfo}</td>
              <td className={styles.actions}>
                {ownSession && table.tableState === 'DUELING' && (
                  <Button size="sm" variant="quiet" icon={<Eye size={14} />} title={`Watch ${table.tableName}, whatever its spectator settings`}
                    onClick={() => watch(table)}>Watch</Button>
                )}
                {atLeast(role, 'admin') && (
                  <Button size="sm" variant="danger" icon={<Trash2 size={14} />} onClick={() => setPending(table)}>Remove</Button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Reports({ api, role }: { api: AdminApi; role: StaffRole }) {
  const client = useQueryClient();
  const [openOnly, setOpenOnly] = useState(true);
  const reports = useQuery({ queryKey: ['admin', 'reports', openOnly], queryFn: () => api.adminGetReports(openOnly), refetchInterval: 15_000 });
  const close = useMutation({
    mutationFn: ({ id, resolution }: { id: number; resolution: string }) => api.adminCloseReport(id, resolution),
    onSettled: () => client.invalidateQueries({ queryKey: ['admin', 'reports'] }),
  });
  const now = reports.dataUpdatedAt;
  return (
    <div className={styles.block}>
      <label className={styles.check}>
        <input type="checkbox" checked={!openOnly} onChange={(event) => setOpenOnly(!event.target.checked)} /> Show closed reports too
      </label>
      {reports.data?.length === 0 && <p className={styles.quiet}>{openOnly ? 'No reports are waiting.' : 'No reports yet.'}</p>}
      <ul className={styles.reports}>
        {(reports.data ?? []).map((report: PlayerReport) => (
          <li key={report.id} className={styles.report}>
            <p><strong>{report.reported}</strong> for <strong>{report.reason}</strong>, by {report.reporter} · {describeAge(report.createdAt ?? 0, now)}</p>
            {report.details && <blockquote>{report.details}</blockquote>}
            {report.gameId && <p className={styles.quiet}>Game {report.gameId}</p>}
            {report.open
              ? <CloseReport api={api} role={role} report={report} onClose={(resolution) => close.mutate({ id: report.id!, resolution })} />
              : <p className={styles.quiet}>Closed: {report.resolution || 'no note'}</p>}
          </li>
        ))}
      </ul>
    </div>
  );
}

function CloseReport({ api, role, report, onClose }: { api: AdminApi; role: StaffRole; report: PlayerReport; onClose(resolution: string): void }) {
  const [note, setNote] = useState('');
  const [error, setError] = useState<string | null>(null);
  const name = report.reported ?? '';
  const why = `report ${report.id}: ${report.reason}`;
  async function act(work: () => Promise<unknown>, done: string) {
    setError(null);
    try {
      await work();
      onClose(note.trim() ? `${done}. ${note.trim()}` : done);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }
  return (
    <div className={styles.inline}>
      <Field label="Note" value={note} onChange={(event) => setNote(event.target.value)} />
      <Button size="sm" onClick={() => act(() => api.adminMuteUser(name, 24 * 60, why), 'Muted for a day')}>Mute a day</Button>
      {atLeast(role, 'admin') && (
        <Button size="sm" onClick={() => act(() => api.adminLockUser(name, 7 * 24 * 60, why), 'Locked out for a week')}>Lock a week</Button>
      )}
      <Button size="sm" variant="quiet" onClick={() => onClose(note.trim() || 'No action')}>Close, no action</Button>
      {error && <span className={styles.error} role="alert">{error}</span>}
    </div>
  );
}

function ClientErrors({ api }: { api: AdminApi }) {
  const client = useQueryClient();
  const [showResolved, setShowResolved] = useState(false);
  const errors = useQuery({ queryKey: ['admin', 'errors', showResolved], queryFn: () => api.adminClientErrors(showResolved), refetchInterval: 15_000 });
  const resolve = useMutation({
    mutationFn: ({ id, resolved }: { id: number; resolved: boolean }) => api.adminClientErrorResolve(id, resolved),
    onSettled: () => client.invalidateQueries({ queryKey: ['admin', 'errors'] }),
  });
  const now = errors.dataUpdatedAt;
  return (
    <div className={styles.block}>
      <label className={styles.check}>
        <input type="checkbox" checked={showResolved} onChange={(event) => setShowResolved(event.target.checked)} /> Show resolved errors too
      </label>
      {errors.error && <p className={styles.error}>{String(errors.error)}</p>}
      {resolve.error && <p className={styles.error} role="alert">{resolve.error.message}</p>}
      {errors.data?.length === 0 && <p className={styles.quiet}>{showResolved ? 'No crash reports from web clients yet.' : 'Nothing open. Each kind of crash shows here once, with how often it happened.'}</p>}
      <ul className={styles.reports}>
        {(errors.data ?? []).map((error: ClientErrorGroup) => (
          <li key={error.id} className={styles.report}>
            <p>
              <strong>{error.message || 'No message'}</strong>
              <span className={styles.tag}>×{error.count ?? 1}</span>
              {error.regressed && <span className={styles.tag}>back again</span>}
              {error.resolved && <span className={styles.tag}>resolved</span>}
            </p>
            <p className={styles.quiet}>
              {error.source} · {error.path} · last {describeAge(error.lastSeen ?? 0, now)}, to {error.lastUser ?? 'a visitor'} on {error.appVersion ?? 'dev'}
              {(error.count ?? 1) > 1 && ` · first ${describeAge(error.firstSeen ?? 0, now)}`}
              {error.resolved && error.resolvedBy && ` · resolved by ${error.resolvedBy}`}
            </p>
            {error.stack && <details><summary>Stack</summary><pre>{error.stack}</pre></details>}
            {error.context && <p className={styles.quiet}>{error.context}</p>}
            <div className={styles.inline}>
              {error.resolved
                ? <Button size="sm" variant="quiet" onClick={() => resolve.mutate({ id: error.id!, resolved: false })}>Open again</Button>
                : <Button size="sm" onClick={() => resolve.mutate({ id: error.id!, resolved: true })}>Resolved</Button>}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Broadcast({ api }: { api: AdminApi }) {
  const [message, setMessage] = useState('');
  const [sent, setSent] = useState<string | null>(null);
  async function submit(event: FormEvent) {
    event.preventDefault();
    await api.adminSendBroadcastMessage(message.trim());
    setSent(message.trim());
    setMessage('');
  }
  return (
    <form className={styles.signIn} onSubmit={submit}>
      <p className={styles.lede}>Every connected player sees this as a message from the server. Start it with “warn” to show it in red.</p>
      <Field label="Message" value={message} maxLength={500} onChange={(event) => setMessage(event.target.value)} />
      <Button type="submit" variant="decision" disabled={!message.trim()}>Send to everyone</Button>
      {sent && <p className={styles.quiet} role="status">Sent: {sent}</p>}
    </form>
  );
}
