import { Check, Copy, ExternalLink, Heart } from 'lucide-react';
import { useState } from 'react';
import { Link } from 'react-router-dom';
import type { SupportStatus } from '../../protocol/generated/views';
import { registerMessages, useI18n, useT } from '../i18n';
import messages from '../i18n/en/support';
import { RichText } from '../i18n/RichText';
import { ButtonLink } from '../ui/Button';
import { useSupportStatus } from './supportData';
import { goalProgress, monthName, usd } from './supportModel';
import styles from './SupportBar.module.css';

registerMessages(messages);

/**
 * The month's server costs and how far players got: amount against the goal, a way to support the server in a new tab,
 * and, signed in, the code that finds the player's account. Quiet on purpose: it sits on Home and the sign-in screen
 * (lazily, only where the server takes support) and leads the /support page.
 */
export function SupportBar({ place }: { place: 'home' | 'login' | 'page' }) {
  const status = useSupportStatus().data;
  if (!status?.enabled) return null;
  return <GoalBar status={status} place={place} />;
}

export function GoalBar({ status, place }: { status: SupportStatus; place: 'home' | 'login' | 'page' }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const progress = goalProgress(status);
  const amount = usd(status.monthCents ?? 0, locale);
  const goal = usd(status.goalCents ?? 0, locale);
  const url = status.kofiUrl || status.sponsorsUrl;
  const code = status.you?.code;
  const note = progress.aheadCents > 0
    ? t('support.bar.ahead', { amount: usd(progress.aheadCents, locale) })
    : progress.reached ? t('support.bar.reached') : t('support.bar.left', { amount: usd(progress.leftCents, locale) });

  return (
    <section className={[styles.bar, styles[place]].join(' ')} aria-labelledby={`support-bar-${place}`}>
      <div className={styles.head}>
        <h2 id={`support-bar-${place}`} className={styles.label}>{t('support.bar', { month: monthName(status.monthLabel, locale) })}</h2>
        <span className={styles.amount}>{t('support.bar.amount', { amount, goal })}</span>
      </div>
      <div
        className={styles.track}
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(progress.fraction * 100)}
        aria-label={t('support.bar.progress', { amount, goal })}
      >
        <span className={progress.reached ? styles.full : undefined} style={{ width: `${Math.round(progress.fraction * 100)}%` }} />
      </div>
      {place === 'login' ? (
        // under the sign-in form: two quiet links, the details are a page away
        <div className={styles.foot}>
          {url && (
            <a className={styles.link} href={url} target="_blank" rel="noreferrer" aria-label={`${t('support.cta')} (${t('support.cta.newTab')})`}>
              <Heart size={13} aria-hidden="true" className={styles.heart} /> {t('support.cta')}
            </a>
          )}
          <Link className={styles.link} to="/support">{t('support.more')}</Link>
        </div>
      ) : (
        <div className={styles.foot}>
          <span className={styles.note}>
            {note}
            {(status.supporters ?? 0) > 0 && <> · {t('support.bar.supporters', { count: status.supporters ?? 0 })}</>}
          </span>
          {url && (
            <ButtonLink
              variant={place === 'page' ? 'decision' : 'print'}
              size="sm"
              className={styles.cta}
              href={url}
              target="_blank"
              rel="noreferrer"
              icon={<ExternalLink size={14} />}
              aria-label={`${t('support.cta')} (${t('support.cta.newTab')})`}
            >
              <Heart size={14} aria-hidden="true" className={styles.heart} /> {t('support.cta')}
            </ButtonLink>
          )}
          {status.kofiUrl && status.sponsorsUrl && (
            <a className={styles.link} href={status.sponsorsUrl} target="_blank" rel="noreferrer">{t('support.cta.sponsors')}</a>
          )}
          {place !== 'page' && <Link className={styles.link} to="/support">{t('support.more')}</Link>}
        </div>
      )}
      {code && <SupportCode code={code} />}
    </section>
  );
}

/** The player's support code, with the way it finds their account. */
function SupportCode({ code }: { code: string }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  return (
    <p className={styles.code}>
      <span><RichText text={t('support.code')} parts={{ code: <code>{code}</code> }} /></span>
      <button
        type="button"
        className={styles.copy}
        aria-label={copied ? t('support.code.copied') : t('support.code.copy')}
        title={copied ? t('support.code.copied') : t('support.code.copy')}
        onClick={() => {
          void navigator.clipboard?.writeText(code).then(() => setCopied(true)).catch(() => undefined);
        }}
      >
        {copied ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
      </button>
      <small>{t('support.code.hint')}</small>
    </p>
  );
}
