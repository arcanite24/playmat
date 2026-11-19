import { Crown, Flame, Swords, Trophy } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Navigate, useSearchParams } from 'react-router-dom';
import type { LadderOpponent, LadderResult, LadderState } from '../../protocol/generated/views';
import { toWire } from '../decks/deckModel';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { useDeckRoster } from '../stores/useDeckRoster';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { CareerAvatar } from './CareerScreen';
import { Pips } from './Portraits';
import { climb, movement, pickFormat, pipRow, rankAt, rankKey, record, type PipState } from './rankedModel';
import { playRanked, useRankedOpponent, useRankedResult, useRankedState } from './rankedData';
import { Versus } from './Versus';
import modes from './CareerModes.module.css';
import styles from './Ranked.module.css';

registerMessages(messages);

/**
 * Ranked (#431): one ladder a format, Bronze to Mythic, against bots near your hidden rating. Pick a deck legal in the
 * format, meet the opponent the server picked (it holds until played), play; after the game the pips move here.
 */
export function CareerRankedScreen() {
  const t = useT();
  const career = useCareerState();
  const [params, setParams] = useSearchParams();
  const wanted = params.get('format');
  const tableId = params.get('table');
  const first = useRankedState(pickFormat(undefined, wanted), !!career.data?.enabled);
  const format = pickFormat(first.data?.formats, wanted);
  const state = useRankedState(format, !!career.data?.enabled);

  if (career.isPending) return <p className={modes.note}>{t('career.loading')}</p>;
  if (!career.data?.enabled) return <Navigate to="/career" replace />;
  const formats = state.data?.formats ?? first.data?.formats ?? ['standard', 'pioneer'];
  return (
    <div className={modes.page}>
      <div className={modes.scroll}>
        <header className={modes.modeHead}>
          <div>
            <p className={modes.kicker}><Trophy size={16} aria-hidden="true" /> {t('career.ranked.kicker')}</p>
            <h1 className={modes.modeTitle}>{t('career.ranked.title')}</h1>
            <p className={modes.lead}>{t('career.ranked.lead')}</p>
          </div>
          <div className={styles.formats} role="tablist" aria-label={t('career.ranked.formats')}>
            {formats.map((item) => (
              <button
                key={item}
                type="button"
                role="tab"
                aria-selected={item === format}
                className={[styles.format, item === format ? styles.formatOn : ''].join(' ')}
                onClick={() => setParams({ format: item })}
                data-nav
              >
                {t(`career.ranked.format.${item === 'pioneer' ? 'pioneer' : 'standard'}`)}
              </button>
            ))}
          </div>
        </header>
        {tableId && <GameResult tableId={tableId} />}
        {state.isPending && <p className={modes.note}>{t('career.loading')}</p>}
        {state.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
        {state.data && (
          <div className={modes.split}>
            <div className={styles.main}>
              <Standing state={state.data} />
              <Recent state={state.data} />
            </div>
            <Queue key={format} format={format} state={state.data} />
          </div>
        )}
      </div>
    </div>
  );
}

function Emblem({ rank, size = 'lg' }: { rank: string | undefined; size?: 'sm' | 'lg' }) {
  const t = useT();
  const known = rank ?? 'bronze';
  return (
    <span className={[styles.emblem, styles[`rank_${known}`] ?? '', size === 'sm' ? styles.emblemSm : ''].join(' ')} role="img" aria-label={t(rankKey(rank))}>
      {rank === 'mythic' ? <Crown aria-hidden="true" /> : <Trophy aria-hidden="true" />}
    </span>
  );
}

function PipRow({ pips, label }: { pips: PipState[]; label: string }) {
  if (pips.length === 0) return null;
  return (
    <span className={styles.pips} role="img" aria-label={label}>
      {pips.map((pip, index) => (
        <span key={index} className={[styles.pip, styles[`pip_${pip}`]].join(' ')} style={{ animationDelay: `${index * 120}ms` }} />
      ))}
    </span>
  );
}

function Standing({ state }: { state: LadderState }) {
  const t = useT();
  const step = state.step ?? 0;
  const at = rankAt(step);
  const games = record(state);
  return (
    <section className={[styles.card, styles[`rank_${at.rank}`]].join(' ')} aria-labelledby="ranked-standing">
      <Emblem rank={state.rank} />
      <div className={styles.cardBody}>
        <h2 id="ranked-standing" className={styles.rankName}>{state.label ?? t(rankKey(state.rank))}</h2>
        <PipRow pips={pipRow(step)} label={t('career.ranked.pips', { pips: at.pips, of: state.pipsPerTier ?? 4 })} />
        {state.mythicRating != null && <p className={styles.mythic}>{t('career.ranked.mythicRating', { rating: state.mythicRating })}</p>}
        <div className={styles.climb} aria-hidden="true"><span style={{ width: `${climb(step) * 100}%` }} /></div>
        <p className={modes.small}>
          {games.games === 0
            ? t('career.ranked.noGames')
            : t('career.ranked.record', { won: games.won, lost: games.lost, drew: games.drew })}
          {(state.peakStep ?? 0) > step ? ` · ${t('career.ranked.peak', { rank: state.peakLabel ?? '' })}` : ''}
        </p>
        {(state.streak ?? 0) >= 2 && (
          <p className={styles.streak}><Flame size={14} aria-hidden="true" /> {t('career.ranked.streak', { count: state.streak ?? 0 })}</p>
        )}
      </div>
    </section>
  );
}

function OpponentCard({ opponent }: { opponent: LadderOpponent }) {
  const t = useT();
  return (
    <div className={styles.opponent}>
      <CareerAvatar id={opponent.avatar} name={opponent.name ?? ''} size={64} />
      <div>
        <b className={styles.opponentName}>{opponent.name}</b>
        {opponent.title && <small className={modes.small}>{titleWords(opponent.title)}</small>}
        <p className={modes.small}>
          {t('career.ranked.plays', { deck: opponent.deckName ?? '' })} <Pips colors={opponent.colors} size="sm" />
        </p>
      </div>
    </div>
  );
}

/** "ladder-gold" → "Ladder gold": the catalog's names live in the locker; this is a fallback for the card */
function titleWords(id: string): string {
  const words = id.replace(/^t-/, '').replace(/-/g, ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function Queue({ format, state }: { format: string; state: LadderState }) {
  const t = useT();
  const opponent = useRankedOpponent(format);
  const choices = useDeckRoster();
  const loaded = useDecks((decks) => decks.loaded);
  const refresh = useDecks((decks) => decks.refresh);
  const loadList = useDecks((decks) => decks.loadList);
  const busy = usePlay((play) => play.phase !== 'idle');
  const deckKey = `playmat.rankedDeck.${format}`;
  const [deckId, setDeckId] = useState<string | null>(() => read(deckKey));
  const [versus, setVersus] = useState(false);

  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);

  // decks built for the format first
  const decks = useMemo(() => {
    const fits = (deck: (typeof choices)[number]) => (deck.starter?.format ?? '').toLowerCase().includes(format);
    return [...choices].sort((a, b) => Number(fits(b)) - Number(fits(a)));
  }, [choices, format]);
  const deck = decks.find((item) => item.id === deckId) ?? null;

  function choose(id: string) {
    setDeckId(id);
    write(deckKey, id);
  }

  const them = opponent.data;
  return (
    <aside className={styles.queue} aria-labelledby="ranked-next">
      <h2 id="ranked-next" className={modes.subhead}>{t('career.ranked.next')}</h2>
      {opponent.isPending && <p className={modes.note}>{t('career.loading')}</p>}
      {opponent.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
      {them && <OpponentCard opponent={them} />}
      <label className={styles.field}>
        <span>{t('career.ranked.deck')}</span>
        <select value={deckId ?? ''} onChange={(event) => choose(event.target.value)} disabled={!loaded} data-nav>
          <option value="" disabled>{loaded ? t('career.ranked.chooseDeck') : t('career.loading')}</option>
          {decks.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
        </select>
      </label>
      <p className={modes.small}>{t('career.ranked.legal', { format: t(`career.ranked.format.${format === 'pioneer' ? 'pioneer' : 'standard'}`) })}</p>
      <div className={modes.actions}>
        <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={!deck || !them || busy || !!state.playing} onClick={() => setVersus(true)} data-nav>
          {t('career.ranked.play')}
        </Button>
      </div>
      {state.playing && <p className={modes.small}>{t('career.ranked.playingNow')}</p>}
      <ul className={modes.lines} aria-label={t('career.ranked.rules')}>
        <li>{t('career.ranked.ruleBo1')}</li>
        <li>{t('career.ranked.ruleNoAssists')}</li>
        <li>{t('career.ranked.ruleLeave')}</li>
      </ul>
      {versus && them && deck && (
        <Versus
          kicker={`${t('career.ranked.title')} · ${state.label ?? ''}`}
          opponent={{ name: them.name ?? '', colors: them.colors, tagline: them.deckName }}
          deck={{ name: deck.name, art: deck.cover, colors: deck.colors.join('') }}
          stakes={[t('career.ranked.stakeWin'), t(state.step && state.step >= 16 ? 'career.ranked.stakeLoss' : 'career.ranked.stakeBronze')]}
          onFight={async () => playRanked(format, toWire(await loadList(deck.id)))}
          onClose={() => setVersus(false)}
        />
      )}
    </aside>
  );
}

function Recent({ state }: { state: LadderState }) {
  const t = useT();
  const games = state.recent ?? [];
  if (games.length === 0) return null;
  return (
    <section aria-labelledby="ranked-recent">
      <h2 id="ranked-recent" className={modes.subhead}>{t('career.ranked.recent')}</h2>
      <ul className={styles.recent}>
        {games.map((game) => (
          <li key={game.id} className={styles[`game_${game.status ?? 'playing'}`]}>
            <b>{t(`career.ranked.status.${(['won', 'lost', 'drew'] as const).find((s) => s === game.status) ?? 'playing'}`)}</b>
            <span>{game.opponent?.name ?? ''}</span>
            <small className={modes.small}>{game.deck}</small>
          </li>
        ))}
      </ul>
    </section>
  );
}

function GameResult({ tableId }: { tableId: string }) {
  const t = useT();
  const result = useRankedResult(tableId);
  if (result.isPending) return <p className={modes.note} role="status">{t('career.ranked.counting')}</p>;
  if (!result.data) return null;
  return <ResultCard result={result.data} />;
}

function ResultCard({ result }: { result: LadderResult }) {
  const t = useT();
  const moved = movement(result);
  const after = rankAt(result.stepAfter ?? 0);
  const reached = result.reached ?? [];
  return (
    <section className={[styles.result, styles[`result_${result.status ?? 'drew'}`]].join(' ')} role="status" aria-label={t('career.ranked.lastGame')}>
      <Emblem rank={after.rank} size="sm" />
      <div className={styles.cardBody}>
        <h2 className={styles.resultTitle}>
          {t(`career.ranked.status.${(['won', 'lost', 'drew'] as const).find((s) => s === result.status) ?? 'drew'}`)}
          {result.opponent?.name ? ` · ${result.opponent.name}` : ''}
        </h2>
        <PipRow pips={pipRow(result.stepAfter ?? 0, result.stepBefore ?? 0)} label={result.labelAfter ?? ''} />
        <p className={modes.small}>
          {t(`career.ranked.moved.${moved}`, { from: result.labelBefore ?? '', to: result.labelAfter ?? '' })}
          {result.bonus ? ` · ${t('career.ranked.bonus')}` : ''}
        </p>
        {reached.map((rank) => (
          <p key={rank} className={styles.reached}><Trophy size={14} aria-hidden="true" /> {t('career.ranked.reached', { rank: t(rankKey(rank)) })}</p>
        ))}
        {((result.cosmetics ?? []).length > 0 || (result.tokens ?? 0) > 0) && (
          <p className={modes.small}>
            {[
              (result.cosmetics ?? []).length > 0 ? t('career.ranked.cosmetics', { count: (result.cosmetics ?? []).length }) : null,
              (result.tokens ?? 0) > 0 ? t('career.ranked.tokens', { count: result.tokens ?? 0 }) : null,
            ].filter(Boolean).join(' · ')}
          </p>
        )}
      </div>
    </section>
  );
}

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // a private window keeps no choice
  }
}
