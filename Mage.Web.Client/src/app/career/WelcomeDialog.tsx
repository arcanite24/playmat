import { useState } from 'react';
import { SIGILS } from '../../core/social/identity';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { readJson, writeJson } from '../stores/persist';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { SigilIcon } from '../ui/SigilIcon';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { useCareerState, useProgressionShown } from './careerData';
import styles from './WelcomeDialog.module.css';

registerMessages(messages);

const SEEN_KEY = 'playmat.progression.welcomed';

/**
 * Welcome to progression (B282), once per account on this browser: levels, quests and packs, and that free play stays
 * unlimited; pick an avatar on the way.
 */
export default function WelcomeDialog() {
  const t = useT();
  const user = useSession((session) => session.userName).toLowerCase();
  const shown = useProgressionShown();
  const state = useCareerState();
  const avatarId = useSettings((settings) => settings.settings.avatarId);
  const [seen, setSeen] = useState(() => readJson<string[]>(SEEN_KEY, []).includes(user));
  if (seen || !shown || !state.data?.profile) return null;
  const close = () => {
    writeJson(SEEN_KEY, [...new Set([...readJson<string[]>(SEEN_KEY, []), user])]);
    setSeen(true);
  };

  return (
    <Dialog open onOpenChange={(open) => !open && close()} title={t('career.welcome.title')} width="md"
      footer={<Button variant="decision" onClick={close}>{t('career.welcome.go')}</Button>}>
      <ul className={styles.points}>
        <li>{t('career.welcome.levels')}</li>
        <li>{t('career.welcome.quests')}</li>
        <li>{t('career.welcome.packs')}</li>
        <li>{t('career.welcome.free')}</li>
      </ul>
      <p className={styles.pick}>{t('career.welcome.avatar')}</p>
      <div className={styles.sigils} role="radiogroup" aria-label={t('career.welcome.avatar')}>
        {SIGILS.map((sigil) => (
          <button key={sigil.id} type="button" role="radio" aria-checked={avatarId === sigil.id} aria-label={sigil.label}
            className={[styles.sigil, avatarId === sigil.id ? styles.on : ''].join(' ')}
            onClick={() => useSettings.getState().update({ avatarId: sigil.id })}>
            <SigilIcon sigil={sigil.key} size={24} />
          </button>
        ))}
      </div>
    </Dialog>
  );
}
