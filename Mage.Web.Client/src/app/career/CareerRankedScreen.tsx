import { Crown, Eye, Flame, Shield, Swords, Trophy } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Link, Navigate, useSearchParams } from 'react-router-dom';
import type { LadderDeckStat, LadderOpponent, LadderResult, LadderState } from '../../protocol/generated/views';
import { toWire } from '../decks/deckModel';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { useDeckRoster } from '../stores/useDeckRoster';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { useCareerState } from './careerData';
import { CareerAvatar } from './CareerScreen';
import { Pips } from './Portraits';
import { climb, formatKey, movement, pickFormat, pipRow, rankAt, rankKey, record, titleWords, type PipState } from './rankedModel';
import {
  playRanked,
  useLegalRankedDecks,
  useMythicStandings,
  useRankedGameDeck,
  useRankedOpponent,
  useRankedResult,
  useRankedState,
  useSeasonEnd,
} from './rankedData';
import { RankedDeckPicker } from './RankedDeckPicker';
import { RankedIntro } from './RankedIntro';
import { SeasonBoards } from './SeasonBoards';
import { Versus } from './Versus';
import modes from './CareerModes.module.css';
import styles from './Ranked.module.css';

registerMessages(messages);

const ARCHETYPES = new Set([
  'aggro',
  'burn',
  'tempo',
  'midrange',
  'control',
  'ramp',
  'combo',
  'graveyard',
  'sacrifice',
  'tribal',
]);

function archetypeKey(arch: string | undefined): MessageKey {
  const k = (arch ?? '').toLowerCase();
  return (ARCHETYPES.has(k) ? `career.ranked.arch.${k}` : 'career.ranked.arch.midrange') as MessageKey;
}

/**
 * Ranked (#431 + Playbooks & Rotation overhaul): one ladder a format, Bronze to Mythic, against a wide
 * pool of format-legal AI decks near your hidden rating. Losing always rotates to a different deck.
 */
