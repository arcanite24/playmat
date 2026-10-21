import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query';
import { Gift, Link2, Undo2 } from 'lucide-react';
import { useEffect, useId, useState, type FormEvent } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { Payment } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { ConfirmAction } from './ConfirmAction';
import { formatDate, money } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const PROVIDERS = [
  { value: '', label: 'All' },
  { value: 'kofi', label: 'Ko-fi' },
  { value: 'github', label: 'GitHub Sponsors' },
] as const;

function thisMonth(): string {
  return new Date().toISOString().slice(0, 7);
}

/**
 * Support for the server (admins and the owner; moderators don't see payments): a month of payments against the goal,
 * the unmatched ones to link to an account by hand, revoking a refunded payment and giving the thank-you by hand. Every
 * action goes in the audit log.
 */
export function SupportPage({ api }: { api: AdminApi }) {
  const [month, setMonth] = useState(thisMonth);
  const [unmatched, setUnmatched] = useState(false);
  const [provider, setProvider] = useState('');
  const [granting, setGranting] = useState(false);
  const ledger = useQuery({
    queryKey: ['admin', 'support', month, unmatched, provider],
    queryFn: () => api.adminSupportLedger(unmatched ? null : month, unmatched, provider || null),
    placeholderData: keepPreviousData,
    refetchInterval: 60_000,
  });
  const data = ledger.data;
  const payments = data?.payments ?? [];

  return (
    <div className={styles.block}>
      <div className={styles.toolbar}>
        <label className={styles.select}>
          Month (UTC)
          <input type="month" value={month} disabled={unmatched} onChange={(event) => setMonth(event.target.value || thisMonth())} />
        </label>
        <label className={styles.select}>
          Provider
          <select value={provider} onChange={(event) => setProvider(event.target.value)}>
            {PROVIDERS.map((choice) => <option key={choice.value} value={choice.value}>{choice.label}</option>)}
          </select>
        </label>
        <label className={styles.check}>
          <input type="checkbox" checked={unmatched} onChange={(event) => setUnmatched(event.target.checked)} />
          Unmatched only{data?.unmatched ? ` (${data.unmatched})` : ''}
        </label>
        <Button size="sm" variant="quiet" icon={<Gift size={14} />} onClick={() => setGranting(true)}>Give the thank-you by hand</Button>
      </div>
      {granting && <Grant api={api} onDone={() => setGranting(false)} />}
      {ledger.error && <p className={styles.error}>{String(ledger.error)}</p>}

      {data && !unmatched && (
        <dl className={styles.numbers}>
          <div className={styles.stat}><dt>{data.month} total</dt><dd>{money(data.monthCents ?? 0)}</dd></div>
          <div className={styles.stat}><dt>Goal</dt><dd>{money(data.goalCents ?? 0)}</dd></div>
          <div className={styles.stat}><dt>Supporters</dt><dd>{data.supporters ?? 0}</dd></div>
          <div className={styles.stat}><dt>Unmatched (all months)</dt><dd>{data.unmatched ?? 0}</dd></div>
        </dl>
      )}
      {(data?.otherCurrencies ?? []).length > 0 && (
        <p className={styles.alarm}>
          Not counted toward the goal (not US dollars): {data!.otherCurrencies!.join(', ')}. Set the Ko-fi page to USD.
        </p>
      )}

      {data && payments.length === 0 && (
        <p className={styles.quiet}>{unmatched ? 'Every payment found its account.' : 'No payments this month yet.'}</p>
      )}
      {payments.length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr><th>Received</th><th>Amount</th><th>From</th><th>Account</th><th><span className="visually-hidden">Actions</span></th></tr>
          </thead>
          <tbody>
            {payments.map((payment) => <PaymentRow key={payment.id} api={api} payment={payment} />)}
          </tbody>
        </table>
      )}
    </div>
  );
}

