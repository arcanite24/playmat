import { Check, Crown, ScrollText, Sparkles, Swords } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerArchenemyEncounter } from '../../protocol/generated/views';
import { isCommanderFormat } from '../../core/decks/formats';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { useDeckRoster } from '../stores/useDeckRoster';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { playMode, useArchenemy, useMetaLibrary } from './careerModesData';
import { useSceneArt } from './careerScene';
import { ModeResult } from './ModeParts';
import { PortraitCard } from './Portraits';
import { Versus } from './Versus';
import styles from './CareerModes.module.css';

registerMessages(messages);

/**
 * Archenemy PvE mode (Phase 5.2): Hunt a 60-life Scheme Boss or Become the Archenemy (1v3)
 * with a randomized 20-card Scheme Deck — optionally layered with Planechase.
 */
export function CareerArchenemyScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const archenemy = useArchenemy(!!profile);
  const [chosen, setChosen] = useState<string | null>(null);
  const encounters = archenemy.data?.encounters ?? [];
  const schemes = archenemy.data?.schemes ?? [];
  const current = encounters.find((item) => item.id === chosen) ?? encounters.find((item) => !item.beaten) ?? encounters[0];

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;

  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="archenemy" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{t('career.archenemy.title')}</h1>
            <p className={styles.lead}>{t('career.archenemy.lead')}</p>
          </div>
          {encounters.length > 0 && (
            <p className={styles.progressNote}>
              {t('career.horde.beatenCount', { beaten: encounters.filter((item) => item.beaten).length, total: encounters.length })}
            </p>
          )}
        </header>
        {archenemy.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {archenemy.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {current && (
          <div className={styles.split}>
            <div className={styles.mainCol}>
              <ul className={styles.hordeList} aria-label={t('career.archenemy.title')}>
                {encounters.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      className={[styles.hordeTile, item.id === current.id ? styles.puzzleTileOn : ''].join(' ')}
                      aria-pressed={item.id === current.id}
                      onClick={() => setChosen(item.id ?? null)}
                      data-nav
                    >
                      <b>{item.name}</b>
                      <small>{t(item.mode === 'become' ? 'career.archenemy.becomeTag' : 'career.archenemy.huntTag')}</small>
                      {item.beaten && <Check size={16} aria-label={t('career.horde.beaten')} />}
                    </button>
                  </li>
                ))}
              </ul>
              <EncounterDetail key={current.id} encounter={current} />
            </div>
            <aside className={styles.sideCol}>
              <section className={styles.card} aria-labelledby="archenemy-schemes">
                <h2 id="archenemy-schemes" className={styles.cardLabel}>
                  <ScrollText size={14} aria-hidden="true" /> {t('career.archenemy.schemes', { count: schemes.length })}
                </h2>
                <ul className={[styles.names, styles.namesScroll].join(' ')}>
                  {schemes.map((scheme) => (
                    <li key={scheme.id}>
                      <b>{scheme.name}</b>: {scheme.text}
                    </li>
                  ))}
                </ul>
              </section>
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}

function EncounterDetail({ encounter }: { encounter: CareerArchenemyEncounter }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const meta = useMetaLibrary(true);
  const roster = useDeckRoster();
  const loaded = useDecks((d) => d.loaded);
  const refresh = useDecks((d) => d.refresh);
  const loadList = useDecks((d) => d.loadList);
  const [selection, setSelection] = useState('');
  const [planeChase, setPlaneChase] = useState(false);
  const [versus, setVersus] = useState(false);

  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);

  const personalDecks = useMemo(
    () => roster.filter((d) => (encounter.commander ? isCommanderFormat(d.format) && d.cardCount >= 98 : d.cardCount >= 40 && !isCommanderFormat(d.format))),
    [roster, encounter.commander],
  );
  const metaFormats = useMemo(
    () => (meta.data ?? []).filter((f) => (encounter.commander ? f.id === 'commander' : f.id !== 'commander' && f.id !== 'limited')),
    [meta.data, encounter.commander],
  );
  const defaultSel = personalDecks[0] ? `deck:${personalDecks[0].id}` : metaFormats[0]?.decks?.[0]?.file ? `meta:${metaFormats[0].decks[0].file}` : '';
  const activeSel = selection || defaultSel;
  const deckName = activeSel.startsWith('deck:')
    ? personalDecks.find((d) => `deck:${d.id}` === activeSel)?.name ?? ''
    : metaFormats.flatMap((f) => f.decks ?? []).find((d) => `meta:${d.file}` === activeSel)?.name ?? '';

  useSceneArt(encounter.cover);

  return (
    <article className={styles.detail} aria-labelledby="archenemy-name">
      <div className={styles.detailArt}>
        <PortraitCard name={encounter.name ?? ''} cover={encounter.cover} boss size="sm" />
      </div>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <h2 id="archenemy-name">{encounter.name}</h2>
          {encounter.beaten && <span className={[styles.stateTag, styles.tag_done].join(' ')}>{t('career.horde.beaten')}</span>}
        </header>
        {encounter.text && <p className={styles.small}>{encounter.text}</p>}
        <ul className={styles.twists}>
          <li>
            <Crown size={14} aria-hidden="true" />{' '}
            {t('career.archenemy.lifeRule', { archenemy: encounter.archenemyLife ?? 60, challengers: encounter.challengerLife ?? 20 })}
          </li>
          <li>{t('career.archenemy.rivals', { names: (encounter.opponents ?? []).join(' · ') })}</li>
          {planeChase && <li><Sparkles size={14} aria-hidden="true" /> {t('career.planechase.active')}</li>}
        </ul>
        <label className={styles.field}>
          <span>{t('career.horde.deck')}</span>
          <select value={activeSel} onChange={(event) => setSelection(event.target.value)} disabled={!activeSel}>
            {personalDecks.length > 0 && (
              <optgroup label={t('career.horde.yourDecks')}>
                {personalDecks.map((deck) => <option key={deck.id} value={`deck:${deck.id}`}>{deck.name}</option>)}
              </optgroup>
            )}
            {metaFormats.map((format) => (
              <optgroup key={format.id} label={format.name}>
                {(format.decks ?? []).map((deck) => <option key={deck.file} value={`meta:${deck.file}`}>{deck.name}</option>)}
              </optgroup>
            ))}
          </select>
        </label>
        <label className={styles.field}>
          <input type="checkbox" checked={planeChase} onChange={(event) => setPlaneChase(event.target.checked)} />{' '}
          <span>{t('career.planechase.toggle')}</span>
        </label>
        <div className={styles.actions}>
          <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={playing || !activeSel} onClick={() => setVersus(true)} data-nav>
            {t('career.archenemy.play')}
          </Button>
        </div>
      </div>
      {versus && (
        <Versus
          kicker={t('career.archenemy.title')}
          opponent={{ name: encounter.name ?? '', tagline: (encounter.opponents ?? []).join(' · '), cover: encounter.cover, boss: true }}
          deck={{ name: deckName }}
          stakes={[
            t('career.archenemy.lifeRule', { archenemy: encounter.archenemyLife ?? 60, challengers: encounter.challengerLife ?? 20 }),
            ...(planeChase ? [t('career.planechase.active')] : []),
          ]}
          onFight={() => playMode('archenemy', async () => {
            if (activeSel.startsWith('deck:')) {
              const id = activeSel.slice('deck:'.length);
              const list = await loadList(id);
              return api.careerArchenemyPlay(encounter.id ?? '', null, toWire(list) as never, planeChase);
            }
            const file = activeSel.slice('meta:'.length);
            return api.careerArchenemyPlay(encounter.id ?? '', file, null, planeChase);
          })}
          onClose={() => setVersus(false)}
        />
      )}
    </article>
  );
}
