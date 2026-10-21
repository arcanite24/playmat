import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Ban, Download, KeyRound, Mic, MicOff, Power, ShieldCheck, Trash2, Unlock } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { AccountDetail, AccountRow, StaffResetCode } from '../../protocol/generated/views';
import { downloadText } from '../ui/download';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { AccountProgression } from './AccountProgression';
import { ConfirmAction } from './ConfirmAction';
import { atLeast, describeAge, formatDate, MUTE_CHOICES, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const PAGE = 50;

/** Every registered account: find one, then look into it and act on it, online or not. */
export function AccountsPage({ api, role }: { api: AdminApi; role: StaffRole }) {
  const [open, setOpen] = useState<string | null>(null);
  if (open) return <Account api={api} role={role} name={open} onBack={() => setOpen(null)} />;
  return <AccountList api={api} onOpen={setOpen} />;
}

function AccountList({ api, onOpen }: { api: AdminApi; onOpen(name: string): void }) {
  const [search, setSearch] = useState('');
  const [query, setQuery] = useState('');
  const [offset, setOffset] = useState(0);
  // ask the server once typing pauses
  useEffect(() => {
    const timer = setTimeout(() => {
      setQuery(search.trim());
      setOffset(0);
    }, 250);
    return () => clearTimeout(timer);
  }, [search]);
  const page = useQuery({
    queryKey: ['admin', 'accounts', query, offset],
    queryFn: () => api.adminAccounts(query || null, offset),
    placeholderData: keepPreviousData,
  });
  const total = page.data?.total ?? 0;
  const now = page.dataUpdatedAt;
  return (
    <div className={styles.block}>
      <div className={styles.toolbar}>
        <Field label="Find an account" value={search} placeholder="name or email" spellCheck={false} onChange={(event) => setSearch(event.target.value)} />
        <span className={styles.quiet} role="status">
          {page.data ? (total === 0 ? 'No accounts match.' : `${offset + 1}–${Math.min(offset + PAGE, total)} of ${total.toLocaleString()}`) : 'Loading…'}
        </span>
      </div>
      {page.error && <p className={styles.error}>{String(page.error)}</p>}
      <table className={styles.table}>
        <thead><tr><th>Account</th><th>Status</th><th>Last seen</th><th><span className="visually-hidden">Open</span></th></tr></thead>
        <tbody>
          {(page.data?.accounts ?? []).map((account) => (
            <tr key={account.name}>
              <td>
                <button type="button" className={styles.linkButton} onClick={() => onOpen(account.name!)}>{account.name}</button>
                {account.email && <div className={styles.quiet}>{account.email}{account.emailVerified ? ' ✓ confirmed' : ' · not confirmed'}</div>}
              </td>
              <td><StatusChips account={account} now={now} /></td>
              <td>{account.online ? 'online now' : account.lastConnection ? describeAge(account.lastConnection, now) : 'never'}</td>
              <td className={styles.actions}><Button size="sm" variant="quiet" onClick={() => onOpen(account.name!)}>Open</Button></td>
            </tr>
          ))}
        </tbody>
      </table>
      {total > PAGE && (
        <div className={styles.inline}>
          <Button size="sm" variant="quiet" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE))}>Previous</Button>
          <Button size="sm" variant="quiet" disabled={offset + PAGE >= total} onClick={() => setOffset(offset + PAGE)}>Next</Button>
        </div>
      )}
    </div>
  );
}

function StatusChips({ account, now }: { account: AccountRow; now: number }) {
  return (
    <span className={styles.chips}>
      {account.online && <span className={[styles.chip, styles.chipOn].join(' ')}>online</span>}
      {!account.active && <span className={[styles.chip, styles.chipOff].join(' ')}>deactivated</span>}
      {(account.lockedUntil ?? 0) > now && <span className={[styles.chip, styles.chipOff].join(' ')} title={`until ${formatDate(account.lockedUntil!)}`}>locked</span>}
      {(account.mutedUntil ?? 0) > now && <span className={styles.chip} title={`until ${formatDate(account.mutedUntil!)}`}>muted</span>}
      {account.staffRole && <span className={styles.chip}>{account.staffRole}</span>}
      {account.allowlisted && <span className={styles.chip}>listed</span>}
      {account.career && <span className={styles.chip}>Career</span>}
    </span>
  );
}

type Pending = 'lock' | 'deactivate' | 'delete' | 'reset';

