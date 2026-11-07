import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Check, Copy, Link2, Trash2, UserPlus, X } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { AccessOverview, AccessRequest, AllowEntry, Invite } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { ConfirmAction } from './ConfirmAction';
import { describeAge, formatDate, inviteLink, inviteState } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const ACCESS_KEY = ['admin', 'access'];

/**
 * Who may play here: open, or a contributors server with an allowlist, invite links and access requests; and the
 * notice everyone sees on sign-in and Home.
 */
export function AccessPage({ api }: { api: AdminApi }) {
  const access = useQuery({ queryKey: ACCESS_KEY, queryFn: () => api.adminAccess(), refetchInterval: 15_000 });
  if (access.error) return <p className={styles.error}>{String(access.error)}</p>;
  if (!access.data) return <p className={styles.quiet}>Loading…</p>;
  const data = access.data;
  const now = access.dataUpdatedAt;
  return (
    <div className={styles.overview}>
      <Mode api={api} data={data} />
      <Requests api={api} requests={data.requests ?? []} now={now} mail={!!data.mail} />
      <Invites api={api} invites={data.invites ?? []} now={now} />
      <Allowlist api={api} entries={data.allowlist ?? []} owner={data.owner ?? ''} now={now} closed={data.mode === 'allowlist'} />
      <Notice api={api} notice={data.notice ?? ''} />
    </div>
  );
}

function useRefresh() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: ACCESS_KEY });
}

