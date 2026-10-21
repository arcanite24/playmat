import { useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import { api } from '../connection';
import { useT } from '../i18n';
import { EMAIL_STATUS, useEmailStatus } from '../queries';
import { readJson, writeJson } from '../stores/persist';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import styles from './EmailConfirm.module.css';

const DISMISSED_KEY = 'playmat.emailBanner.dismissed';

/** Whoever this device dismissed the banner for, per server and account. */
function dismissKey(): string {
  const { serverUrl, userName } = useSession.getState();
  return `${serverUrl}|${userName.toLowerCase()}`;
}

/**
 * On Home until the address is confirmed or the player closes it: what confirming is for, and the code form. Settings,
 * Account has the same form for good.
 */
export function EmailBanner() {
  const t = useT();
  const status = useEmailStatus().data;
  const [dismissed, setDismissed] = useState(() => readJson<string[]>(DISMISSED_KEY, []).includes(dismissKey()));
  if (!status?.email || status.verified || !status.mail || dismissed) return null;

  function dismiss() {
    writeJson(DISMISSED_KEY, [...readJson<string[]>(DISMISSED_KEY, []).slice(-20), dismissKey()]);
    setDismissed(true);
  }

  return (
    <section className={styles.banner} aria-labelledby="email-banner-title">
      <div className={styles.bannerHead}>
        <h3 id="email-banner-title" className={styles.bannerTitle}>{t('email.banner.title')}</h3>
        <button type="button" className={styles.close} onClick={dismiss} aria-label={t('email.banner.dismiss')}>
          <X size={16} aria-hidden="true" />
        </button>
      </div>
      <p className={styles.text}>{t('email.banner.text', { email: status.email })}</p>
      <EmailCodeForm />
    </section>
  );
}

/** The mailed code, typed back; a new code on request (once a minute). */
export function EmailCodeForm() {
  const t = useT();
  const queryClient = useQueryClient();
  const [code, setCode] = useState('');
  const [working, setWorking] = useState<'verify' | 'send' | null>(null);
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null);

  async function verify(event: FormEvent) {
    event.preventDefault();
    if (code.length !== 6) return;
    setWorking('verify');
    setMessage(null);
    try {
      if (await api.accountEmailVerify(code)) {
        setMessage({ error: false, text: t('email.confirmed') });
        await queryClient.invalidateQueries({ queryKey: EMAIL_STATUS });
      } else {
        setMessage({ error: true, text: t('email.wrongCode') });
      }
    } catch (error) {
      setMessage({ error: true, text: error instanceof Error ? error.message : String(error) });
    } finally {
      setWorking(null);
    }
  }

  async function send() {
    setWorking('send');
    setMessage(null);
    try {
      await api.accountEmailVerifyStart();
      setCode('');
      setMessage({ error: false, text: t('email.sent') });
    } catch (error) {
      setMessage({ error: true, text: error instanceof Error ? error.message : String(error) });
    } finally {
      setWorking(null);
    }
  }

  return (
    <form className={styles.form} onSubmit={verify}>
      <Field
        label={t('email.code')}
        value={code}
        inputMode="numeric"
        autoComplete="one-time-code"
        maxLength={6}
        onChange={(event) => setCode(event.target.value.replace(/\D/g, ''))}
        hint={t('email.codeHint')}
        className={styles.code}
      />
      <div className={styles.actions}>
        <Button type="submit" variant="print" size="sm" busy={working === 'verify'} disabled={code.length !== 6 || working !== null}>
          {t('email.confirm')}
        </Button>
        <Button variant="quiet" size="sm" busy={working === 'send'} disabled={working !== null} onClick={() => void send()}>
          {t('email.send')}
        </Button>
      </div>
      {message && <p className={message.error ? styles.error : styles.ok} role={message.error ? 'alert' : 'status'}>{message.text}</p>}
    </form>
  );
}