function Account({ api, role, name, onBack }: { api: AdminApi; role: StaffRole; name: string; onBack(): void }) {
  const client = useQueryClient();
  const detail = useQuery({ queryKey: ['admin', 'account', name], queryFn: () => api.adminAccount(name) });
  const [minutes, setMinutes] = useState<number>(MUTE_CHOICES[1].minutes);
  const [pending, setPending] = useState<Pending | null>(null);
  const [code, setCode] = useState<StaffResetCode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const admin = atLeast(role, 'admin');
  const choice = MUTE_CHOICES.find((c) => c.minutes === minutes)?.label ?? `${minutes} minutes`;

  async function refresh() {
    await client.invalidateQueries({ queryKey: ['admin', 'account', name] });
    await client.invalidateQueries({ queryKey: ['admin', 'accounts'] });
  }
  async function run(work: () => Promise<unknown>) {
    setError(null);
    try {
      await work();
      await refresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }
  async function exportAccount() {
    await run(async () => downloadText(`${name}-account.json`, await api.adminAccountExport(name), 'application/json'));
  }

  if (detail.error) return <div className={styles.block}><BackLink onBack={onBack} /><p className={styles.error}>{String(detail.error)}</p></div>;
  if (!detail.data) return <p className={styles.quiet}>Loading…</p>;
  const data: AccountDetail = detail.data;
  const account = data.account ?? {};
  const now = detail.dataUpdatedAt;
  const muted = (account.mutedUntil ?? 0) > now;
  const locked = (account.lockedUntil ?? 0) > now;

  return (
    <div className={styles.overview}>
      <div className={styles.block}>
        <BackLink onBack={onBack} />
        <div className={styles.sectionHead}>
          <h2>{account.name}</h2>
          <StatusChips account={account} now={now} />
        </div>
        <p className={styles.quiet}>
          {account.email ? `${account.email} (${account.emailVerified ? 'confirmed' : 'not confirmed'}) · ` : ''}
          {account.online ? 'online now' : account.lastConnection ? `last seen ${describeAge(account.lastConnection, now)}` : 'never signed in'}
          {data.githubLogin ? ` · GitHub ${data.githubLogin}` : ''}
          {muted ? ` · muted until ${formatDate(account.mutedUntil!)}` : ''}
          {locked ? ` · locked until ${formatDate(account.lockedUntil!)}` : ''}
        </p>
        <dl className={[styles.numbers, styles.numbersTight].join(' ')}>
          <div className={styles.stat}><dt>Matches</dt><dd>{data.matches ?? 0}</dd></div>
          <div className={styles.stat}><dt>Matches left</dt><dd>{data.matchesQuit ?? 0}</dd></div>
          <div className={styles.stat}><dt>Events</dt><dd>{data.tourneys ?? 0}</dd></div>
          <div className={styles.stat}><dt>Rating</dt><dd>{data.rating || '—'}</dd></div>
          <div className={styles.stat}><dt>Decks on the server</dt><dd>{data.decks ?? 0}</dd></div>
        </dl>
      </div>

      <section className={styles.block} aria-labelledby="account-actions">
        <h2 id="account-actions">Actions</h2>
        <div className={styles.inline}>
          <label className={styles.select}>
            <span>{admin ? 'Mute or lock for' : 'Mute for'}</span>
            <select value={minutes} onChange={(event) => setMinutes(Number(event.target.value))}>
              {MUTE_CHOICES.map((c) => <option key={c.minutes} value={c.minutes}>{c.label}</option>)}
            </select>
          </label>
          {muted
            ? <Button size="sm" icon={<Mic size={14} />} onClick={() => run(() => api.adminMuteUser(name, 0))}>Unmute</Button>
            : <Button size="sm" icon={<MicOff size={14} />} onClick={() => run(() => api.adminMuteUser(name, minutes))}>Mute</Button>}
          {admin && (locked
            ? <Button size="sm" icon={<Unlock size={14} />} onClick={() => run(() => api.adminLockUser(name, 0))}>Unlock</Button>
            : <Button size="sm" icon={<Ban size={14} />} onClick={() => setPending('lock')}>Lock</Button>)}
          {admin && (account.active
            ? <Button size="sm" icon={<Power size={14} />} onClick={() => setPending('deactivate')}>Deactivate</Button>
            : <Button size="sm" icon={<ShieldCheck size={14} />} onClick={() => run(() => api.adminActivateUser(name, true))}>Activate</Button>)}
          {admin && <Button size="sm" icon={<KeyRound size={14} />} onClick={() => setPending('reset')}>Reset code</Button>}
          {admin && <Button size="sm" variant="quiet" icon={<Download size={14} />} onClick={exportAccount}>Export</Button>}
          {admin && <Button size="sm" variant="danger" icon={<Trash2 size={14} />} onClick={() => setPending('delete')}>Delete</Button>}
        </div>
        {error && <p className={styles.error} role="alert">{error}</p>}
        {code && (
          <div className={styles.confirm} role="status">
            <h3>Reset code for {name}</h3>
            <p className={styles.code}>{code.code}</p>
            <p>The player enters it under <strong>Forgot password</strong> with {code.email}. It works until {formatDate(code.expiresAt ?? 0)}, once.
              {code.emailed ? ' It was emailed to them as well.' : ' It was not emailed: pass it on yourself. It is not shown again.'}</p>
            <div className={styles.inline}><Button size="sm" variant="quiet" onClick={() => setCode(null)}>Done</Button></div>
          </div>
        )}
        {pending === 'lock' && (
          <ConfirmAction title={`Lock ${name} out`} what={`${name} is signed out and can't sign in for ${choice}.`} confirmLabel="Lock out"
            onCancel={() => setPending(null)} onConfirm={async (reason) => {
              await api.adminLockUser(name, minutes, reason);
              setPending(null);
              await refresh();
            }} />
        )}
        {pending === 'deactivate' && (
          <ConfirmAction title={`Deactivate ${name}`} what={`${name} is signed out and can't sign in until the account is activated again.`} confirmLabel="Deactivate"
            onCancel={() => setPending(null)} onConfirm={async (reason) => {
              await api.adminActivateUser(name, false, reason);
              setPending(null);
              await refresh();
            }} />
        )}
        {pending === 'reset' && (
          <ConfirmAction title={`Reset code for ${name}`} what="A one-time code that lets the player choose a new password. Their old password keeps working until they do." confirmLabel="Make the code"
            onCancel={() => setPending(null)} onConfirm={async (reason) => {
              setCode(await api.adminResetCode(name, reason));
              setPending(null);
              await refresh();
            }} />
        )}
        {pending === 'delete' && (
          <ConfirmAction title={`Delete ${name}`}
            what={<>The account goes with its decks, Career, reports, stats and feedback, and can't be brought back. <button type="button" className={styles.linkButton} onClick={exportAccount}>Export it first</button>.</>}
            confirmLabel="Delete for good"
            onCancel={() => setPending(null)} onConfirm={async (reason) => {
              await api.adminDeleteAccount(name, reason);
              await client.invalidateQueries({ queryKey: ['admin', 'accounts'] });
              onBack();
            }} />
        )}
      </section>

      {(data.sessions?.length ?? 0) > 0 && (
        <section className={styles.block} aria-labelledby="account-sessions">
          <h2 id="account-sessions">Sessions</h2>
          <table className={styles.table}>
            <thead><tr><th>Status</th>{admin && <th>Address</th>}<th>Playing</th><th>Connected</th><th>Last active</th></tr></thead>
            <tbody>
              {data.sessions!.map((session, index) => (
                <tr key={session.sessionId || index}>
                  <td>{session.state}</td>
                  {admin && <td>{session.host}</td>}
                  <td className={styles.clip}>{session.playing}</td>
                  <td>{describeAge(session.connectedAt ?? 0, now)}</td>
                  <td>{describeAge(session.lastActivity ?? 0, now)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section className={styles.block} aria-labelledby="account-reports">
        <h2 id="account-reports">Reports</h2>
        {(data.reports?.length ?? 0) === 0 ? <p className={styles.quiet}>No reports by or about {name}.</p> : (
          <ul className={styles.reports}>
            {data.reports!.map((report) => (
              <li key={report.id} className={styles.report}>
                <p>{report.reporter === name ? <>Reported <strong>{report.reported}</strong></> : <>Reported by {report.reporter}</>} for <strong>{report.reason}</strong> · {describeAge(report.createdAt ?? 0, now)}{report.open ? '' : ` · closed: ${report.resolution || 'no note'}`}</p>
                {report.details && <blockquote>{report.details}</blockquote>}
              </li>
            ))}
          </ul>
        )}
      </section>

      {(data.feedback?.length ?? 0) > 0 && (
        <section className={styles.block} aria-labelledby="account-feedback">
          <h2 id="account-feedback">Feedback sent</h2>
          <ul className={styles.list}>
            {data.feedback!.map((entry) => <li key={entry.id}>{entry.title} <span className={styles.quiet}>· {entry.status} · {describeAge(entry.createdAt ?? 0, now)}</span></li>)}
          </ul>
        </section>
      )}

      <AccountProgression api={api} role={role} name={account.name ?? name} now={now} />

      <section className={styles.block} aria-labelledby="account-audit">
        <h2 id="account-audit">History</h2>
        {(data.audit?.length ?? 0) === 0 ? <p className={styles.quiet}>Staff haven't acted on {name}.</p> : (
          <table className={styles.table}>
            <thead><tr><th>When</th><th>By</th><th>What</th><th>Detail</th><th>Reason</th></tr></thead>
            <tbody>
              {data.audit!.map((entry) => (
                <tr key={entry.id}><td>{formatDate(entry.at ?? 0)}</td><td>{entry.actor}</td><td>{entry.action}</td><td>{entry.detail}</td><td>{entry.reason}</td></tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

function BackLink({ onBack }: { onBack(): void }) {
  return (
    <div>
      <Button size="sm" variant="quiet" icon={<ArrowLeft size={14} />} onClick={onBack}>All accounts</Button>
    </div>
  );
}
