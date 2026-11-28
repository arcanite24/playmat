import { Check, Crown, Sparkles, Swords, Users } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerThgPair, CareerThgPartner } from '../../protocol/generated/views';
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
import { playMode, useMetaLibrary, useThg } from './careerModesData';
import { useSceneArt } from './careerScene';
import { ModeResult } from './ModeParts';
import { PortraitCard } from './Portraits';
import { Versus } from './Versus';
import styles from './CareerModes.module.css';

registerMessages(messages);

/**
 * Two-Headed Giant Solo mode (Phase 5.4): pair up with a chosen AI partner against an opposing 2-head team
 * across Constructed (30 life) and Commander (40 life), with optional Planechase.
 */
export function CareerThgScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const thg = useThg(!!profile);
  const [chosen, setChosen] = useState<string | null>(null);
  const pairs = thg.data?.pairs ?? [];
  const partners = thg.data?.partners ?? [];
  const current = pairs.find((item) => item.id === chosen) ?? pairs.find((item) => !item.beaten) ?? pairs[0];

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;

  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="thg" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{t('career.thg.title')}</h1>
            <p className={styles.lead}>{t('career.thg.lead')}</p>
          </div>
          {pairs.length > 0 && (
            <p className={styles.progressNote}>
              {t('career.horde.beatenCount', { beaten: pairs.filter((item) => item.beaten).length, total: pairs.length })}
            </p>
          )}
        </header>
        {thg.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {thg.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {current && (
          <div className={styles.split}>
            <div className={styles.mainCol}>
              <ul className={styles.hordeList} aria-label={t('career.thg.title')}>
                {pairs.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      className={[styles.hordeTile, item.id === current.id ? styles.puzzleTileOn : ''].join(' ')}
                      aria-pressed={item.id === current.id}
                      onClick={() => setChosen(item.id ?? null)}
                      data-nav
                    >
                      <b>{item.name}</b>
                      <small>{item.commander ? t('career.thg.commanderTag') : t('career.thg.constructedTag')}</small>
                      {item.beaten && <Check size={16} aria-label={t('career.horde.beaten')} />}
                    </button>
                  </li>
                ))}
              </ul>
              <PairDetail key={current.id} pair={current} partners={partners} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function PairDetail({ pair, partners }: { pair: CareerThgPair; partners: readonly CareerThgPartner[] }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const meta = useMetaLibrary(true);
  const roster = useDeckRoster();
  const loaded = useDecks((d) => d.loaded);
  const refresh = useDecks((d) => d.refresh);
  const loadList = useDecks((d) => d.loadList);
  const [partnerId, setPartnerId] = useState('');
  const [selection, setSelection] = useState('');
  const [planeChase, setPlaneChase] = useState(false);
  const [versus, setVersus] = useState(false);

  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);

  const matchingPartners = useMemo(
    () => partners.filter((p) => Boolean(p.commander) === Boolean(pair.commander)),
    [partners, pair.commander],
  );
  const chosenPartner = matchingPartners.find((p) => p.id === partnerId) ?? matchingPartners[0];

  const personalDecks = useMemo(
    () => roster.filter((d) => (pair.commander ? isCommanderFormat(d.format) && d.cardCount >= 98 : d.cardCount >= 40 && !isCommanderFormat(d.format))),
    [roster, pair.commander],
  );
  const metaFormats = useMemo(
    () => (meta.data ?? []).filter((f) => (pair.commander ? f.id === 'commander' : f.id !== 'commander' && f.id !== 'limited')),
    [meta.data, pair.commander],
  );
  const defaultSel = personalDecks[0] ? `deck:${personalDecks[0].id}` : metaFormats[0]?.decks?.[0]?.file ? `meta:${metaFormats[0].decks[0].file}` : '';
  const activeSel = selection || defaultSel;
  const deckName = activeSel.startsWith('deck:')
    ? personalDecks.find((d) => `deck:${d.id}` === activeSel)?.name ?? ''
    : metaFormats.flatMap((f) => f.decks ?? []).find((d) => `meta:${d.file}` === activeSel)?.name ?? '';

  useSceneArt(pair.cover);

  return (
    <article className={styles.detail} aria-labelledby="thg-name">
      <div className={styles.detailArt}>
        <PortraitCard name={pair.name ?? ''} cover={pair.cover} boss size="sm" />
      </div>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <h2 id="thg-name">{pair.name}</h2>
          {pair.beaten && <span className={[styles.stateTag, styles.tag_done].join(' ')}>{t('career.horde.beaten')}</span>}
        </header>
        {pair.text && <p className={styles.small}>{pair.text}</p>}
        <ul className={styles.twists}>
          <li>
            {pair.commander ? <Crown size={14} aria-hidden="true" /> : <Users size={14} aria-hidden="true" />}{' '}
            {t('career.thg.lifeSummary', { life: pair.teamLife ?? 30 })}
          </li>
          {(pair.enemies?.length ?? 0) > 0 && (
            <li>{t('career.thg.enemiesLabel')}: {pair.enemies!.join(' & ')}</li>
          )}
          {planeChase && <li><Sparkles size={14} aria-hidden="true" /> {t('career.planechase.active')}</li>}
        </ul>

        <div className={styles.thgPartners}>
          <span className={styles.cardLabel}><Users size={13} aria-hidden="true" /> {t('career.thg.partnerLabel')}</span>
          <div className={styles.hordeList} role="radiogroup" aria-label={t('career.thg.partnerLabel')}>
            {matchingPartners.map((partner) => (
              <button
                key={partner.id}
                type="button"
                role="radio"
                aria-checked={chosenPartner?.id === partner.id}
                className={[styles.hordeTile, chosenPartner?.id === partner.id ? styles.puzzleTileOn : ''].join(' ')}
                onClick={() => setPartnerId(partner.id ?? '')}
                data-nav
              >
                <b>{partner.name}</b>
                <small>{partner.archetype}</small>
              </button>
            ))}
          </div>
          {chosenPartner?.text && <p className={styles.small}>{chosenPartner.text}</p>}
        </div>

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
          <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={playing || !chosenPartner || !activeSel} onClick={() => setVersus(true)} data-nav>
            {t('career.thg.play')}
          </Button>
        </div>
      </div>
      {versus && chosenPartner && (
        <Versus
          kicker={t('career.thg.title')}
          opponent={{ name: pair.name ?? '', tagline: (pair.enemies ?? []).join(' & '), cover: pair.cover, boss: true }}
          deck={{ name: `${deckName} + ${chosenPartner.name}` }}
          stakes={[
            t('career.thg.lifeSummary', { life: pair.teamLife ?? 30 }),
            ...(planeChase ? [t('career.planechase.active')] : []),
          ]}
          onFight={() => playMode('thg', async () => {
            if (activeSel.startsWith('deck:')) {
              const id = activeSel.slice('deck:'.length);
              const list = await loadList(id);
              return api.careerThgPlay(pair.id ?? '', chosenPartner.id ?? '', null, toWire(list) as never, planeChase);
            }
            const file = activeSel.slice('meta:'.length);
            return api.careerThgPlay(pair.id ?? '', chosenPartner.id ?? '', file, null, planeChase);
          })}
          onClose={() => setVersus(false)}
        />
      )}
    </article>
  );
}
