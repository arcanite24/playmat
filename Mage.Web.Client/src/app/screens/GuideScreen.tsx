import { BookOpen, Dumbbell, GraduationCap, Play } from 'lucide-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/guide';
import { useServerInfo } from '../queries';
import { rosterOf, useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import styles from './GuideScreen.module.css';

registerMessages(messages);

type Tab = 'basics' | 'formats';

const TURN_STEPS = [
  ['guide.basics.turn.untap', 'guide.basics.turn.untapDesc'],
  ['guide.basics.turn.upkeep', 'guide.basics.turn.upkeepDesc'],
  ['guide.basics.turn.draw', 'guide.basics.turn.drawDesc'],
  ['guide.basics.turn.main1', 'guide.basics.turn.main1Desc'],
  ['guide.basics.turn.combat', 'guide.basics.turn.combatDesc'],
  ['guide.basics.turn.main2', 'guide.basics.turn.main2Desc'],
  ['guide.basics.turn.end', 'guide.basics.turn.endDesc'],
] as const;

const FORMAT_CARDS = [
  { id: 'standard', name: 'guide.format.standard.name', meta: 'guide.format.standard.meta', body: 'guide.format.standard.body' },
  { id: 'pioneer', name: 'guide.format.pioneer.name', meta: 'guide.format.pioneer.meta', body: 'guide.format.pioneer.body' },
  { id: 'modern', name: 'guide.format.modern.name', meta: 'guide.format.modern.meta', body: 'guide.format.modern.body' },
  { id: 'legacy', name: 'guide.format.legacy.name', meta: 'guide.format.legacy.meta', body: 'guide.format.legacy.body' },
  { id: 'pauper', name: 'guide.format.pauper.name', meta: 'guide.format.pauper.meta', body: 'guide.format.pauper.body' },
  { id: 'commander', name: 'guide.format.commander.name', meta: 'guide.format.commander.meta', body: 'guide.format.commander.body' },
  { id: 'limited', name: 'guide.format.limited.name', meta: 'guide.format.limited.meta', body: 'guide.format.limited.body' },
] as const;

/** Plain-language Magic gameplay refresher, format guide, and direct launchers for interactive tutorials (#468). */
export function GuideScreen() {
  const t = useT();
  const navigate = useNavigate();
  const career = useServerInfo().data?.career === true;
  const playPhase = usePlay((state) => state.phase);
  const [tab, setTab] = useState<Tab>('basics');
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function startGuidedGame() {
    setStarting(true);
    setError(null);
    try {
      const store = useDecks.getState();
      if (!store.loaded) await store.refresh();
      const state = useDecks.getState();
      const deckId = state.selectedId ?? rosterOf(state)[0]?.id ?? null;
      if (!deckId) {
        setError(t('guide.prologue.error'));
        return;
      }
      await usePlay.getState().playVsAi(deckId, { guided: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : t('guide.prologue.error'));
    } finally {
      setStarting(false);
    }
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>{t('guide.title')}</h1>
        <p className={styles.lede}>{t('guide.lede')}</p>
      </header>

      <section className={styles.actionsSection} aria-labelledby="guide-actions-heading">
        <h2 id="guide-actions-heading" className={styles.sectionHeading}>{t('guide.actions.title')}</h2>
        <div className={styles.actionGrid}>
          <div className={styles.actionCard}>
            <h3><Play size={16} aria-hidden="true" /> {t('guide.prologue.title')}</h3>
            <p>{t('guide.prologue.text')}</p>
            <Button
              variant="decision"
              size="sm"
              busy={starting || playPhase !== 'idle'}
              onClick={() => void startGuidedGame()}
            >
              {t(starting ? 'guide.prologue.starting' : 'guide.prologue.button')}
            </Button>
          </div>

          {career && (
            <div className={styles.actionCard}>
              <h3><GraduationCap size={16} aria-hidden="true" /> {t('guide.academy.title')}</h3>
              <p>{t('guide.academy.text')}</p>
              <Button variant="print" size="sm" onClick={() => navigate('/career/academy')}>
                {t('guide.academy.button')}
              </Button>
            </div>
          )}

          <div className={styles.actionCard}>
            <h3><Dumbbell size={16} aria-hidden="true" /> {t('guide.train.title')}</h3>
            <p>{t('guide.train.text')}</p>
            <Button variant="quiet" size="sm" onClick={() => navigate('/train')}>
              {t('guide.train.button')}
            </Button>
          </div>
        </div>
        {error && <p className={styles.error} role="alert">{error}</p>}
      </section>

      <div className={styles.tabs} role="tablist" aria-label={t('guide.tabsLabel')}>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'basics'}
          className={tab === 'basics' ? styles.tabOn : styles.tab}
          onClick={() => setTab('basics')}
        >
          {t('guide.tab.basics')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'formats'}
          className={tab === 'formats' ? styles.tabOn : styles.tab}
          onClick={() => setTab('formats')}
        >
          {t('guide.tab.formats')}
        </button>
      </div>

      {tab === 'basics' ? (
        <div className={styles.stack} role="tabpanel">
          <div className={styles.cardGrid}>
            <article className={styles.card}>
              <h3>{t('guide.basics.goal.title')}</h3>
              <p>{t('guide.basics.goal.body')}</p>
            </article>
            <article className={styles.card}>
              <h3>{t('guide.basics.lands.title')}</h3>
              <p>{t('guide.basics.lands.body')}</p>
            </article>
          </div>

          <article className={styles.card}>
            <h3>{t('guide.basics.turn.title')}</h3>
            <ol className={styles.turnList}>
              {TURN_STEPS.map(([nameKey, descKey], index) => (
                <li key={nameKey} className={styles.turnStep}>
                  <strong>{index + 1}. {t(nameKey)}</strong>
                  <span>{t(descKey)}</span>
                </li>
              ))}
            </ol>
          </article>

          <div className={styles.cardGrid}>
            <article className={styles.card}>
              <h3>{t('guide.basics.combat.title')}</h3>
              <p>{t('guide.basics.combat.body')}</p>
            </article>
            <article className={styles.card}>
              <h3>{t('guide.basics.stack.title')}</h3>
              <p>{t('guide.basics.stack.body')}</p>
            </article>
          </div>

          <article className={styles.card}>
            <h3>{t('guide.basics.types.title')}</h3>
            <p>{t('guide.basics.types.body')}</p>
          </article>

          <article className={styles.card}>
            <h3><BookOpen size={16} aria-hidden="true" /> {t('guide.basics.returning.title')}</h3>
            <ul className={styles.bulletList}>
              <li>{t('guide.basics.returning.item1')}</li>
              <li>{t('guide.basics.returning.item2')}</li>
              <li>{t('guide.basics.returning.item3')}</li>
              <li>{t('guide.basics.returning.item4')}</li>
              <li>{t('guide.basics.returning.item5')}</li>
            </ul>
          </article>
        </div>
      ) : (
        <div className={styles.stack} role="tabpanel">
          <p className={styles.formatsIntro}>{t('guide.formats.intro')}</p>
          <div className={styles.cardGrid}>
            {FORMAT_CARDS.map((item) => (
              <article key={item.id} className={styles.card}>
                <h3>{t(item.name)}</h3>
                <span className={styles.metaBadge}>{t(item.meta)}</span>
                <p>{t(item.body)}</p>
                {item.id === 'limited' && (
                  <div className={styles.cardFooter}>
                    <Button variant="quiet" size="sm" onClick={() => navigate('/events')}>
                      {t('guide.format.limited.cta')}
                    </Button>
                  </div>
                )}
              </article>
            ))}
          </div>
          <div className={styles.footActions}>
            <Button variant="print" size="sm" onClick={() => navigate('/decks')}>
              {t('guide.formats.buildCta')}
            </Button>
            {career && (
              <Button variant="quiet" size="sm" onClick={() => navigate('/career/academy')}>
                {t('guide.academy.button')}
              </Button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
