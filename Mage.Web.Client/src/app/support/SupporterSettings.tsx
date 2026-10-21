import { useState } from 'react';
import { Link } from 'react-router-dom';
import { api } from '../connection';
import { formatDate, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/support';
import { useServerInfo } from '../queries';
import styles from '../screens/SettingsDialog.module.css';
import { refreshSupport, useSupportPerks } from './supportData';

registerMessages(messages);

/**
 * Settings, Account: supporting the server. Whether the account's name goes on the supporters wall (off until chosen),
 * and where to choose what of the thank-you set it wears (Settings, Cosmetics); a player without them sees what support
 * would give, once.
 */
export function SupporterSettings() {
  const t = useT();
  const support = useServerInfo().data?.support === true;
  const perks = useSupportPerks();
  const [saving, setSaving] = useState(false);
  const you = perks.data;
  if (!you) return null;
  const thanked = (you.perks ?? []).length > 0;
  if (!thanked && !support) return null;

  async function setWall(show: boolean) {
    setSaving(true);
    try {
      await api.supportSetWall(show);
      refreshSupport();
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className={styles.accountSection} aria-labelledby="account-support">
      <h3 id="account-support" className={styles.rowLabel}>{t('support.settings')}</h3>
      <p className={styles.rowDetail}>
        {you.supporter ? t('support.settings.active') : thanked ? t('support.settings.thanks') : t('support.settings.none')}
        {thanked && you.since ? ` (${formatDate(you.since, { dateStyle: 'medium' })})` : ''}
        {' '}<Link to="/support">{t('support.more')}</Link>
      </p>
      <Row label={t('support.settings.wall')} detail={t('support.settings.wall.detail')}>
        <Switch label={t('support.settings.wall')} checked={!!you.wall} disabled={saving} onChange={(value) => void setWall(value)} />
      </Row>
      {thanked && <p className={styles.rowDetail}>{t('support.settings.wear')}</p>}
    </section>
  );
}

function Row({ label, detail, children }: { label: string; detail?: string; children: React.ReactNode }) {
  return (
    <div className={styles.row}>
      <div>
        <div className={styles.rowLabel}>{label}</div>
        {detail && <div className={styles.rowDetail}>{detail}</div>}
      </div>
      {children}
    </div>
  );
}

function Switch({ label, checked, onChange, disabled }: { label: string; checked: boolean; onChange(value: boolean): void; disabled?: boolean }) {
  return (
    <button type="button" role="switch" aria-checked={checked} aria-label={label} disabled={disabled} className={styles.switch} onClick={() => onChange(!checked)}>
      <span className={styles.knob} />
    </button>
  );
}
