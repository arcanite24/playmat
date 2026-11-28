import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { api } from '../connection';
import { t as translate, useT } from '../i18n';
import { useEmailStatus } from '../queries';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { downloadText } from '../ui/download';
import { Field } from '../ui/Field';
import { EmailCodeForm } from './EmailConfirm';
import { SupporterSettings } from '../support/SupporterSettings';
import styles from './SettingsDialog.module.css';

/** Settings, Account: the account's email and confirming it, supporting the server, downloading its data, deleting it. */
export function AccountSettings() {
  const t = useT();
  const status = useEmailStatus();
  const email = status.data;
  const userName = useSession((state) => state.userName);
  const [downloading, setDownloading] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  async function download() {
    setDownloading(true);
    setProblem(null);
    try {
      const json = await api.accountExport();
      downloadText(`${userName}-data.json`, json, 'application/json');
    } catch (error) {
      setProblem(error instanceof Error ? error.message : String(error));
    } finally {
      setDownloading(false);
    }
  }

  return (
    <>
      <section className={styles.accountSection} aria-labelledby="account-email">
        <h3 id="account-email" className={styles.rowLabel}>{t('settings.account.email')}</h3>
        {status.isError && <p className={styles.rowDetail} role="alert">{status.error instanceof Error ? status.error.message : String(status.error)}</p>}
        {email && (
          <>
            <p className={styles.rowDetail}>
              {email.email
                ? t(email.verified ? 'settings.account.email.verified' : 'settings.account.email.unverified', { email: email.email })
                : t('settings.account.email.none')}
            </p>
            {email.email && !email.verified && (email.mail ? <EmailCodeForm /> : <p className={styles.rowDetail}>{t('settings.account.email.noMail')}</p>)}
          </>
        )}
      </section>

      <SupporterSettings />

      <section className={styles.accountSection} aria-labelledby="account-data">
        <h3 id="account-data" className={styles.rowLabel}>{t('settings.account.data')}</h3>
        <p className={styles.rowDetail}>{t('settings.account.data.detail')}</p>
        <div className={styles.accountActions}>
          <Button variant="print" size="sm" busy={downloading} onClick={() => void download()}>{t('settings.account.download')}</Button>
        </div>
        {problem && <p className={styles.rowDetail} role="alert">{problem}</p>}
      </section>

      <section className={styles.accountSection} aria-labelledby="account-delete">
        <h3 id="account-delete" className={styles.rowLabel}>{t('settings.account.delete')}</h3>
        <p className={styles.rowDetail}>{t('settings.account.delete.detail')}</p>
        <div className={styles.accountActions}>
          <Button variant="danger" size="sm" onClick={() => setDeleting(true)}>{t('settings.account.delete')}</Button>
        </div>
      </section>

      <section className={styles.accountSection} aria-labelledby="account-legal">
        <h3 id="account-legal" className={styles.rowLabel}>{t('settings.account.legal')}</h3>
        <p className={styles.accountActions}>
          <Link to="/privacy">{t('legal.privacyLink')}</Link>
          <Link to="/terms">{t('legal.termsLink')}</Link>
        </p>
      </section>

      <DeleteAccountDialog open={deleting} onOpenChange={setDeleting} userName={userName} github={!!email?.github} oidc={!!email?.oidc} />
    </>
  );
}

/** The last step: the account's name typed out, and its password (not needed for an account that signs in with GitHub or the identity provider). */
function DeleteAccountDialog({ open, onOpenChange, userName, github, oidc }: { open: boolean; onOpenChange(open: boolean): void; userName: string; github: boolean; oidc: boolean }) {
  const t = useT();
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [working, setWorking] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const ready = name.trim() === userName && (github || oidc || !!password);

  async function remove(event: FormEvent) {
    event.preventDefault();
    if (!ready) return;
    setWorking(true);
    setProblem(null);
    // the deletion kills the session server-side, so the provider's end-session URL is fetched first: without
    // the hop, a warm provider session lets the same provider user silently re-create the deleted account on
    // the next button click. The server resolves the hint (this sign-in's token, or the account's last one),
    // so the sign-in kind doesn't matter here
    let logoutUrl: string | null = null;
    if (useSession.getState().phase === 'signedIn') {
      try {
        logoutUrl = (await api.oidcLogout())?.url ?? null;
      } catch {
        // the sign-out stands anyway
      }
    }
    try {
      await api.accountDelete(password, name.trim());
    } catch (error) {
      setProblem(error instanceof Error ? error.message : String(error));
      setWorking(false);
      return;
    }
    if (logoutUrl) {
      window.location.assign(logoutUrl);
      return;
    }
    // gone: to the sign-in screen, which says so, and this device forgets the account
    navigate('/login', { replace: true, state: { notice: translate('login.deleted') } });
    await useSession.getState().signOut();
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (working) return;
        onOpenChange(next);
        if (!next) {
          setName('');
          setPassword('');
          setProblem(null);
        }
      }}
      title={t('settings.account.delete.title', { name: userName })}
      width="sm"
    >
      <form className={styles.accountSection} onSubmit={remove}>
        <p>{t('settings.account.delete.text')}</p>
        {(github || oidc) && <p className={styles.rowDetail}>{t(github ? 'settings.account.delete.github' : 'settings.account.delete.oidc')}</p>}
        <Field label={t('settings.account.delete.name')} value={name} autoComplete="off" spellCheck={false} onChange={(event) => setName(event.target.value)} />
        <Field label={t('settings.account.delete.password')} type="password" value={password} autoComplete="current-password" onChange={(event) => setPassword(event.target.value)} />
        {problem && <p className={styles.rowDetail} role="alert">{problem}</p>}
        <div className={styles.accountActions}>
          <Button type="submit" variant="danger" busy={working} disabled={!ready}>{t('settings.account.delete.confirm')}</Button>
          <Button variant="quiet" disabled={working} onClick={() => onOpenChange(false)}>{t('settings.account.delete.cancel')}</Button>
        </div>
      </form>
    </Dialog>
  );
}
