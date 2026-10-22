import { Check, Swords } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerHorde } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { playMode, useHordes, useMetaLibrary } from './careerModesData';
import { useSceneArt } from './careerScene';
import { constructedFormats, hordeLines } from './ladderModel';
import { LineList, ModeResult } from './ModeParts';
import { PortraitCard } from './Portraits';
import { ScoreBoard } from './ScoreBoard';
import { Versus } from './Versus';
import styles from './CareerModes.module.css';

registerMessages(messages);

/**
 * Hordes (S036): a deck that plays itself, flipping permanents onto the battlefield every upkeep and attacking with
 * everything. Bring a deck from the meta library and live through its turns; the board ranks turns lived, then life.
 */
export function CareerHordesScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const hordes = useHordes(!!profile);
  const [chosen, setChosen] = useState<string | null>(null);
  const list = hordes.data ?? [];
  const horde = list.find((item) => item.id === chosen) ?? list.find((item) => !item.beaten) ?? list[0];

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="horde" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{t('career.horde.title')}</h1>
            <p className={styles.lead}>{t('career.horde.lead')}</p>
          </div>
          {list.length > 0 && (
            <p className={styles.progressNote}>{t('career.horde.beatenCount', { beaten: list.filter((item) => item.beaten).length, total: list.length })}</p>
          )}
        </header>
        {hordes.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {hordes.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {hordes.data && list.length === 0 && <p className={styles.note}>{t('career.horde.none')}</p>}
        {horde && (
          <div className={styles.split}>
            <div className={styles.mainCol}>
              <ul className={styles.hordeList} aria-label={t('career.horde.title')}>
                {list.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      className={[styles.hordeTile, item.id === horde.id ? styles.puzzleTileOn : ''].join(' ')}
                      aria-pressed={item.id === horde.id}
                      onClick={() => setChosen(item.id ?? null)}
                      data-nav
                    >
                      <b>{item.name}</b>
                      <small>{t('career.horde.survive', { count: item.survive ?? 0 })}</small>
                      {item.beaten && <Check size={16} aria-label={t('career.horde.beaten')} />}
                    </button>
                  </li>
                ))}
              </ul>
              <HordeDetail key={horde.id} horde={horde} />
            </div>
            <div className={styles.sideCol}>
              <ScoreBoard mode="horde" board={horde.id ?? ''} title={t('career.horde.board', { name: horde.name ?? '' })} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** One horde: its rules, your best, a deck to bring, and the fight. */
function HordeDetail({ horde }: { horde: CareerHorde }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const meta = useMetaLibrary(true);
  const formats = useMemo(() => constructedFormats(meta.data), [meta.data]);
  const [deckFile, setDeckFile] = useState('');
  const [versus, setVersus] = useState(false);
  const rules = hordeLines(horde);
  const chosenFile = deckFile || formats[0]?.decks[0]?.file || '';
  const chosenDeck = formats.flatMap((format) => format.decks).find((deck) => deck.file === chosenFile);
  useSceneArt(horde.cover);

  return (
    <article className={styles.detail} aria-labelledby="horde-name">
      <div className={styles.detailArt}>
        <PortraitCard name={horde.name ?? ''} cover={horde.cover} boss size="sm" />
      </div>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <h2 id="horde-name">{horde.name}</h2>
          {horde.beaten && <span className={[styles.stateTag, styles.tag_done].join(' ')}>{t('career.horde.beaten')}</span>}
        </header>
        {horde.text && <p className={styles.small}>{horde.text}</p>}
        <h3 className={styles.subhead}>{t('career.horde.rules')}</h3>
        <LineList lines={rules} className={styles.twists} />
        <h3 className={styles.subhead}>{t('career.challenge.yours')}</h3>
        <p className={styles.small}>
          {(horde.attempts ?? 0) === 0
            ? t('career.horde.notYet')
            : horde.bestTurns != null
              ? t('career.horde.best', { turns: horde.bestTurns, life: horde.bestLife ?? 0, count: horde.attempts ?? 0 })
              : t('career.horde.tries', { count: horde.attempts ?? 0 })}
        </p>
        <label className={styles.field}>
          <span>{t('career.horde.deck')}</span>
          <select value={chosenFile} onChange={(event) => setDeckFile(event.target.value)} disabled={formats.length === 0}>
            {formats.map((format) => (
              <optgroup key={format.id} label={format.name}>
                {format.decks.map((deck) => <option key={deck.file} value={deck.file}>{deck.name}</option>)}
              </optgroup>
            ))}
          </select>
        </label>
        {meta.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        <div className={styles.actions}>
          <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={playing || !chosenFile} onClick={() => setVersus(true)} data-nav>
            {t((horde.attempts ?? 0) > 0 ? 'career.horde.again' : 'career.horde.play')}
          </Button>
        </div>
      </div>
      {versus && (
        <Versus
          kicker={t('career.horde.title')}
          opponent={{ name: horde.name ?? '', tagline: horde.text, cover: horde.cover, boss: true }}
          deck={{ name: chosenDeck?.name ?? '', colors: chosenDeck?.colors }}
          stakes={rules.map((line) => t(line.key, line.vars))}
          onFight={() => playMode('horde', () => api.careerHordePlay(horde.id ?? '', chosenFile))}
          onClose={() => setVersus(false)}
        />
      )}
    </article>
  );
}
