import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ExternalLink, LogOut, RefreshCw } from 'lucide-react';
import { useMemo, useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import {
  createSwarmAdminClient,
  DEFAULT_SWARM_URL,
  formatCredit,
  formatUnixAge,
  type TrustLevel,
} from '../swarm/swarmApi';
import swarmStyles from '../swarm/SwarmScreen.module.css';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import styles from './AdminScreen.module.css';

const URL_STORAGE_KEY = 'mana.admin.url';
const TOKEN_STORAGE_KEY = 'mana.admin.token';

type SwarmAdminTab = 'inbox' | 'rounds' | 'workers' | 'audit';
const TRUST_LEVELS: TrustLevel[] = ['probation', 'normal', 'reputable', 'trusted', 'banned'];

/**
 * Mana@Home coordinator administration tab inside `/admin#swarm`.
 * Connects directly to the coordinator using a staff admin token held in sessionStorage for the current tab.
 */
export function SwarmPage() {
  const [urlInput, setUrlInput] = useState(() => localStorage.getItem(URL_STORAGE_KEY) || DEFAULT_SWARM_URL);
  const [tokenInput, setTokenInput] = useState('');
  const [session, setSession] = useState<{ url: string; token: string } | null>(() => {
    const saved = sessionStorage.getItem(TOKEN_STORAGE_KEY);
    const savedUrl = localStorage.getItem(URL_STORAGE_KEY) || DEFAULT_SWARM_URL;
    return saved ? { url: savedUrl, token: saved } : null;
  });

  function connect(e: FormEvent) {
    e.preventDefault();
    const cleanUrl = urlInput.trim().replace(/\/+$/, '');
    const cleanToken = tokenInput.trim();
    if (!cleanUrl || !cleanToken) return;
    localStorage.setItem(URL_STORAGE_KEY, cleanUrl);
    sessionStorage.setItem(TOKEN_STORAGE_KEY, cleanToken);
    setTokenInput('');
    setSession({ url: cleanUrl, token: cleanToken });
  }

  function disconnect() {
    sessionStorage.removeItem(TOKEN_STORAGE_KEY);
    setSession(null);
  }

  if (!session) {
    return (
      <div className={styles.overview}>
        <div className={swarmStyles.toolbar}>
          <div>
            <h2 className={swarmStyles.cardTitle}>Mana@Home coordinator</h2>
            <p className={styles.quiet}>
              Sign in with a coordinator admin token (`playmat-swarm token --role admin`) to manage verification inbox,
              rounds, workers, trust tiers, quarantine, and credits.
            </p>
          </div>
          <Link to="/swarm" className={styles.back}>
            Public tracker <ExternalLink size={14} aria-hidden="true" />
          </Link>
        </div>

        <form className={styles.signIn} onSubmit={connect}>
          <Field
            label="Coordinator URL"
            type="url"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            required
          />
          <Field
            label="Admin bearer token"
            type="password"
            value={tokenInput}
            onChange={(e) => setTokenInput(e.target.value)}
            autoComplete="off"
            placeholder="mana_admin_..."
            required
          />
          <Button type="submit" variant="decision" disabled={!urlInput.trim() || !tokenInput.trim()}>
            Connect coordinator
          </Button>
        </form>
      </div>
    );
  }

  return <SwarmAdminConsole url={session.url} token={session.token} onDisconnect={disconnect} />;
}

function SwarmAdminConsole({
  url,
  token,
  onDisconnect,
}: {
  url: string;
  token: string;
  onDisconnect: () => void;
}) {
  const [tab, setTab] = useState<SwarmAdminTab>('inbox');
  const [notice, setNotice] = useState<string | null>(null);
  const client = useMemo(() => createSwarmAdminClient(url, token), [url, token]);
  const qc = useQueryClient();

  const statusQ = useQuery({
    queryKey: ['swarmAdmin', url, 'status'],
    queryFn: () => client.status(),
    refetchInterval: 15_000,
  });

  function refreshAll() {
    void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
  }

  return (
    <div className={styles.overview}>
      <div className={swarmStyles.toolbar}>
        <div>
          <h2 className={swarmStyles.cardTitle}>Mana@Home admin</h2>
          <p className={styles.quiet}>Connected to {url}</p>
        </div>
        <div className={swarmStyles.chips}>
          <Link to="/swarm" className={styles.back}>
            Public tracker <ExternalLink size={14} aria-hidden="true" />
          </Link>
          <Button size="sm" variant="quiet" icon={<RefreshCw size={14} />} onClick={refreshAll}>
            Refresh
          </Button>
          <Button size="sm" variant="quiet" icon={<LogOut size={14} />} onClick={onDisconnect}>
            Disconnect
          </Button>
        </div>
      </div>

      {notice && <p className={styles.quiet} role="status">{notice}</p>}
      {statusQ.isError && <p className={styles.error} role="alert">{String(statusQ.error)}</p>}

      {statusQ.data && (
        <dl className={swarmStyles.kpiGrid}>
          <div className={swarmStyles.kpi}>
            <dt>Active hosts</dt>
            <dd>{statusQ.data.hosts}</dd>
            <span className={swarmStyles.kpiNote}>{statusQ.data.threads} threads</span>
          </div>
          <div className={swarmStyles.kpi}>
            <dt>Games accepted</dt>
            <dd>{statusQ.data.games_accepted.toLocaleString()}</dd>
            <span className={swarmStyles.kpiNote}>{statusQ.data.games_last_hour} last hour</span>
          </div>
          <div className={swarmStyles.kpi}>
            <dt>Queue</dt>
            <dd>{statusQ.data.units.queued ?? 0}</dd>
            <span className={swarmStyles.kpiNote}>{statusQ.data.units.leased ?? 0} leased</span>
          </div>
          <div className={swarmStyles.kpi}>
            <dt>Open inbox</dt>
            <dd>{statusQ.data.open_inbox}</dd>
            <span className={swarmStyles.kpiNote}>{statusQ.data.units.quarantined ?? 0} quarantined units</span>
          </div>
        </dl>
      )}

      <div className={swarmStyles.segGroup} role="tablist" aria-label="Swarm admin sections">
        {(
          [
            ['inbox', `Inbox (${statusQ.data?.open_inbox ?? 0})`],
            ['rounds', 'Rounds & Queue'],
            ['workers', 'Workers & Tokens'],
            ['audit', 'Audit Log'],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? `${swarmStyles.segBtn} ${swarmStyles.segBtnActive}` : swarmStyles.segBtn}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      {tab === 'inbox' && <InboxSection url={url} client={client} onNotice={setNotice} />}
      {tab === 'rounds' && <RoundsSection url={url} client={client} onNotice={setNotice} />}
      {tab === 'workers' && <WorkersSection url={url} client={client} onNotice={setNotice} />}
      {tab === 'audit' && <AuditSection url={url} client={client} />}
    </div>
  );
}

type AdminClient = ReturnType<typeof createSwarmAdminClient>;

function InboxSection({
  url,
  client,
  onNotice,
}: {
  url: string;
  client: AdminClient;
  onNotice: (msg: string) => void;
}) {
  const [filter, setFilter] = useState<'open' | 'resolved' | 'dismissed'>('open');
  const [reason, setReason] = useState('');
  const qc = useQueryClient();

  const inboxQ = useQuery({
    queryKey: ['swarmAdmin', url, 'inbox', filter],
    queryFn: () => client.inbox(filter),
  });

  const resolveM = useMutation({
    mutationFn: (vars: { id: number; action: 'confirm-cheat' | 'false-positive' | 'order-rerun' | 'dismiss' }) =>
      client.resolveInbox(vars.id, vars.action, reason),
    onSuccess: (_data, vars) => {
      onNotice(`Inbox item #${vars.id} resolved (${vars.action}).`);
      setReason('');
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Resolve failed: ${String(e)}`),
  });

  return (
    <section className={swarmStyles.card}>
      <div className={swarmStyles.toolbar}>
        <h3 className={swarmStyles.cardTitle}>Verification inbox</h3>
        <div className={swarmStyles.segGroup} role="group" aria-label="Inbox status filter">
          {(['open', 'resolved', 'dismissed'] as const).map((st) => (
            <button
              key={st}
              type="button"
              className={filter === st ? `${swarmStyles.segBtn} ${swarmStyles.segBtnActive}` : swarmStyles.segBtn}
              onClick={() => setFilter(st)}
            >
              {st}
            </button>
          ))}
        </div>
      </div>

      {filter === 'open' && (
        <Field
          label="Resolution note (optional)"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Written into the audit log when resolving an item"
        />
      )}

      {inboxQ.isError ? (
        <p className={styles.error}>{String(inboxQ.error)}</p>
      ) : !inboxQ.data ? (
        <p className={styles.quiet}>Loading inbox...</p>
      ) : inboxQ.data.length === 0 ? (
        <p className={styles.quiet}>No {filter} items in the verification inbox.</p>
      ) : (
        <div className={swarmStyles.tableWrap}>
          <table className={swarmStyles.table}>
            <thead>
              <tr>
                <th>ID</th>
                <th>Kind</th>
                <th>Account / Unit</th>
                <th>Detail</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {inboxQ.data.map((item) => (
                <tr key={item.id}>
                  <td>#{item.id}</td>
                  <td><span className={`${swarmStyles.chip} ${swarmStyles.chipBad}`}>{item.kind}</span></td>
                  <td>
                    <div>{item.account ?? '-'} {item.host_name ? `(${item.host_name})` : ''}</div>
                    {item.unit && <div className={swarmStyles.mono}>{item.unit}</div>}
                  </td>
                  <td className={styles.quiet}>{item.detail}</td>
                  <td>
                    {item.status === 'open' ? (
                      <div className={swarmStyles.chips}>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={resolveM.isPending}
                          onClick={() => resolveM.mutate({ id: item.id, action: 'confirm-cheat' })}
                        >
                          Confirm cheat
                        </Button>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={resolveM.isPending}
                          onClick={() => resolveM.mutate({ id: item.id, action: 'false-positive' })}
                        >
                          False positive
                        </Button>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={resolveM.isPending}
                          onClick={() => resolveM.mutate({ id: item.id, action: 'order-rerun' })}
                        >
                          Order re-run
                        </Button>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={resolveM.isPending}
                          onClick={() => resolveM.mutate({ id: item.id, action: 'dismiss' })}
                        >
                          Dismiss
                        </Button>
                      </div>
                    ) : (
                      <span className={styles.quiet}>
                        {item.resolution ?? item.status} by {item.resolved_by ?? '-'}
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function RoundsSection({
  url,
  client,
  onNotice,
}: {
  url: string;
  client: AdminClient;
  onNotice: (msg: string) => void;
}) {
  const [priorityDrafts, setPriorityDrafts] = useState<Record<number, string>>({});
  const [unitIdInput, setUnitIdInput] = useState('');
  const qc = useQueryClient();

  const roundsQ = useQuery({
    queryKey: ['swarmAdmin', url, 'rounds'],
    queryFn: () => client.rounds(),
  });

  const priorityM = useMutation({
    mutationFn: ({ id, priority }: { id: number; priority: number }) => client.setPriority(id, priority),
    onSuccess: (r) => {
      onNotice(`Round ${r.id} priority set to ${r.priority}.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Priority update failed: ${String(e)}`),
  });

  const cancelM = useMutation({
    mutationFn: ({ id, includeLeased }: { id: number; includeLeased: boolean }) =>
      client.cancelRound(id, 'admin cancel', includeLeased),
    onSuccess: (res) => {
      onNotice(`Cancelled ${res.cancelled} unit(s) in round ${res.round.id}.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Cancel failed: ${String(e)}`),
  });

  const trainedM = useMutation({
    mutationFn: (id: number) => client.markTrained(id),
    onSuccess: (_d, id) => {
      onNotice(`Round ${id} marked trained.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Mark trained failed: ${String(e)}`),
  });

  const unitActionM = useMutation({
    mutationFn: async ({ id, action }: { id: string; action: 'reissue' | 'rerun' }) => {
      if (action === 'reissue') {
        await client.reissueUnit(id);
        return `Reissued ${id}.`;
      }
      const res = await client.orderRerun(id, 'admin ordered rerun');
      return `Queued re-run ${res.verify_unit} for ${id}.`;
    },
    onSuccess: (msg) => {
      onNotice(msg);
      setUnitIdInput('');
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Unit action failed: ${String(e)}`),
  });

  return (
    <>
      <section className={swarmStyles.card}>
        <h3 className={swarmStyles.cardTitle}>Rounds</h3>
        {roundsQ.isError ? (
          <p className={styles.error}>{String(roundsQ.error)}</p>
        ) : !roundsQ.data ? (
          <p className={styles.quiet}>Loading rounds...</p>
        ) : (
          <div className={swarmStyles.tableWrap}>
            <table className={swarmStyles.table}>
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Project / Run</th>
                  <th>Priority</th>
                  <th>Units</th>
                  <th className={swarmStyles.num}>Games</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {roundsQ.data.map((r) => {
                  const draft = priorityDrafts[r.id] ?? String(r.priority);
                  return (
                    <tr key={r.id}>
                      <td>#{r.id}</td>
                      <td>{r.project}/{r.run} r{r.round}</td>
                      <td>
                        <div className={swarmStyles.chips}>
                          <input
                            type="number"
                            aria-label={`Priority for round ${r.id}`}
                            value={draft}
                            onChange={(e) => setPriorityDrafts((d) => ({ ...d, [r.id]: e.target.value }))}
                            style={{ width: 72, padding: '4px 6px', background: 'var(--mat-900)', border: '1px solid var(--ink-faint)', borderRadius: 6 }}
                          />
                          <Button
                            size="sm"
                            variant="quiet"
                            disabled={priorityM.isPending}
                            onClick={() => priorityM.mutate({ id: r.id, priority: Number(draft) || 0 })}
                          >
                            Set
                          </Button>
                        </div>
                      </td>
                      <td className={styles.quiet}>
                        {Object.entries(r.units).map(([k, v]) => `${k}:${v}`).join(' · ')}
                        {r.verify_pending > 0 ? ` · verify:${r.verify_pending}` : ''}
                      </td>
                      <td className={swarmStyles.num}>{r.games.toLocaleString()}</td>
                      <td>
                        <div className={swarmStyles.chips}>
                          <Button
                            size="sm"
                            variant="quiet"
                            disabled={trainedM.isPending}
                            onClick={() => trainedM.mutate(r.id)}
                          >
                            Mark trained
                          </Button>
                          <Button
                            size="sm"
                            variant="quiet"
                            disabled={cancelM.isPending}
                            onClick={() => cancelM.mutate({ id: r.id, includeLeased: false })}
                          >
                            Cancel queued
                          </Button>
                          <Button
                            size="sm"
                            variant="quiet"
                            disabled={cancelM.isPending}
                            onClick={() => cancelM.mutate({ id: r.id, includeLeased: true })}
                          >
                            Cancel all
                          </Button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section className={swarmStyles.card}>
        <h3 className={swarmStyles.cardTitle}>Unit tools</h3>
        <div className={swarmStyles.toolbar}>
          <Field
            label="Unit ID"
            value={unitIdInput}
            onChange={(e) => setUnitIdInput(e.target.value)}
            placeholder="e.g. main-r1-u000"
          />
          <div className={swarmStyles.chips}>
            <Button
              size="sm"
              variant="quiet"
              disabled={!unitIdInput.trim() || unitActionM.isPending}
              onClick={() => unitActionM.mutate({ id: unitIdInput.trim(), action: 'rerun' })}
            >
              Order re-run
            </Button>
            <Button
              size="sm"
              variant="quiet"
              disabled={!unitIdInput.trim() || unitActionM.isPending}
              onClick={() => unitActionM.mutate({ id: unitIdInput.trim(), action: 'reissue' })}
            >
              Reissue unit
            </Button>
          </div>
        </div>
      </section>
    </>
  );
}

function WorkersSection({
  url,
  client,
  onNotice,
}: {
  url: string;
  client: AdminClient;
  onNotice: (msg: string) => void;
}) {
  const [search, setSearch] = useState('');
  const [newName, setNewName] = useState('');
  const [newRole, setNewRole] = useState<'worker' | 'trainer' | 'admin'>('worker');
  const [newTrust, setNewTrust] = useState<TrustLevel>('probation');
  const [issuedToken, setIssuedToken] = useState<string | null>(null);
  const [creditDelta, setCreditDelta] = useState('');
  const [creditReason, setCreditReason] = useState('');
  const qc = useQueryClient();

  const workersQ = useQuery({
    queryKey: ['swarmAdmin', url, 'workers'],
    queryFn: () => client.workers(),
  });

  const mintM = useMutation({
    mutationFn: () => client.newToken(newRole, newName.trim(), newTrust),
    onSuccess: (res) => {
      setIssuedToken(res.token);
      setNewName('');
      onNotice(`Minted ${newRole} token #${res.id}. Copy it now; it is shown only once.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Mint token failed: ${String(e)}`),
  });

  const trustM = useMutation({
    mutationFn: ({ id, trust }: { id: number; trust: TrustLevel }) => client.setTrust(id, trust, 'admin panel'),
    onSuccess: (_d, vars) => {
      onNotice(`Token #${vars.id} trust set to ${vars.trust}.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Trust update failed: ${String(e)}`),
  });

  const quarantineM = useMutation({
    mutationFn: ({ id, restore }: { id: number; restore: boolean }) =>
      restore ? client.restoreToken(id, 'admin restore') : client.quarantineToken(id, 'admin quarantine'),
    onSuccess: (res, vars) => {
      onNotice(
        `${vars.restore ? 'Restored' : 'Quarantined'} token #${vars.id}: ${res.units_affected} unit(s), ${res.credits_affected} credit entry(ies).`,
      );
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Action failed: ${String(e)}`),
  });

  const revokeM = useMutation({
    mutationFn: (id: number) => client.revokeToken(id),
    onSuccess: (_d, id) => {
      onNotice(`Revoked token #${id}.`);
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Revoke failed: ${String(e)}`),
  });

  const creditM = useMutation({
    mutationFn: ({ id, credit, reason }: { id: number; credit: number; reason: string }) =>
      client.adjustCredit(id, credit, reason),
    onSuccess: (_d, vars) => {
      onNotice(`Adjusted token #${vars.id} credit by ${vars.credit}.`);
      setCreditDelta('');
      setCreditReason('');
      void qc.invalidateQueries({ queryKey: ['swarmAdmin', url] });
    },
    onError: (e) => onNotice(`Credit adjustment failed: ${String(e)}`),
  });

  const filtered = (workersQ.data ?? []).filter((w) => {
    const q = search.trim().toLowerCase();
    if (!q) return true;
    return w.name.toLowerCase().includes(q) || w.hosts.some((h) => h.name.toLowerCase().includes(q));
  });

  return (
    <>
      <section className={swarmStyles.card}>
        <h3 className={swarmStyles.cardTitle}>Issue token</h3>
        <div className={swarmStyles.toolbar}>
          <Field label="Account name" value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="contributor-name" />
          <label className={styles.quiet}>
            Role{' '}
            <select value={newRole} onChange={(e) => setNewRole(e.target.value as 'worker' | 'trainer' | 'admin')}>
              <option value="worker">worker</option>
              <option value="trainer">trainer</option>
              <option value="admin">admin</option>
            </select>
          </label>
          <label className={styles.quiet}>
            Trust{' '}
            <select value={newTrust} onChange={(e) => setNewTrust(e.target.value as TrustLevel)}>
              {TRUST_LEVELS.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
          </label>
          <Button size="sm" variant="decision" disabled={!newName.trim() || mintM.isPending} onClick={() => mintM.mutate()}>
            Create token
          </Button>
        </div>
        {issuedToken && (
          <div className={swarmStyles.codeBlock}>
            <pre>{issuedToken}</pre>
            <button
              type="button"
              className={swarmStyles.copyBtn}
              onClick={() => void navigator.clipboard.writeText(issuedToken)}
              aria-label="Copy newly issued token"
            >
              Copy token
            </button>
          </div>
        )}
      </section>

      <section className={swarmStyles.card}>
        <div className={swarmStyles.toolbar}>
          <h3 className={swarmStyles.cardTitle}>Accounts and hosts ({filtered.length})</h3>
          <Field label="Filter accounts" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search account or host" />
        </div>

        <div className={swarmStyles.toolbar}>
          <Field label="Credit delta" value={creditDelta} onChange={(e) => setCreditDelta(e.target.value)} placeholder="e.g. 50 or -25" />
          <Field label="Credit reason (required)" value={creditReason} onChange={(e) => setCreditReason(e.target.value)} placeholder="Reason for manual credit adjustment" />
        </div>

        {workersQ.isError ? (
          <p className={styles.error}>{String(workersQ.error)}</p>
        ) : !workersQ.data ? (
          <p className={styles.quiet}>Loading workers...</p>
        ) : (
          <div className={swarmStyles.tableWrap}>
            <table className={swarmStyles.table}>
              <thead>
                <tr>
                  <th>Account</th>
                  <th>Trust</th>
                  <th className={swarmStyles.num}>Units / Miss</th>
                  <th className={swarmStyles.num}>Credit</th>
                  <th>Hosts</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((w) => (
                  <tr key={w.id}>
                    <td>
                      <strong>{w.name}</strong> <span className={styles.quiet}>({w.role}{w.revoked ? ', revoked' : ''})</span>
                    </td>
                    <td>
                      <select
                        aria-label={`Trust tier for ${w.name}`}
                        value={w.trust}
                        onChange={(e) => trustM.mutate({ id: w.id, trust: e.target.value as TrustLevel })}
                      >
                        {TRUST_LEVELS.map((t) => <option key={t} value={t}>{t}</option>)}
                      </select>
                    </td>
                    <td className={swarmStyles.num}>
                      {w.accepted_units} / {w.mismatches}
                    </td>
                    <td className={swarmStyles.num}>
                      <div>{formatCredit(w.credit_total)}</div>
                      {w.credit_frozen > 0 && <div className={styles.alarm}>{formatCredit(w.credit_frozen)} frozen</div>}
                    </td>
                    <td className={styles.quiet}>
                      {w.hosts.length === 0 ? '-' : w.hosts.map((h) => `${h.name} (${h.os}, ${h.threads}t)`).join(', ')}
                    </td>
                    <td>
                      <div className={swarmStyles.chips}>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={quarantineM.isPending}
                          onClick={() => quarantineM.mutate({ id: w.id, restore: false })}
                        >
                          Quarantine
                        </Button>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={quarantineM.isPending}
                          onClick={() => quarantineM.mutate({ id: w.id, restore: true })}
                        >
                          Restore
                        </Button>
                        <Button
                          size="sm"
                          variant="quiet"
                          disabled={!creditDelta.trim() || !creditReason.trim() || creditM.isPending}
                          onClick={() => creditM.mutate({ id: w.id, credit: Number(creditDelta), reason: creditReason.trim() })}
                        >
                          Adjust credit
                        </Button>
                        {!w.revoked && (
                          <Button
                            size="sm"
                            variant="quiet"
                            disabled={revokeM.isPending}
                            onClick={() => revokeM.mutate(w.id)}
                          >
                            Revoke
                          </Button>
                        )}
                      </div>
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

function AuditSection({ url, client }: { url: string; client: AdminClient }) {
  const [action, setAction] = useState('');
  const [actor, setActor] = useState('');

  const auditQ = useQuery({
    queryKey: ['swarmAdmin', url, 'audit', action, actor],
    queryFn: () => client.audit({ action: action.trim() || undefined, actor: actor.trim() || undefined, limit: 100 }),
  });

  return (
    <section className={swarmStyles.card}>
      <div className={swarmStyles.toolbar}>
        <h3 className={swarmStyles.cardTitle}>Coordinator audit log</h3>
        <div className={swarmStyles.chips}>
          <Field label="Action" value={action} onChange={(e) => setAction(e.target.value)} placeholder="e.g. trust, cancel" />
          <Field label="Actor" value={actor} onChange={(e) => setActor(e.target.value)} placeholder="e.g. owner" />
        </div>
      </div>

      {auditQ.isError ? (
        <p className={styles.error}>{String(auditQ.error)}</p>
      ) : !auditQ.data ? (
        <p className={styles.quiet}>Loading audit log...</p>
      ) : auditQ.data.length === 0 ? (
        <p className={styles.quiet}>No audit entries match.</p>
      ) : (
        <div className={swarmStyles.tableWrap}>
          <table className={swarmStyles.table}>
            <thead>
              <tr>
                <th>Time</th>
                <th>Actor</th>
                <th>Action</th>
                <th>Detail</th>
              </tr>
            </thead>
            <tbody>
              {auditQ.data.map((ev) => (
                <tr key={ev.id}>
                  <td className={styles.quiet}>{formatUnixAge(ev.at)}</td>
                  <td>{ev.actor}</td>
                  <td><span className={swarmStyles.chip}>{ev.action}</span></td>
                  <td className={swarmStyles.mono}>{ev.detail}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
