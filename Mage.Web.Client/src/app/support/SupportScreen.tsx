import { ArrowLeft, Heart } from 'lucide-react';
import type { ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { APP_NAME } from '../brand';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/support';
import { useSession } from '../stores/session';
import page from '../screens/AboutScreen.module.css';
import { GoalBar } from './SupportBar';
import { useSupportStatus } from './supportData';
import styles from './SupportScreen.module.css';

registerMessages(messages);

const PERKS: MessageKey[] = ['support.perk.sleeve', 'support.perk.playmat', 'support.perk.frame', 'support.perk.title', 'support.perk.flair', 'support.perk.pet'];
const RULES: MessageKey[] = ['support.rule.free', 'support.rule.cosmetic', 'support.rule.voluntary', 'support.rule.kept', 'support.rule.refunds'];

/**
 * Supporting the server, in full: the month's goal, what it pays for, the thank-you, who helped this month, and the
 * rules (free forever, cosmetic only, kept for good). Readable before signing in.
 */
export function SupportScreen() {
  const t = useT();
  const navigate = useNavigate();
  const signedIn = useSession((state) => state.phase === 'signedIn');
  const query = useSupportStatus();
  const status = query.data;
  const app = { app: APP_NAME };

  return (
    <main className={styles.page}>
      <div className={page.column}>
        <button type="button" className={page.back} onClick={() => (window.history.length > 1 ? navigate(-1) : navigate('/'))}>
          <ArrowLeft size={16} aria-hidden="true" /> {t('support.back')}
        </button>
        <h1 className={page.title}>{t('support.title')}</h1>
        <p className={page.lede}>{t('support.lede', app)}</p>

        {query.isError && <p role="alert">{t('support.unreachable')}</p>}
        {status && !status.enabled && <p role="status">{t('support.off')}</p>}
        {status?.enabled && <GoalBar status={status} place="page" />}

        {status?.enabled && (status.costs ?? []).length > 0 && (
          <Section id="support-costs" title={t('support.costs')}>
            <ul>{status.costs!.map((line, index) => <li key={index}>{line}</li>)}</ul>
            <p>{t('support.costs.month')}</p>
          </Section>
        )}

        <Section id="support-perks" title={t('support.perks')}>
          <p>{t('support.perks.text')}</p>
          <ul>{PERKS.map((key) => <li key={key}>{t(key)}</li>)}</ul>
          <figure className={styles.corgi}>
            <picture>
              <source srcSet="/support/party-corgi-still.png" media="(prefers-reduced-motion: reduce)" />
              <img src="/support/party-corgi.gif" width={160} height={176} loading="lazy" alt={t('support.corgi.alt')} />
            </picture>
            <figcaption>{t('support.corgi.caption')}</figcaption>
          </figure>
          <p>{t('support.perks.status')}</p>
        </Section>

        {status?.enabled && (
          <Section id="support-how" title={t('support.how')}>
            <p>{t('support.how.text', app)}</p>
            {!signedIn && <p>{t('support.how.signIn')}</p>}
          </Section>
        )}

        {status?.enabled && (
          <Section id="support-wall" title={t('support.wall')}>
            {(status.publicSupporters ?? []).length > 0 ? (
              <ul className={styles.wall}>
                {status.publicSupporters!.map((name, index) => (
                  <li key={`${name}-${index}`}><Heart size={12} aria-hidden="true" className={styles.heart} /> {name}</li>
                ))}
              </ul>
            ) : (
              <p>{t('support.wall.empty')}</p>
            )}
            <p className={styles.note}>{t('support.wall.note')}</p>
          </Section>
        )}

        <Section id="support-rules" title={t('support.rules')}>
          <ul>{RULES.map((key) => <li key={key}>{t(key, app)}</li>)}</ul>
        </Section>

        <p className={[page.foot, styles.prose].join(' ')}>
          <Link to="/terms">{t('legal.termsLink')}</Link> · <Link to="/privacy">{t('legal.privacyLink')}</Link> · <Link to="/about">{t('shell.about')}</Link> · <Link to="/">{t('support.backTo', app)}</Link>
        </p>
      </div>
    </main>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section className={[page.section, styles.prose].join(' ')} aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {children}
    </section>
  );
}
