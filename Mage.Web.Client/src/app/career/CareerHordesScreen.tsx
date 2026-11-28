import { Check, Crown, Infinity as InfinityIcon, Swords } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerHorde } from '../../protocol/generated/views';
import { isCommanderFormat } from '../../core/decks/formats';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { useDeckRoster } from '../stores/useDeckRoster';
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

function difficultyKey(difficulty: string | undefined): MessageKey {
  if (difficulty === 'mythic') return 'career.horde.diff.mythic';
  if (difficulty === 'heroic') return 'career.horde.diff.heroic';
  return 'career.horde.diff.normal';
}

/**
 * Hordes 2.0: ten hordes across Normal, Heroic, and Mythic difficulties. Bring any personal deck,
 * Career deck, 100-card Commander deck (auto-scaled to 40 life & 12 waves), or Meta Library loaner,
 * with optional Endless Waves mode.
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
                      <small>{t(difficultyKey(item.difficulty))} · {t('career.horde.survive', { count: item.survive ?? 0 })}</small>
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

/** One horde: its rules, your best, any deck to bring (personal, Commander, or meta), Endless toggle, and the fight. */
function HordeDetail({ horde }: { horde: CareerHorde }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const meta = useMetaLibrary(true);
  const roster = useDeckRoster();
  const loaded = useDecks((d) => d.loaded);
  const refresh = useDecks((d) => d.refresh);
  const loadList = useDecks((d) => d.loadList);
  const formats = useMemo(() => {
    const base = constructedFormats(meta.data);
    const cmdr = (meta.data ?? []).find((f) => f.id === 'commander');
    if (cmdr && cmdr.decks && cmdr.decks.length > 0 && !base.some((f) => f.id === 'commander')) {
      return [...base, { id: cmdr.id ?? 'commander', name: cmdr.name ?? 'Commander', decks: cmdr.decks }];
    }
    return base;
  }, [meta.data]);
  const personalDecks = useMemo(() => roster.filter((d) => d.cardCount >= 40), [roster]);
  const [selection, setSelection] = useState('');
  const [endless, setEndless] = useState(false);
  const [versus, setVersus] = useState(false);
  const rules = hordeLines(horde);

  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);

  const defaultSel = personalDecks[0]
    ? `deck:${personalDecks[0].id}`
    : formats[0]?.decks[0]?.file
      ? `meta:${formats[0].decks[0].file}`
      : '';
  const activeSel = selection || defaultSel;
  const pickedPersonal = activeSel.startsWith('deck:') ? personalDecks.find((d) => `deck:${d.id}` === activeSel) : null;
  const pickedMeta = activeSel.startsWith('meta:')
    ? formats.flatMap((format) => format.decks).find((deck) => `meta:${deck.file}` === activeSel)
    : null;
  const isCommanderDeck = pickedPersonal
    ? isCommanderFormat(pickedPersonal.format) && pickedPersonal.cardCount >= 98
    : !!pickedMeta?.deckType?.includes('Commander');
  const chosenName = pickedPersonal?.name ?? pickedMeta?.name ?? '';
  const chosenColors = pickedPersonal ? pickedPersonal.colors.join('') : pickedMeta?.colors;

  useSceneArt(horde.cover);

  return (
    <article className={styles.detail} aria-labelledby="horde-name">
      <div className={styles.detailArt}>
        <PortraitCard name={horde.name ?? ''} cover={horde.cover} boss size="sm" />
      </div>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <h2 id="horde-name">{horde.name}</h2>
          <span className={styles.stateTag}>{t(difficultyKey(horde.difficulty))}</span>
          {horde.beaten && <span className={[styles.stateTag, styles.tag_done].join(' ')}>{t('career.horde.beaten')}</span>}
        </header>
        {horde.text && <p className={styles.small}>{horde.text}</p>}
        <h3 className={styles.subhead}>{t('career.horde.rules')}</h3>
        <LineList lines={rules} className={styles.twists} />
        {isCommanderDeck && (
          <p className={styles.small}><Crown size={14} aria-hidden="true" /> {t('career.horde.commanderScale')}</p>
        )}
        {endless && (
          <p className={styles.small}><InfinityIcon size={14} aria-hidden="true" /> {t('career.horde.endlessRule')}</p>
        )}
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
          <select value={activeSel} onChange={(event) => setSelection(event.target.value)} disabled={!activeSel}>
            {personalDecks.length > 0 && (
              <optgroup label={t('career.horde.yourDecks')}>
                {personalDecks.map((deck) => <option key={deck.id} value={`deck:${deck.id}`}>{deck.name}</option>)}
              </optgroup>
            )}
            {formats.map((format) => (
              <optgroup key={format.id} label={format.name}>
                {format.decks.map((deck) => <option key={deck.file} value={`meta:${deck.file}`}>{deck.name}</option>)}
              </optgroup>
            ))}
          </select>
        </label>
        <label className={styles.field}>
          <input type="checkbox" checked={endless} onChange={(event) => setEndless(event.target.checked)} />{' '}
          <span>{t('career.horde.endlessToggle')}</span>
        </label>
        {meta.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        <div className={styles.actions}>
          <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={playing || !activeSel} onClick={() => setVersus(true)} data-nav>
            {t((horde.attempts ?? 0) > 0 ? 'career.horde.again' : 'career.horde.play')}
          </Button>
        </div>
      </div>
      {versus && (
        <Versus
          kicker={endless ? `${t('career.horde.title')} · ${t('career.horde.endlessTag')}` : t('career.horde.title')}
          opponent={{ name: horde.name ?? '', tagline: horde.text, cover: horde.cover, boss: true }}
          deck={{ name: chosenName, colors: chosenColors }}
          stakes={[
            ...rules.map((line) => t(line.key, line.vars)),
            ...(isCommanderDeck ? [t('career.horde.commanderScale')] : []),
            ...(endless ? [t('career.horde.endlessRule')] : []),
          ]}
          onFight={() => playMode('horde', async () => {
            if (pickedPersonal) {
              const list = await loadList(pickedPersonal.id);
              return api.careerHordePlay(horde.id ?? '', null, toWire(list) as never, endless);
            }
            return api.careerHordePlay(horde.id ?? '', pickedMeta?.file ?? '', null, endless);
          })}
          onClose={() => setVersus(false)}
        />
      )}
    </article>
  );
}
