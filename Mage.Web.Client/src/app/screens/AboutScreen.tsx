import { ArrowLeft } from 'lucide-react';
import { Link, useNavigate } from 'react-router-dom';
import { APP_NAME, APP_VERSION } from '../brand';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/about';
import { RichText } from '../i18n/RichText';
import styles from './AboutScreen.module.css';

const XMAGE_URL = 'https://github.com/magefree/mage';
const SOURCE_URL = 'https://github.com/arcanite24/playmat';
const FAN_CONTENT_POLICY_URL = 'https://company.wizards.com/en/legal/fancontentpolicy';
const SCRYFALL_URL = 'https://scryfall.com';

registerMessages(messages);

/** Credits, licences and the legal notices a public server needs. Readable before signing in. */
export function AboutScreen() {
  const navigate = useNavigate();
  const t = useT();
  const app = { app: APP_NAME };
  const translation = t('about.wotc.translation', app);
  return (
    <main className={styles.page}>
      <div className={styles.column}>
        <button type="button" className={styles.back} onClick={() => (window.history.length > 1 ? navigate(-1) : navigate('/'))}>
          <ArrowLeft size={16} aria-hidden="true" /> {t('about.back')}
        </button>
        <h1 className={styles.title}>{t('about.title', app)}</h1>
        <p className={styles.lede}>{t('about.lede', app)}</p>
        <p className={styles.version}>{t('about.version', { version: APP_VERSION })}</p>

        <section className={styles.section} aria-labelledby="about-wotc">
          <h2 id="about-wotc">{t('about.wotc')}</h2>
          {/* the Fan Content Policy notice is quoted in English, as Wizards words it; a translation follows */}
          <p lang="en">
            <RichText
              text={t('about.wotc.policy', app)}
              parts={{ policy: <a href={FAN_CONTENT_POLICY_URL} target="_blank" rel="noreferrer">{t('about.wotc.policyLink')}</a> }}
            />
          </p>
          {translation && <p>{translation}</p>}
          <p>{t('about.wotc.trademarks')}</p>
        </section>

        <section className={styles.section} aria-labelledby="about-scryfall">
          <h2 id="about-scryfall">{t('about.images')}</h2>
          <p>
            <RichText
              text={t('about.images.text', app)}
              parts={{ scryfall: <a href={SCRYFALL_URL} target="_blank" rel="noreferrer">Scryfall</a> }}
            />
          </p>
        </section>

        <section className={styles.section} aria-labelledby="about-xmage">
          <h2 id="about-xmage">{t('about.source')}</h2>
          <p>
            <RichText
              text={t('about.source.text', app)}
              parts={{
                xmage: <a href={XMAGE_URL} target="_blank" rel="noreferrer">XMage</a>,
                repository: <a href={SOURCE_URL} target="_blank" rel="noreferrer">{t('about.source.repository')}</a>,
              }}
            />
          </p>
        </section>

        <section className={styles.section} aria-labelledby="about-type">
          <h2 id="about-type">{t('about.type')}</h2>
          <ul>
            <li>{t('about.type.barlow')}</li>
            <li>{t('about.type.mana')}</li>
            <li>{t('about.type.ui')}</li>
          </ul>
        </section>

        <section className={styles.section} aria-labelledby="about-data">
          <h2 id="about-data">{t('about.data')}</h2>
          <ul>
            <li>{t('about.data.name')}</li>
            <li>{t('about.data.decks')}</li>
            <li>{t('about.data.reports')}</li>
            <li>{t('about.data.errors')}</li>
          </ul>
          <p>{t('about.data.chat')}</p>
          <p>
            <RichText
              text={t('about.data.more')}
              parts={{ privacy: <Link to="/privacy">{t('legal.privacyLink')}</Link>, terms: <Link to="/terms">{t('legal.termsLink')}</Link> }}
            />
          </p>
        </section>

        <section className={styles.section} aria-labelledby="about-support">
          <h2 id="about-support">{t('about.support')}</h2>
          <p>
            <RichText text={t('about.support.text', app)} parts={{ support: <Link to="/support">{t('about.support.link')}</Link> }} />
          </p>
        </section>

        <p className={styles.foot}><Link to="/">{t('about.backTo', app)}</Link></p>
      </div>
    </main>
  );
}