export function CareerRankedScreen() {
  const t = useT();
  const career = useCareerState();
  const [params, setParams] = useSearchParams();
  const wanted = params.get('format') ?? read(LAST_FORMAT);
  const tableId = params.get('table');
  const rawQueue = params.has('format') ? params.get('queue') : read(LAST_QUEUE);
  const initialQueue: 'open' | 'bo3' | 'pod' = rawQueue === 'bo3' ? 'bo3' : rawQueue === 'pod' ? 'pod' : 'open';
  const first = useRankedState(pickFormat(undefined, wanted === 'limited' ? 'standard' : wanted), !!career.data?.enabled, initialQueue === 'pod' && wanted !== 'commander' ? 'open' : initialQueue);
  const baseFormats = first.data?.formats ?? ['standard', 'pioneer'];
  const allFormats = baseFormats.includes('limited') ? baseFormats : [...baseFormats, 'limited'];
  const format = pickFormat(allFormats, wanted);
  const isCommander = format === 'commander';
  const isLimited = format === 'limited';
  const isBrawl = format === 'brawl';
  const queue: 'open' | 'bo3' | 'pod' = isCommander
    ? (initialQueue === 'pod' ? 'pod' : 'open')
    : isLimited || isBrawl
      ? 'open'
      : (initialQueue === 'bo3' ? 'bo3' : 'open');
  const state = useRankedState(format, !!career.data?.enabled, queue);
  const [intro, setIntro] = useState(false);
  const [inspectGameId, setInspectGameId] = useState<string | null>(null);
  const [versus, setVersus] = useState(false);

  useEffect(() => {
    if (!first.data?.formats) return;
    write(LAST_FORMAT, format);
    write(LAST_QUEUE, queue);
  }, [first.data?.formats, format, queue]);

  const go = (next: { format?: string; queue?: 'open' | 'bo3' | 'pod' }) => {
    const nextFormat = next.format ?? format;
    const requestedQueue = next.queue ?? queue;
    const nextQueue = nextFormat === 'commander'
      ? (requestedQueue === 'pod' ? 'pod' : 'open')
      : nextFormat === 'limited' || nextFormat === 'brawl'
        ? 'open'
        : (requestedQueue === 'bo3' ? 'bo3' : 'open');
    const query: Record<string, string> = { format: nextFormat };
    if (nextQueue === 'bo3' || nextQueue === 'pod') query.queue = nextQueue;
    setParams(query);
  };

  if (career.isPending) return <p className={modes.note}>{t('career.loading')}</p>;
  if (!career.data?.enabled) return <Navigate to="/career" replace />;
  const serverFormats = state.data?.formats ?? first.data?.formats ?? ['standard', 'pioneer'];
  const formats = serverFormats.includes('limited') ? serverFormats : [...serverFormats, 'limited'];
  const queueTabs: readonly ('open' | 'bo3' | 'pod')[] = isCommander
    ? ['open', 'pod']
    : isLimited || isBrawl
      ? []
      : ['open', 'bo3'];
  const kickerKey: MessageKey = isLimited
    ? 'career.ranked.kickerLimited'
    : queue === 'bo3'
      ? 'career.ranked.kickerBo3'
      : queue === 'pod'
        ? 'career.ranked.kickerPod'
        : 'career.ranked.kicker';
  return (
    <div className={modes.page}>
      <div className={`${modes.scroll} ${modes.capped}`}>
        <header className={modes.modeHead}>
          <div>
            <p className={modes.kicker}><Trophy size={16} aria-hidden="true" /> {t(kickerKey)}</p>
            <h1 className={modes.modeTitle}>{t('career.ranked.title')}</h1>
            <p className={modes.lead}>
              {t('career.ranked.lead')}{' '}
              <button type="button" className={styles.howLink} onClick={() => setIntro(true)} data-nav>{t('career.ranked.howItWorks')}</button>
            </p>
          </div>
          <div className={styles.switches}>
            {queueTabs.length > 1 && (
              <div className={styles.formats} role="tablist" aria-label={t('career.ranked.queues')}>
                {queueTabs.map((item) => (
                  <button
                    key={item}
                    type="button"
                    role="tab"
                    aria-selected={item === queue}
                    className={[styles.format, item === queue ? styles.formatOn : ''].join(' ')}
                    onClick={() => go({ queue: item })}
                    data-nav
                  >
                    {t(
                      item === 'bo3'
                        ? 'career.ranked.queue.bo3'
                        : item === 'pod'
                          ? 'career.ranked.queue.pod'
                          : isCommander
                            ? 'career.ranked.queue.duel'
                            : 'career.ranked.queue.bo1',
                    )}
                  </button>
                ))}
              </div>
            )}
            <div className={styles.formats} role="tablist" aria-label={t('career.ranked.formats')}>
              {formats.map((item) => (
                <button
                  key={item}
                  type="button"
                  role="tab"
                  aria-selected={item === format}
                  className={[styles.format, item === format ? styles.formatOn : ''].join(' ')}
                  onClick={() => go({ format: item })}
                  data-nav
                >
                  {t(formatKey(item))}
                </button>
              ))}
            </div>
          </div>
        </header>
        <RankedIntro open={intro} onClose={() => setIntro(false)} />
        {tableId && (
          <GameResult
            tableId={tableId}
            onInspect={() => setInspectGameId(tableId)}
            onNextMatch={() => setVersus(true)}
          />
        )}
        {state.isPending && <p className={modes.note}>{t('career.loading')}</p>}
        {state.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
        {state.data && (
          <div className={modes.split}>
            <div className={styles.main}>
              <Standing state={state.data} />
              <Analytics state={state.data} />
              <Recent state={state.data} onInspect={(id) => setInspectGameId(id)} />
              <MythicStandings format={format} queue={queue} />
              <SeasonBoards state={state.data} />
            </div>
            {isLimited ? (
              <aside className={styles.queue} aria-labelledby="ranked-limited-queue">
                <h2 id="ranked-limited-queue" className={modes.subhead}>{t('career.ranked.limitedTitle')}</h2>
                <p className={modes.small}>{t('career.ranked.limitedLead')}</p>
                <div className={modes.actions}>
                  <Link to="/career/limited" data-nav>
                    <Button variant="decision" size="lg" icon={<Swords size={18} />}>{t('career.ranked.limitedStart')}</Button>
                  </Link>
                </div>
                <ul className={modes.lines} aria-label={t('career.ranked.rules')}>
                  <li>{t('career.ranked.limitedRule1')}</li>
                  <li>{t('career.ranked.limitedRule2')}</li>
                  <li>{t('career.ranked.limitedRule3')}</li>
                </ul>
              </aside>
            ) : (
              <Queue
                key={`${format}-${queue}`}
                format={format}
                queue={queue}
                state={state.data}
                versus={versus}
                setVersus={setVersus}
              />
            )}
          </div>
        )}
        <OpponentDeckDialog gameId={inspectGameId} onClose={() => setInspectGameId(null)} />
      </div>
    </div>
  );
}

export function Emblem({ rank, size = 'lg' }: { rank: string | undefined; size?: 'sm' | 'lg' }) {
  const t = useT();
  const known = rank ?? 'bronze';
  return (
    <span className={[styles.emblem, styles[`rank_${known}`] ?? '', size === 'sm' ? styles.emblemSm : ''].join(' ')} role="img" aria-label={t(rankKey(rank))}>
      {rank === 'mythic' ? <Crown aria-hidden="true" /> : <Trophy aria-hidden="true" />}
    </span>
  );
}

export function PipRow({ pips, label }: { pips: PipState[]; label: string }) {
  if (pips.length === 0) return null;
  return (
    <span className={styles.pips} role="img" aria-label={label}>
      {pips.map((pip, index) => (
        <span key={index} className={[styles.pip, styles[`pip_${pip}`]].join(' ')} style={{ animationDelay: `${index * 120}ms` }} />
      ))}
    </span>
  );
}

export function Standing({ state }: { state: LadderState }) {
  const t = useT();
  const step = state.step ?? 0;
  const at = rankAt(step);
  const games = record(state);
  useSeasonEnd(state);
  const stamps = (state.seasons ?? []).slice(0, 4);
  return (
    <section className={[styles.card, styles[`rank_${at.rank}`]].join(' ')} aria-labelledby="ranked-standing">
      <Emblem rank={state.rank} />
      <div className={styles.cardBody}>
        {state.promotionNext && state.promotionTarget && (
          <p className={styles.promoBanner}>
            <Trophy size={12} aria-hidden="true" /> {t('career.ranked.promotion', { rank: t(rankKey(state.promotionTarget)) })}
          </p>
        )}
        <h2 id="ranked-standing" className={styles.rankName}>{state.label ?? t(rankKey(state.rank))}</h2>
        <PipRow pips={pipRow(step)} label={t('career.ranked.pips', { pips: at.pips, of: state.pipsPerTier ?? 4 })} />
        {state.mythicRating != null && <p className={styles.mythic}>{t('career.ranked.mythicRating', { rating: state.mythicRating })}</p>}
        <div className={styles.climb} aria-hidden="true"><span style={{ width: `${climb(step) * 100}%` }} /></div>
        <div className={styles.metaRow}>
          <p className={modes.small}>
            {games.games === 0
              ? t('career.ranked.noGames')
              : t('career.ranked.record', { won: games.won, lost: games.lost, drew: games.drew })}
            {(state.peakStep ?? 0) > step ? ` · ${t('career.ranked.peak', { rank: state.peakLabel ?? '' })}` : ''}
          </p>
          {state.floorLabel && (
            <span className={styles.badge}><Shield size={11} aria-hidden="true" /> {t('career.ranked.floor', { rank: state.floorLabel })}</span>
          )}
        </div>
        {(state.streak ?? 0) >= 2 && (
          <p className={styles.streak}><Flame size={14} aria-hidden="true" /> {t('career.ranked.streak', { count: state.streak ?? 0 })}</p>
        )}
        {state.nextRewardRank && (
          <p className={modes.small}>
            {t(state.nextRewardToken ? 'career.ranked.nextRewardToken' : 'career.ranked.nextReward', { rank: t(rankKey(state.nextRewardRank)) })}
          </p>
        )}
        {state.seasonEnds ? (
          <p className={modes.small}>{t('career.season.left', { n: (state.season ?? 0) + 1, count: state.seasonDaysLeft ?? 0 })}</p>
        ) : null}
        {stamps.length > 0 && (
          <p className={modes.small}>
            {t('career.season.stamps', { list: stamps.map((past) => t('career.season.stamp', { n: (past.season ?? 0) + 1, rank: past.peakLabel ?? '' })).join(' · ') })}
          </p>
        )}
      </div>
    </section>
  );
}

export function OpponentCard({ opponent }: { opponent: LadderOpponent }) {
  const t = useT();
  const h2hTotal = (opponent.vsWon ?? 0) + (opponent.vsLost ?? 0);
  return (
    <div className={styles.opponent}>
      <CareerAvatar id={opponent.avatar} name={opponent.name ?? ''} size={64} />
      <div>
        <b className={styles.opponentName}>{opponent.name}</b>
        {opponent.title && <small className={modes.small}>{titleWords(opponent.title)}</small>}
        <p className={modes.small}>
          {opponent.deckHidden || !opponent.deckName ? (
            <>
              {t('career.ranked.deckHidden')}
              {h2hTotal > 0 ? ` · ${t('career.ranked.h2h', { won: opponent.vsWon ?? 0, lost: opponent.vsLost ?? 0 })}` : ''}
            </>
          ) : (
            <>
              {t('career.ranked.plays', { deck: opponent.deckName })} <Pips colors={opponent.colors} size="sm" />
            </>
          )}
        </p>
      </div>
    </div>
  );
}

function Queue({ format, queue, state, versus, setVersus }: {
  format: string;
  queue: 'open' | 'bo3' | 'pod';
  state: LadderState;
  versus: boolean;
  setVersus: (open: boolean) => void;
}) {
  const t = useT();
  const opponent = useRankedOpponent(format, true, queue);
  const choices = useDeckRoster();
  const loaded = useDecks((decks) => decks.loaded);
  const refresh = useDecks((decks) => decks.refresh);
  const loadList = useDecks((decks) => decks.loadList);
  const busy = usePlay((play) => play.phase !== 'idle');
  const deckKey = `playmat.rankedDeck.${format}`;
  const [deckId, setDeckId] = useState<string | null>(() => read(deckKey));

  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);

  const legalQuery = useLegalRankedDecks(format, choices, loaded);
  const decks = legalQuery.data ?? [];
  const checkingDecks = !loaded || legalQuery.isPending;

  const deckStatByName = useMemo(() => {
    const map = new Map<string, LadderDeckStat>();
    for (const s of state.deckStats ?? []) {
      if (s.deck) map.set(s.deck.toLowerCase(), s);
    }
    return map;
  }, [state.deckStats]);

  const deck = decks.find((item) => item.id === deckId) ?? decks[0] ?? null;
  const them = opponent.data;
  const formatLabel = t(formatKey(format));

  function choose(id: string) {
    setDeckId(id);
    write(deckKey, id);
  }

  const field = state.fieldMeta ?? [];
  const isPod = queue === 'pod';
  const bracketProtected = format === 'commander' && (state.step ?? 0) < 32;
  const stakes = [
    ...(state.promotionNext && state.promotionTarget
      ? [t('career.ranked.promotionStake', { rank: t(rankKey(state.promotionTarget)) })]
      : []),
    t(isPod ? 'career.ranked.stakePod' : 'career.ranked.stakeWin'),
    t(state.step && state.step >= 16 ? 'career.ranked.stakeLoss' : 'career.ranked.stakeBronze'),
  ];

  return (
    <aside className={styles.queue} aria-labelledby="ranked-next">
      <div className={styles.field}>
        <h2 id="ranked-next" className={modes.subhead}>{t('career.ranked.deck')}</h2>
        <RankedDeckPicker
          formatLabel={formatLabel}
          decks={decks}
          selected={deck}
          loading={checkingDecks}
          stats={deckStatByName}
          onSelect={choose}
        />
      </div>

      <div className={modes.actions}>
        <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={!deck || !them || busy || !!state.playing} onClick={() => setVersus(true)} data-nav>
          {t('career.ranked.play')}
        </Button>
      </div>
      {state.playing && <p className={modes.small}>{t('career.ranked.playingNow')}</p>}

      {field.length > 0 && (
        <div className={styles.fieldSection}>
          <h3 className={modes.subhead}>{t('career.ranked.fieldTitle')}</h3>
          <p className={modes.small}>{t('career.ranked.fieldLead')}</p>
          <ul className={styles.fieldList}>
            {field.map((entry) => {
              const played = (entry.won ?? 0) + (entry.lost ?? 0);
              return (
                <li key={entry.archetype} className={styles.fieldItem}>
                  <b>{t(archetypeKey(entry.archetype))}</b>
                  <small className={modes.small}>
                    {t('career.ranked.fieldDecks', { count: entry.decks ?? 0 })}
                    {played > 0 ? ` · ${entry.won ?? 0}–${entry.lost ?? 0}` : ''}
                  </small>
                </li>
              );
            })}
          </ul>
        </div>
      )}

      <ul className={modes.lines} aria-label={t('career.ranked.rules')}>
        <li>{t(queue === 'bo3' ? 'career.ranked.ruleBo3' : queue === 'pod' ? 'career.ranked.rulePod' : 'career.ranked.ruleBo1')}</li>
        {bracketProtected && <li>{t('career.ranked.ruleBracketProtected')}</li>}
        <li>{t('career.ranked.ruleNoAssists')}</li>
        <li>{t('career.ranked.ruleLeave')}</li>
      </ul>
      {versus && them && deck && (
        <Versus
          kicker={`${state.promotionNext && state.promotionTarget ? `${t('career.ranked.promotion', { rank: t(rankKey(state.promotionTarget)) })} · ` : ''}${t('career.ranked.title')}${queue === 'bo3' ? ` · ${t('career.ranked.queue.bo3')}` : queue === 'pod' ? ` · ${t('career.ranked.queue.pod')}` : ''} · ${state.label ?? ''}`}
          opponent={{
            name: isPod ? t('career.ranked.podTableTitle', { lead: them.name ?? '' }) : (them.name ?? ''),
            avatar: them.avatar,
            tagline: them.title ? titleWords(them.title) : undefined,
          }}
          deck={{ name: deck.name, art: deck.cover, colors: deck.colors.join('') }}
          stakes={stakes}
          onFight={async () => {
            choose(deck.id);
            await playRanked(format, toWire(await loadList(deck.id)), queue);
          }}
          onClose={() => setVersus(false)}
        />
      )}
    </aside>
  );
}