function PaymentRow({ api, payment }: { api: AdminApi; payment: Payment }) {
  const client = useQueryClient();
  const [linking, setLinking] = useState(false);
  const [revoking, setRevoking] = useState(false);
  const [removePerks, setRemovePerks] = useState(false);
  const refresh = () => client.invalidateQueries({ queryKey: ['admin', 'support'] });
  return (
    <>
      <tr className={payment.revoked ? styles.revoked : undefined}>
        <td>{formatDate(payment.receivedAt ?? 0)}</td>
        <td>
          {money(payment.amountCents ?? 0, payment.currency)}
          <div className={styles.quiet}>{payment.provider === 'github' ? 'GitHub' : 'Ko-fi'} · {payment.kind}{payment.tier ? ` · ${payment.tier}` : ''}</div>
        </td>
        <td className={styles.clip} title={payment.message ?? undefined}>
          {payment.fromName || '—'}{payment.isPublic ? '' : ' (private)'}
          {payment.message && <div className={styles.quiet}>{payment.message}</div>}
        </td>
        <td>
          {payment.account
            ? <>{payment.account}<div className={styles.quiet}>{payment.linkedBy === 'staff' ? 'linked by staff' : 'linked automatically'}</div></>
            : <span className={styles.tag}>unmatched</span>}
          {payment.revoked && <div className={styles.alarm}>revoked: {payment.revokedReason}</div>}
        </td>
        <td className={styles.actions}>
          {!payment.account && !payment.revoked && (
            <Button size="sm" variant="quiet" icon={<Link2 size={14} />} onClick={() => setLinking((open) => !open)}>Link to account</Button>
          )}
          {!payment.revoked && (
            <Button size="sm" variant="quiet" icon={<Undo2 size={14} />} onClick={() => setRevoking(true)}>Revoke</Button>
          )}
        </td>
      </tr>
      {linking && (
        <tr><td colSpan={5}><LinkPayment api={api} payment={payment} onDone={() => { setLinking(false); void refresh(); }} /></td></tr>
      )}
      {revoking && (
        <tr>
          <td colSpan={5}>
            <ConfirmAction
              title={`Revoke payment ${payment.id}`}
              what={(
                <>
                  For a refund or a chargeback: it stops counting toward the month. The thank-you it gave stays unless you choose
                  otherwise (and stays anyway while another payment of the account stands for it).{' '}
                  <label className={styles.check}>
                    <input type="checkbox" checked={removePerks} onChange={(event) => setRemovePerks(event.target.checked)} />
                    Take back the thank-you it gave
                  </label>
                </>
              )}
              confirmLabel="Revoke"
              onConfirm={async (reason) => {
                await api.adminSupportRevoke(payment.id!, reason, removePerks);
                setRevoking(false);
                await refresh();
              }}
              onCancel={() => setRevoking(false)}
            />
          </td>
        </tr>
      )}
    </>
  );
}

/** Find the account a payment belongs to (by name or email) and link it; the thank-you follows as if it linked itself. */
function LinkPayment({ api, payment, onDone }: { api: AdminApi; payment: Payment; onDone(): void }) {
  const listId = useId();
  const [name, setName] = useState('');
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setQuery(name.trim()), 250);
    return () => clearTimeout(timer);
  }, [name]);
  const matches = useQuery({
    queryKey: ['admin', 'accounts', query, 0],
    queryFn: () => api.adminAccounts(query || null, 0),
    enabled: query.length >= 2,
  });

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.adminSupportLink(payment.id!, name.trim());
      onDone();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className={styles.confirm} onSubmit={submit} aria-label={`Link payment ${payment.id}`}>
      <div className={styles.inline}>
        <Field label="Account (name or email)" value={name} list={listId} autoFocus spellCheck={false} onChange={(event) => setName(event.target.value)} />
        <datalist id={listId}>
          {(matches.data?.accounts ?? []).map((account) => (
            <option key={account.name} value={account.name}>{account.email ?? ''}</option>
          ))}
        </datalist>
        <Button type="submit" size="sm" busy={busy} disabled={!name.trim()}>Link</Button>
        <Button type="button" size="sm" variant="quiet" onClick={onDone}>Cancel</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
    </form>
  );
}

/** The thank-you set for support that came some other way (a bank transfer, a payment that can't be linked). */
function Grant({ api, onDone }: { api: AdminApi; onDone(): void }) {
  const [name, setName] = useState('');
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.adminSupportGrant(name.trim(), reason.trim());
      setDone(name.trim());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className={styles.confirm} onSubmit={submit} aria-label="Give the thank-you by hand">
      <h3>Give the thank-you by hand</h3>
      <div className={styles.inline}>
        <Field label="Account" value={name} required spellCheck={false} onChange={(event) => setName(event.target.value)} />
        <Field label="Reason (kept in the audit log)" value={reason} maxLength={500} required onChange={(event) => setReason(event.target.value)} />
      </div>
      {done && <p className={styles.quiet} role="status">{done} has the thank-you set; they see it on their next visit.</p>}
      {error && <p className={styles.error} role="alert">{error}</p>}
      <div className={styles.inline}>
        <Button type="submit" size="sm" busy={busy} disabled={!name.trim() || !reason.trim()}>Give</Button>
        <Button type="button" size="sm" variant="quiet" onClick={onDone}>Close</Button>
      </div>
    </form>
  );
}
