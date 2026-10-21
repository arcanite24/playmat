import { ArrowLeft } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { APP_NAME } from '../brand';
import { api, rpc } from '../connection';
import { formatDate, registerMessages, useT, type MessageKey } from '../i18n';
import aboutMessages from '../i18n/en/about';
import messages from '../i18n/en/legal';
import { RichText } from '../i18n/RichText';
import { useSession } from '../stores/session';
import styles from './AboutScreen.module.css';

registerMessages(aboutMessages);
registerMessages(messages);

/** when the privacy policy and terms last changed: change it with them */
const UPDATED = new Date(2026, 9, 13);
const FAN_CONTENT_POLICY_URL = 'https://company.wizards.com/en/legal/fancontentpolicy';

const COLLECTED: MessageKey[] = [
  'privacy.collect.account', 'privacy.collect.addresses', 'privacy.collect.games', 'privacy.collect.chat',
  'privacy.collect.decks', 'privacy.collect.reports', 'privacy.collect.errors', 'privacy.collect.payments',
];
const OTHERS: MessageKey[] = [
  'privacy.others.hetzner', 'privacy.others.cloudflare', 'privacy.others.scryfall', 'privacy.others.mail',
  'privacy.others.payments', 'privacy.others.github', 'privacy.others.discord',
];
const RIGHTS: MessageKey[] = ['privacy.rights.download', 'privacy.rights.delete', 'privacy.rights.correct'];

/** What the server keeps about players, why, how long and who else touches it. Readable before signing in. */
export function PrivacyScreen() {
  const t = useT();
  const app = { app: APP_NAME };
  return (
    <LegalPage title={t('privacy.title')} lede={t('privacy.lede', app)}>
      <Section id="privacy-collect" title={t('privacy.collect')}>
        <ul>{COLLECTED.map((key) => <li key={key}>{t(key)}</li>)}</ul>
      </Section>
      <Section id="privacy-how-long" title={t('privacy.howLong')}>
        <p>{t('privacy.howLong.text')}</p>
      </Section>
      <Section id="privacy-others" title={t('privacy.others')}>
        <ul>{OTHERS.map((key) => <li key={key}>{t(key)}</li>)}</ul>
      </Section>
      <Section id="privacy-storage" title={t('privacy.storage')}>
        <p>{t('privacy.storage.text', app)}</p>
      </Section>
      <Section id="privacy-rights" title={t('privacy.rights')}>
        <ul>{RIGHTS.map((key) => <li key={key}>{t(key)}</li>)}</ul>
        <Contact />
      </Section>
    </LegalPage>
  );
}

/** The few rules of playing here, and the Fan Content Policy notice. Readable before signing in. */
export function TermsScreen() {
  const t = useT();
  const app = { app: APP_NAME };
  const translation = t('about.wotc.translation', app);
  return (
    <LegalPage title={t('terms.title')} lede={t('terms.lede', app)}>
      <Section id="terms-fan" title={t('terms.fan')}>
        {/* the Fan Content Policy notice is quoted in English, as Wizards words it; a translation follows */}
        <p lang="en">
          <RichText
            text={t('about.wotc.policy', app)}
            parts={{ policy: <a href={FAN_CONTENT_POLICY_URL} target="_blank" rel="noreferrer">{t('about.wotc.policyLink')}</a> }}
          />
        </p>
        {translation && <p>{translation}</p>}
      </Section>
      <Section id="terms-warranty" title={t('terms.warranty')}>
        <p>{t('terms.warranty.text')}</p>
      </Section>
      <Section id="terms-conduct" title={t('terms.conduct')}>
        <p>{t('terms.conduct.text', { report: t('terms.conduct.report') })}</p>
      </Section>
      <Section id="terms-staff" title={t('terms.staff')}>
        <p>{t('terms.staff.text')}</p>
      </Section>
      <Section id="terms-support" title={t('terms.support')}>
        <p>
          <RichText text={t('terms.support.text', app)} parts={{ support: <Link to="/support">{t('terms.support.link')}</Link> }} />
        </p>
      </Section>
      <Section id="terms-account" title={t('terms.account')}>
        <p>{t('terms.account.text')}</p>
      </Section>
      <Section id="terms-changes" title={t('terms.changes')}>
        <p>{t('terms.changes.text')}</p>
        <Contact />
      </Section>
    </LegalPage>
  );
}

function LegalPage({ title, lede, children }: { title: string; lede: string; children: ReactNode }) {
  const navigate = useNavigate();
  const t = useT();
  return (
    <main className={styles.page}>
      <div className={styles.column}>
        <button type="button" className={styles.back} onClick={() => (window.history.length > 1 ? navigate(-1) : navigate('/'))}>
          <ArrowLeft size={16} aria-hidden="true" /> {t('about.back')}
        </button>
        <h1 className={styles.title}>{title}</h1>
        <p className={styles.lede}>{lede}</p>
        <p>{t('legal.updated', { date: formatDate(UPDATED, { dateStyle: 'long' }) })}</p>
        {children}
        <p className={styles.foot}>
          <Link to="/privacy">{t('legal.privacyLink')}</Link> · <Link to="/terms">{t('legal.termsLink')}</Link> · <Link to="/about">{t('shell.about')}</Link> · <Link to="/">{t('about.backTo', { app: APP_NAME })}</Link>
        </p>
      </div>
    </main>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section className={styles.section} aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {children}
    </section>
  );
}

/** The server's contact address (XMAGE_CONTACT_EMAIL), or Feedback when it has none or can't be reached. */
function Contact() {
  const t = useT();
  const email = useContactEmail();
  return (
    <p>
      <RichText
        text={t('legal.contact')}
        parts={{ contact: email ? <a href={`mailto:${email}`}>{email}</a> : <>{t('legal.contact.feedback')}</> }}
      />
    </p>
  );
}

/** Asked of the server this app talks to; a page opened before signing in connects to it first. */
function useContactEmail(): string | null {
  const [email, setEmail] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // a sign-in may be opening the connection already: wait for it rather than open another
        for (let waited = 0; rpc.getStatus() === 'connecting' && waited < 5000; waited += 150) {
          await new Promise((resolve) => setTimeout(resolve, 150));
        }
        if (rpc.getStatus() !== 'open') await rpc.connect(useSession.getState().serverUrl);
        const info = await api.serverInfo();
        if (!cancelled) setEmail(info.contactEmail ?? null);
      } catch {
        // an older or unreachable server: Feedback it is
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
  return email;
}