function Mode({ api, data }: { api: AdminApi; data: AccessOverview }) {
  const refresh = useRefresh();
  const [switching, setSwitching] = useState(false);
  const [signedOut, setSignedOut] = useState<string[] | null>(null);
  const closed = data.mode === 'allowlist';
  const target = closed ? 'open' : 'allowlist';
  return (
    <section aria-labelledby="access-mode" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="access-mode">Who may play</h2>
        <span className={[styles.chip, closed ? styles.chipOn : ''].join(' ')}>{closed ? 'Contributors only' : 'Open to everyone'}</span>
      </div>
      <p className={styles.lede}>
        {closed
          ? 'Only the owner, staff, and the accounts and email addresses on the allowlist get in: at registration, sign-in, on remembered devices and restored sessions.'
          : 'Anyone who registers can play. Switch to an allowlist to make this a contributors server.'}
      </p>
      {!data.accounts && !closed && (
        <p className={styles.quiet}>An allowlist needs accounts: without them anyone can sign in with any name. Turn on XMAGE_AUTH first.</p>
      )}
      <p className={styles.quiet}>
        {data.github ? 'Sign in with GitHub is on.' : 'Sign in with GitHub is off (XMAGE_GITHUB_CLIENT_ID and XMAGE_GITHUB_CLIENT_SECRET).'}
        {data.githubRepo && (data.githubProblem
          ? <span className={styles.alarm}> The people of {data.githubRepo} couldn't be fetched: {data.githubProblem}</span>
          : ` ${data.githubPeople ?? 0} people of ${data.githubRepo} get in when they sign in with GitHub${data.githubSyncedAt ? ` (fetched ${formatDate(data.githubSyncedAt)})` : ''}.`)}
      </p>
      <p className={styles.quiet}>
        {data.oidc ? `Sign in with ${data.oidcName} is on.` : 'Sign in with the identity provider is off (XMAGE_OIDC_DISCOVERY, XMAGE_OIDC_CLIENT_ID and XMAGE_OIDC_CLIENT_SECRET).'}
      </p>
      {data.oidcIssuer && <p className={styles.quiet}>The identity provider's issuer is {data.oidcIssuer}.</p>}
      <p className={styles.quiet}>
        {data.oidcOnly ? `Sign-in is ${data.oidcName}-only (XMAGE_OIDC_ONLY).` : 'Sign-in with the identity provider only is off.'}
      </p>
      {signedOut && signedOut.length > 0 && <p className={styles.quiet} role="status">Signed out: {signedOut.join(', ')}</p>}
      {switching ? (
        <ConfirmAction
          title={closed ? 'Open the server' : 'Make it contributors only'}
          what={closed ? 'Anyone who registers will be able to play.' : 'Everyone not on the allowlist is refused from now on.'}
          preview={() => api.adminAccessPreview(target)}
          previewKey={['mode', target]}
          confirmLabel={closed ? 'Open it' : 'Close it'}
          onConfirm={async (reason) => {
            setSignedOut(await api.adminSetAccessMode(target, reason));
            setSwitching(false);
            await refresh();
          }}
          onCancel={() => setSwitching(false)}
        />
      ) : (
        <div>
          <Button size="sm" variant={closed ? 'quiet' : 'decision'} disabled={!closed && !data.accounts} onClick={() => setSwitching(true)}>
            {closed ? 'Open to everyone…' : 'Switch to an allowlist…'}
          </Button>
        </div>
      )}
    </section>
  );
}

function Requests({ api, requests, now, mail }: { api: AdminApi; requests: AccessRequest[]; now: number; mail: boolean }) {
  const refresh = useRefresh();
  const decide = useMutation({
    mutationFn: ({ id, approve }: { id: number; approve: boolean }) => api.adminAccessDecide(id, approve),
    onSettled: refresh,
  });
  const open = requests.filter((request) => request.status === 'open');
  const decided = requests.filter((request) => request.status !== 'open');
  return (
    <section aria-labelledby="access-requests" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="access-requests">Requests</h2>
        {open.length > 0 && <span className={[styles.chip, styles.chipOn].join(' ')}>{open.length} waiting</span>}
      </div>
      {decide.error && <p className={styles.error}>{String(decide.error)}</p>}
      {open.length === 0 && <p className={styles.quiet}>Nobody is waiting. People without an invite ask from the sign-in screen while the server is contributors only.</p>}
      <ul className={styles.reports}>
        {open.map((request) => (
          <li key={request.id} className={styles.report}>
            <p><strong>{request.name}</strong> · {request.email} · {describeAge(request.createdAt ?? 0, now)}</p>
            {request.note && <blockquote>{request.note}</blockquote>}
            <div className={styles.inline}>
              <Button size="sm" variant="decision" icon={<Check size={14} />} busy={decide.isPending && decide.variables?.id === request.id}
                onClick={() => decide.mutate({ id: request.id!, approve: true })}>Approve</Button>
              <Button size="sm" variant="quiet" icon={<X size={14} />} onClick={() => decide.mutate({ id: request.id!, approve: false })}>Decline</Button>
              <span className={styles.quiet}>Approving allowlists {request.email}{mail ? ' and emails them.' : '. The server has no mail, so tell them yourself.'}</span>
            </div>
          </li>
        ))}
      </ul>
      {decided.length > 0 && (
        <details>
          <summary className={styles.quiet}>Decided ({decided.length})</summary>
          <table className={styles.table}>
            <thead><tr><th>Name</th><th>Email</th><th>Decision</th><th>By</th><th>When</th></tr></thead>
            <tbody>
              {decided.map((request) => (
                <tr key={request.id}>
                  <td>{request.name}</td>
                  <td>{request.email}</td>
                  <td><span className={[styles.chip, request.status === 'approved' ? styles.chipOn : styles.chipOff].join(' ')}>{request.status}</span></td>
                  <td>{request.decidedBy}</td>
                  <td>{formatDate(request.decidedAt ?? 0)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      )}
    </section>
  );
}

function Invites({ api, invites, now }: { api: AdminApi; invites: Invite[]; now: number }) {
  const refresh = useRefresh();
  const [note, setNote] = useState('');
  const [uses, setUses] = useState(1);
  const [days, setDays] = useState(7);
  const [made, setMade] = useState<Invite | null>(null);
  const [copied, setCopied] = useState(false);
  const create = useMutation({
    mutationFn: () => api.adminInviteCreate(note.trim(), uses, days),
    onSuccess: (invite) => {
      setMade(invite);
      setCopied(false);
      setNote('');
    },
    onSettled: refresh,
  });
  const revoke = useMutation({ mutationFn: (code: string) => api.adminInviteRevoke(code), onSettled: refresh });
  const link = made?.code ? inviteLink(made.code, window.location.origin) : '';

  async function copy() {
    try {
      await navigator.clipboard.writeText(link);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    create.mutate();
  }

  return (
    <section aria-labelledby="access-invites" className={styles.block}>
      <h2 id="access-invites">Invite links</h2>
      <p className={styles.lede}>An invite registers an account and puts it on the allowlist in one step.</p>
      <form className={styles.inline} onSubmit={submit}>
        <Field label="Who it's for" value={note} maxLength={300} onChange={(event) => setNote(event.target.value)} placeholder="e.g. a GitHub handle" />
        <Field label="Uses" type="number" min={1} max={100} className={styles.number} value={uses} onChange={(event) => setUses(Number(event.target.value) || 1)} />
        <Field label="Days" type="number" min={1} max={90} className={styles.number} value={days} onChange={(event) => setDays(Number(event.target.value) || 1)} />
        <Button type="submit" size="sm" icon={<Link2 size={15} />} busy={create.isPending}>Make a link</Button>
      </form>
      {create.error && <p className={styles.error}>{String(create.error)}</p>}
      {made && (
        <div className={styles.inline} role="status">
          <code className={styles.mono}>{link}</code>
          <Button size="sm" variant="quiet" icon={<Copy size={14} />} onClick={copy}>{copied ? 'Copied' : 'Copy'}</Button>
        </div>
      )}
      {invites.length > 0 && (
        <table className={styles.table}>
          <thead><tr><th>For</th><th>Used</th><th>State</th><th>Expires</th><th>By</th><th><span className="visually-hidden">Actions</span></th></tr></thead>
          <tbody>
            {invites.map((invite) => {
              const state = inviteState(invite, now);
              return (
                <tr key={invite.code}>
                  <td>{invite.note || <span className={styles.quiet}>no note</span>}{invite.usedBy && <div className={styles.quiet}>{invite.usedBy}</div>}</td>
                  <td>{invite.uses} of {invite.maxUses}</td>
                  <td><span className={[styles.chip, state === 'active' ? styles.chipOn : ''].join(' ')}>{state}</span></td>
                  <td>{formatDate(invite.expiresAt ?? 0)}</td>
                  <td>{invite.createdBy}</td>
                  <td className={styles.actions}>
                    {state === 'active' && (
                      <Button size="sm" variant="quiet" icon={<X size={14} />} onClick={() => revoke.mutate(invite.code!)}>Revoke</Button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </section>
  );
}

function Allowlist({ api, entries, owner, now, closed }: { api: AdminApi; entries: AllowEntry[]; owner: string; now: number; closed: boolean }) {
  const refresh = useRefresh();
  const [who, setWho] = useState('');
  const [note, setNote] = useState('');
  const [filter, setFilter] = useState('');
  const [removing, setRemoving] = useState<AllowEntry | null>(null);
  const [signedOut, setSignedOut] = useState<string[] | null>(null);
  const add = useMutation({
    mutationFn: () => api.adminAllowAdd(who.trim(), note.trim()),
    onSuccess: () => {
      setWho('');
      setNote('');
    },
    onSettled: refresh,
  });
  const shown = entries.filter((entry) => {
    const text = `${entry.name ?? ''} ${entry.email ?? ''} ${entry.note ?? ''}`.toLowerCase();
    return !filter || text.includes(filter.toLowerCase());
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    add.mutate();
  }

  return (
    <section aria-labelledby="access-allowlist" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="access-allowlist">Allowlist</h2>
        <span className={styles.quiet}>{entries.length} {entries.length === 1 ? 'entry' : 'entries'}{owner ? ` · ${owner} is the owner and always allowed` : ''}</span>
      </div>
      {!closed && <p className={styles.quiet}>The server is open, so the list has no effect until it is switched to an allowlist.</p>}
      <form className={styles.inline} onSubmit={submit}>
        <Field label="Account name or email" value={who} maxLength={254} onChange={(event) => setWho(event.target.value)} />
        <Field label="Note" value={note} maxLength={300} onChange={(event) => setNote(event.target.value)} placeholder="e.g. a GitHub handle" />
        <Button type="submit" size="sm" icon={<UserPlus size={15} />} disabled={!who.trim()} busy={add.isPending}>Allow</Button>
      </form>
      {add.error && <p className={styles.error}>{String(add.error)}</p>}
      {signedOut && signedOut.length > 0 && <p className={styles.quiet} role="status">Signed out: {signedOut.join(', ')}</p>}
      {removing && (
        <ConfirmAction
          title={`Take ${removing.name ?? removing.email} off the list`}
          what={closed ? 'They can no longer sign in, and are signed out if they are online.' : 'The server is open, so nobody is signed out now.'}
          preview={() => api.adminAllowRemovePreview(removing.id!)}
          previewKey={['allow', removing.id]}
          confirmLabel="Take off"
          onConfirm={async (reason) => {
            setSignedOut(await api.adminAllowRemove(removing.id!, reason));
            setRemoving(null);
            await refresh();
          }}
          onCancel={() => setRemoving(null)}
        />
      )}
      {entries.length > 8 && <Field label="Find" value={filter} onChange={(event) => setFilter(event.target.value)} />}
      {entries.length === 0
        ? <p className={styles.quiet}>Nobody is listed yet. Add people by account name or email, or send them an invite link.</p>
        : (
          <table className={styles.table}>
            <thead><tr><th>Who</th><th>Note</th><th>Added</th><th><span className="visually-hidden">Actions</span></th></tr></thead>
            <tbody>
              {shown.map((entry) => (
                <tr key={entry.id}>
                  <td>{entry.name ?? entry.email}{entry.email && <span className={styles.tag}>email</span>}</td>
                  <td className={styles.clip}>{entry.note}</td>
                  <td>{entry.addedBy} · {describeAge(entry.addedAt ?? 0, now)}</td>
                  <td className={styles.actions}>
                    <Button size="sm" variant="quiet" icon={<Trash2 size={14} />} onClick={() => setRemoving(entry)}>Remove</Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
    </section>
  );
}

function Notice({ api, notice }: { api: AdminApi; notice: string }) {
  const refresh = useRefresh();
  const [text, setText] = useState(notice);
  const [saved, setSaved] = useState(false);
  const save = useMutation({
    mutationFn: () => api.adminSetNotice(text.trim()),
    onSuccess: () => setSaved(true),
    onSettled: refresh,
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    save.mutate();
  }
  return (
    <section aria-labelledby="access-notice" className={styles.block}>
      <h2 id="access-notice">Notice</h2>
      <form className={styles.block} onSubmit={submit}>
        <label className={styles.select} htmlFor="notice-text">
          <span>Shown on sign-in and Home. Empty hides it.</span>
        </label>
        <textarea id="notice-text" className={styles.textarea} value={text} maxLength={500}
          placeholder="Builds from master. Data may be reset."
          onChange={(event) => {
            setText(event.target.value);
            setSaved(false);
          }} />
        <div className={styles.inline}>
          <Button type="submit" size="sm" busy={save.isPending} disabled={text.trim() === notice}>Save the notice</Button>
          {saved && <span className={styles.quiet} role="status">Saved.</span>}
          {save.error && <span className={styles.error}>{String(save.error)}</span>}
        </div>
      </form>
    </section>
  );
}