function Analytics({ state }: { state: LadderState }) {
  const t = useT();
  const [tab, setTab] = useState<'decks' | 'matchups' | 'contracts'>('decks');
  const decks = state.deckStats ?? [];
  const matchups = state.archetypeStats ?? [];
  const contracts = state.contracts ?? [];
  if (decks.length === 0 && matchups.length === 0 && contracts.length === 0) return null;

  return (
    <section className={styles.panel} aria-labelledby="ranked-analytics">
      <div className={styles.panelHead}>
        <h2 id="ranked-analytics" className={modes.subhead}>{t('career.ranked.analytics')}</h2>
        <div className={styles.formats} role="tablist">
          {(['decks', 'matchups', 'contracts'] as const).map((k) => (
            <button
              key={k}
              type="button"
              role="tab"
              aria-selected={tab === k}
              className={[styles.format, tab === k ? styles.formatOn : ''].join(' ')}
              onClick={() => setTab(k)}
            >
              {t(k === 'decks' ? 'career.ranked.tabDecks' : k === 'matchups' ? 'career.ranked.tabMatchups' : 'career.ranked.tabContracts')}
            </button>
          ))}
        </div>
      </div>

      {tab === 'decks' && (
        decks.length === 0 ? <p className={modes.small}>{t('career.ranked.noStats')}</p> : (
          <ul className={styles.statList}>
            {decks.map((d) => {
              const total = (d.won ?? 0) + (d.lost ?? 0);
              const pct = total > 0 ? Math.round(((d.won ?? 0) * 100) / total) : 0;
              return (
                <li key={d.deck} className={styles.statRow}>
                  <b>{d.deck}</b>
                  <span className={modes.small}>{d.won ?? 0}–{d.lost ?? 0}{(d.drew ?? 0) > 0 ? `–${d.drew}` : ''}</span>
                  <span className={styles.badge}>{pct}%</span>
                </li>
              );
            })}
          </ul>
        )
      )}

      {tab === 'matchups' && (
        matchups.length === 0 ? <p className={modes.small}>{t('career.ranked.noStats')}</p> : (
          <ul className={styles.statList}>
            {matchups.map((m) => {
              const total = (m.won ?? 0) + (m.lost ?? 0);
              const pct = total > 0 ? Math.round(((m.won ?? 0) * 100) / total) : 0;
              return (
                <li key={m.archetype} className={styles.statRow}>
                  <b>{t(archetypeKey(m.archetype))}</b>
                  <span className={modes.small}>{m.won ?? 0}–{m.lost ?? 0}{(m.drew ?? 0) > 0 ? `–${m.drew}` : ''}</span>
                  <span className={styles.badge}>{pct}%</span>
                </li>
              );
            })}
          </ul>
        )
      )}

      {tab === 'contracts' && (
        <ul className={styles.statList}>
          {contracts.map((c) => {
            const pct = c.target ? Math.min(100, Math.round(((c.progress ?? 0) * 100) / c.target)) : 0;
            return (
              <li key={c.id} className={styles.statRow}>
                <span>{t(`career.ranked.contract.${c.id}` as MessageKey)}</span>
                <span className={styles.badge}>{c.progress ?? 0} / {c.target ?? 1}</span>
                <span />
                <div className={styles.contractBar} aria-hidden="true"><span style={{ width: `${pct}%` }} /></div>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

function MythicStandings({ format, queue }: { format: string; queue: 'open' | 'bo3' | 'pod' }) {
  const t = useT();
  const [includeBots, setIncludeBots] = useState(true);
  const query = useMythicStandings(format, queue, includeBots, format !== 'limited');
  const rows = (query.data ?? []).slice(0, 15);
  if (format === 'limited' || (query.isSuccess && rows.length === 0 && !includeBots)) return null;

  return (
    <section className={styles.panel} aria-labelledby="ranked-mythic-standings">
      <div className={styles.panelHead}>
        <h2 id="ranked-mythic-standings" className={modes.subhead}>{t('career.ranked.mythicStandings')}</h2>
        <div className={styles.formats} role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={includeBots}
            className={[styles.format, includeBots ? styles.formatOn : ''].join(' ')}
            onClick={() => setIncludeBots(true)}
          >
            {t('career.ranked.includeBots')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={!includeBots}
            className={[styles.format, !includeBots ? styles.formatOn : ''].join(' ')}
            onClick={() => setIncludeBots(false)}
          >
            {t('career.ranked.humansOnly')}
          </button>
        </div>
      </div>
      {query.isPending && <p className={modes.note}>{t('career.loading')}</p>}
      {rows.length > 0 && (
        <ol className={styles.mythicList}>
          {rows.map((row) => (
            <li key={`${row.rank}-${row.name}-${row.bot ? 'b' : 'h'}`} className={[styles.mythicRow, row.you ? styles.mythicYou : ''].join(' ')}>
              <span className={modes.small}>#{row.rank}</span>
              <span className={styles.recentOpp}>
                <b>{row.name}</b>
                {row.bot && <span className={styles.badge}>{t('career.ranked.botTag')}</span>}
                {row.you && <span className={styles.badge}>{t('career.ranked.youTag')}</span>}
              </span>
              <b className={modes.small}>{row.rating}</b>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export function Recent({ state, onInspect }: { state: LadderState; onInspect?: (gameId: string) => void }) {
  const t = useT();
  const games = state.recent ?? [];
  if (games.length === 0) return null;
  return (
    <section aria-labelledby="ranked-recent">
      <h2 id="ranked-recent" className={modes.subhead}>{t('career.ranked.recent')}</h2>
      <ul className={styles.recent}>
        {games.map((game) => (
          <li key={game.id} className={[styles.recentRow, styles[`game_${game.status ?? 'playing'}`]].join(' ')}>
            <b>
              {t(`career.ranked.status.${(['won', 'lost', 'drew'] as const).find((s) => s === game.status) ?? 'playing'}`)}
              {game.gameScore ? ` ${game.gameScore}` : ''}
            </b>
            <span className={styles.recentOpp}>
              <span>{game.opponent?.name ?? ''}</span>
              {game.turns ? <small className={modes.small}>{t('career.ranked.turns', { count: game.turns })}</small> : null}
            </span>
            <small className={modes.small}>{game.deck}</small>
            {game.inspectable && game.id && onInspect ? (
              <button type="button" className={styles.inspectBtn} onClick={() => onInspect(game.id!)}>
                {t('career.ranked.inspectDeck')}
              </button>
            ) : <span />}
          </li>
        ))}
      </ul>
    </section>
  );
}

function OpponentDeckDialog({ gameId, onClose }: { gameId: string | null; onClose(): void }) {
  const t = useT();
  const deck = useRankedGameDeck(gameId);
  if (!gameId) return null;
  const mainCount = (deck.data?.main ?? []).reduce((sum, c) => sum + (c.amount ?? 0), 0);
  const sbCount = (deck.data?.sideboard ?? []).reduce((sum, c) => sum + (c.amount ?? 0), 0);
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={deck.data
        ? t('career.ranked.deckModalTitle', { opponent: deck.data.opponentName ?? '', deck: deck.data.deckName ?? '' })
        : t('career.ranked.inspectDeck')}
      width="md"
    >
      {deck.isPending && <p className={modes.note}>{t('career.loading')}</p>}
      {deck.isError && <p className={modes.error}>{t('career.failed')}</p>}
      {deck.data && (
        <>
          <p className={modes.small}>
            {deck.data.archetype && <span className={styles.badge}>{t(archetypeKey(deck.data.archetype))}</span>}{' '}
            {deck.data.colors && <Pips colors={deck.data.colors} size="sm" />}{' '}
            {deck.data.tagline}
          </p>
          <div className={styles.deckColumns}>
            <div>
              <h3 className={modes.subhead}>{t('career.ranked.mainboard', { count: mainCount })}</h3>
              <ul className={styles.cardLines}>
                {(deck.data.main ?? []).map((c) => (
                  <li key={c.name}><span className={styles.cardQty}>{c.amount}</span><span>{c.name}</span></li>
                ))}
              </ul>
            </div>
            {sbCount > 0 && (
              <div>
                <h3 className={modes.subhead}>{t('career.ranked.sideboard', { count: sbCount })}</h3>
                <ul className={styles.cardLines}>
                  {(deck.data.sideboard ?? []).map((c) => (
                    <li key={c.name}><span className={styles.cardQty}>{c.amount}</span><span>{c.name}</span></li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </>
      )}
    </Dialog>
  );
}

export function GameResult({ tableId, onInspect, onNextMatch }: {
  tableId: string;
  onInspect?: () => void;
  onNextMatch?: () => void;
}) {
  const t = useT();
  const result = useRankedResult(tableId);
  if (result.isPending) return <p className={modes.note} role="status">{t('career.ranked.counting')}</p>;
  if (!result.data) return null;
  return <ResultCard result={result.data} onInspect={onInspect} onNextMatch={onNextMatch} />;
}

export function ResultCard({ result, onInspect, onNextMatch }: {
  result: LadderResult;
  onInspect?: () => void;
  onNextMatch?: () => void;
}) {
  const t = useT();
  const moved = movement(result);
  const after = rankAt(result.stepAfter ?? 0);
  const reached = result.reached ?? [];
  const podNames = (result.podOpponents ?? []).map((item) => item.name).filter(Boolean).join(', ');
  const placement = result.placement;
  const placementLabel = placement && placement >= 1 && placement <= 4
    ? t(`career.ranked.placement.${placement}` as MessageKey)
    : null;
  return (
    <section className={[styles.result, styles[`result_${result.status ?? 'drew'}`]].join(' ')} role="status" aria-label={t('career.ranked.lastGame')}>
      <Emblem rank={after.rank} size="sm" />
      <div className={styles.cardBody}>
        <h2 className={styles.resultTitle}>
          {placementLabel ?? t(`career.ranked.status.${(['won', 'lost', 'drew'] as const).find((s) => s === result.status) ?? 'drew'}`)}
          {podNames ? ` · ${podNames}` : result.opponent?.name ? ` · ${result.opponent.name}` : ''}
        </h2>
        <PipRow pips={pipRow(result.stepAfter ?? 0, result.stepBefore ?? 0)} label={result.labelAfter ?? ''} />
        <p className={modes.small}>
          {t(`career.ranked.moved.${moved}`, { from: result.labelBefore ?? '', to: result.labelAfter ?? '' })}
          {result.bonus ? ` · ${t('career.ranked.bonus')}` : ''}
        </p>
        {reached.map((rank) => (
          <p key={rank} className={styles.reached}><Trophy size={14} aria-hidden="true" /> {t('career.ranked.reached', { rank: t(rankKey(rank)) })}</p>
        ))}
        {((result.cosmetics ?? []).length > 0 || (result.tokens ?? 0) > 0 || (result.packs ?? 0) > 0 || Object.keys(result.wildcards ?? {}).length > 0) && (
          <p className={modes.small}>
            {[
              (result.cosmetics ?? []).length > 0 ? t('career.ranked.cosmetics', { count: (result.cosmetics ?? []).length }) : null,
              (result.tokens ?? 0) > 0 ? t('career.ranked.tokens', { count: result.tokens ?? 0 }) : null,
              (result.packs ?? 0) > 0 ? t('career.ranked.packs', { count: result.packs ?? 0 }) : null,
              ...Object.entries(result.wildcards ?? {}).filter(([, count]) => count > 0)
                .map(([rarity, count]) => t('career.ranked.wildcard', { count, rarity: t(wildcardKey(rarity)) })),
            ].filter(Boolean).join(' · ')}
          </p>
        )}
        {(onInspect || onNextMatch) && (
          <div className={styles.resultActions}>
            {onNextMatch && (
              <Button variant="decision" size="sm" icon={<Swords size={14} />} onClick={onNextMatch}>
                {t('career.ranked.nextMatch')}
              </Button>
            )}
            {onInspect && result.opponent?.botId && (
              <Button variant="quiet" size="sm" icon={<Eye size={14} />} onClick={onInspect}>
                {t('career.ranked.inspectDeck')}
              </Button>
            )}
          </div>
        )}
      </div>
    </section>
  );
}

function wildcardKey(rarity: string): MessageKey {
  return rarity === 'mythic' ? 'career.rarity.mythic' : rarity === 'rare' ? 'career.rarity.rare' : rarity === 'uncommon' ? 'career.rarity.uncommon' : 'career.rarity.common';
}

const LAST_FORMAT = 'playmat.ranked.format';
const LAST_QUEUE = 'playmat.ranked.queue';

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
